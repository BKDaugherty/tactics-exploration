//! A Module for tracking some basic Enemy behaviors!

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    battle::{Enemy, UnitCommandMessage},
    battle_phase::{PhaseMessage, PhaseMessageType, PlayerEnemyPhase},
    player::Player,
    unit::{Unit, UnitActionCompletedMessage},
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
    mut message_reader: MessageReader<PhaseMessage>,
    mut conductor: ResMut<EnemyTurnConductorResource>,
    enemy_units: Query<(Entity, &Unit), With<Enemy>>,
) {
    for message in message_reader.read() {
        let PhaseMessageType::PhaseBegin(phase) = message.0;
        if phase == PlayerEnemyPhase::Enemy {
            for (e, unit) in enemy_units.iter() {
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

#[derive(Component)]
pub struct EnemyActionInProgress {}

/// Compute and perform an action
pub fn plan_enemy_action(
    mut commands: Commands,
    query: Query<(Entity, &Unit), (With<ActiveEnemy>, Without<EnemyActionInProgress>)>,
    mut writer: MessageWriter<UnitCommandMessage>,
) {
    // There should only be at most one ActiveEnemy but :shrug:
    for (enemy, enemy_unit) in query {
        commands.entity(enemy).insert(EnemyActionInProgress {});

        // TODO: Refactor UnitCommand handling and message so it doesn't rely on Player or PlayerCursorState, and just
        // uses the Unit!
        //
        // Oh and also implement some realy Computations of the best action to take here!
        info!("Taking action for {:?}", enemy_unit.name);
        writer.write(UnitCommandMessage {
            player: Player::One,
            command: crate::battle::UnitCommand::Wait,
            unit: enemy,
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
        if let Some(e) = query.get(message.unit).ok() {
            match message.action {
                // If we just finished moving or attacking,
                // remove the action in progress component so the "plan_enemy_action"
                // can run again next frame.
                crate::unit::UnitAction::Move | crate::unit::UnitAction::Attack => {
                    commands.entity(e).remove::<(EnemyActionInProgress)>();
                }
                // If we waited, cleanup all EnemyPhase components on this enemy.
                // This will allow us to select the next enemy, or end the turn!
                crate::unit::UnitAction::Wait => {
                    commands
                        .entity(e)
                        .remove::<(ActiveEnemy, EnemyActionInProgress)>();
                }
            }
        }
    }
}

mod behaviors {
    use super::*;

    #[derive(Component)]
    pub struct EnemyAiBehavior {
        behavior: Behavior,
    }

    enum Behavior {
        /// The Pacifist simply waits
        Pacfist,
    }
}
