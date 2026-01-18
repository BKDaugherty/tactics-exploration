//! A Module for tracking some basic Enemy behaviors!

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    battle::Enemy,
    battle_phase::{PhaseMessage, PhaseMessageType, PlayerEnemyPhase, UnitPhaseResources},
    combat::{
        AttackIntent,
        skills::{ATTACK_SKILL_ID, Targeting},
    },
    enemy::behaviors::EnemyAiBehavior,
    grid::{
        GridManager, GridManagerResource, GridPosition, GridPositionChangeResult,
        manhattan_distance,
    },
    unit::{
        CombatActionMarker, DIRECTION_VECS, MovementRequest, Unit, UnitActionCompletedMessage,
        UnitExecuteAction, UnitExecuteActionMessage, build_attack_space_options,
        get_valid_moves_for_unit,
    },
};

#[derive(Component)]
pub struct ActiveEnemy {}

#[derive(Resource)]
pub struct EnemyTurnConductorResource(pub EnemyTurnConductor);
pub struct EnemyTurnConductor {
    queue: VecDeque<Entity>,
}

pub fn init_enemy_ai_system(mut commands: Commands) {
    commands.insert_resource(EnemyTurnConductorResource(EnemyTurnConductor {
        queue: VecDeque::default(),
    }));
}

pub fn begin_enemy_phase(
    mut commands: Commands,
    mut message_reader: MessageReader<PhaseMessage>,
    mut conductor: ResMut<EnemyTurnConductorResource>,
    enemy_units: Query<(Entity, &Unit), With<Enemy>>,
) {
    for message in message_reader.read() {
        let PhaseMessageType::PhaseBegin(phase) = message.0;
        if phase == PlayerEnemyPhase::Enemy {
            for (e, unit) in enemy_units.iter() {
                // Clean up any potential stale references to Enemy Behaviors
                commands
                    .entity(e)
                    .remove::<(ActiveEnemy, PlannedEnemyAction, EnemyActionInProgress)>();

                if unit.downed() {
                    continue;
                }

                info!("Adding {:?} to Enemy Turn List", unit.name);
                conductor.0.queue.push_front(e);
            }
        }
    }
}

pub fn select_next_enemy(
    mut commands: Commands,
    mut conductor: ResMut<EnemyTurnConductorResource>,
    enemies: Query<(Entity, &Unit), With<ActiveEnemy>>,
) {
    // There's already an ActiveEnemy!
    if !enemies.is_empty() {
        return;
    }

    let Some(enemy) = conductor.0.queue.pop_front() else {
        info!("No more enemies for the EnemyTurnConductor to select!");
        return;
    };

    info!("{:?} is the new active enemy", enemy);

    // Activate the current enemy
    commands.entity(enemy).insert(ActiveEnemy {});
}

#[derive(Component, Debug)]
pub struct PlannedEnemyAction {
    action_queue: VecDeque<PlannedAction>,
}

#[derive(Clone, Debug)]
pub struct PlannedAction {
    action: UnitExecuteAction,
}

fn find_targets_by_distance(
    enemy_unit: &Unit,
    enemy_pos: GridPosition,
    unit_query_with_position: Query<(Entity, &Unit, &GridPosition)>,
) -> Vec<(Entity, Unit, GridPosition, u32)> {
    let mut possible_targets = Vec::new();
    // Some of this could probably be re-used for other behaviors
    for (e, unit, unit_pos) in unit_query_with_position {
        // We don't want trappers just randomly attacking walls (or maybe we do?)
        // so we use "against_me" here.
        if !enemy_unit.team.against_me(&unit.team) || unit.downed() {
            continue;
        }

        let distance = manhattan_distance(unit_pos, &enemy_pos);
        possible_targets.push((e, unit.clone(), *unit_pos, distance));
    }
    possible_targets
}

/// If there's an enemy in range, target it!
///
/// TODO: Would be nice to specify lifetimes here to not clone
fn target_enemy_in_range(
    grid_manager: &GridManager,
    enemy_unit: &Unit,
    enemy_pos: GridPosition,
    unit_query_with_position: Query<(Entity, &Unit, &GridPosition)>,
) -> Option<(Entity, GridPosition)> {
    let mut target = None;

    let attack_options =
        build_attack_space_options(grid_manager, &Targeting::TargetInRange(1), &enemy_pos);

    for (e, unit, unit_pos) in unit_query_with_position {
        if attack_options.contains(&unit_pos)
            && !unit.downed()
            && enemy_unit.team.against_me(&unit.team)
        {
            target = Some((e, *unit_pos));
            break;
        }
    }

    target
}

pub fn plan_enemy_action(
    grid_manager: Res<GridManagerResource>,
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &Unit,
            &UnitPhaseResources,
            &EnemyAiBehavior,
            &GridPosition,
        ),
        (With<ActiveEnemy>, Without<PlannedEnemyAction>),
    >,
    // Used for obstruction checks among other things
    unit_query: Query<(Entity, &Unit)>,
    // Used for finding a good target for an attack
    unit_query_with_position: Query<(Entity, &Unit, &GridPosition)>,
) {
    // There should only be at most one ActiveEnemy but :shrug:
    for (enemy, enemy_unit, resources, behavior, enemy_pos) in query {
        // Plan the unit's action
        info!("Planning action for {:?}", enemy_unit.name);
        let planned_action = match &behavior.behavior {
            behaviors::Behavior::Pacifist => PlannedEnemyAction {
                action_queue: VecDeque::from([PlannedAction {
                    action: UnitExecuteAction::Wait,
                }]),
            },
            behaviors::Behavior::Wanderer => {
                let valid_moves = get_valid_moves_for_unit(
                    &grid_manager.grid_manager,
                    MovementRequest {
                        origin: *enemy_pos,
                        unit: enemy_unit.clone(),
                        movement_points_available: resources.movement_points_left_in_phase,
                    },
                    unit_query,
                );

                let mut actions = VecDeque::from([PlannedAction {
                    action: UnitExecuteAction::Wait,
                }]);

                if let Some((_, the_move)) = valid_moves.iter().next() {
                    actions.push_front(PlannedAction {
                        action: UnitExecuteAction::Move(the_move.clone()),
                    });
                }

                PlannedEnemyAction {
                    action_queue: actions,
                }
            }
            behaviors::Behavior::Berserker => {
                let mut action_queue = VecDeque::new();
                // Can I attack someone where I am right now?
                let target = target_enemy_in_range(
                    &grid_manager.grid_manager,
                    enemy_unit,
                    *enemy_pos,
                    unit_query_with_position,
                )
                .map(|t| t.0);

                // Move toward the closest one
                match target {
                    Some(t) => {
                        action_queue.push_front(PlannedAction {
                            action: UnitExecuteAction::Attack(AttackIntent {
                                attacker: enemy,
                                defender: t,
                                skill: ATTACK_SKILL_ID,
                            }),
                        });
                    }
                    None => {
                        let mut possible_targets = find_targets_by_distance(
                            enemy_unit,
                            *enemy_pos,
                            unit_query_with_position,
                        );
                        possible_targets
                            .sort_by(|(_, _, _, dist), (_, _, _, dist2)| dist.cmp(dist2));

                        let valid_moves = get_valid_moves_for_unit(
                            &grid_manager.grid_manager,
                            MovementRequest {
                                origin: *enemy_pos,
                                unit: enemy_unit.clone(),
                                movement_points_available: resources.movement_points_left_in_phase,
                            },
                            unit_query,
                        );

                        let close_enough = 1;
                        let mut choice = None;
                        let mut choices = Vec::new();
                        for (
                            possible_target,
                            possible_target_unit,
                            possible_target_pos,
                            dist_from_me,
                        ) in possible_targets
                        {
                            for (pos, valid_move) in &valid_moves {
                                let resulting_dist = manhattan_distance(&pos, &possible_target_pos);
                                if resulting_dist <= close_enough {
                                    let target = target_enemy_in_range(
                                        &grid_manager.grid_manager,
                                        enemy_unit,
                                        *pos,
                                        unit_query_with_position,
                                    )
                                    .map(|t| t.0);

                                    match target {
                                        Some(t) => {
                                            choice = Some((t, valid_move.clone()));
                                        }
                                        None => continue,
                                    }
                                } else {
                                    choices.push((valid_move, resulting_dist));
                                }
                            }
                        }

                        if let Some((t, valid_move)) = choice {
                            action_queue.push_back(PlannedAction {
                                action: UnitExecuteAction::Move(valid_move),
                            });

                            action_queue.push_back(PlannedAction {
                                action: UnitExecuteAction::Attack(AttackIntent {
                                    attacker: enemy,
                                    defender: t,
                                    skill: ATTACK_SKILL_ID,
                                }),
                            });
                        } else {
                            if let Some((valid_move, _)) = choices
                                .into_iter()
                                .min_by(|(_, dist), (_, dist2)| dist.cmp(dist2))
                            {
                                action_queue.push_back(PlannedAction {
                                    action: UnitExecuteAction::Move(valid_move.clone()),
                                });
                            }
                        }
                    }
                }

                // Okay lots of planning, don't forget to end our turn.
                action_queue.push_back(PlannedAction {
                    action: UnitExecuteAction::Wait,
                });
                PlannedEnemyAction { action_queue }
            }

            behaviors::Behavior::Trapper => {
                let mut action_queue = VecDeque::new();
                let valid_moves = get_valid_moves_for_unit(
                    &grid_manager.grid_manager,
                    MovementRequest {
                        origin: *enemy_pos,
                        unit: enemy_unit.clone(),
                        movement_points_available: resources.movement_points_left_in_phase,
                    },
                    unit_query,
                );

                let possible_targets =
                    find_targets_by_distance(enemy_unit, *enemy_pos, unit_query_with_position);

                // Find the closest unit (assume we can get to them for now!)
                if let Some((target_entity, _, target_pos, _)) = possible_targets
                    .into_iter()
                    .min_by(|(_, _, _, dist), (_, _, _, dist2)| dist.cmp(dist2))
                {
                    // TODO: Try to optimize being behind the target by using FacingDirection
                    //
                    // Assumes an Enemy Range of 1
                    for delta in DIRECTION_VECS {
                        let GridPositionChangeResult::Moved(possible_move) = grid_manager
                            .grid_manager
                            .change_position_with_bounds(target_pos, delta)
                        else {
                            continue;
                        };

                        // I can move here and attack the unit. Let's do it!
                        if let Some(valid_move) = valid_moves.get(&possible_move) {
                            action_queue.extend([
                                PlannedAction {
                                    action: UnitExecuteAction::Move(valid_move.clone()),
                                },
                                PlannedAction {
                                    action: UnitExecuteAction::Attack(AttackIntent {
                                        attacker: enemy,
                                        defender: target_entity,
                                        skill: ATTACK_SKILL_ID,
                                    }),
                                },
                            ]);
                            break;
                        }
                    }
                }

                // Wait even if we didn't find a target.
                action_queue.push_back(PlannedAction {
                    action: UnitExecuteAction::Wait,
                });

                PlannedEnemyAction { action_queue }
            }
            otherwise => {
                warn!(
                    "No Enemy AI programmed for {:?} yet! Defaulting to waiting",
                    otherwise
                );
                PlannedEnemyAction {
                    action_queue: VecDeque::from([PlannedAction {
                        action: UnitExecuteAction::Wait,
                    }]),
                }
            }
        };

        commands.entity(enemy).insert(planned_action);
    }
}

#[derive(Component)]
pub struct EnemyActionInProgress {}

/// Compute and perform an action
pub fn execute_enemy_action(
    mut commands: Commands,
    wait_for_no_attacks_ongoing: Query<Entity, With<CombatActionMarker>>,
    mut query: Query<
        (Entity, &Unit, &mut PlannedEnemyAction),
        (With<ActiveEnemy>, Without<EnemyActionInProgress>),
    >,
    mut writer: MessageWriter<UnitExecuteActionMessage>,
) {
    // Don't execute any actions until all AttackExecutions have been drained.
    if !wait_for_no_attacks_ongoing.is_empty() {
        return;
    }

    // There should only be at most one ActiveEnemy but :shrug:
    for (enemy, enemy_unit, mut action) in query.iter_mut() {
        commands.entity(enemy).insert(EnemyActionInProgress {});

        let Some(next_action) = action.action_queue.pop_front() else {
            error!(
                "No action for unit, {:?} but has planned enemy action? this shouldn't ever happen",
                enemy
            );
            continue;
        };

        info!(
            "Taking action: {:?}, for {:?}",
            next_action, enemy_unit.name
        );
        writer.write(UnitExecuteActionMessage {
            entity: enemy,
            action: next_action.action,
        });
    }
}

pub struct EnemyAiBundle {}

pub fn resolve_enemy_action(
    mut commands: Commands,
    mut reader: MessageReader<UnitActionCompletedMessage>,
    query: Query<Entity, (With<ActiveEnemy>, With<EnemyActionInProgress>)>,
) {
    for message in reader.read() {
        // Assume for now that the enemy only has one "action"
        if let Ok(e) = query.get(message.unit) {
            match message.action {
                // If we just finished moving or attacking,
                // remove the action in progress component so the "execute_enemy_action"
                // can run again next frame.
                crate::unit::UnitAction::Move | crate::unit::UnitAction::Attack => {
                    commands.entity(e).remove::<EnemyActionInProgress>();
                }
                // If we waited, cleanup all EnemyPhase components on this enemy.
                // This will allow us to select the next enemy, or end the turn!
                crate::unit::UnitAction::Wait => {
                    commands
                        .entity(e)
                        .remove::<(ActiveEnemy, EnemyActionInProgress, PlannedEnemyAction)>();
                }
            }
        }
    }
}

pub mod behaviors {
    use super::*;

    #[derive(Component)]
    pub struct EnemyAiBehavior {
        pub behavior: Behavior,
    }

    /// Would be interesting to link this to other behaviors.
    /// IE, you might want a Berserker that goes for the Weakest unit, or a Berserker that goes for
    /// the strongest unit
    #[derive(Debug)]
    pub enum Behavior {
        /// The Pacifist simply waits
        Pacifist,
        /// This enemy just moves around 'randomly'
        Wanderer,
        /// This enemy lies in wait for a unit to enter it's "danger zone"
        /// Then this unit moves to attack it!
        Trapper,
        /// This enemy hunts the closest unit not on it's team
        Berserker,
    }
}
