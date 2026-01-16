use bevy::prelude::*;
use bevy::sprite::Anchor;
use leafwing_input_manager::prelude::ActionState;

use crate::animation::animation_db::registered_sprite_ids::{
    TT_UNIT_ANIMATED_SPRITE_ID, TT_WEAPON_ANIMATED_SPRITE_ID,
};
use crate::animation::animation_db::{AnimationDB, AnimationKey, AnimationStartIndexKey};
use crate::animation::{
    AnimationFollower, Direction, FacingDirection, TinytacticsAssets, UnitAnimationKind,
    UnitAnimationPlayer,
};
use crate::assets::sounds::{SoundManagerParam, UiSound};
use crate::battle::{
    BattleEntity, Enemy, UnitSelectionBackMessage, UnitSelectionMessage, UnitUiCommandMessage,
};
use crate::battle_phase::UnitPhaseResources;
use crate::combat::AttackIntent;
use crate::combat::skills::{SkillDBResource, Targeting, UnitSkills};
use crate::enemy::behaviors::EnemyAiBehavior;
use crate::gameplay_effects::ActiveEffects;
use crate::grid::{GridManager, GridMovement, GridPosition, GridVec, manhattan_distance};
use crate::grid_cursor::LockedOn;
use crate::player::{Player, PlayerCursorState, PlayerInputAction, PlayerState};
use crate::unit::overlay::{OverlaysMessage, TileOverlayBundle};
use crate::{enemy, grid, grid_cursor, player};

use std::collections::{HashMap, HashSet, VecDeque};

#[derive(PartialEq, Eq, Debug, Reflect, Clone)]
pub enum ObstacleType {
    /// Obstacles of neutral type block anything
    Neutral,
    /// Obstacles of filter type allow the set of teams they carry through
    Filter(HashSet<Team>),
}

/// The id for a team (do I need this?)
#[derive(PartialEq, Eq, Hash, Debug, Reflect, Clone, Copy)]
pub struct Team(u32);

impl Team {
    pub fn against_me(&self, team: &Team) -> bool {
        team != self && *team != NEUTRAL_TEAM
    }
}

pub const PLAYER_TEAM: Team = Team(1);
pub const ENEMY_TEAM: Team = Team(2);
/// Meant for obstacles? This abstraction is a bit silly atm.
pub const NEUTRAL_TEAM: Team = Team(0);

/// A unit! Units can't share spaces (for now I guess)
///
/// The base controllable entity that can exist and do things on the map
/// Units would have stats, skills, etc?
#[derive(Component, Debug, Reflect, Clone)]
pub struct Unit {
    pub name: String,
    pub stats: Stats,
    pub obstacle: ObstacleType,
    pub team: Team,
}

impl Unit {
    /// Whether or not the unit is at 0 health
    pub fn downed(&self) -> bool {
        self.stats.health == 0
    }

    // Not downed, but less than 30% of max health is "critical"
    pub fn critical_health(&self) -> bool {
        !self.downed() && (self.stats.health as f32 / self.stats.max_health as f32) <= 0.3
    }
}

/// Lowkey, should Magic Power be Neutral, and AttackPower be Physical or
/// something like that? Or is it fun having a Strength / Def?
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, Copy)]
pub enum ElementalType {
    Fire,
}

/// Is it worth storing things like this?
///
/// I imagine I will still want to display in UIs
/// why someones stats are what they are?
#[derive(Debug, Clone, Reflect, PartialEq, Eq)]
pub struct StatAttribute {
    current_value: u32,
    base_value: u32,
}

#[derive(Debug, Reflect, Clone)]
pub struct Stats {
    pub max_health: u32,
    pub strength: u32,
    pub magic_power: u32,
    pub defense: u32,
    /// At the moment, elemental_affinity is both (Str, Def) for the element
    pub elemental_affinities: HashMap<ElementalType, u32>,
    // TODO: Should stats represent the current state?
    pub health: u32,
    pub movement: u32,
}

impl Stats {
    fn new() -> Self {
        Self {
            max_health: 0,
            strength: 0,
            magic_power: 0,
            defense: 0,
            movement: 0,
            health: 0,
            elemental_affinities: HashMap::new(),
        }
    }

    fn with_health(&mut self, health: u32) -> &mut Self {
        self.health = health;
        self.max_health = health;
        self
    }

    fn with_strength(&mut self, strength: u32) -> &mut Self {
        self.strength = strength;
        self
    }

    fn with_elemental_affinity(&mut self, element: ElementalType, affinity: u32) -> &mut Self {
        let _ = self.elemental_affinities.insert(element, affinity);
        self
    }

    fn with_magic_power(&mut self, p: u32) -> &mut Self {
        self.magic_power = p;
        self
    }

    fn with_movement(&mut self, p: u32) -> &mut Self {
        self.movement = p;
        self
    }

    fn with_defense(&mut self, p: u32) -> &mut Self {
        self.defense = p;
        self
    }
}

#[derive(Bundle)]
pub struct UnitBundle {
    pub unit: Unit,
    pub player: crate::player::Player,
    pub grid_position: crate::grid::GridPosition,
    pub sprite: Sprite,
    pub transform: Transform,
    pub facing_direction: FacingDirection,
    pub animation_player: UnitAnimationPlayer,
    pub anchor: Anchor,
    pub phase_resources: UnitPhaseResources,
    pub active_effects: ActiveEffects,
}

#[derive(Debug, Clone, Copy)]
pub enum ObstacleSprite {
    Rock,
    Bush,
}

impl std::fmt::Display for ObstacleSprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObstacleSprite::Rock => write!(f, "Rock"),
            ObstacleSprite::Bush => write!(f, "Bush"),
        }
    }
}

impl ObstacleSprite {
    fn tt_sprite_index(&self) -> usize {
        match self {
            ObstacleSprite::Rock => 64,
            ObstacleSprite::Bush => 48,
        }
    }
}

pub fn spawn_obstacle_unit(
    commands: &mut Commands,
    tt_assets: &TinytacticsAssets,
    grid_position: crate::grid::GridPosition,
    obstacle_sprite_type: ObstacleSprite,
) -> Entity {
    commands
        .spawn((
            crate::grid::init_grid_to_world_transform(&grid_position),
            grid_position,
            Unit {
                stats: Stats::new()
                    .with_health(3)
                    .with_defense(0)
                    .with_movement(0)
                    .to_owned(),
                obstacle: ObstacleType::Neutral,
                team: Team(0),
                name: obstacle_sprite_type.to_string(),
            },
            BattleEntity {},
            Sprite {
                image: tt_assets.tile_spritesheet.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: tt_assets.tile_layout.clone(),
                    index: obstacle_sprite_type.tt_sprite_index(),
                }),
                color: Color::WHITE,
                ..Default::default()
            },
            TINY_TACTICS_ANCHOR,
            UnitPhaseResources::default(),
            ActiveEffects {
                effects: Vec::new(),
            },
        ))
        .id()
}

// TODO: I really need to clean up our Unit creation process
pub fn spawn_enemy(
    commands: &mut Commands,
    unit_name: String,
    tt_assets: &Res<TinytacticsAssets>,
    anim_db: &AnimationDB,
    grid_position: crate::grid::GridPosition,
    spritesheet: Handle<Image>,
    skills: UnitSkills,
    team: Team,
) {
    let transform = crate::grid::init_grid_to_world_transform(&grid_position);
    let direction = Direction::SW;
    let animation_start_index = anim_db
        .get_start_index(&AnimationStartIndexKey {
            facing_direction: Some(direction.animation_direction()),
            key: AnimationKey {
                animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::IdleWalk.into(),
            },
        })
        .expect("Must have animation data");

    let unit_e = commands
        .spawn((
            Unit {
                stats: Stats::new()
                    .with_health(10)
                    .with_defense(1)
                    .with_movement(3)
                    .to_owned(),
                obstacle: ObstacleType::Filter(HashSet::from([team])),
                team,
                name: unit_name,
            },
            grid_position,
            Sprite {
                image: spritesheet,
                texture_atlas: Some(TextureAtlas {
                    layout: tt_assets.tt_unit_layout.clone(),
                    index: (*animation_start_index).into(),
                }),
                color: Color::linear_rgb(1.0, 1.0, 1.0),
                flip_x: direction.should_flip_across_y(),
                ..Default::default()
            },
            transform,
            FacingDirection(crate::animation::Direction::SW),
            UnitAnimationPlayer::new(TT_UNIT_ANIMATED_SPRITE_ID),
            TINY_TACTICS_ANCHOR,
            UnitPhaseResources::default(),
            Enemy {},
            EnemyAiBehavior {
                behavior: enemy::behaviors::Behavior::Trapper,
            },
            BattleEntity {},
            skills,
            ActiveEffects {
                effects: Vec::new(),
            },
        ))
        .id();

    let weapon = commands
        .spawn((
            Sprite {
                image: tt_assets.scepter_spritesheet.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: tt_assets.weapon_layout.clone(),
                    index: 0,
                }),
                flip_x: direction.should_flip_across_y(),
                ..Default::default()
            },
            AnimationFollower {
                leader: unit_e,
                animated_sprite_id: TT_WEAPON_ANIMATED_SPRITE_ID,
            },
            Visibility::Hidden,
            TINY_TACTICS_ANCHOR,
        ))
        .id();

    commands.entity(unit_e).add_child(weapon);
}

pub const TINY_TACTICS_ANCHOR: Anchor = Anchor(Vec2::new(0., -0.25));

/// Temporary function for spawning a test unit
pub fn spawn_unit(
    commands: &mut Commands,
    unit_name: String,
    tt_assets: &Res<TinytacticsAssets>,
    grid_position: crate::grid::GridPosition,
    spritesheet: Handle<Image>,
    texture_atlas: TextureAtlas,
    weapon_spritesheet: Handle<Image>,
    skills: UnitSkills,
    player: crate::player::Player,
    team: Team,
    direction: Direction,
) {
    let transform = crate::grid::init_grid_to_world_transform(&grid_position);
    let unit = commands
        .spawn((
            UnitBundle {
                unit: Unit {
                    stats: Stats::new()
                        .with_health(13)
                        .with_defense(1)
                        .with_magic_power(1)
                        .with_movement(4)
                        .with_strength(2)
                        .to_owned(),
                    obstacle: ObstacleType::Filter(HashSet::from([team])),
                    team,
                    name: unit_name,
                },
                grid_position,
                sprite: Sprite {
                    image: spritesheet,
                    texture_atlas: Some(texture_atlas),
                    color: Color::linear_rgb(1.0, 1.0, 1.0),
                    flip_x: direction.should_flip_across_y(),
                    custom_size: Some(Vec2::splat(32.)),
                    ..Default::default()
                },
                transform,
                player,
                facing_direction: FacingDirection(direction),
                animation_player: UnitAnimationPlayer::new(TT_UNIT_ANIMATED_SPRITE_ID),
                anchor: TINY_TACTICS_ANCHOR,
                phase_resources: UnitPhaseResources::default(),
                active_effects: ActiveEffects {
                    effects: Vec::new(),
                },
            },
            BattleEntity {},
            skills,
        ))
        .id();

    let weapon = commands
        .spawn((
            Sprite {
                image: weapon_spritesheet,
                texture_atlas: Some(TextureAtlas {
                    layout: tt_assets.weapon_layout.clone(),
                    index: 0,
                }),
                flip_x: direction.should_flip_across_y(),
                ..Default::default()
            },
            AnimationFollower {
                leader: unit,
                animated_sprite_id: TT_WEAPON_ANIMATED_SPRITE_ID,
            },
            Visibility::Hidden,
            TINY_TACTICS_ANCHOR,
        ))
        .id();

    commands.entity(unit).add_child(weapon);
}

fn end_move(
    overlay_message_writer: &mut MessageWriter<OverlaysMessage>,
    player: &Player,
    player_state: &mut PlayerState,
) {
    overlay_message_writer.write(OverlaysMessage {
        player: *player,
        action: overlay::OverlaysAction::Despawn,
    });
    // Change Player State back to idle
    player_state.cursor_state = player::PlayerCursorState::Idle;
}

#[derive(Clone, Debug)]
pub struct MovementRequest {
    pub origin: GridPosition,
    pub unit: Unit,
    pub movement_points_available: u32,
}

#[derive(Debug)]
enum UnitMovementSelection {
    Selected(Entity),
    NoPlayerUnitOnTile,
}

fn spawn_overlays(
    commands: &mut Commands,
    tile_overlay_assets: &Res<overlay::TileOverlayAssets>,
    player: Player,
    grid_positions: Vec<GridPosition>,
    grid_manager: &mut GridManager,
    index: usize,
) {
    for grid_pos in grid_positions {
        let e = commands
            .spawn((TileOverlayBundle::new(
                grid_pos,
                tile_overlay_assets.tile_overlay_image_handle.clone(),
                tile_overlay_assets.tile_overlay_atlas_layout_handle.clone(),
                player,
                index,
            ),))
            .id();
        grid_manager.add_entity(e, grid_pos);
    }
}

pub const DIRECTION_VECS: [GridVec; 4] = [
    GridVec { x: 1, y: 0 },
    GridVec { x: 0, y: 1 },
    GridVec { x: -1, y: 0 },
    GridVec { x: 0, y: -1 },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidMove {
    pub target: GridPosition,
    pub path: Vec<GridPosition>,
    movement_used: u32,
}

/// Search for valid moves, exploring the grid until we are out of movement stat using bfs
pub fn get_valid_moves_for_unit(
    grid_manager: &GridManager,
    movement: MovementRequest,
    unit_query: Query<(Entity, &Unit)>,
) -> HashMap<GridPosition, ValidMove> {
    let movement_left = movement.movement_points_available;

    let mut spaces_explored = HashSet::new();
    let mut queue = VecDeque::new();
    let mut valid_moves = HashMap::new();
    queue.push_back((
        movement.origin,
        movement_left as i32,
        vec![movement.origin],
        false,
    ));

    while let Some((to_explore, movement_left, path, is_obstructed)) = queue.pop_front() {
        if !is_obstructed && !spaces_explored.insert(to_explore) {
            continue;
        };

        if to_explore != movement.origin && !is_obstructed {
            valid_moves.insert(
                to_explore,
                ValidMove {
                    target: to_explore,
                    path: path.clone(),
                    movement_used: movement.unit.stats.movement - movement_left as u32,
                },
            );
        }

        let movement_after_moved_onto_tile = movement_left - 1;
        if movement_after_moved_onto_tile < 0 {
            continue;
        }

        for dir in DIRECTION_VECS {
            // Skip running into walls of the Grid
            let grid::GridPositionChangeResult::Moved(grid_pos) =
                grid_manager.change_position_with_bounds(to_explore, dir)
            else {
                continue;
            };

            // Can the unit move to `grid_pos`?
            // Assumes that there is only one unit on a tile.
            //
            // TODO: Could cache this if this query is expensivo
            let unit_on_target = grid_manager
                .get_by_position(&grid_pos)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|e| unit_query.get(*e).ok())
                .next()
                .flatten();

            if let Some((_, unit)) = unit_on_target {
                match &unit.obstacle {
                    // Can't move here, or through here.
                    ObstacleType::Neutral => {
                        continue;
                    }
                    // Can move through here, but can't move here.
                    ObstacleType::Filter(hash_set) => {
                        if !hash_set.contains(&movement.unit.team) && !unit.downed() {
                            continue;
                        } else {
                            let mut new_path = path.clone();
                            new_path.push(grid_pos);
                            queue.push_back((
                                grid_pos,
                                movement_after_moved_onto_tile,
                                new_path,
                                true,
                            ))
                        }
                    }
                }
            } else {
                let mut new_path = path.clone();
                new_path.push(grid_pos);
                queue.push_back((grid_pos, movement_after_moved_onto_tile, new_path, false))
            };
        }
    }

    valid_moves
}

// TODO: This abstraction kind of sucks. It's really hard to get what I want out of it
fn get_singleton_component_on_grid_by_player<F, T>(
    cursor_grid_pos: &GridPosition,
    grid_manager: &GridManager,
    query: F,
) -> Option<(Entity, Player, T)>
where
    F: Fn(&Entity) -> Option<(Entity, Player, T)>,
{
    let entities_at_pos = grid_manager
        .get_by_position(cursor_grid_pos)
        .cloned()
        .unwrap_or_default();
    entities_at_pos.iter().map(query).next().flatten()
}

fn select_unit_for_movement<F>(
    cursor_grid_pos: &GridPosition,
    grid_manager: &GridManager,
    unit_player_from_entity_query: F,
) -> UnitMovementSelection
where
    F: Fn(&Entity) -> Option<(Entity, Player, Unit)>,
{
    let unit = get_singleton_component_on_grid_by_player(
        cursor_grid_pos,
        grid_manager,
        unit_player_from_entity_query,
    );
    if let Some((entity, _, _)) = unit {
        return UnitMovementSelection::Selected(entity);
    }
    UnitMovementSelection::NoPlayerUnitOnTile
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackOption {
    target: Entity,
    grid_position: GridPosition,
}

// Ideally runs directly after the UnitUiCommand was emitted
pub fn unlock_cursor_after_unit_ui_command(
    mut commands: Commands,
    mut unit_command_message: MessageReader<UnitUiCommandMessage>,
    cursor_query: Query<(Entity, &Player), With<LockedOn>>,
) {
    for message in unit_command_message.read() {
        for (e, cursor_player) in cursor_query.iter() {
            if *cursor_player != message.player {
                continue;
            }

            commands.entity(e).remove::<LockedOn>();
        }
    }
}

#[derive(Message)]
pub struct UnitExecuteActionMessage {
    pub entity: Entity,
    pub action: UnitExecuteAction,
}

#[derive(Clone, Debug)]
pub enum UnitExecuteAction {
    Move(ValidMove),
    Attack(AttackIntent),
    Wait,
}

/// Marker component for systems that want to wait until combat is over.
#[derive(Reflect, Debug, Component)]
pub struct CombatActionMarker;

pub fn execute_unit_actions(
    mut commands: Commands,
    mut reader: MessageReader<UnitExecuteActionMessage>,
    mut command_completed_writer: MessageWriter<UnitActionCompletedMessage>,
    // I don't love that I do this here since I do the other things out of band, but I don't
    // really need to wait for anything else to wait so :shrug:
    mut unit_phase_resources: Query<&mut UnitPhaseResources>,
) {
    for message in reader.read() {
        match &message.action {
            UnitExecuteAction::Move(valid_move) => {
                commands
                    .entity(message.entity)
                    .insert(GridMovement::new(valid_move.path.clone(), 0.4));
            }
            UnitExecuteAction::Attack(attack_intent) => {
                commands.spawn((CombatActionMarker, attack_intent.clone()));
            }
            UnitExecuteAction::Wait => {
                if let Ok(mut resources) = unit_phase_resources.get_mut(message.entity) {
                    resources.waited = true;
                }

                command_completed_writer.write(UnitActionCompletedMessage {
                    unit: message.entity,
                    action: UnitAction::Wait,
                });
            }
        }
    }
}

/// Builds a bounds square then filters for hamming distance
pub fn radius_range_at_position(
    grid_manager: &GridManager,
    origin: &GridPosition,
    range: u32,
) -> Vec<GridPosition> {
    let bounds = DIRECTION_VECS.map(|t| {
        grid_manager
            .change_position_with_bounds(*origin, t.scale(range as i32))
            .position()
    });

    // Note that we panic here if there are no valid positions, but there should always be valid positions
    let (x_min, x_max, y_min, y_max) = (
        bounds
            .iter()
            .map(|t| t.x)
            .min()
            .expect("No minimum x value"),
        bounds
            .iter()
            .map(|t| t.x)
            .max()
            .expect("No maximum x value"),
        bounds
            .iter()
            .map(|t| t.y)
            .min()
            .expect("No minimum y value"),
        bounds
            .iter()
            .map(|t| t.y)
            .max()
            .expect("No maximum y value"),
    );

    let mut targets = Vec::new();

    for x in x_min..=x_max {
        for y in y_min..=y_max {
            let target_pos = GridPosition { x, y };

            if manhattan_distance(origin, &target_pos) <= range {
                targets.push(target_pos);
            }
        }
    }

    targets
}

pub fn build_attack_space_options(
    grid_manager: &GridManager,
    targeting: &Targeting,
    origin: &GridPosition,
) -> Vec<GridPosition> {
    match targeting {
        Targeting::TargetInRange(range) => radius_range_at_position(grid_manager, origin, *range),
    }
}

// TODO: Don't make this dependent on PlayerGameStates
// We need to drive the interaction between the cursor
// in a better way. (Which will enable us to use this for AIs too!)
//
// PlayerGameStates probably could just be an Event we pass.
pub fn handle_unit_ui_command(
    grid_manager_res: Res<grid::GridManagerResource>,
    skill_db: Res<SkillDBResource>,
    mut player_state: ResMut<player::PlayerGameStates>,
    mut unit_command_message: MessageReader<UnitUiCommandMessage>,
    mut overlay_message_writer: MessageWriter<OverlaysMessage>,
    mut controlled_unit_query: Query<(Entity, &Unit, &mut UnitPhaseResources, &GridPosition)>,
    unit_query: Query<(Entity, &Unit)>,
    mut execute_action_writer: MessageWriter<UnitExecuteActionMessage>,
) {
    for message in unit_command_message.read() {
        let Some(player_state) = player_state.player_state.get_mut(&message.player) else {
            log::error!("No player state found for player {:?}", message.player);
            continue;
        };

        let Some((unit_entity, unit, unit_resources, position)) =
            controlled_unit_query.get_mut(message.unit).ok()
        else {
            log::error!("No Unit found for Command message: {:?}", message);
            continue;
        };

        match message.command {
            crate::battle::UnitCommand::Cancel => {
                player_state.cursor_state = player::PlayerCursorState::Idle;
            }
            crate::battle::UnitCommand::Move => {
                let req = MovementRequest {
                    origin: *position,
                    unit: unit.clone(),
                    movement_points_available: unit_resources.movement_points_left_in_phase,
                };
                let valid_moves =
                    get_valid_moves_for_unit(&grid_manager_res.grid_manager, req, unit_query);

                // Change Player State to moving the unit (Only do this when )
                player_state.cursor_state = player::PlayerCursorState::MovingUnit(
                    unit_entity,
                    *position,
                    valid_moves.clone(),
                );

                overlay_message_writer.write(OverlaysMessage {
                    player: message.player,
                    action: overlay::OverlaysAction::Spawn {
                        spawn_type: overlay::OverlaysType::Move,
                        positions: valid_moves.keys().cloned().collect(),
                    },
                });
            }
            crate::battle::UnitCommand::UseSkill(skill_id) => {
                let skill = skill_db.skill_db.get_skill(&skill_id);

                let mut options_for_attack = Vec::new();

                // TODO: It'd be nice to block this before this point
                // in le UI
                if unit_resources.action_points_left_in_phase < skill.cost.ap.into() {
                    warn!("Unit is attempting to attack with no AP!");
                    continue;
                }

                let target_options = build_attack_space_options(
                    &grid_manager_res.grid_manager,
                    &skill.targeting,
                    position,
                );

                // Assume all units have the same attack range for now
                for possible_attack_pos in &target_options {
                    // Is there a unit that can be attacked there?
                    //
                    // TODO: Add some form of "targeting options" or something for
                    // deciding if you can cast this on an enemy or player or self or not
                    if let Some((target_entity, _)) = grid_manager_res
                        .grid_manager
                        .get_by_position(possible_attack_pos)
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|e| unit_query.get(*e).ok())
                        .next()
                    {
                        options_for_attack.push(AttackOption {
                            target: target_entity,
                            grid_position: *possible_attack_pos,
                        });
                    }
                }

                let options_map: HashMap<GridPosition, AttackOption> = options_for_attack
                    .iter()
                    .cloned()
                    .map(|t| (t.grid_position, t))
                    .collect();

                // I hate this abstraction lol
                player_state.cursor_state = player::PlayerCursorState::LookingForTargetWithAttack(
                    unit_entity,
                    options_map,
                    skill_id,
                );

                overlay_message_writer.write(OverlaysMessage {
                    player: message.player,
                    action: overlay::OverlaysAction::Spawn {
                        spawn_type: overlay::OverlaysType::Attack,
                        positions: target_options,
                    },
                });
            }
            crate::battle::UnitCommand::Wait => {
                execute_action_writer.write(UnitExecuteActionMessage {
                    entity: message.unit,
                    action: UnitExecuteAction::Wait,
                });

                player_state.cursor_state = player::PlayerCursorState::Idle;
            }
            crate::battle::UnitCommand::Attack => {
                error!("Attacks are deprecated, don't ya know?");
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_unit_cursor_actions(
    mut commands: Commands,
    grid_manager_res: Res<grid::GridManagerResource>,
    mut player_state: ResMut<player::PlayerGameStates>,
    player_query: Query<(&Player, &ActionState<PlayerInputAction>)>,
    mut cursor_query: Query<
        (Entity, &Player, &mut grid::GridPosition),
        (With<grid_cursor::Cursor>, Without<LockedOn>),
    >,
    player_unit_query: Query<(Entity, &player::Player, &Unit)>,
    mut overlay_message_writer: MessageWriter<OverlaysMessage>,
    mut unit_selection_message: MessageWriter<UnitSelectionMessage>,
    mut unit_selection_back_message: MessageWriter<UnitSelectionBackMessage>,
    mut execute_action_writer: MessageWriter<UnitExecuteActionMessage>,
    sounds: SoundManagerParam,
) {
    for (player, action_state) in player_query.iter() {
        for (cursor_entity, cursor_player, mut cursor_grid_pos) in cursor_query.iter_mut() {
            if player != cursor_player {
                continue;
            }

            let Some(player_state) = player_state.player_state.get_mut(player) else {
                log::error!("No player state found for player {:?}", player);
                continue;
            };

            // If the cursor is idle, and there's a unit at the cursor position,
            // generate overlays using that unit's movement
            if player_state.cursor_state == player::PlayerCursorState::Idle
                && action_state.just_pressed(&PlayerInputAction::Select)
            {
                let selection = select_unit_for_movement(
                    &cursor_grid_pos,
                    &grid_manager_res.grid_manager,
                    |entity| {
                        // Get the first Unit owned by this player, and then clone the values to satisfy lifetimes.
                        let queried = player_unit_query
                            .get(*entity)
                            .ok()
                            .filter(|(_, p, _)| **p == *player);
                        queried.map(|(a, b, c)| (a, *b, c.clone()))
                    },
                );

                match selection {
                    UnitMovementSelection::Selected(entity) => {
                        unit_selection_message.write(UnitSelectionMessage {
                            entity,
                            player: *player,
                        });

                        commands.entity(cursor_entity).insert(LockedOn {});
                        sounds.play_sound(&mut commands, UiSound::OpenMenu);
                    }
                    UnitMovementSelection::NoPlayerUnitOnTile => {
                        sounds.play_sound(&mut commands, UiSound::Error);
                        warn!("Selected tile with no player unit");
                    }
                }
            }
            // If we're moving a unit, and we press select again, attempt to move the unit to that position
            else if let player::PlayerCursorState::MovingUnit(
                unit_entity,
                original_position,
                mut valid_moves,
            ) = player_state.cursor_state.clone()
            {
                if action_state.just_pressed(&PlayerInputAction::Select) {
                    // TODO: What to do if this changes between start and end of movement?
                    let Some(valid_move) = valid_moves.remove(&cursor_grid_pos) else {
                        log::warn!("Attempting to move to invalid position");
                        sounds.play_sound(&mut commands, UiSound::Error);
                        continue;
                    };

                    // TODO: Remove this bad check for watching if a player moves onto one of our valid moves
                    let unit_at_position = get_singleton_component_on_grid_by_player(
                        &cursor_grid_pos,
                        &grid_manager_res.grid_manager,
                        |entity| {
                            player_unit_query
                                .get(*entity)
                                .ok()
                                .map(|(a, b, c)| (a, *b, c))
                        },
                    );

                    if unit_at_position.is_some() {
                        log::warn!(
                            "Cannot move unit to position {:?} because it is occupied",
                            cursor_grid_pos
                        );
                        sounds.play_sound(&mut commands, UiSound::Error);
                        continue;
                    }

                    sounds.play_sound(&mut commands, UiSound::Select);
                    execute_action_writer.write(UnitExecuteActionMessage {
                        entity: unit_entity,
                        action: UnitExecuteAction::Move(valid_move),
                    });

                    end_move(&mut overlay_message_writer, player, player_state);
                } else if action_state.just_pressed(&PlayerInputAction::Deselect) {
                    end_move(&mut overlay_message_writer, player, player_state);

                    // Snap the cursor position back to the origin, and re-open the menu
                    *cursor_grid_pos = original_position;

                    unit_selection_back_message.write(UnitSelectionBackMessage { player: *player });

                    commands.entity(cursor_entity).insert(LockedOn {});

                    sounds.play_sound(&mut commands, UiSound::Cancel);
                }
            } else if let PlayerCursorState::LookingForTargetWithAttack(
                unit_entity,
                mut valid_attack_moves,
                skill_id,
            ) = player_state.cursor_state.clone()
            {
                if action_state.just_pressed(&PlayerInputAction::Select) {
                    // TODO: This selection logic is the same. I wonder if I could just have the cursor here handle selection
                    // based on state, and then have some other system take over?
                    let Some(valid_move) = valid_attack_moves.remove(&cursor_grid_pos) else {
                        log::warn!("Attempting to attack an invalid position");
                        sounds.play_sound(&mut commands, UiSound::Error);
                        continue;
                    };

                    execute_action_writer.write(UnitExecuteActionMessage {
                        entity: unit_entity,
                        action: UnitExecuteAction::Attack(AttackIntent {
                            attacker: unit_entity,
                            defender: valid_move.target,
                            skill: skill_id,
                        }),
                    });

                    sounds.play_sound(&mut commands, UiSound::Select);

                    end_move(&mut overlay_message_writer, player, player_state);
                } else if action_state.just_pressed(&PlayerInputAction::Deselect) {
                    end_move(&mut overlay_message_writer, player, player_state);

                    let Some(unit_pos) = grid_manager_res.grid_manager.get_by_id(&unit_entity)
                    else {
                        log::warn!("Oh no somehow our unit disappeared!");
                        continue;
                    };

                    // Snap the cursor position back to the origin, and re-open the menu
                    *cursor_grid_pos = unit_pos;

                    unit_selection_back_message.write(UnitSelectionBackMessage { player: *player });

                    commands.entity(cursor_entity).insert(LockedOn {});

                    sounds.play_sound(&mut commands, UiSound::Cancel);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum UnitAction {
    Move,
    Attack,
    Wait,
}

#[derive(Message)]
pub struct UnitActionCompletedMessage {
    pub unit: Entity,
    pub action: UnitAction,
}

pub mod jobs {
    use crate::{
        assets::sprite_db::{SpriteId, TinyTacticsSprites},
        combat::skills::{SkillCategoryId, SkillId},
    };

    use super::*;

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Reflect)]
    pub enum UnitJob {
        Knight,
        Mage,
        Archer,
        Mercenary,
    }

    impl UnitJob {
        pub fn name(&self) -> String {
            match self {
                UnitJob::Knight => "Knight".to_string(),
                UnitJob::Mage => "Mage".to_string(),
                UnitJob::Archer => "Archer".to_string(),
                UnitJob::Mercenary => "Mercenary".to_string(),
            }
        }

        pub fn description(&self) -> String {
            match self {
                UnitJob::Knight => {
                    "Charge your enemies and hold them in place to protect your allies".to_string()
                }
                UnitJob::Mage => {
                    "Channel primal energy to support your allies and devastate your foes"
                        .to_string()
                }
                UnitJob::Archer => {
                    "Use your bow to inflict pain and effects upon your enemies from range"
                        .to_string()
                }
                UnitJob::Mercenary => {
                    "Hit your enemies hard. Don't worry about \"playing it safe\"".to_string()
                }
            }
        }

        /// I'm not stoked on this function long term, but nice for the
        /// demo. Job probably shouldn't determine base sprite.
        pub fn base_sprite_id(&self) -> SpriteId {
            match self {
                UnitJob::Knight => TinyTacticsSprites::Cleric.into(),
                UnitJob::Mage => TinyTacticsSprites::Mage.into(),
                UnitJob::Archer => TinyTacticsSprites::Fighter.into(),
                UnitJob::Mercenary => TinyTacticsSprites::Fighter.into(),
            }
        }

        /// I'm also not stoked on this long term, but for the demo we aren't doing
        /// progressions, so Jobs need to determine skills!
        pub fn base_unit_skills(&self) -> UnitSkills {
            match self {
                UnitJob::Knight => UnitSkills {
                    learned_skills: HashSet::new(),
                    equipped_skill_categories: Vec::from(&[SkillCategoryId(4)]),
                },
                UnitJob::Mage => UnitSkills {
                    learned_skills: HashSet::from([SkillId(2), SkillId(8)]),
                    equipped_skill_categories: Vec::from(&[SkillCategoryId(1)]),
                },
                UnitJob::Archer => UnitSkills {
                    learned_skills: HashSet::from([SkillId(6), SkillId(5)]),
                    equipped_skill_categories: Vec::from(&[SkillCategoryId(5)]),
                },
                UnitJob::Mercenary => UnitSkills {
                    learned_skills: HashSet::from([SkillId(3)]),
                    equipped_skill_categories: Vec::from(&[SkillCategoryId(6)]),
                },
            }
        }

        pub fn demo_sprite_id(&self) -> SpriteId {
            match self {
                UnitJob::Knight => SpriteId(11),
                UnitJob::Mage => SpriteId(8),
                UnitJob::Archer => SpriteId(9),
                UnitJob::Mercenary => SpriteId(10),
            }
        }
    }
}

pub mod overlay {

    use bevy::image::ImageSampler;

    use crate::grid::init_grid_to_world_transform;

    use super::*;
    #[derive(Component)]
    pub struct TileOverlay {}

    #[derive(Bundle)]
    pub struct TileOverlayBundle {
        grid_position: grid::GridPosition,
        sprite: Sprite,
        transform: Transform,
        tile_overlay: TileOverlay,
        player: Player,
    }

    impl TileOverlayBundle {
        pub fn new(
            grid_position: grid::GridPosition,
            spritesheet: Handle<Image>,
            atlas_layout_handle: Handle<TextureAtlasLayout>,
            player: Player,
            spritesheet_index: usize,
        ) -> Self {
            let mut initial_transform = init_grid_to_world_transform(&grid_position);
            initial_transform.translation.z -= 50.;
            Self {
                grid_position,
                sprite: Sprite {
                    image: spritesheet,
                    texture_atlas: Some(TextureAtlas {
                        layout: atlas_layout_handle,
                        index: spritesheet_index,
                    }),
                    custom_size: None,
                    // TODO: Replace this with just a single image that's White and then use Color to
                    // change the color?
                    color: Color::linear_rgba(1.0, 1.0, 1.0, 0.7),
                    ..Default::default()
                },
                transform: initial_transform,
                tile_overlay: TileOverlay {},
                player,
            }
        }
    }

    #[derive(Resource, Default)]
    pub struct TileOverlayAssets {
        pub tile_overlay_image_handle: Handle<Image>,
        pub cursor_image: Handle<Image>,
        pub tile_overlay_atlas_layout_handle: Handle<TextureAtlasLayout>,
    }

    // This system reads all AssetEvents for the Image type and attempts to set the ImageSampler values to nearest to stop some texture bleeding
    pub fn on_asset_event(
        mut events: MessageReader<AssetEvent<Image>>,
        asset_handles: Res<TileOverlayAssets>,
        mut images: ResMut<Assets<Image>>,
    ) {
        for event in events.read() {
            // You can check the type of event and the specific handle
            if let AssetEvent::LoadedWithDependencies { id } = event
                && *id == asset_handles.tile_overlay_image_handle.id()
            {
                info!("Our specific image asset and its dependencies are loaded!");
                if let Some(image) = images.get_mut(*id) {
                    image.sampler = ImageSampler::nearest();
                }
            }
        }
    }

    // New event for overlay spawning
    #[derive(Message)]
    pub struct OverlaysMessage {
        pub player: Player,
        pub action: OverlaysAction,
    }

    #[derive(Debug)]
    pub enum OverlaysType {
        Interact,
        Move,
        Attack,
    }

    #[derive(Debug)]
    pub enum OverlaysAction {
        Spawn {
            spawn_type: OverlaysType,
            positions: Vec<GridPosition>,
        },
        Despawn,
    }

    /// Handle an OverlaysAction for spawning and despawning overlays
    pub fn handle_overlays_events_system(
        mut commands: Commands,
        mut grid_manager_res: ResMut<grid::GridManagerResource>,
        tile_overlay_assets: Res<overlay::TileOverlayAssets>,
        overlay_query: Query<(Entity, &Player), With<TileOverlay>>,
        mut events: MessageReader<OverlaysMessage>,
    ) {
        for event in events.read() {
            if let OverlaysAction::Spawn {
                spawn_type,
                positions,
            } = &event.action
            {
                // TODO: Stop using indices for iso_color and replace with white image
                // that can be overriden via Color of sprite.
                let index = match spawn_type {
                    OverlaysType::Interact => 2,
                    OverlaysType::Move => 1,
                    OverlaysType::Attack => 3,
                };
                spawn_overlays(
                    &mut commands,
                    &tile_overlay_assets,
                    event.player,
                    positions.clone(),
                    &mut grid_manager_res.grid_manager,
                    index,
                );
            } else if let OverlaysAction::Despawn = &event.action {
                for (entity, overlay_player) in overlay_query.iter() {
                    if overlay_player == &event.player {
                        grid_manager_res.grid_manager.remove_entity(&entity);
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use crate::{
        battle::{UnitCommand, UnitSelectionMessage, UnitUiCommandMessage},
        battle_phase::{
            PhaseMessage, UnitPhaseResources, check_should_advance_phase, init_phase_system,
            prepare_for_phase,
        },
        grid::{
            self, GridManager, GridManagerResource, GridMovement, GridPosition,
            sync_grid_positions_to_manager,
        },
        grid_cursor,
        player::{self, Player, PlayerGameStates, PlayerInputAction, PlayerState},
        unit::{
            PLAYER_TEAM, Stats, Unit, UnitActionCompletedMessage, UnitExecuteActionMessage,
            execute_unit_actions, handle_unit_cursor_actions, handle_unit_ui_command,
            overlay::OverlaysMessage, unlock_cursor_after_unit_ui_command,
        },
    };
    use bevy::{
        app::{App, Update},
        ecs::{schedule::IntoScheduleConfigs, system::RunSystemOnce},
        time::Time,
        transform::components::Transform,
    };
    use leafwing_input_manager::{plugin::InputManagerPlugin, prelude::ActionState};

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn create_test_app() -> App {
        let mut app = App::new();
        app.add_message::<OverlaysMessage>();
        app.add_message::<UnitSelectionMessage>();
        app.add_message::<UnitUiCommandMessage>();
        app.add_message::<UnitExecuteActionMessage>();
        app.add_message::<UnitActionCompletedMessage>();
        app.add_message::<PhaseMessage>();
        app.insert_resource(GridManagerResource {
            grid_manager: GridManager::new(6, 6),
        });
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PlayerGameStates {
            player_state: HashMap::from([(Player::PlayerId(1), PlayerState::default())]),
        });
        app.add_plugins(InputManagerPlugin::<PlayerInputAction>::default());
        app
    }

    #[test]
    fn test_movement_system() -> anyhow::Result<()> {
        init_logger();
        let mut app = create_test_app();

        let player = Player::PlayerId(1);

        // Spawn player to handle movement events
        let player_entity = app
            .world_mut()
            .spawn(player::PlayerBundle::new(player))
            .id();
        let unit_entity = app
            .world_mut()
            .spawn((
                // TODO: Make constructor
                Unit {
                    stats: Stats::new().with_movement(2).with_health(5).to_owned(),
                    team: PLAYER_TEAM,
                    obstacle: crate::unit::ObstacleType::Filter(HashSet::from([PLAYER_TEAM])),
                    name: "Bob".to_string(),
                },
                player,
                GridPosition { x: 2, y: 2 },
                Transform::default(),
                UnitPhaseResources::default(),
            ))
            .id();

        // Spawn a cursor at (2, 2) for Player::One
        let cursor_entity = app
            .world_mut()
            .spawn((grid_cursor::Cursor {}, player, GridPosition { x: 2, y: 2 }))
            .id();

        // Setup the phase system
        app.world_mut()
            .run_system_once(init_phase_system)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        app.world_mut()
            .run_system_once(check_should_advance_phase::<Player>)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;
        app.world_mut()
            .run_system_once(prepare_for_phase::<Player>)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        app.world_mut()
            .run_system_once(sync_grid_positions_to_manager)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        {
            let mut action_state = app
                .world_mut()
                .get_mut::<ActionState<PlayerInputAction>>(player_entity)
                .unwrap();
            action_state.press(&PlayerInputAction::Select);
        }

        // Run the movement system (simulate one frame)
        app.world_mut()
            .run_system_once(handle_unit_cursor_actions)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        // Handle Unit Movement should incur a message to the UI to say that the given unit was selected.
        // It then expects to be told what the player wants to do with the unit. Let's assume they want to
        // move.
        app.world_mut().write_message(UnitUiCommandMessage {
            unit: unit_entity,
            player,
            command: UnitCommand::Move,
        });

        app.world_mut()
            .run_system_once(handle_unit_ui_command)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        app.world_mut()
            .run_system_once(unlock_cursor_after_unit_ui_command)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        // Let's also simulate a move of the cursor to a valid destination, and
        // press Select.
        app.world_mut()
            .get_mut::<GridPosition>(cursor_entity)
            .unwrap()
            .x = 3;

        {
            let mut action_state = app
                .world_mut()
                .get_mut::<ActionState<PlayerInputAction>>(player_entity)
                .unwrap();
            action_state.press(&PlayerInputAction::Select);
        }

        // UnitMovement should spawn some GridMovement, let's let that resolve and validate that our entity is moved to the correct space.
        app.world_mut()
            .run_system_once(handle_unit_cursor_actions)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        app.world_mut()
            .run_system_once(execute_unit_actions)
            .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        assert!(app.world().get::<GridMovement>(unit_entity).is_some());

        // Infinite loops if GridMovement isn't working so nice
        while app.world().get::<GridMovement>(unit_entity).is_some() {
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(std::time::Duration::from_secs_f32(0.1)); // Advance by 0.1s per step
            }
            log::debug!("Running sync_grid_movement to transform");
            app.world_mut()
                .run_system_once(grid::resolve_grid_movement)
                .map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;
        }

        // Assert the unit is at the final position
        let final_pos = app.world().get::<GridPosition>(unit_entity).unwrap();
        assert_eq!(*final_pos, GridPosition { x: 3, y: 2 });

        // Assert GridMovement component is removed
        assert!(app.world().get::<GridMovement>(unit_entity).is_none());

        Ok(())
    }
}
