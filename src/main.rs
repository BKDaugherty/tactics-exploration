use std::collections::HashMap;

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::ActionState;
use tactics_exploration::grid::{self, GridManager, GridMovement, GridPosition, GridVec, grid_to_world, init_grid_to_world_transform };
use tactics_exploration::player::{Player, PlayerInputAction};
use tactics_exploration::unit::overlay::{OverlaysMessage, TileOverlayAssets, on_asset_event, handle_overlays_events_system};
use tactics_exploration::unit::{self, handle_unit_movement, spawn_unit};
use tactics_exploration::{Ground, grid_cursor, player};

fn main() {
    App::new()
        // Add Bevy's default plugins
        .add_plugins(DefaultPlugins)
        // TODO: Dev Mode
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        // Add the bevy_ecs_tiled plugin
        // bevy_ecs_tilemap::TilemapPlugin will be added automatically if needed
        .add_plugins(TiledPlugin::default())
        // .add_plugins(TiledDebugPluginGroup)
        .add_plugins(InputManagerPlugin::<PlayerInputAction>::default())
        // Add your startup system and run the app
        .add_message::<OverlaysMessage>()
        .add_systems(Startup, startup)
        .add_systems(Startup, startup_load_overlay_sprite_data.after(startup))
        .add_systems(Startup, create_cursor_for_players.after(startup_load_overlay_sprite_data))
        .add_systems(Update, (change_zoom, grid::sync_grid_positions_to_manager, grid::sync_grid_position_to_transform, grid::sync_grid_movement_to_transform, on_asset_event))
        .add_systems(Update, grid_cursor::handle_cursor_movement)
        .add_systems(Update, handle_overlays_events_system)
        .add_systems(Update, handle_unit_movement)
        .run();
}

/// Resource because one of them? Split screen maybe would need two?
#[derive(Debug, Resource)]
struct CameraSettings {
    pub zoom_value: f32,
}

pub const SQUARE_GRID_BOUNDS : u32 = 8;


fn create_cursor_for_players(
    mut commands: Commands,
    tile_overlay_assets: Res<TileOverlayAssets>,
) {
    grid_cursor::spawn_cursor(
        &mut commands,
        tile_overlay_assets.cursor_image.clone(),
        player::Player::One,
    );

    grid_cursor::spawn_cursor(&mut commands, tile_overlay_assets.cursor_image.clone(), player::Player::Two);
}

fn startup_load_overlay_sprite_data(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let debug_color_spritesheet = asset_server.load("random-assets/iso_color.png");
    let cursor_image = asset_server.load("cursor.png");
    let example_unit = asset_server.load("tinytactics_battlekiti_v1_0/20240427cleric-weakNE.png");
    commands.insert_resource(TileOverlayAssets {
        tile_overlay_image_handle: debug_color_spritesheet.clone(),
        tile_overlay_atlas_layout_handle: {
            let layout = TextureAtlasLayout::from_grid(UVec2::new(64, 32), 6, 1, None, None);
            texture_atlas_layouts.add(layout)
        },
        cursor_image: cursor_image.clone(),
    });

    // TODO: Remove me
    spawn_unit(&mut commands, GridPosition {x: 1, y: 6}, example_unit.clone(), Player::One);
}



fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Spawn a 2D camera
    let mut t = init_grid_to_world_transform(&GridPosition {x: 3, y: 3});
    t.translation.z = 0.;

    let camera_settings = CameraSettings {
        zoom_value: 0.5
    };
    
    commands.spawn((
        Name::new("Main Camera"),
        Camera2d,
        Projection::from(
            OrthographicProjection { 
                scale: camera_settings.zoom_value,
                ..OrthographicProjection::default_2d()
            }),
        t,
    ));
    
    commands.insert_resource(camera_settings);

    // Load a map asset and retrieve its handle
    let map_handle: Handle<TiledMapAsset> = asset_server.load("random-assets/example-map.tmx");
    // let map_handle: Handle<TiledMapAsset> = asset_server.load("example-map.tmx");

    // Spawn a new entity with the TiledMap component
    commands.spawn(TiledMap(map_handle));
    
    commands.insert_resource(grid::GridManagerResource {
        grid_manager: GridManager::new(SQUARE_GRID_BOUNDS, SQUARE_GRID_BOUNDS)
    });

    commands.insert_resource(player::PlayerGameStates {
        player_state: HashMap::from([
            (Player::One, player::PlayerState::default()),
            (Player::Two, player::PlayerState::default()),
        ])
    });

    commands.spawn((
        Name::new("Player One"),
        player::PlayerBundle::new(Player::One),
    ));

    commands.spawn((
        Name::new("Player Two"),
        player::PlayerBundle::new(Player::Two),
    ));
}

fn change_zoom(
    mut camera: Single<&mut Projection, With<Camera>>,
    mut camera_settings: ResMut<CameraSettings>,
    player_query: Query<(&Player, &ActionState<PlayerInputAction>)>
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
                },
                _ => return,
            } 
        }
    }
}
