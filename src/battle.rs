//! Top level module for a Battle

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_common_assets::json::JsonAssetPlugin;
use bevy_ecs_tiled::prelude::{TiledMap, TiledMapAsset};

use crate::{
    GameState,
    animation::{
        AnimationMarkerMessage, TinytacticsAssets,
        combat::{apply_animation_on_attack_phase, update_facing_direction_on_attack},
        idle_animation_system, startup_load_tinytactics_assets,
        tinytactics::AnimationAsset,
        unit_animation_tick_system, update_facing_direction_on_movement,
    },
    assets::{CURSOR_PATH, EXAMPLE_MAP_2_PATH, EXAMPLE_MAP_PATH, GRADIENT_PATH, OVERLAY_PATH},
    battle_menu::{
        activate_battle_ui, battle_ui_setup, handle_battle_ui_interactions, update_player_ui_info,
    },
    battle_phase::{
        PhaseMessage, check_should_advance_phase, init_phase_system, is_running_enemy_phase,
        is_running_player_phase, refresh_units_at_beginning_of_phase,
    },
    bevy_ecs_tilemap_example,
    camera::change_zoom,
    combat::{
        advance_attack_phase_based_on_attack_animation_markers, attack_execution_despawner,
        attack_impact_system, attack_intent_system,
    },
    enemy::{
        begin_enemy_phase, execute_enemy_action, init_enemy_ai_system, plan_enemy_action,
        resolve_enemy_action, select_next_enemy,
    },
    grid::{self, GridManager, GridPosition},
    grid_cursor,
    menu::menu_navigation::{handle_menu_cursor_navigation, highlight_menu_option},
    player::{self, Player},
    unit::{
        ENEMY_TEAM, PLAYER_TEAM, UnitActionCompletedMessage, UnitExecuteActionMessage,
        execute_unit_actions, handle_unit_cursor_actions, handle_unit_ui_command,
        overlay::{OverlaysMessage, TileOverlayAssets, handle_overlays_events_system},
        spawn_enemy, spawn_obstacle_unit, spawn_unit, unlock_cursor_after_unit_command,
    },
};

// TODO: Need to decide how we want to
// represent enemy's vs. teams.
#[derive(Component)]
pub struct Enemy {}

#[derive(Message, Debug)]
pub struct UnitSelectionMessage {
    /// Unit that was selected
    pub entity: Entity,
    /// Player that selected entity
    pub player: Player,
}

#[derive(Message, Debug, Clone)]
pub struct UnitUiCommandMessage {
    /// Player that sent command
    pub player: Player,
    /// The Command itself
    pub command: UnitCommand,
    /// The Entity for the Unit the command is about
    pub unit: Entity,
}

#[derive(Clone, Debug)]
pub enum UnitCommand {
    Move,
    Attack,
    Wait,
    Cancel,
}

/// All logic necessary during a battle
pub fn battle_plugin(app: &mut App) {
    app.add_message::<OverlaysMessage>()
        .add_message::<UnitSelectionMessage>()
        .add_message::<UnitUiCommandMessage>()
        .add_message::<AnimationMarkerMessage>()
        .add_message::<PhaseMessage>()
        .add_message::<UnitActionCompletedMessage>()
        .add_message::<UnitExecuteActionMessage>()
        // .add_plugins(TiledPlugin::default())
        // .add_plugins(TiledDebugPluginGroup)
        .add_plugins((
            TilemapPlugin,
            bevy_ecs_tilemap_example::tiled::TiledMapPlugin,
        ))
        .add_plugins(JsonAssetPlugin::<AnimationAsset>::new(&[".json"]))
        .add_systems(OnEnter(GameState::Battle), load_battle_asset_resources)
        .add_systems(
            OnEnter(GameState::Battle),
            (
                load_demo_battle_scene_2.after(load_battle_asset_resources),
                init_phase_system,
                init_enemy_ai_system,
                battle_ui_setup,
            ),
        )
        .add_systems(
            Update,
            (
                check_should_advance_phase::<Player>,
                refresh_units_at_beginning_of_phase::<Player>
                    .after(check_should_advance_phase::<Player>),
                check_should_advance_phase::<Enemy>,
                refresh_units_at_beginning_of_phase::<Enemy>
                    .after(check_should_advance_phase::<Enemy>),
            )
                .run_if(in_state(GameState::Battle)),
        )
        .add_systems(
            Update,
            (
                // Grid Movement + Transform
                grid::resolve_grid_movement,
                grid::sync_grid_position_to_transform,
                grid::sync_grid_positions_to_manager,
                grid_cursor::handle_cursor_movement,
                // Unit Movement + Overlay UI
                handle_overlays_events_system,
                handle_unit_ui_command,
                activate_battle_ui.run_if(is_running_player_phase),
                handle_battle_ui_interactions.run_if(is_running_player_phase),
                unlock_cursor_after_unit_command.after(handle_unit_ui_command),
                // Player UI System
                handle_unit_cursor_actions.run_if(is_running_player_phase),
                execute_unit_actions,
                // Menu UI
                highlight_menu_option,
                handle_menu_cursor_navigation,
                // Combat
                attack_intent_system,
                attack_impact_system,
                attack_execution_despawner,
                // Battle Camera Zoom
                change_zoom,
                // UI
                update_player_ui_info,
            )
                .run_if(in_state(GameState::Battle)),
        )
        .add_systems(
            Update,
            (
                // Animation
                unit_animation_tick_system,
                update_facing_direction_on_movement,
                idle_animation_system,
                // AnimationCombat
                advance_attack_phase_based_on_attack_animation_markers,
                apply_animation_on_attack_phase,
                update_facing_direction_on_attack,
            )
                .run_if(in_state(GameState::Battle)),
        )
        .add_systems(
            Update,
            (
                begin_enemy_phase,
                select_next_enemy,
                plan_enemy_action,
                execute_enemy_action,
                resolve_enemy_action,
            )
                .chain()
                .after(refresh_units_at_beginning_of_phase::<Enemy>)
                .run_if(in_state(GameState::Battle))
                .run_if(is_running_enemy_phase),
        );
}

const DEMO_SQUARE_GRID_BOUNDS: u32 = 8;
const DEMO_2_GRID_BOUNDS_X: u32 = 12;
const DEMO_2_GRID_BOUNDS_Y: u32 = 7;

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
            let layout = TextureAtlasLayout::from_grid(
                UVec2::new(grid::TILE_X_SIZE as u32, grid::TILE_Y_SIZE as u32),
                6,
                1,
                None,
                None,
            );
            texture_atlas_layouts.add(layout)
        },
        cursor_image: cursor_image.clone(),
    });

    startup_load_tinytactics_assets(&mut commands, &asset_server, &mut texture_atlas_layouts);
}

use bevy_ecs_tilemap::prelude::*;

pub fn load_demo_battle_scene_2(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tt_assets: Res<TinytacticsAssets>,
) {
    let map_handle =
        bevy_ecs_tilemap_example::tiled::TiledMapHandle(asset_server.load(EXAMPLE_MAP_2_PATH));

    // Spawn "Background Sprite"
    let background_image = asset_server.load(GRADIENT_PATH);
    commands.spawn((
        Sprite {
            image: background_image,
            texture_atlas: None,
            color: Color::linear_rgb(1.0, 1.0, 1.0),
            ..Default::default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, -10.0)),
    ));

    commands.spawn(bevy_ecs_tilemap_example::tiled::TiledMapBundle {
        tiled_map: map_handle,
        render_settings: TilemapRenderSettings {
            // Map size is 12x12 so we'll have render chunks that are:
            // 12 tiles wide and 1 tile tall.
            render_chunk_size: UVec2::new(3, 1),
            y_sort: true,
        },
        ..Default::default()
    });

    commands.insert_resource(grid::GridManagerResource {
        grid_manager: GridManager::new(DEMO_2_GRID_BOUNDS_X, DEMO_2_GRID_BOUNDS_Y),
    });

    // Spawn players and player cursors
    let cursor_image: Handle<Image> = asset_server.load(CURSOR_PATH);

    let player_1_grid_pos = GridPosition { x: 0, y: 1 };
    let player_2_grid_pos = GridPosition { x: 0, y: 5 };
    let enemy_1_grid_pos = GridPosition { x: 7, y: 3 };
    let enemy_2_grid_pos = GridPosition { x: 4, y: 2 };
    let enemy_3_grid_pos = GridPosition { x: 4, y: 4 };

    load_demo_battle_players(&mut commands);

    spawn_unit(
        &mut commands,
        "Brond".to_string(),
        &tt_assets,
        player_1_grid_pos,
        tt_assets.fighter_spritesheet.clone(),
        tt_assets.iron_axe_spritesheet.clone(),
        tt_assets.unit_layout.clone(),
        tt_assets.weapon_layout.clone(),
        Player::One,
        PLAYER_TEAM,
    );
    spawn_unit(
        &mut commands,
        "Coral".to_string(),
        &tt_assets,
        player_2_grid_pos,
        tt_assets.mage_spritesheet.clone(),
        tt_assets.scepter_spritesheet.clone(),
        tt_assets.unit_layout.clone(),
        tt_assets.weapon_layout.clone(),
        Player::Two,
        PLAYER_TEAM,
    );

    spawn_enemy(
        &mut commands,
        "Jimothy Timbers".to_string(),
        &tt_assets,
        enemy_1_grid_pos,
        tt_assets.cleric_spritesheet.clone(),
        tt_assets.unit_layout.clone(),
        ENEMY_TEAM,
    );

    spawn_enemy(
        &mut commands,
        "Chaumwer".to_string(),
        &tt_assets,
        enemy_2_grid_pos,
        tt_assets.cleric_spritesheet.clone(),
        tt_assets.unit_layout.clone(),
        ENEMY_TEAM,
    );

    spawn_enemy(
        &mut commands,
        "Deege".to_string(),
        &tt_assets,
        enemy_3_grid_pos,
        tt_assets.cleric_spritesheet.clone(),
        tt_assets.unit_layout.clone(),
        ENEMY_TEAM,
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

    // Spawn Obstacles
    let obstacle_locations = [
        GridPosition { x: 2, y: 0 },
        GridPosition { x: 2, y: 6 },
        GridPosition { x: 5, y: 1 },
        GridPosition { x: 7, y: 2 },
        GridPosition { x: 6, y: 5 },
        GridPosition { x: 10, y: 1 },
    ];

    let mut obstacle_entities = Vec::new();
    for obstacle_location in obstacle_locations {
        let e = spawn_obstacle_unit(&mut commands, obstacle_location);
        obstacle_entities.push(e);
    }

    let mut static_obstacles = commands.spawn(Name::new("Static Demo Map Obstacles"));
    static_obstacles.add_children(&obstacle_entities);
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

    let player_1_grid_pos = GridPosition { x: 1, y: 3 };
    let player_2_grid_pos = GridPosition { x: 4, y: 6 };
    let enemy_grid_pos = GridPosition { x: 5, y: 2 };

    load_demo_battle_players(&mut commands);

    spawn_unit(
        &mut commands,
        "Brond".to_string(),
        &tt_assets,
        player_1_grid_pos,
        tt_assets.fighter_spritesheet.clone(),
        tt_assets.iron_axe_spritesheet.clone(),
        tt_assets.unit_layout.clone(),
        tt_assets.weapon_layout.clone(),
        Player::One,
        PLAYER_TEAM,
    );
    spawn_unit(
        &mut commands,
        "Coral".to_string(),
        &tt_assets,
        player_2_grid_pos,
        tt_assets.mage_spritesheet.clone(),
        tt_assets.scepter_spritesheet.clone(),
        tt_assets.unit_layout.clone(),
        tt_assets.weapon_layout.clone(),
        Player::Two,
        PLAYER_TEAM,
    );

    spawn_enemy(
        &mut commands,
        "Jimothy Timbers".to_string(),
        &tt_assets,
        enemy_grid_pos,
        tt_assets.cleric_spritesheet.clone(),
        tt_assets.unit_layout.clone(),
        ENEMY_TEAM,
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

    let mut obstacle_entities = Vec::new();
    for obstacle_location in obstacle_locations {
        let e = spawn_obstacle_unit(&mut commands, obstacle_location);
        obstacle_entities.push(e);
    }

    let mut static_obstacles = commands.spawn(Name::new("Static Demo Map Obstacles"));
    static_obstacles.add_children(&obstacle_entities);
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
