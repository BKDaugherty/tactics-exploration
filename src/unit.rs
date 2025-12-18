use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::grid::{GridManager, GridMovement, GridPosition, GridVec};
use crate::unit::overlay::{OverlaysMessage, TileOverlay, TileOverlayBundle};
use crate::{grid, grid_cursor, player};
use crate::player::{Player, PlayerInputAction, PlayerState};

use std::collections::{BTreeSet, HashSet, VecDeque};

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

pub const PLAYER_TEAM: Team = Team(1);
pub const ENEMY_TEAM: Team = Team(2);

/// A unit! Units can't share spaces (for now I guess)
/// 
/// The base controllable entity that can exist and do things on the map
/// Units would have stats, skills, etc?
#[derive(Component, Debug, Reflect, Clone)]
pub struct Unit {
    pub stats: Stats,
    pub obstacle: ObstacleType,
    pub team: Team,
    // effect_modifiers: ()
    // equipment?
}

#[derive(Debug, Reflect, Clone)]
pub struct Stats {
    pub max_health: u32,
    pub strength: u32,
    pub health: u32,
    pub movement: u32,
}

#[derive(Bundle)]
pub struct UnitBundle {
    pub unit: Unit,
    pub player: crate::player::Player,
    pub grid_position: crate::grid::GridPosition,
    pub sprite: Sprite,
    pub transform: Transform,
}

pub fn spawn_obstacle_unit(
    commands: &mut Commands,
    grid_position: crate::grid::GridPosition,
) {
    commands.spawn(
        (
            grid_position,
            Unit {
                stats: Stats { max_health: 0, strength: 0, health: 0, movement: 0 },
                obstacle: ObstacleType::Neutral,
                team: Team(0),
            },

        )
    );
}

/// Temporary function for spawning a test unit
pub fn spawn_unit(
    commands: &mut Commands,
    grid_position: crate::grid::GridPosition,
    spritesheet: Handle<Image>,
    player: crate::player::Player,
    team: Team,
) {
    let transform = crate::grid::init_grid_to_world_transform(&grid_position);
    commands.spawn((
        UnitBundle {
            unit: Unit {
                stats: Stats {
                    max_health: 10,
                    health: 10,
                    strength: 5,
                    movement: 2,
                },
                obstacle: ObstacleType::Filter(HashSet::from([team])),
                team,
            },
            grid_position,
            sprite: Sprite {
                image: spritesheet,
                color: Color::linear_rgb(1.0, 1.0, 1.0),
                ..Default::default()
            },
            transform,
            player,
        },
    ));
}

fn end_move(
    overlay_message_writer: &mut MessageWriter<OverlaysMessage>,
    player: &Player,
    player_state: &mut PlayerState,
) {
    
    overlay_message_writer.write(OverlaysMessage { player: *player, action: overlay::OverlaysAction::Despawn });
    // Change Player State back to idle
    player_state.cursor_state = player::PlayerCursorState::Idle;
}


#[derive(Debug)]
pub struct Movement {
    origin: GridPosition,
    unit: Unit,
}

#[derive(Debug)]
enum UnitMovementSelection {
    Selected(Entity, Movement),
    NoPlayerUnitOnTile,
}

fn spawn_overlays(
    commands: &mut Commands,
    tile_overlay_assets: &Res<overlay::TileOverlayAssets>,
    player: Player,
    grid_positions: Vec<GridPosition>,
    grid_manager: &mut GridManager,
) {
    for grid_pos in grid_positions {
        let e = commands.spawn((
            TileOverlayBundle::new(grid_pos,
                tile_overlay_assets.tile_overlay_image_handle.clone(),
                tile_overlay_assets.tile_overlay_atlas_layout_handle.clone(),
                player
            ),
        )).id();
        grid_manager.add_entity(e, grid_pos);
    }
}

pub const DIRECTION_VECS: [GridVec; 4] = [
    GridVec {x: 1, y: 0},
    GridVec {x: 0, y: 1},
    GridVec {x: -1, y: 0},
    GridVec {x: 0, y: -1}
];

/// Search for valid moves, exploring the grid until we are out of movement stat using bfs
fn get_valid_moves_for_unit(
    grid_manager: &GridManager,
    movement: Movement,
    unit_query: Query<(Entity, &Unit)>
)-> Vec<GridPosition> {
    let movement_left = movement.unit.stats.movement;

    let mut spaces_explored = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((movement.origin, movement_left as i32, false));

    while let Some((to_explore, movement_left, is_obstructed)) = queue.pop_front() {
        if !is_obstructed && !spaces_explored.insert(to_explore) {
            continue
        };

        let movement_after_moved_onto_tile = movement_left  - 1; 
        if movement_after_moved_onto_tile < 0 {
            continue;
        }

        for dir in DIRECTION_VECS {
            // Skip running into walls of the Grid
            let grid::GridPositionChangeResult::Moved(grid_pos) = grid_manager.change_position_with_bounds(to_explore, dir) else {
                continue;
            };

            // Can the unit move to `grid_pos`?
            // Assumes that there is only one unit on a tile.
            // 
            // TODO: Could cache this if this query is expensivo
            let obstacle_on_target = grid_manager.get_by_position(&grid_pos).cloned().unwrap_or_default().iter().map(|e| {
                unit_query.get(*e).ok().map(|(_, u)| u.obstacle.clone())
            }).next().flatten();


            if let Some(obstacle) = obstacle_on_target {
                match obstacle {
                    // Can't move here, or through here.
                    ObstacleType::Neutral => {
                        continue;
                    }
                    // Can move through here, but can't move here.
                    ObstacleType::Filter(hash_set) => {
                        if !hash_set.contains(&movement.unit.team) {
                            continue;
                        } else {
                            queue.push_back(
                               (grid_pos, movement_after_moved_onto_tile, true)
                            )
                        }
                    }
                }
            } else {
                queue.push_back(
                    (grid_pos, movement_after_moved_onto_tile, false)
                )
            };
        }
    }

    spaces_explored.into_iter().collect()
}

// TODO: This abstraction kind of sucks. It's really hard to get what I want out of it
fn get_singleton_component_on_grid_by_player<F, T>(
    cursor_grid_pos: &GridPosition,
    grid_manager: &GridManager,
    query: F
) -> Option<(Entity, Player, T)>
where F: Fn(&Entity) -> Option<(Entity, Player, T)> {
    let entities_at_pos = grid_manager.get_by_position(&cursor_grid_pos).cloned().unwrap_or_default();
    entities_at_pos.iter().map(query).next().flatten()
}

fn select_unit_for_movement<F>(
    cursor_grid_pos: &GridPosition,
    grid_manager: &GridManager,
    unit_player_from_entity_query: F,
)-> UnitMovementSelection
where F: Fn(&Entity) -> Option<(Entity, Player, Unit)> 
{
    let unit = get_singleton_component_on_grid_by_player(cursor_grid_pos, grid_manager, unit_player_from_entity_query);
    if let Some((entity, _, unit )) = unit {
        return UnitMovementSelection::Selected(entity, Movement {
            origin: *cursor_grid_pos,
            unit: unit.clone(),
        });
    }
    return UnitMovementSelection::NoPlayerUnitOnTile;
}

fn handle_select_unit_for_movement(
    overlay_message_writer: &mut MessageWriter<OverlaysMessage>,
    player_unit_query: Query<(Entity, &player::Player, &Unit)>,
    unit_query: Query<(Entity, &Unit)>,
    grid_manager: &mut GridManager,
    player_state: &mut PlayerState,
    cursor_grid_pos: &GridPosition,
    player: &Player
) {
    log::debug!("Player {:?}, is selecting unit", player);
    // Note that here we don't allow a given player to access a Unit that is not associated with them
    let selection = select_unit_for_movement(&cursor_grid_pos, grid_manager, |entity|  {
        // Get the first Unit owned by this player, and then clone the values to satisfy lifetimes.
        let queried = player_unit_query.get(*entity).ok().filter(|(_, p, _)| **p == *player);
        queried.map(|(a, b, c)| (a, *b, c.clone()))
    });

    match selection {
        UnitMovementSelection::Selected(entity, movement) => {
            let valid_moves = get_valid_moves_for_unit(grid_manager, movement, unit_query);
            // Change Player State to moving the unit
            player_state.cursor_state = player::PlayerCursorState::MovingUnit(entity, *cursor_grid_pos, valid_moves.clone());
            overlay_message_writer.write(OverlaysMessage {
                player: *player,
                action: overlay::OverlaysAction::Spawn { positions: valid_moves }
            });
            log::debug!("Selected Player: {:?}", player_state.cursor_state);
        }
        UnitMovementSelection::NoPlayerUnitOnTile => {
            warn!("Selected tile with no player unit");
        }
    }
}

pub fn handle_unit_movement(
    mut commands: Commands,
    mut grid_manager_res: ResMut<grid::GridManagerResource>,
    mut player_state: ResMut<player::PlayerGameStates>,
    player_query: Query<(&Player, &ActionState<PlayerInputAction>)>,
    mut cursor_query: Query<(&Player, &mut grid::GridPosition), With<grid_cursor::Cursor>>,
    player_unit_query: Query<(Entity, &player::Player, &Unit)>,
    unit_query: Query<(Entity, &Unit)>,
    mut overlay_message_writer: MessageWriter<OverlaysMessage>,
) {
    for (player, action_state) in player_query.iter() {
        for (cursor_player, mut cursor_grid_pos) in cursor_query.iter_mut() {
            if player != cursor_player {
                continue;
            }

        let Some(player_state) = player_state.player_state.get_mut(player) else {
            log::error!("No player state found for player {:?}", player);
            continue;
        };

        // If the cursor is idle, and there's a unit at the cursor position, 
        // generate overlays using that unit's movement
        if player_state.cursor_state == player::PlayerCursorState::Idle && action_state.just_pressed(&PlayerInputAction::Select) {
            handle_select_unit_for_movement(&mut overlay_message_writer, player_unit_query, unit_query, &mut grid_manager_res.grid_manager,  player_state, &cursor_grid_pos, player);
        }

        // If we're moving a unit, and we press select again, attempt to move the unit to that position
        else if let player::PlayerCursorState::MovingUnit(unit_entity, original_position, valid_moves) = player_state.cursor_state.clone() {
            if action_state.just_pressed(&PlayerInputAction::Select) {
                // TODO: What to do if this changes between start and end of movement?
                if !valid_moves.contains(&cursor_grid_pos) {
                    log::warn!("Attempting to move to invalid position");
                    continue;
                }

                // Should unit entities have an "Obstruction" component?
                // TODO: I think I actually need to calculate obstructions when the unit was selected (but if so, how do I deal with two units moving at once?)
                let unit_at_position = get_singleton_component_on_grid_by_player(&cursor_grid_pos, &grid_manager_res.grid_manager, |entity| {
                    player_unit_query.get(*entity).ok().map(|(a, b, c)| (a, *b, c))
                });
               
                if unit_at_position.is_some() {
                    log::warn!("Cannot move unit to position {:?} because it is occupied", cursor_grid_pos);
                    continue;
                }

                // Get the path to the new position
                let path = grid_manager_res.grid_manager.get_path(original_position, *cursor_grid_pos);

                commands.entity(unit_entity).insert(GridMovement::new(path, 0.2));

                end_move(
                    &mut overlay_message_writer,
                    player,
                    player_state
                );

            } else if action_state.just_pressed(&PlayerInputAction::Deselect) {
                end_move(
                    &mut overlay_message_writer,
                    player,
                    player_state
                );

                // Snap the cursor position back to the origin
                *cursor_grid_pos = original_position;
            }
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
    ) -> Self {
        Self {
            grid_position,
            sprite: Sprite {
                image: spritesheet,
                texture_atlas: Some(TextureAtlas {
                    layout: atlas_layout_handle,
                    index: 1,
                }), 
                custom_size: None,
                color: Color::linear_rgba(1.0, 1.0, 1.0, 0.3),
                ..Default::default()
            },
            transform: init_grid_to_world_transform(&grid_position),
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
        match event {
            AssetEvent::LoadedWithDependencies { id } => {
                if *id == asset_handles.tile_overlay_image_handle.id() {
                    info!("Our specific image asset and its dependencies are loaded!");
                    if let Some(image) = images.get_mut(*id) {
                        image.sampler = ImageSampler::nearest();
                    }
                }
            }
            _ => {}
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
pub enum OverlaysAction {
    Spawn { positions: Vec<GridPosition> },
    Despawn
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
        if let OverlaysAction::Spawn { positions } = &event.action {
            spawn_overlays(&mut commands, &tile_overlay_assets, event.player, positions.clone(), &mut grid_manager_res.grid_manager);
        }
        else if let OverlaysAction::Despawn = &event.action {
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

    use bevy::{app::App, ecs::system::RunSystemOnce, input::keyboard::{KeyCode, KeyboardInput}, time::{Real, Time, Virtual}, transform::components::Transform};
    use leafwing_input_manager::{plugin::InputManagerPlugin, prelude::{ActionState, Buttonlike}};
    use crate::{grid::{self, GridManager, GridManagerResource, GridMovement, GridPosition, sync_grid_positions_to_manager}, grid_cursor, player::{self, Player, PlayerGameStates, PlayerInputAction, PlayerState}, unit::{PLAYER_TEAM, Stats, Unit, handle_unit_movement, overlay::{OverlaysMessage, handle_overlays_events_system}}};


    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn create_test_app() -> App {
        let mut app = App::new();
        app.add_message::<OverlaysMessage>();
        app.insert_resource(GridManagerResource {
            grid_manager: GridManager::new(6, 6)
        });
        app.insert_resource::<Time>(Time::default()); 
        app.insert_resource(PlayerGameStates {
            player_state: HashMap::from([(Player::One, PlayerState::default())])
        });
        app.add_plugins(InputManagerPlugin::<PlayerInputAction>::default());
        app

    }

    #[test]
    fn test_movement_system() -> anyhow::Result<()> {
        init_logger();
        let mut app = create_test_app();

        // Spawn player to handle movement events
        let player_entity = app.world_mut().spawn(player::PlayerBundle::new(Player::One)).id();
        let unit_entity = app.world_mut().spawn((
            // TODO: Make constructor
            Unit {
                stats: Stats {
                    max_health: 10,
                    health: 10,
                    strength: 5,
                    movement: 2,
                },
                team: PLAYER_TEAM,
                obstacle: crate::unit::ObstacleType::Filter(HashSet::from([PLAYER_TEAM]))
            },
            Player::One,
            GridPosition { x: 2, y: 2 },
            Transform::default(),
        )).id();

        // Spawn a cursor at (2, 2) for Player::One
        let cursor_entity = app.world_mut().spawn((
            grid_cursor::Cursor {},
            Player::One,
            GridPosition { x: 2, y: 2 },
        )).id();

        app.world_mut().run_system_once(sync_grid_positions_to_manager).map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        {
            let mut action_state = app.world_mut().get_mut::<ActionState<PlayerInputAction>>(player_entity).unwrap();
            action_state.press(&PlayerInputAction::Select);
        }

        // Run the movement system (simulate one frame)
        app.world_mut().run_system_once(handle_unit_movement).map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;

        app.world_mut().get_mut::<GridPosition>(cursor_entity).unwrap().x = 3;

        {
            let mut action_state = app.world_mut().get_mut::<ActionState<PlayerInputAction>>(player_entity).unwrap();
            action_state.press(&PlayerInputAction::Select);
        }

        // UnitMovement should spawn some GridMovement, let's let that resolve and validate that our entity is moved to the correct space.
        app.world_mut().run_system_once(handle_unit_movement).map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;
        
        assert!(app.world().get::<GridMovement>(unit_entity).is_some());

        // Infinite loops if GridMovement isn't working so nice
        while app.world().get::<GridMovement>(unit_entity).is_some() {
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(std::time::Duration::from_secs_f32(0.1));  // Advance by 0.1s per step
            }
            log::debug!("Running sync_grid_movement to transform");
            app.world_mut().run_system_once(grid::sync_grid_movement_to_transform).map_err(|e| anyhow::anyhow!("Failed to run system: {:?}", e))?;
        }

        // Assert the unit is at the final position
        let final_pos = app.world().get::<GridPosition>(unit_entity).unwrap();
        assert_eq!(*final_pos, GridPosition { x: 3, y: 2 });

        // Assert GridMovement component is removed
        assert!(app.world().get::<GridMovement>(unit_entity).is_none());

        Ok(())
    }
}