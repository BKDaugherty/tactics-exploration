use bevy::prelude::*;

/// A unit! Units can't share spaces (for now I guess)
/// 
/// The base controllable entity that can exist and do things on the map
/// Units would have stats, skills, etc?
#[derive(Component, Debug, Reflect)]
pub struct Unit {
    pub stats: Stats,
    // effect_modifiers: ()
    // equipment?
}

#[derive(Debug, Reflect)]
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

/// Temporary function for spawning a test unit
pub fn spawn_unit(
    mut commands: Commands,
    grid_position: crate::grid::GridPosition,
    spritesheet: Handle<Image>,
    player: crate::player::Player,
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
            },
            grid_position,
            sprite: Sprite {
                image: spritesheet,
                color: Color::linear_rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(64.0, 32.0)),
                ..Default::default()
            },
            transform,
            player,
        },
    ));
}