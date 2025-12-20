//! Top level module for a Battle

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_common_assets::json::JsonAssetPlugin;
use bevy_ecs_tiled::{
    prelude::{TiledMap, TiledMapAsset},
    tiled::TiledPlugin,
};

use crate::{
    GameState,
    animation::{
        TinytacticsAssets, animate_sprite, startup_load_tinytactics_assets,
        tinytactics::AnimationAsset, update_facing_direction, update_sprite_on_animation_change,
    },
    assets::{CURSOR_PATH, EXAMPLE_MAP_PATH, OVERLAY_PATH},
    camera::change_zoom,
    grid::{self, GridManager, GridPosition},
    grid_cursor,
    player::{self, Player},
    unit::{
        PLAYER_TEAM, handle_unit_movement,
        overlay::{OverlaysMessage, TileOverlayAssets, handle_overlays_events_system},
        spawn_obstacle_unit, spawn_unit,
    },
};

/// All logic necessary during a battle
pub fn battle_plugin(app: &mut App) {
    app.add_message::<OverlaysMessage>()
        .add_plugins(TiledPlugin::default())
        // I wonder if I should put this guy on the top level if I want to
        // have it be used for the UI too
        .add_plugins(JsonAssetPlugin::<AnimationAsset>::new(&[".json"]))
        .add_systems(OnEnter(GameState::Battle), load_battle_asset_resources)
        .add_systems(
            OnEnter(GameState::Battle),
            load_demo_battle_scene.after(load_battle_asset_resources),
        )
        .add_systems(
            Update,
            (
                // Grid Movement + Transform
                grid::sync_grid_movement_to_transform,
                grid::sync_grid_position_to_transform,
                grid::sync_grid_positions_to_manager,
                grid_cursor::handle_cursor_movement,
                // Unit Movement + Overlay UI
                handle_overlays_events_system,
                handle_unit_movement,
                // Animation
                animate_sprite,
                update_sprite_on_animation_change,
                update_facing_direction,
                // Battle Camera Zoom
                change_zoom,
                // on_asset_event
            )
                .run_if(in_state(GameState::Battle)),
        );
}

const DEMO_SQUARE_GRID_BOUNDS: u32 = 8;

pub fn load_battle_asset_resources(
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

    startup_load_tinytactics_assets(&mut commands, &asset_server, &mut texture_atlas_layouts);
}

/// Loads necessary assets and resources to
/// create a battle
///
/// TODO: Everything in this function should probably be loaded from some
/// data representation as opposed to just hardcoded here.
pub fn load_demo_battle_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tt_assets: Res<TinytacticsAssets>,
) {
    let map_handle: Handle<TiledMapAsset> = asset_server.load(EXAMPLE_MAP_PATH);
    commands.spawn(TiledMap(map_handle));

    commands.insert_resource(grid::GridManagerResource {
        grid_manager: GridManager::new(DEMO_SQUARE_GRID_BOUNDS, DEMO_SQUARE_GRID_BOUNDS),
    });

    // Spawn players and player cursors
    let cursor_image: Handle<Image> = asset_server.load(CURSOR_PATH);

    let player_1_grid_pos = GridPosition { x: 4, y: 6 };
    let player_2_grid_pos = GridPosition { x: 1, y: 3 };

    load_demo_battle_players(&mut commands);

    spawn_unit(
        &mut commands,
        &tt_assets,
        player_1_grid_pos,
        tt_assets.fighter_spritesheet.clone(),
        tt_assets.layout.clone(),
        Player::One,
        PLAYER_TEAM,
    );
    spawn_unit(
        &mut commands,
        &tt_assets,
        player_2_grid_pos,
        tt_assets.mage_spritesheet.clone(),
        tt_assets.layout.clone(),
        Player::Two,
        PLAYER_TEAM,
    );

    grid_cursor::spawn_cursor(
        &mut commands,
        cursor_image.clone(),
        player::Player::One,
        player_1_grid_pos,
    );

    grid_cursor::spawn_cursor(
        &mut commands,
        cursor_image.clone(),
        player::Player::Two,
        player_2_grid_pos,
    );

    let door_location = GridPosition { x: 7, y: 1 };

    // Spawn Obstacles (All walls / corners except the door) + Stools
    let stool_locations = [
        GridPosition { x: 2, y: 3 },
        GridPosition { x: 4, y: 1 },
        GridPosition { x: 4, y: 3 },
        GridPosition { x: 4, y: 5 },
        GridPosition { x: 6, y: 3 },
    ];

    let mut obstacle_locations = Vec::new();
    for i in 0..DEMO_SQUARE_GRID_BOUNDS {
        obstacle_locations.push(GridPosition { x: 0, y: i });
        obstacle_locations.push(GridPosition { x: i, y: 0 });
        obstacle_locations.push(GridPosition {
            x: i,
            y: DEMO_SQUARE_GRID_BOUNDS - 1,
        });
        obstacle_locations.push(GridPosition {
            x: DEMO_SQUARE_GRID_BOUNDS - 1,
            y: i,
        });
    }

    // Remove door location
    obstacle_locations.retain(|t| *t != door_location);

    obstacle_locations.extend_from_slice(&stool_locations);

    for obstacle_location in obstacle_locations {
        spawn_obstacle_unit(&mut commands, obstacle_location);
    }
}

// TODO: This should be based on how many players have joined game,
// and likely should happen on some form of Player Join Screen
fn load_demo_battle_players(commands: &mut Commands) {
    commands.insert_resource(player::PlayerGameStates {
        player_state: HashMap::from([
            (Player::One, player::PlayerState::default()),
            (Player::Two, player::PlayerState::default()),
        ]),
    });
}
