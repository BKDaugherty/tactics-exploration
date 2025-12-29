use bevy::prelude::*;
use std::collections::HashMap;

use crate::{
    battle_phase::UnitPhaseResources,
    unit::{UnitAction, UnitActionCompletedMessage},
};

/// Size of a single tile in world units
pub const TILE_X_SIZE: f32 = 32.0;
pub const TILE_Y_SIZE: f32 = 16.0;

#[derive(Debug)]
pub struct GridManager {
    width: u32,
    height: u32,
    // We keep both indexes up to date for easy lookup at the cost of memory and
    // a lil more expensive updates for now
    entities: HashMap<GridPosition, Vec<Entity>>,
    entity_positions: HashMap<Entity, GridPosition>,
}

pub enum GridPositionChangeResult {
    Moved(GridPosition),
    OutOfBounds(GridPosition),
}

impl GridPositionChangeResult {
    pub fn hit_boundary(&self) -> bool {
        match self {
            GridPositionChangeResult::Moved(_) => false,
            GridPositionChangeResult::OutOfBounds(_) => true,
        }
    }

    pub fn position(&self) -> GridPosition {
        match self {
            GridPositionChangeResult::Moved(pos) => *pos,
            GridPositionChangeResult::OutOfBounds(pos) => *pos,
        }
    }
}

impl GridPosition {
    /// Assumes a lower bound of 0
    const LOWER_BOUND: i32 = 0;

    fn change(&self, bounds: GridPosition, delta: GridVec) -> GridPositionChangeResult {
        let mut new_x = self.x as i32 + delta.x;
        let mut new_y = self.y as i32 + delta.y;

        let mut out_of_bounds = false;

        if new_x < GridPosition::LOWER_BOUND {
            new_x = GridPosition::LOWER_BOUND;
            out_of_bounds = true;
        }

        if new_y < GridPosition::LOWER_BOUND {
            new_y = GridPosition::LOWER_BOUND;
            out_of_bounds = true;
        }

        if new_x > bounds.x as i32 {
            new_x = bounds.x as i32;
            out_of_bounds = true;
        }

        if new_y > bounds.y as i32 {
            new_y = bounds.y as i32;
            out_of_bounds = true;
        }

        let new_position = GridPosition {
            x: new_x as u32,
            y: new_y as u32,
        };

        if out_of_bounds {
            GridPositionChangeResult::OutOfBounds(new_position)
        } else {
            GridPositionChangeResult::Moved(new_position)
        }
    }
}

impl GridManager {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            entities: HashMap::new(),
            entity_positions: HashMap::new(),
        }
    }

    /// Move an entity to a new position on the grid
    pub fn move_entity_to(
        &mut self,
        entity: Entity,
        new_position: GridPosition,
    ) -> anyhow::Result<()> {
        // Remove from old position, if applicable
        if let Some(old_position) = self.entity_positions.get(&entity)
            && let Some(entities_at_old) = self.entities.get_mut(old_position)
        {
            if !entities_at_old.contains(&entity) {
                anyhow::bail!("Entity not found in old position list, but was previously tracked!");
            }
            entities_at_old.retain(|&e| e != entity);
        }

        // Add to new position
        self.entity_positions.insert(entity, new_position);
        self.entities.entry(new_position).or_default().push(entity);

        Ok(())
    }

    /// Add an entity to the grid at a given position
    pub fn add_entity(&mut self, entity: Entity, position: GridPosition) {
        self.entity_positions.insert(entity, position);
        self.entities.entry(position).or_default().push(entity);
    }

    pub fn remove_entity(&mut self, entity: &Entity) {
        if let Some((_, v)) = self.entity_positions.remove_entry(entity) {
            self.entities
                .entry(v)
                .and_modify(|t| t.retain(|e| e != entity));
        }
    }

    pub fn get_by_position(&self, position: &GridPosition) -> Option<&Vec<Entity>> {
        self.entities.get(position)
    }

    pub fn get_by_id(&self, entity: &Entity) -> Option<GridPosition> {
        self.entity_positions.get(entity).copied()
    }

    /// TODO: how bad is it to take &self for just the bounds? Does this affect update fn?
    pub fn change_position_with_bounds(
        &self,
        origin: GridPosition,
        delta: GridVec,
    ) -> GridPositionChangeResult {
        let bounds = GridPosition {
            x: self.width - 1,
            y: self.height - 1,
        };
        origin.change(bounds, delta)
    }
}

#[derive(Debug, Resource)]
pub struct GridManagerResource {
    pub grid_manager: GridManager,
}

/// Sync's the grid positions of entities to the grid manager
///
/// Assumes that entities are already added to the grid manager, but will add them if that happens
/// Ignores entities that are in the middle of moving
#[allow(clippy::type_complexity)]
pub fn sync_grid_positions_to_manager(
    mut grid_manager_res: ResMut<GridManagerResource>,
    changed_grid_query: Query<
        (Entity, &GridPosition),
        (Changed<GridPosition>, Without<GridMovement>),
    >,
) {
    for (entity, grid_position) in changed_grid_query.iter() {
        if let Err(e) = grid_manager_res
            .grid_manager
            .move_entity_to(entity, *grid_position)
        {
            eprintln!("Failed to move entity: {:?}", e);
        }
    }
}

/// System to sync GridPosition to world Transform
///
/// Not so sure about this just yet.
pub fn sync_grid_position_to_transform(
    mut query: Query<(&GridPosition, &mut Transform), Changed<GridPosition>>,
) {
    for (grid_pos, mut transform) in query.iter_mut() {
        // Convert grid coordinates to world coordinates
        let world = grid_to_world(grid_pos, TILE_X_SIZE, TILE_Y_SIZE);
        transform.translation = Vec3::new(world.x, world.y, world.z);
    }
}

#[derive(Component, Hash, PartialEq, Eq, Debug, Copy, Clone, Reflect)]
#[reflect(Component)]
pub struct GridPosition {
    pub x: u32,
    pub y: u32,
}

#[derive(Hash, PartialEq, Debug, Copy, Clone)]
pub struct GridVec {
    pub x: i32,
    pub y: i32,
}

impl GridVec {
    pub fn scale(&self, scalar: i32) -> GridVec {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

pub fn manhattan_distance(a: &GridPosition, b: &GridPosition) -> u32 {
    ((a.x as i32 - b.x as i32).abs() + (a.y as i32 - b.y as i32).abs()) as u32
}

#[derive(Component)]
pub struct GridMovement {
    pub waypoints: Vec<GridPosition>,
    pub current_waypoint_index: usize,
    pub elapsed_time: f32,
    pub duration: f32, // Time to move between waypoints
}

impl GridMovement {
    pub fn new(waypoints: Vec<GridPosition>, duration: f32) -> Self {
        Self {
            waypoints,
            current_waypoint_index: 0,
            elapsed_time: 0.0,
            duration,
        }
    }

    fn current_position(&self) -> Option<&GridPosition> {
        self.waypoints.get(self.current_waypoint_index)
    }

    fn next_position(&self) -> Option<&GridPosition> {
        self.waypoints.get(self.current_waypoint_index + 1)
    }

    fn is_finished(&self) -> bool {
        self.current_waypoint_index >= self.waypoints.len() - 1
    }
}

pub const MAGIC_Z_INDEX_OFFSET: f32 = 600.;

/// Diamond isometric grid conversion
pub fn grid_to_world(grid_pos: &GridPosition, tile_width: f32, tile_height: f32) -> Vec3 {
    let offset_x = grid_pos.x as f32 - 5.;
    let offset_y = grid_pos.y as f32 - 4.;

    let world_x = (offset_x + offset_y as f32) * (tile_width / 2.0);
    let world_y = (offset_x - offset_y) * (tile_height / 2.0);

    Vec3::new(
        world_x,
        world_y,
        MAGIC_Z_INDEX_OFFSET + (grid_pos.y as f32 - grid_pos.x as f32),
    )
}

/// Spawning an entity in the real world from a logical grid pos? Use this to hide some
/// constants that probably shouldn't exist from yourself.
pub fn init_grid_to_world_transform(grid_pos: &GridPosition) -> Transform {
    let world = grid_to_world(grid_pos, TILE_X_SIZE, TILE_Y_SIZE);
    Transform::from_translation(Vec3::new(world.x, world.y, world.z))
}

/// System to resolve GridMovement GridMovement components to Transform components
pub fn resolve_grid_movement(
    mut commands: Commands,
    mut grid_manager_res: ResMut<GridManagerResource>,
    mut query: Query<(
        Entity,
        &mut GridMovement,
        &mut Transform,
        &mut GridPosition,
        &mut UnitPhaseResources,
    )>,
    time: Res<Time>,
    mut action_completed_writer: MessageWriter<UnitActionCompletedMessage>,
) {
    for (entity, mut movement, mut transform, mut grid_pos, mut unit_resources) in query.iter_mut()
    {
        if movement.is_finished() {
            commands.entity(entity).remove::<GridMovement>();
            action_completed_writer.write(UnitActionCompletedMessage {
                unit: entity,
                action: UnitAction::Move,
            });
            continue;
        }

        movement.elapsed_time += time.delta_secs();
        let progress = (movement.elapsed_time / movement.duration).clamp(0.0, 1.0);

        log::debug!("Moving entity {:} at progress {:?}", entity, progress);

        let current = movement
            .current_position()
            .expect("No current position in movement, but movement isn't finished!");
        let next = movement
            .next_position()
            .expect("No next position in movement, but movement isn't finished!");

        let start_world = grid_to_world(current, TILE_X_SIZE, TILE_Y_SIZE);
        let target_world = grid_to_world(next, TILE_X_SIZE, TILE_Y_SIZE);

        let lerped = start_world.lerp(target_world, progress);

        transform.translation = Vec3::new(lerped.x, lerped.y, lerped.z);

        // Move to next waypoint when current one completes
        if progress >= 1.0 {
            grid_pos.x = next.x;
            grid_pos.y = next.y;
            movement.current_waypoint_index += 1;
            movement.elapsed_time = 0.0;

            // Update the GridManager
            if let Err(e) = grid_manager_res
                .grid_manager
                .move_entity_to(entity, *grid_pos)
            {
                log::error!(
                    "Failed to move entity to position: {:?}, entity: {:?}, pos: {:?}",
                    e,
                    entity,
                    grid_pos
                );
            };

            unit_resources.movement_points_left_in_phase = unit_resources
                .movement_points_left_in_phase
                .saturating_sub(1);
        }
    }
}

/// Given a range of movement, return the discrete grid vecs of possible movement options
///
/// Naively assumes no obstacles
pub fn get_movement_options(movement: u32) -> Vec<GridVec> {
    let mut options = Vec::new();
    let movement = movement as i32;

    for dx in -movement..=movement {
        let dy_range = movement - dx.abs();
        for dy in -dy_range..=dy_range {
            if dx == 0 && dy == 0 {
                continue; // Skip the origin
            }
            options.push(GridVec { x: dx, y: dy });
        }
    }

    options
}

#[cfg(test)]
mod test {
    use bevy::ecs::relationship::RelationshipSourceCollection;

    use super::*;

    #[test]
    fn test_grid_manager_add_and_move() {
        let mut grid_manager = GridManager::new(10, 10);
        let entity = Entity::new();
        let initial_position = GridPosition { x: 2, y: 3 };
        let new_position = GridPosition { x: 5, y: 6 };

        grid_manager.add_entity(entity, initial_position.clone());
        assert_eq!(
            grid_manager.entity_positions.get(&entity),
            Some(&initial_position)
        );
        assert_eq!(
            grid_manager.entities.get(&initial_position).unwrap(),
            &vec![entity]
        );

        grid_manager
            .move_entity_to(entity, new_position.clone())
            .unwrap();
        assert_eq!(
            grid_manager.entity_positions.get(&entity),
            Some(&new_position)
        );
        assert_eq!(
            grid_manager.entities.get(&initial_position).unwrap().len(),
            0
        );
        assert_eq!(
            grid_manager.entities.get(&new_position).unwrap(),
            &vec![entity]
        );
    }

    #[test]
    fn test_sync_grid_positions_system() {
        let mut app = App::new();
        app.insert_resource(GridManagerResource {
            grid_manager: GridManager::new(10, 10),
        });
        app.add_systems(Update, sync_grid_positions_to_manager);

        let entity = app.world_mut().spawn((GridPosition { x: 1, y: 1 },)).id();

        let entity_not_on_grid_init = app.world_mut().spawn((GridPosition { x: 2, y: 3 },)).id();

        {
            let mut grid_manager_res = app.world_mut().resource_mut::<GridManagerResource>();
            grid_manager_res
                .grid_manager
                .add_entity(entity, GridPosition { x: 1, y: 1 });
        }

        // Change the GridPosition component of all entities
        {
            let mut query = app.world_mut().query::<&mut GridPosition>();
            for mut grid_pos in query.iter_mut(app.world_mut()) {
                *grid_pos = GridPosition { x: 4, y: 5 };
            }
        }

        // Run the sync system
        app.update();

        let grid_manager_res = app.world().resource::<GridManagerResource>();
        assert_eq!(
            grid_manager_res.grid_manager.get_by_id(&entity),
            Some(GridPosition { x: 4, y: 5 })
        );

        assert_eq!(
            grid_manager_res
                .grid_manager
                .get_by_id(&entity_not_on_grid_init),
            Some(GridPosition { x: 4, y: 5 })
        );
    }

    #[test]
    fn test_get_movement_options() {
        let options = get_movement_options(2);
        let expected_options = vec![
            GridVec { x: -2, y: 0 },
            GridVec { x: -1, y: -1 },
            GridVec { x: -1, y: 0 },
            GridVec { x: -1, y: 1 },
            GridVec { x: 0, y: -2 },
            GridVec { x: 0, y: -1 },
            GridVec { x: 0, y: 1 },
            GridVec { x: 0, y: 2 },
            GridVec { x: 1, y: -1 },
            GridVec { x: 1, y: 0 },
            GridVec { x: 1, y: 1 },
            GridVec { x: 2, y: 0 },
        ];

        for option in &expected_options {
            assert!(options.contains(&option), "Missing option: {:?}", option);
        }

        assert_eq!(
            options.len(),
            expected_options.len(),
            "Unexpected number of options"
        );
    }
}
