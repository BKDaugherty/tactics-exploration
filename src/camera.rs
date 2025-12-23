use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    grid::{GridPosition, init_grid_to_world_transform},
    player::{Player, PlayerInputAction},
};

/// Resource because one of them? Split screen maybe would need two?
#[derive(Debug, Resource)]
pub struct CameraSettings {
    pub zoom_value: f32,
}

pub fn setup_camera(mut commands: Commands) {
    // Spawn a 2D camera
    let mut t = init_grid_to_world_transform(&GridPosition { x: 5, y: 3 });
    t.translation.z = 0.;

    let camera_settings = CameraSettings { zoom_value: 0.4 };

    commands.spawn((
        Name::new("Main Camera"),
        Camera2d,
        Projection::from(OrthographicProjection {
            scale: camera_settings.zoom_value,
            ..OrthographicProjection::default_2d()
        }),
        t,
    ));

    commands.insert_resource(camera_settings);
}

pub fn change_zoom(
    mut camera: Single<&mut Projection, With<Camera>>,
    mut camera_settings: ResMut<CameraSettings>,
    player_query: Query<(&Player, &ActionState<PlayerInputAction>)>,
) {
    for (_, action_state) in player_query.iter() {
        if action_state.just_pressed(&PlayerInputAction::ZoomIn) {
            match **camera {
                Projection::Orthographic(ref mut current_projection) => {
                    current_projection.scale += 0.1;
                    camera_settings.zoom_value = current_projection.scale;
                }
                _ => return,
            }
        } else if action_state.just_pressed(&PlayerInputAction::ZoomOut) {
            match **camera {
                Projection::Orthographic(ref mut current_projection) => {
                    current_projection.scale -= 0.1;
                    camera_settings.zoom_value = current_projection.scale;
                }
                _ => return,
            }
        }
    }
}
