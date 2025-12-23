use crate::grid;
use crate::player;

use bevy::prelude::*;

/// A cursor that can be moved on the grid
#[derive(Component)]
pub struct Cursor {}

#[derive(Component)]
pub struct LockedOn {}

#[derive(Bundle)]
pub struct CursorBundle {
    pub grid_position: grid::GridPosition,
    pub transform: Transform,
    pub sprite: Sprite,
    pub cursor: Cursor,
    /// The controlling player of the spawned cursor entity
    pub player: player::Player,
}

pub fn spawn_cursor(
    commands: &mut Commands,
    image: Handle<Image>,
    player: player::Player,
    initial_grid_pos: grid::GridPosition,
) {
    let mut initial_transform = grid::init_grid_to_world_transform(&initial_grid_pos);

    // Put cursor behind players
    initial_transform.translation.z -= 50.;
    commands.spawn((CursorBundle {
        grid_position: initial_grid_pos,
        transform: initial_transform,
        sprite: Sprite {
            image,
            color: Color::linear_rgb(1.0, 0.0, 1.0),
            ..Default::default()
        },
        cursor: Cursor {},
        player,
    },));
}

/// Translates Input Actions to grid movement for the cursor
pub fn handle_cursor_movement(
    grid_manager: Res<grid::GridManagerResource>,
    input_query: Query<(
        &player::Player,
        &leafwing_input_manager::prelude::ActionState<player::PlayerInputAction>,
    )>,
    mut cursor_query: Query<
        (&player::Player, &mut grid::GridPosition),
        (With<Cursor>, Without<LockedOn>),
    >,
) {
    for (player, action_state) in input_query.iter() {
        for (cursor_player, mut grid_pos) in cursor_query.iter_mut() {
            if player != cursor_player {
                continue;
            }
            let mut delta = grid::GridVec { x: 0, y: 0 };
            if action_state.just_pressed(&player::PlayerInputAction::MoveCursorUp) {
                delta.y -= 1;
            }
            if action_state.just_pressed(&player::PlayerInputAction::MoveCursorDown) {
                delta.y += 1;
            }
            if action_state.just_pressed(&player::PlayerInputAction::MoveCursorLeft) {
                delta.x -= 1;
            }
            if action_state.just_pressed(&player::PlayerInputAction::MoveCursorRight) {
                delta.x += 1;
            }
            *grid_pos = grid_manager
                .grid_manager
                .change_position_with_bounds(*grid_pos, delta)
                .position();
        }
    }
}
