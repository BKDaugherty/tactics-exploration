//! Top level module for a Battle

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_common_assets::json::JsonAssetPlugin;

use crate::{
    GameState,
    animation::{
        AnimationMarkerMessage, Direction, TinytacticsAssets,
        animation_db::{AnimationDB, load_animation_data},
        animation_follower_system, animation_tick_system,
        combat::update_facing_direction_on_attack,
        idle_animation_system, startup_load_tinytactics_assets,
        tinytactics::AnimationAsset,
        update_facing_direction_on_movement,
    },
    assets::{
        CURSOR_PATH, EXAMPLE_MAP_2_PATH, FontResource, GRADIENT_PATH, OVERLAY_PATH,
        sprite_db::{SpriteDB, build_sprite_db},
    },
    battle_menu::{
        battle_menu_ui_definition::{PlayerBattleMenu, battle_ui_setup},
        player_battle_ui_systems::{
            activate_battle_ui, clear_stale_battle_menus_on_activate,
            handle_battle_ui_interactions, reactivate_ui_on_back_message,
        },
        player_info_ui_systems::update_player_ui_info,
    },
    battle_phase::{
        PhaseMessage, StartOfPhaseEffectsMessage, TurnStartMessage,
        advance_after_start_of_phase_effects, check_for_active_effect_damage_on_turn_start,
        check_should_advance_phase, decrement_turn_count_effects_on_turn_start, init_phase_system,
        is_enemy_phase, is_running_enemy_phase, is_running_player_phase,
        phase_ui::{
            BattlePhaseMessageComplete, ShowBattleBannerMessage, banner_animation_system,
            spawn_banner_system,
        },
        prepare_for_phase, start_phase,
    },
    bevy_ecs_tilemap_example,
    camera::change_zoom,
    combat::{
        CombatStageComplete, ImpactEvent, attack_execution_despawner, attack_intent_system,
        check_combat_timeline_should_advance, cleanup_vfx_on_animation_complete,
        handle_combat_stage_enter, impact_event_handler, listen_for_combat_conditions,
        skills::{SkillId, UnitSkills, setup_skill_system},
    },
    enemy::{
        begin_enemy_phase, execute_enemy_action, init_enemy_ai_system, plan_enemy_action,
        resolve_enemy_action, select_next_enemy,
    },
    grid::{self, GridManager, GridPosition},
    grid_cursor,
    join_game_menu::get_sprite_resources_for_job,
    menu::{
        menu_navigation::{
            self, ActiveMenu, GameMenuLatch, handle_menu_cursor_navigation, highlight_menu_option,
        },
        show_active_game_menu_only,
        ui_consts::{SELECTABLE_BUTTON_BACKGROUND, UI_MENU_BACKGROUND, UI_TEXT_COLOR},
    },
    player::{self, Player, RegisteredBattlePlayers},
    projectile::{ProjectileArrived, projectile_arrival_system, projectile_bezier_system},
    unit::{
        CombatActionMarker, ENEMY_TEAM, ObstacleSprite, PLAYER_TEAM, Unit,
        UnitActionCompletedMessage, UnitExecuteActionMessage, execute_unit_actions,
        handle_unit_cursor_actions, handle_unit_ui_command,
        overlay::{OverlaysMessage, TileOverlayAssets, handle_overlays_events_system},
        spawn_enemy, spawn_obstacle_unit, spawn_unit, unlock_cursor_after_unit_ui_command,
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

/// If a player has selected a terminal node in the BattleUI, but then clicks back
/// they send this message to tell us not to reset the battle menu, but just bring it back
/// into focus.
///
/// TODO: Think about merging this into UnitSelectionMessage.
#[derive(Message)]
pub struct UnitSelectionBackMessage {
    /// The player that sent the message
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
    UseSkill(SkillId),
}

pub fn god_mode_plugin(app: &mut App) {
    app.add_systems(Update, handle_god_mode_input);
}

pub fn handle_god_mode_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_unit_query: Query<&mut Unit, (With<Player>, Without<Enemy>)>,
    mut enemy_unit_query: Query<&mut Unit, (With<Enemy>, Without<Player>)>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyP) {
        for mut player in player_unit_query.iter_mut() {
            player.stats.health = 0;
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyK) {
        for mut enemy in enemy_unit_query.iter_mut() {
            enemy.stats.health = 0;
        }
    }
}

/// All logic necessary during a battle
pub fn battle_plugin(app: &mut App) {
    app.add_message::<OverlaysMessage>()
        .add_message::<UnitSelectionMessage>()
        .add_message::<UnitUiCommandMessage>()
        .add_message::<UnitSelectionBackMessage>()
        .add_message::<AnimationMarkerMessage>()
        .add_message::<PhaseMessage>()
        .add_message::<UnitActionCompletedMessage>()
        .add_message::<UnitExecuteActionMessage>()
        .add_message::<ShowBattleBannerMessage>()
        .add_message::<BattlePhaseMessageComplete>()
        .add_message::<CombatStageComplete>()
        .add_message::<ImpactEvent>()
        .add_message::<ProjectileArrived>()
        .add_message::<TurnStartMessage>()
        .add_message::<StartOfPhaseEffectsMessage>()
        // .add_plugins(TiledPlugin::default())
        // .add_plugins(TiledDebugPluginGroup)
        .add_plugins((
            TilemapPlugin,
            bevy_ecs_tilemap_example::tiled::TiledMapPlugin,
        ))
        .add_plugins(JsonAssetPlugin::<AnimationAsset>::new(&[".json"]))
        .add_systems(
            OnEnter(GameState::Battle),
            (
                load_battle_asset_resources,
                load_animation_data,
                build_sprite_db,
            ),
        )
        .add_systems(
            OnEnter(GameState::Battle),
            (
                load_demo_battle_scene.after(load_battle_asset_resources),
                init_phase_system,
                init_enemy_ai_system,
                setup_skill_system,
                battle_ui_setup,
            ),
        )
        .add_systems(
            Update,
            (
                check_should_advance_phase::<Player>,
                check_should_advance_phase::<Enemy>,
                prepare_for_phase::<Player>.after(check_should_advance_phase::<Player>),
                prepare_for_phase::<Enemy>.after(check_should_advance_phase::<Enemy>),
                decrement_turn_count_effects_on_turn_start::<Player>,
                decrement_turn_count_effects_on_turn_start::<Enemy>,
                check_for_active_effect_damage_on_turn_start::<Player>,
                check_for_active_effect_damage_on_turn_start::<Enemy>,
                advance_after_start_of_phase_effects,
                spawn_banner_system,
                banner_animation_system,
                start_phase,
            )
                .run_if(in_state(GameState::Battle))
                .chain(),
        )
        .add_systems(
            Update,
            (begin_enemy_phase)
                .run_if(is_enemy_phase)
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
                clear_stale_battle_menus_on_activate.run_if(is_running_player_phase),
                handle_battle_ui_interactions.run_if(is_running_player_phase),
                // hide_battle_ui_on_unit_ui_command.run_if(is_running_player_phase),
                unlock_cursor_after_unit_ui_command.after(handle_battle_ui_interactions),
                // Player UI System
                handle_unit_cursor_actions.run_if(is_running_player_phase),
                reactivate_ui_on_back_message.run_if(is_running_player_phase),
                execute_unit_actions,
                // Menu UI
                highlight_menu_option,
                handle_menu_cursor_navigation,
                // Combat
                attack_intent_system,
                attack_execution_despawner,
                // Battle Camera Zoom
                // UI
                update_player_ui_info,
            )
                .run_if(in_state(GameState::Battle)),
        )
        .add_systems(Update, change_zoom.run_if(in_state(GameState::Battle)))
        .add_systems(
            Update,
            (
                // Animation
                animation_tick_system,
                animation_follower_system.after(animation_tick_system),
                update_facing_direction_on_movement,
                idle_animation_system,
                update_facing_direction_on_attack,
                cleanup_vfx_on_animation_complete,
            )
                .run_if(in_state(GameState::Battle)),
        )
        .add_systems(
            Update,
            (
                listen_for_combat_conditions,
                check_combat_timeline_should_advance,
                handle_combat_stage_enter,
                impact_event_handler,
            )
                .chain()
                .run_if(in_state(GameState::Battle)),
        )
        .add_systems(
            Update,
            (
                select_next_enemy,
                plan_enemy_action,
                execute_enemy_action,
                resolve_enemy_action,
            )
                .chain()
                .after(prepare_for_phase::<Enemy>)
                .run_if(in_state(GameState::Battle))
                .run_if(is_running_enemy_phase),
        )
        .add_systems(
            Update,
            (projectile_bezier_system, projectile_arrival_system)
                .run_if(in_state(GameState::Battle)),
        )
        .add_systems(
            Update,
            check_battle_complete.run_if(in_state(GameState::Battle)),
        )
        .add_systems(
            OnEnter(GameState::BattleResolution),
            spawn_battle_resolution_ui,
        )
        .add_systems(
            Update,
            show_active_game_menu_only::<
                (Without<ActiveMenu>, With<PlayerBattleMenu>),
                (With<ActiveMenu>, With<PlayerBattleMenu>),
            >,
        )
        .add_systems(
            Update,
            (handle_menu_cursor_navigation, highlight_menu_option)
                .run_if(in_state(GameState::BattleResolution)),
        )
        .add_observer(handle_battle_resolution_ui_buttons)
        .add_systems(OnExit(GameState::BattleResolution), cleanup_battle);
}

const DEMO_2_GRID_BOUNDS_X: u32 = 12;
const DEMO_2_GRID_BOUNDS_Y: u32 = 7;

#[derive(Debug)]
pub enum BattleEndCondition {
    Victory,
    Defeat,
}

#[derive(Resource)]
pub struct BattleResultResource(pub BattleResult);

#[derive(Debug)]
pub struct BattleResult {
    pub battle_condition: BattleEndCondition,
}

#[derive(Debug, Clone, Component)]
pub enum BattleResolutionMenuAction {
    MainMenu,
    Quit,
}

pub fn spawn_battle_resolution_ui(
    mut commands: Commands,
    battle_result: Res<BattleResultResource>,
    fonts: Res<FontResource>,
) {
    let ui_container = commands
        .spawn((
            Name::new("BattleResolutionUI"),
            BorderRadius::all(percent(20)),
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::SpaceEvenly,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                ..Default::default()
            },
            BattleEntity {},
        ))
        .id();

    let resolution_buttons_container = commands
        .spawn((
            Name::new("ResolutionButtonContainer"),
            Node {
                height: percent(40),
                width: percent(30),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                padding: UiRect::horizontal(percent(2)),
                ..Default::default()
            },
            BackgroundColor(UI_MENU_BACKGROUND),
            BorderRadius::all(percent(20)),
        ))
        .id();

    let button_node = Node {
        width: percent(100),
        height: percent(20),
        border: UiRect::all(px(2)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..Default::default()
    };

    let button_font = TextFont {
        font_size: 33.,
        font: fonts.badge.clone(),
        ..Default::default()
    };

    let (condition_text, color) = match battle_result.0.battle_condition {
        BattleEndCondition::Victory => ("Victory", Color::linear_rgb(0.4, 0.7, 0.4)),
        BattleEndCondition::Defeat => ("Defeat", Color::linear_rgb(0.7, 0.4, 0.4)),
    };

    let condition_node = commands
        .spawn((
            Name::new("BattleResolutionCondition"),
            Node {
                width: percent(40),
                height: percent(40),
                flex_direction: FlexDirection::Column,
                padding: UiRect::top(percent(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BorderRadius::all(percent(20)),
            BackgroundColor(UI_MENU_BACKGROUND),
            children![
                (
                    TextColor(color),
                    TextFont {
                        font: fonts.pixelify_sans_medium.clone(),
                        font_size: 65.,
                        ..Default::default()
                    },
                    Text(condition_text.to_string()),
                ),
                (
                    TextColor(UI_TEXT_COLOR),
                    TextFont {
                        font: fonts.pixelify_sans_regular.clone(),
                        font_size: 32.,
                        ..Default::default()
                    },
                    Text("Thanks for playing! :)".to_string()),
                )
            ],
        ))
        .id();

    let main_menu_button = commands
        .spawn((
            Name::new("MainMenuButton"),
            Button,
            BorderRadius::all(percent(20)),
            button_node.clone(),
            BackgroundColor(SELECTABLE_BUTTON_BACKGROUND),
            BattleResolutionMenuAction::MainMenu,
            children![(
                Text::new("Main Menu"),
                button_font.clone(),
                TextColor(Color::WHITE),
            ),],
        ))
        .id();

    let quit_button = commands
        .spawn((
            Name::new("QuitButton"),
            Button,
            BorderRadius::all(percent(20)),
            button_node.clone(),
            BackgroundColor(SELECTABLE_BUTTON_BACKGROUND),
            BattleResolutionMenuAction::Quit,
            children![(
                Text::new("Quit"),
                button_font.clone(),
                TextColor(Color::WHITE),
            ),],
        ))
        .id();

    let mut battle_resolution_menu = menu_navigation::GameMenuGrid::new_vertical();
    battle_resolution_menu.push_button_to_stack(main_menu_button);
    battle_resolution_menu.push_button_to_stack(quit_button);

    let menu = commands
        .spawn((
            battle_resolution_menu,
            menu_navigation::GameMenuController {
                players: HashSet::from([Player::PrePlayer]),
            },
            ActiveMenu {},
            GameMenuLatch::default(),
        ))
        .id();

    commands
        .entity(resolution_buttons_container)
        .add_children(&[main_menu_button, quit_button, menu]);

    commands
        .entity(ui_container)
        .add_children(&[condition_node, resolution_buttons_container]);
}

// TODO: Almost exactly the same code as `main_menu::main_menu_action`
//
// Not that it's complicated, but maybe worth visiting to see if there's a
// better paradigm we can use here. Currently I think the flexibility is probably
// worth keeping this level of duplication.
pub fn handle_battle_resolution_ui_buttons(
    mut click: On<Pointer<Click>>,
    menu_button: Query<&BattleResolutionMenuAction, With<Button>>,
    mut app_exit_writer: MessageWriter<AppExit>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    let button_entity = click.entity;
    if let Some(menu_button_action) = menu_button.get(button_entity).ok() {
        click.propagate(false);
        match menu_button_action {
            BattleResolutionMenuAction::Quit => {
                app_exit_writer.write(AppExit::Success);
            }
            BattleResolutionMenuAction::MainMenu => {
                game_state.set(GameState::MainMenu);
            }
        }
    }
}

// Naively assumes the BattleObjective is to defeat all enemies
pub fn check_battle_complete(
    mut commands: Commands,
    player_unit_query: Query<&Unit, With<Player>>,
    enemy_unit_query: Query<&Unit, With<Enemy>>,
    mut game_state: ResMut<NextState<GameState>>,
    combat_marker_query: Query<Entity, With<CombatActionMarker>>,
) {
    // Wait until combat is finished before calling the fight complete
    if !combat_marker_query.is_empty() {
        return;
    }

    // All Players have been downed :(
    if player_unit_query.iter().all(|t| t.downed()) {
        commands.insert_resource(BattleResultResource(BattleResult {
            battle_condition: BattleEndCondition::Defeat,
        }));
        game_state.set(GameState::BattleResolution);
    }
    // All Enemies have been downed :)
    else if enemy_unit_query.iter().all(|t| t.downed()) {
        commands.insert_resource(BattleResultResource(BattleResult {
            battle_condition: BattleEndCondition::Victory,
        }));
        game_state.set(GameState::BattleResolution);
    }
}

// TODO: Show an animation describing state probably before moving on to MainMenu
// Maybe even require some button input to move on
pub fn on_battle_resolution(
    result: Res<BattleResultResource>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    log::info!("Battle Result: {:?}", result.0.battle_condition);
    game_state.set(GameState::MainMenu);
}

/// A marker component for things that should get removed when the battle is over.
#[derive(Component)]
pub struct BattleEntity {}

pub fn cleanup_battle(
    mut commands: Commands,
    query: Query<Entity, With<BattleEntity>>,
    // TODO: Figure out a better way to clean up TileMaps that are *in*
    // the battle. Probably not a big deal atm, and I don't really want to touch
    // that tiled map loader code lol.
    tilemaps: Query<Entity, With<TilePos>>,
) {
    for e in tilemaps {
        commands.entity(e).despawn();
    }

    for e in query {
        commands.entity(e).despawn();
    }
}

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

pub fn spawn_background_gradient(mut commands: Commands, asset_server: Res<AssetServer>) {
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
}

pub fn load_demo_battle_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    registered_players: Res<RegisteredBattlePlayers>,
    tt_assets: Res<TinytacticsAssets>,
    anim_db: Res<AnimationDB>,
    sprite_db: Res<SpriteDB>,
) {
    let map_handle =
        bevy_ecs_tilemap_example::tiled::TiledMapHandle(asset_server.load(EXAMPLE_MAP_2_PATH));

    commands.spawn((
        bevy_ecs_tilemap_example::tiled::TiledMapBundle {
            tiled_map: map_handle,
            render_settings: TilemapRenderSettings {
                // Map size is 12x12 so we'll have render chunks that are:
                // 12 tiles wide and 1 tile tall.
                render_chunk_size: UVec2::new(3, 1),
                y_sort: true,
            },
            ..Default::default()
        },
        BattleEntity {},
    ));

    commands.insert_resource(grid::GridManagerResource {
        grid_manager: GridManager::new(DEMO_2_GRID_BOUNDS_X, DEMO_2_GRID_BOUNDS_Y),
    });

    // Spawn players and player cursors
    let cursor_image: Handle<Image> = asset_server.load(CURSOR_PATH);

    let mut valid_player_positions = vec![
        GridPosition { x: 0, y: 1 },
        GridPosition { x: 0, y: 2 },
        GridPosition { x: 0, y: 3 },
        GridPosition { x: 0, y: 4 },
        GridPosition { x: 0, y: 5 },
    ];

    let enemy_1_grid_pos = GridPosition { x: 7, y: 3 };
    let enemy_2_grid_pos = GridPosition { x: 4, y: 2 };
    let enemy_3_grid_pos = GridPosition { x: 4, y: 4 };

    load_demo_battle_players(&mut commands, &registered_players);

    for (player, player_unit_info) in registered_players.players.iter() {
        let Some(position) = valid_player_positions.pop() else {
            log::warn!("Not enough valid player positions for all registered players!");
            break;
        };

        let Ok((image, texture_atlas)) = get_sprite_resources_for_job(
            &anim_db,
            &sprite_db,
            player_unit_info,
            Direction::NE,
            false,
        ) else {
            error!(
                "Something went wrong while getting the sprite resources for the Unit's save file"
            );
            continue;
        };

        // TODO: Support equipment for a given player!
        let weapon_sheet = tt_assets.iron_axe_spritesheet.clone();

        spawn_unit(
            &mut commands,
            player_unit_info.save_file_key.name.to_string(),
            &tt_assets,
            position,
            image,
            texture_atlas,
            weapon_sheet,
            player_unit_info.job.base_unit_skills(),
            *player,
            PLAYER_TEAM,
            Direction::NE,
        );

        grid_cursor::spawn_cursor(&mut commands, cursor_image.clone(), *player, position);
    }

    if registered_players.players.len() > 1 {
        spawn_enemy(
            &mut commands,
            "Deege".to_string(),
            &tt_assets,
            &anim_db,
            enemy_2_grid_pos,
            tt_assets.cleric_spritesheet.clone(),
            UnitSkills {
                learned_skills: HashSet::new(),
                equipped_skill_categories: Vec::new(),
            },
            ENEMY_TEAM,
        );

        spawn_enemy(
            &mut commands,
            "Chaumwer".to_string(),
            &tt_assets,
            &anim_db,
            enemy_3_grid_pos,
            tt_assets.cleric_spritesheet.clone(),
            UnitSkills {
                learned_skills: HashSet::new(),
                equipped_skill_categories: Vec::new(),
            },
            ENEMY_TEAM,
        );
    }

    spawn_enemy(
        &mut commands,
        "Jimothy Timbers".to_string(),
        &tt_assets,
        &anim_db,
        enemy_1_grid_pos,
        tt_assets.cleric_spritesheet.clone(),
        UnitSkills {
            learned_skills: HashSet::new(),
            equipped_skill_categories: Vec::new(),
        },
        ENEMY_TEAM,
    );

    // Spawn Obstacles
    let obstacle_locations = [
        (GridPosition { x: 2, y: 0 }, ObstacleSprite::Bush),
        (GridPosition { x: 2, y: 6 }, ObstacleSprite::Bush),
        (GridPosition { x: 5, y: 1 }, ObstacleSprite::Rock),
        (GridPosition { x: 7, y: 2 }, ObstacleSprite::Rock),
        (GridPosition { x: 6, y: 5 }, ObstacleSprite::Rock),
        (GridPosition { x: 10, y: 1 }, ObstacleSprite::Rock),
    ];

    let mut obstacle_entities = Vec::new();
    for (obstacle_location, sprite_type) in obstacle_locations {
        let e = spawn_obstacle_unit(&mut commands, &tt_assets, obstacle_location, sprite_type);
        obstacle_entities.push(e);
    }
}

fn load_demo_battle_players(commands: &mut Commands, players: &RegisteredBattlePlayers) {
    let mut player_game_state_map = HashMap::new();

    for player in players.players.keys() {
        player_game_state_map.insert(*player, player::PlayerState::default());
    }

    commands.insert_resource(player::PlayerGameStates {
        player_state: player_game_state_map,
    });
}
