use std::collections::{HashMap, HashSet};

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::ActionState;
use tactics_exploration::assets::{CURSOR_PATH, EXAMPLE_MAP_PATH, EXAMPLE_UNIT_PATH, OVERLAY_PATH};
use tactics_exploration::grid::{self, GridManager, GridMovement, GridPosition, GridVec, grid_to_world, init_grid_to_world_transform };
use tactics_exploration::player::{Player, PlayerInputAction};
use tactics_exploration::unit::overlay::{OverlaysMessage, TileOverlayAssets, on_asset_event, handle_overlays_events_system};
use tactics_exploration::unit::{self, PLAYER_TEAM, handle_unit_movement, spawn_obstacle_unit, spawn_unit};
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
        .add_systems(Startup, populate_demo_map)
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

fn startup_load_overlay_sprite_data(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let debug_color_spritesheet = asset_server.load(OVERLAY_PATH);
    let cursor_image: Handle<Image> = asset_server.load(CURSOR_PATH);

    // TODO: Better asset management resources
    commands.insert_resource(TileOverlayAssets {
        tile_overlay_image_handle: debug_color_spritesheet.clone(),
        tile_overlay_atlas_layout_handle: {
            let layout = TextureAtlasLayout::from_grid(UVec2::new(64, 32), 6, 1, None, None);
            texture_atlas_layouts.add(layout)
        },
        cursor_image: cursor_image.clone(),
    });
}

fn populate_demo_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Spawn players and player cursors
    let example_unit = asset_server.load(EXAMPLE_UNIT_PATH);
    let cursor_image: Handle<Image> = asset_server.load(CURSOR_PATH);

    let player_1_grid_pos = GridPosition {x: 4, y: 6};
    let player_2_grid_pos = GridPosition {x: 1, y: 3};

    spawn_unit(&mut commands, player_1_grid_pos, example_unit.clone(), Player::One, PLAYER_TEAM);
    spawn_unit(&mut commands, player_2_grid_pos, example_unit.clone(), Player::Two, PLAYER_TEAM);

    grid_cursor::spawn_cursor(
        &mut commands,
        cursor_image.clone(),
        player::Player::One,
        player_1_grid_pos
    );

    grid_cursor::spawn_cursor(
        &mut commands, 
        cursor_image.clone(), 
        player::Player::Two,
        player_2_grid_pos
    );

    let door_location = GridPosition {x: 7, y: 1};

    // Spawn Obstacles (All walls / corners except the door)
    let stool_locations = [
        GridPosition { x: 2, y: 3},
        GridPosition { x: 4, y: 1},
        GridPosition { x: 4, y: 3},
        GridPosition { x: 4, y: 5},
        GridPosition { x: 6 , y: 3}
    ];

    let mut obstacle_locations = Vec::new();
    for i in 0..SQUARE_GRID_BOUNDS {
        // Set X to zero
        obstacle_locations.push(GridPosition {x: 0, y: i});
        obstacle_locations.push(GridPosition {x: i, y: 0});
        obstacle_locations.push(GridPosition {x: i, y: SQUARE_GRID_BOUNDS - 1});
        obstacle_locations.push(GridPosition {x: SQUARE_GRID_BOUNDS - 1, y: i});
    }

    // Remove door location
    obstacle_locations.retain(|t| *t != door_location);

    obstacle_locations.extend_from_slice(&stool_locations);

    for obstacle_location in obstacle_locations {
        spawn_obstacle_unit(&mut commands, obstacle_location);
    }
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
    let map_handle: Handle<TiledMapAsset> = asset_server.load(EXAMPLE_MAP_PATH);
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
