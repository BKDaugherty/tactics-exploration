use std::collections::{HashMap, HashSet};

use anyhow::Context;
use bevy::prelude::*;
use bevy_pkv::PkvStore;

use crate::{
    GameState,
    animation::{
        Direction, UnitAnimationKind,
        animation_db::{
            AnimationDB, AnimationKey, AnimationStartIndexKey,
            registered_sprite_ids::TT_UNIT_ANIMATED_SPRITE_ID,
        },
    },
    assets::{
        sounds::{SoundManager, UiSound},
        sprite_db::SpriteDB,
    },
    battle_menu::player_battle_ui_systems::NestedDynamicMenu,
    menu::menu_navigation::{
        ActiveMenu, GameMenuController, GameMenuGrid, GameMenuLatch, check_latch_on_axis_move,
        handle_menu_cursor_navigation, highlight_menu_option,
    },
    player::{self, Player, RegisteredBattlePlayers},
    save_game::{
        SaveFileColor, SaveFileKey, SaveFiles, UnitSave, UnitSaveV1, upgrade_save_file_to_latest,
    },
    unit::jobs::UnitJob,
};

use bevy_simple_text_input::{
    TextInput, TextInputInactive, TextInputPlaceholder, TextInputPlugin, TextInputSettings,
    TextInputTextFont, TextInputValue,
};

pub fn join_game_plugin(app: &mut App) {
    app.add_plugins(TextInputPlugin)
        .add_systems(
            OnEnter(GameState::JoinGame),
            (join_game_cleanup, join_game_menu_setup).chain(),
        )
        .add_systems(
            Update,
            (
                handle_menu_cursor_navigation,
                highlight_menu_option,
                wait_for_joining_player,
                show_the_active_player_game_menu_only,
                handle_unload_unit,
                handle_button_commands,
                handle_horizontal_selection::<UnitJob>,
                handle_horizontal_selection::<SaveFileColor>,
                display_job_info_horizontal_selector,
                display_colors_for_horizontal_selector,
                handle_deselect_join_game_ready,
            )
                .run_if(in_state(GameState::JoinGame)),
        )
        .add_observer(highlight_button_on_join_game_added)
        .add_observer(highlight_button_on_join_game_removed);
}

#[derive(Resource, Default)]
pub struct JoinedPlayers(pub HashMap<Player, JoinedPlayerData>);

#[derive(Debug, Clone, Reflect, Default)]
pub enum LoadedUnitState {
    #[default]
    NoUnit,
    LoadedUnit(UnitSaveV1),
    ReadyUnit(UnitSaveV1),
}

#[derive(Clone, Debug, Reflect)]
pub struct JoinedPlayerData {
    controller: PlayerController,
    input_entity: Entity,
    unit_state: LoadedUnitState,
}

#[derive(Debug, Clone, Copy, Reflect)]
pub enum PlayerController {
    Gamepad(Entity),
    Keyboard,
}

/// Marker component for the PlayersUIContainer
#[derive(Component)]
pub struct PlayersUIContainer;

pub fn join_game_cleanup(
    mut commands: Commands,
    query: Query<Entity, With<JoinedPlayerSpecificInputManager>>,
) {
    for e in query {
        commands.entity(e).despawn();
    }
}

pub fn join_game_menu_setup(mut commands: Commands) {
    commands.insert_resource(JoinedPlayers::default());
    commands.insert_resource(RegisteredBattlePlayers {
        players: HashMap::new(),
    });
    build_ui(&mut commands);
}

fn build_ui(commands: &mut Commands) {
    let screen_space = commands
        .spawn((
            Node {
                height: percent(100),
                width: percent(100),
                ..Default::default()
            },
            DespawnOnExit(GameState::JoinGame),
        ))
        .id();

    let bottom_space = commands
        .spawn((
            Node {
                align_self: AlignSelf::FlexEnd,
                height: percent(100),
                width: percent(100),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceEvenly,
                justify_items: JustifyItems::Center,
                ..Default::default()
            },
            PlayersUIContainer,
        ))
        .id();

    commands.entity(screen_space).add_child(bottom_space);
}

#[derive(Component)]
pub struct PlayerGameMenu;

/// Maybe this should be an observer or something?
fn show_the_active_player_game_menu_only(
    mut inactive_menu: Query<
        &mut Node,
        (
            With<PlayerGameMenu>,
            Without<ActiveMenu>,
            Without<JoinGameMenuPlayerReady>,
        ),
    >,
    mut active_menu: Query<&mut Node, (With<PlayerGameMenu>, With<ActiveMenu>)>,
) {
    for mut node in active_menu.iter_mut() {
        node.display = Display::Flex;
    }

    for mut node in inactive_menu.iter_mut() {
        node.display = Display::None;
    }
}

fn add_player_ui(commands: &mut Commands, parent: Entity, player: Player) -> Entity {
    let player_block_container = commands
        .spawn((
            Node {
                height: percent(100),
                width: percent(24.),
                ..Default::default()
            },
            BackgroundColor(Color::linear_rgb(0.2, 0.2, 0.2)),
        ))
        .id();

    let name_input_id = commands
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: percent(20),
                border: UiRect::all(percent(0.5)),
                ..default()
            },
            TextInput,
            TextInputTextFont(TextFont {
                font_size: 34.,
                ..default()
            }),
            TextInputPlaceholder {
                value: "Enter Name".to_string(),
                ..default()
            },
            TextInputInactive(true),
            TextInputSettings {
                retain_on_submit: true,
                ..default()
            },
        ))
        .id();

    let character_job_selector = commands
        .spawn((
            Button,
            BorderColor::all(Color::NONE),
            Node {
                width: percent(100),
                height: percent(40),
                border: UiRect::all(percent(0.5)),
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            HorizontalSelector {
                current_index: 0,
                options: vec![
                    UnitJob::Archer,
                    UnitJob::Knight,
                    UnitJob::Mercenary,
                    UnitJob::Mage,
                ],
            },
            children![
                (Text("Placeholder".to_string()), JobNameDisplay),
                (Text("Placeholder".to_string()), JobDescriptionDisplay)
            ],
        ))
        .id();

    commands
        .entity(name_input_id)
        .insert(UiCommands::FocusTextInput(name_input_id));

    let character_color_selector = commands
        .spawn((
            Button,
            BorderColor::all(Color::NONE),
            Node {
                width: percent(100),
                height: percent(40),
                border: UiRect::all(percent(0.5)),
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            HorizontalSelector {
                current_index: 0,
                options: vec![
                    SaveFileColor::Red,
                    SaveFileColor::Blue,
                    SaveFileColor::Green,
                ],
            },
            children![(Text("Save Color".to_string()), SaveFileColorText),],
        ))
        .id();

    let create_character_button = commands
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: percent(20),
                justify_content: JustifyContent::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,

                border: UiRect::all(percent(0.5)),
                ..Default::default()
            },
            children![Text::new("Create Character")],
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::NONE),
            UiCommands::CreateCharacter(CreateCharacterCommand {
                text_input_entity: name_input_id,
                job_selector_entity: character_job_selector,
                color_selector_entity: character_color_selector,
            }),
        ))
        .id();

    let mut new_character_menu = GameMenuGrid::new_vertical();
    new_character_menu.push_buttons_to_stack(&[
        name_input_id,
        character_job_selector,
        character_color_selector,
        create_character_button,
    ]);

    let new_character_screen = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                display: Display::None,
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                align_content: AlignContent::SpaceEvenly,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            GameMenuController {
                players: HashSet::from([player]),
            },
            GameMenuLatch::default(),
            PlayerGameMenu,
            new_character_menu,
        ))
        .add_children(&[
            name_input_id,
            character_job_selector,
            character_color_selector,
            create_character_button,
        ])
        .id();

    let mut menu = GameMenuGrid::new_vertical();
    let new_character_button = commands
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: percent(20),
                justify_content: JustifyContent::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(percent(0.5)),
                ..Default::default()
            },
            children![Text::new("New Character")],
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::NONE),
            UiCommands::OpenNestedScreen(new_character_screen),
        ))
        .id();

    let load_character_button = commands
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: percent(20),
                justify_content: JustifyContent::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(percent(0.5)),
                ..Default::default()
            },
            children![Text::new("Load Character")],
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::NONE),
            UiCommands::OpenLoadCharacterScreen,
        ))
        .id();

    let delete_all_button = commands
        .spawn((
            Button,
            Node {
                width: percent(100),
                height: percent(20),
                justify_content: JustifyContent::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(percent(0.5)),
                ..Default::default()
            },
            children![Text::new("(DEV) Delete All Data")],
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::NONE),
            UiCommands::ErasePkvData,
        ))
        .id();
    menu.push_buttons_to_stack(&[
        new_character_button,
        load_character_button,
        delete_all_button,
    ]);

    let character_load_or_new_screen = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                ..Default::default()
            },
            GameMenuController {
                players: HashSet::from([player]),
            },
            menu,
            ActiveMenu {},
            GameMenuLatch::default(),
            PlayerGameMenu,
        ))
        .add_children(&[
            new_character_button,
            load_character_button,
            delete_all_button,
        ])
        .id();

    commands
        .entity(player_block_container)
        .add_children(&[character_load_or_new_screen, new_character_screen]);

    commands.entity(parent).add_child(player_block_container);
    player_block_container
}

fn join_game(
    commands: &mut Commands,
    joined_players: &mut JoinedPlayers,
    player_ui_parent: Entity,
    controller: PlayerController,
) -> anyhow::Result<()> {
    // TODO: This is super stupid lol.
    let players_count = joined_players.0.len();
    // Limit to 2 as the BattleUI currently panics if Player::Three or Player::Four are
    // added. Need to define new offsets in `build_battle_grid_ui`
    let player = if players_count >= 2 {
        anyhow::bail!("Maximum number of players reached.");
    } else {
        match players_count {
            0 => Player::One,
            1 => Player::Two,
            _ => unreachable!(),
        }
    };

    let input_map = match controller {
        PlayerController::Gamepad(entity) => Player::get_input_map_with_gamepad(entity),
        PlayerController::Keyboard => player.get_keyboard_input_map(),
    };

    let e = add_player_ui(commands, player_ui_parent, player);
    let player_input = commands
        .spawn((
            input_map,
            player,
            ControlledUiBlock { entity: e },
            JoinedPlayerSpecificInputManager,
        ))
        .id();
    let _ = joined_players.0.insert(
        player,
        JoinedPlayerData {
            controller,
            input_entity: player_input,
            unit_state: LoadedUnitState::default(),
        },
    );

    Ok(())
}

#[derive(Component)]
pub struct JoinedPlayerSpecificInputManager;

/// Wait for specific inputs from a Gamepad or Controller to allow a player to join the game.
fn wait_for_joining_player(
    mut commands: Commands,
    mut joined_players: ResMut<JoinedPlayers>,
    sounds: Res<SoundManager>,
    gamepads: Query<(Entity, &Gamepad)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    players_ui_container: Single<Entity, With<PlayersUIContainer>>,
) {
    for (gamepad_entity, gamepad) in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::LeftTrigger)
            && gamepad.just_pressed(GamepadButton::RightTrigger)
        {
            if joined_players.0.iter().any(|(_, v)| match v.controller {
                PlayerController::Gamepad(e) => e == gamepad_entity,
                _ => false,
            }) {
                warn!("Gamepad {:?} is already registered!", gamepad_entity);
                continue;
            }

            if let Err(e) = join_game(
                &mut commands,
                &mut joined_players,
                players_ui_container.entity(),
                PlayerController::Gamepad(gamepad_entity),
            ) {
                error!("Failed to add player: {:?}", e);
            } else {
                sounds.play_sound(&mut commands, UiSound::OpenMenu);
            }
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyJ) {
        if !joined_players.0.iter().any(|(_, v)| match v.controller {
            PlayerController::Keyboard => true,
            _ => false,
        }) {
            if let Err(e) = join_game(
                &mut commands,
                &mut joined_players,
                players_ui_container.entity(),
                PlayerController::Keyboard,
            ) {
                error!("Failed to add player: {:?}", e);
            }
        } else {
            warn!("The Keyboard is already registered to a player!");
        }
    }
}

#[derive(Component)]
pub struct ControlledUiBlock {
    entity: Entity,
}

#[derive(Component)]
pub struct JoinGameMenuPlayerReady;

#[derive(Component)]
pub struct HasReadyButton {
    entity: Entity,
}

#[derive(Component)]
pub struct ReadyButtonMarker;

// TODO: This should probably just emit events with the Command tied to this Entity or something
// and then each system can handle the commands individually?
fn handle_button_commands(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &GameMenuController,
            &GameMenuGrid,
            Option<&NestedDynamicMenu>,
        ),
        With<ActiveMenu>,
    >,
    input_query: Query<(
        &player::Player,
        &ControlledUiBlock,
        &leafwing_input_manager::prelude::ActionState<player::PlayerInputAction>,
    )>,
    ui_command_query: Query<&UiCommands>,
    mut text_input_query: Query<(Entity, &mut TextInputInactive)>,
    mut joined_players: ResMut<JoinedPlayers>,
    text_input_value_query: Query<&TextInputValue>,
    job_selector: Query<&HorizontalSelector<UnitJob>>,
    color_selector: Query<&HorizontalSelector<SaveFileColor>>,
    mut save_files: ResMut<SaveFiles>,
    mut pkv_store: ResMut<PkvStore>,
    anim_db: Res<AnimationDB>,
    sprite_db: Res<SpriteDB>,
    mut registered_players: ResMut<RegisteredBattlePlayers>,
    mut next_state: ResMut<NextState<GameState>>,
    sounds: Res<SoundManager>,
) {
    for (player, controlled_ui_block, action_state) in input_query {
        for (menu_e, controller, menu, nested) in query {
            if !controller.players.contains(player) {
                continue;
            }

            if action_state.just_pressed(&player::PlayerInputAction::Select) {
                let Some(highlighted_option) = menu
                    .get_active_menu_option()
                    .and_then(|t| ui_command_query.get(*t).ok())
                else {
                    warn!("No option for menu?");
                    continue;
                };
                match highlighted_option {
                    UiCommands::FocusTextInput(entity) => {
                        for (text_e, mut text_input_active) in text_input_query.iter_mut() {
                            if text_e == *entity {
                                text_input_active.0 = !text_input_active.0;
                            } else {
                                text_input_active.0 = true;
                            }
                        }
                    }
                    UiCommands::OpenNestedScreen(entity) => {
                        commands.entity(menu_e).remove::<ActiveMenu>();
                        commands
                            .entity(*entity)
                            .insert((ActiveMenu {}, NestedDynamicMenu { parent: menu_e }));
                    }
                    UiCommands::CreateCharacter(command) => {
                        let save_info = match handle_create_character_command(
                            &mut save_files,
                            &mut pkv_store,
                            text_input_value_query,
                            job_selector,
                            color_selector,
                            command,
                        ) {
                            Err(e) => {
                                error!("Failed creating character: {:?}", e);
                                continue;
                            }
                            Ok(t) => t,
                        };

                        let Some(player_state) = joined_players.0.get_mut(player) else {
                            error!("No player state for player: {:?}", player);
                            continue;
                        };

                        player_state.unit_state = LoadedUnitState::LoadedUnit(save_info.clone());

                        // The current parent menu is the parent menu we want here surprisingly
                        // This UI logic is hot garbage and probably shouldn't be "dynamic"
                        let Some(parent) = nested.map(|t| t.parent) else {
                            error!(
                                "Somehow the Create Character Screen didn't have a parent menu!"
                            );
                            continue;
                        };

                        let Ok(unit_preview_screen) = build_unit_preview_screen(
                            &mut commands,
                            &sprite_db,
                            &anim_db,
                            save_info,
                            controlled_ui_block.entity,
                            *player,
                        ) else {
                            error!("Failed building unit preview screen");
                            continue;
                        };

                        commands.entity(menu_e).remove::<ActiveMenu>();

                        // Does this logically make sense? Or if you go back from here should you
                        // go back to the "New or Load Character Screen" because you have side effects?
                        commands
                            .entity(unit_preview_screen)
                            .insert((ActiveMenu {}, NestedDynamicMenu { parent }));
                    }
                    UiCommands::OpenLoadCharacterScreen => {
                        let load_file_screen = build_load_file_screen(
                            &mut commands,
                            &joined_players,
                            &save_files,
                            controlled_ui_block.entity,
                            *player,
                        );
                        // TODO: Maybe take active menu once you're spawned?
                        commands
                            .entity(load_file_screen)
                            .insert(NestedDynamicMenu { parent: menu_e });
                        commands.entity(menu_e).remove::<ActiveMenu>();
                    }
                    UiCommands::ErasePkvData => {
                        if let Err(e) = pkv_store.clear() {
                            error!("Failed to clear PKV store: {:?}", e);
                        }
                        save_files.save_file_keys.clear();
                    }
                    UiCommands::LoadCharacter(save_file_key) => {
                        // Check race condition to see if this already has been loaded
                        if joined_players.0.values().any(|t| match &t.unit_state {
                            LoadedUnitState::ReadyUnit(e) | LoadedUnitState::LoadedUnit(e) => {
                                e.save_file_key == *save_file_key
                            }
                            LoadedUnitState::NoUnit => false,
                        }) {
                            error!(
                                "Can't load file {:?} as it's already being used! Stale menus everywhere!",
                                save_file_key
                            );
                            continue;
                        }

                        let save_file = pkv_store.get::<UnitSave>(&save_file_key.pkv_key());

                        let Ok(save_file) = save_file else {
                            error!("Failed loading character: {:?}", save_file);
                            continue;
                        };

                        let Ok(v1_save) = upgrade_save_file_to_latest(save_file) else {
                            error!("Failed upgrading save file to latest version");
                            continue;
                        };

                        let Some(player_state) = joined_players.0.get_mut(&player) else {
                            error!("No player state for active player: {:?}", player);
                            continue;
                        };

                        player_state.unit_state = LoadedUnitState::LoadedUnit(v1_save.clone());

                        let Ok(unit_preview_screen) = build_unit_preview_screen(
                            &mut commands,
                            &sprite_db,
                            &anim_db,
                            v1_save,
                            controlled_ui_block.entity,
                            *player,
                        ) else {
                            error!("Failed building unit preview screen");
                            continue;
                        };

                        commands.entity(menu_e).remove::<ActiveMenu>();
                        commands
                            .entity(unit_preview_screen)
                            .insert((ActiveMenu {}, NestedDynamicMenu { parent: menu_e }));
                    }
                    UiCommands::PlayerReadyForBattle(player, save_info) => {
                        commands.entity(menu_e).remove::<ActiveMenu>();
                        commands.entity(menu_e).insert(JoinGameMenuPlayerReady);

                        let Some(player_state) = joined_players.0.get_mut(player) else {
                            error!("No player state for registered player: {:?}", player);
                            continue;
                        };

                        player_state.unit_state = LoadedUnitState::ReadyUnit(save_info.clone());

                        if joined_players
                            .0
                            .values()
                            .all(|t| matches!(t.unit_state, LoadedUnitState::ReadyUnit(..)))
                        {
                            for (k, value) in &joined_players.0 {
                                if let LoadedUnitState::ReadyUnit(t) = &value.unit_state {
                                    registered_players.players.insert(*k, t.clone());
                                }
                            }

                            next_state.set(GameState::Battle);
                        }
                    }
                }
            }

            if action_state.just_pressed(&player::PlayerInputAction::Deselect) {
                if let Some(parent) = nested.map(|t| t.parent) {
                    // TODO: This leaves some dangling menus!
                    commands.entity(menu_e).remove::<ActiveMenu>();
                    commands.entity(parent).insert(ActiveMenu {});

                    sounds.play_sound(&mut commands, UiSound::Cancel);
                } else {
                    // Despawn the players UI
                    commands.entity(controlled_ui_block.entity).despawn();

                    if let Some(t) = joined_players.0.remove(player) {
                        commands.entity(t.input_entity).despawn();
                    }

                    sounds.play_sound(&mut commands, UiSound::CloseMenu);
                }
            }
        }
    }
}

fn handle_deselect_join_game_ready(
    mut commands: Commands,
    mut joined_players: ResMut<JoinedPlayers>,
    query: Query<(Entity, &GameMenuController), With<JoinGameMenuPlayerReady>>,
    input_query: Query<(
        &player::Player,
        &leafwing_input_manager::prelude::ActionState<player::PlayerInputAction>,
    )>,
) {
    for (player, action_state) in input_query {
        for (e, controller) in query {
            if !controller.players.contains(player) {
                continue;
            }

            if action_state.just_pressed(&player::PlayerInputAction::Deselect) {
                let Some(t) = joined_players.0.get_mut(player) else {
                    error!("No player data for {:?}", player);
                    continue;
                };

                // Runtime state invariants are bad :(
                let LoadedUnitState::ReadyUnit(unit) = &t.unit_state else {
                    error!(
                        "UnitState invalid during DeselectJoinGameReady: {:?}",
                        t.unit_state
                    );
                    continue;
                };

                t.unit_state = LoadedUnitState::LoadedUnit(unit.clone());

                commands
                    .entity(e)
                    .insert(ActiveMenu {})
                    .remove::<JoinGameMenuPlayerReady>();
            }
        }
    }
}

/// TODO: Not stoked on this being derived from Job, but :shrug:
pub fn get_sprite_resources_for_job(
    anim_db: &AnimationDB,
    sprite_db: &SpriteDB,
    unit_save: &UnitSaveV1,
    direction: Direction,
) -> anyhow::Result<(Handle<Image>, TextureAtlas)> {
    let sprite_id = unit_save.job.base_sprite_id();

    let Some(image) = sprite_db.sprite_id_to_handle.get(&sprite_id).cloned() else {
        anyhow::bail!("No image found for SpriteId: {:?}", sprite_id);
    };

    let Some(start_index) = anim_db.get_start_index(&AnimationStartIndexKey {
        facing_direction: Some(direction.animation_direction()),
        key: AnimationKey {
            animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
            animation_id: UnitAnimationKind::IdleWalk.into(),
        },
    }) else {
        anyhow::bail!(
            "No Start index found for TT_UNIT_ANIMATED_SPRITE_ID: {:?}",
            TT_UNIT_ANIMATED_SPRITE_ID
        );
    };

    let Some(texture_atlas_handle) = anim_db.get_atlas(&TT_UNIT_ANIMATED_SPRITE_ID) else {
        anyhow::bail!(
            "No texture atlas found for AnimatedSpriteId: {:?}",
            TT_UNIT_ANIMATED_SPRITE_ID
        );
    };
    Ok((
        image,
        TextureAtlas {
            layout: texture_atlas_handle,
            index: *start_index as usize,
        },
    ))
}

#[derive(Component)]
struct UnitPreviewScreen;

fn build_unit_preview_screen(
    commands: &mut Commands,
    sprite_db: &SpriteDB,
    anim_db: &AnimationDB,
    unit_save: UnitSaveV1,
    player_ui_parent: Entity,
    player: Player,
) -> anyhow::Result<Entity> {
    let unit_name = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(10),
                border: UiRect::all(percent(0.5)),
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            BackgroundColor(unit_save.save_file_key.color.color()),
            children![Text(unit_save.save_file_key.name.clone())],
        ))
        .id();

    let (image, texture_atlas) =
        get_sprite_resources_for_job(anim_db, sprite_db, &unit_save, Direction::SE)
            .context("Getting Sprite resources for Unit Job")?;

    let unit_preview_image = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(40),
                justify_content: JustifyContent::Center,
                align_content: AlignContent::Center,
                ..Default::default()
            },
            ImageNode::from_atlas_image(image, texture_atlas),
        ))
        .id();

    let ready_button = commands
        .spawn((
            Button,
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::NONE),
            UiCommands::PlayerReadyForBattle(player, unit_save),
            Node {
                width: percent(100),
                height: percent(10),
                border: UiRect::all(percent(0.5)),
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ReadyButtonMarker,
            children![Text::new("Ready!")],
        ))
        .id();

    let mut menu = GameMenuGrid::new_vertical();
    menu.push_button_to_stack(ready_button);

    let unit_preview_screen = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::SpaceEvenly,
                justify_items: JustifyItems::Center,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                display: Display::None,
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
            PlayerGameMenu,
            ActiveMenu {},
            GameMenuController {
                players: HashSet::from([player]),
            },
            GameMenuLatch::default(),
            HasReadyButton {
                entity: ready_button,
            },
            menu,
            UnitPreviewScreen,
        ))
        .add_children(&[unit_name, unit_preview_image, ready_button])
        .id();

    commands
        .entity(player_ui_parent)
        .add_child(unit_preview_screen);

    Ok(unit_preview_screen)
}

fn build_load_file_screen(
    commands: &mut Commands,
    joined_players: &JoinedPlayers,
    files: &SaveFiles,
    player_ui_parent: Entity,
    player: Player,
) -> Entity {
    let mut load_menu = GameMenuGrid::new_vertical();
    let load_screen = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                display: Display::None,
                padding: UiRect::top(percent(1.)),
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
            PlayerGameMenu,
            ActiveMenu {},
            GameMenuController {
                players: HashSet::from([player]),
            },
            GameMenuLatch::default(),
        ))
        .id();

    for save_file_key in &files.save_file_keys {
        if joined_players
            .0
            .values()
            .filter_map(|t| match &t.unit_state {
                LoadedUnitState::NoUnit => None,
                LoadedUnitState::ReadyUnit(e) | LoadedUnitState::LoadedUnit(e) => Some(e),
            })
            .any(|t| t.save_file_key == *save_file_key)
        {
            continue;
        }

        let button = commands
            .spawn((
                Button,
                BorderColor::all(Color::NONE),
                Node {
                    width: percent(100),
                    height: percent(10),
                    border: UiRect::all(percent(0.5)),
                    justify_items: JustifyItems::Center,
                    justify_content: JustifyContent::SpaceEvenly,
                    align_items: AlignItems::Center,
                    align_content: AlignContent::SpaceEvenly,
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::top(percent(0.5)).with_bottom(percent(0.5)),
                    ..Default::default()
                },
                BackgroundColor(save_file_key.color.color()),
                UiCommands::LoadCharacter(save_file_key.clone()),
                children![Text(save_file_key.name.clone())],
            ))
            .id();
        load_menu.push_button_to_stack(button);
        commands.entity(load_screen).add_child(button);
    }

    commands.entity(load_screen).insert(load_menu);
    commands.entity(player_ui_parent).add_child(load_screen);
    load_screen
}

#[derive(Component)]
pub struct HorizontalSelector<T> {
    options: Vec<T>,
    current_index: u32,
}

pub enum HortDirection {
    East,
    West,
}

impl<T> HorizontalSelector<T> {
    pub fn apply_index(&mut self, h: HortDirection) {
        let mut current_index = self.current_index as i32;
        match h {
            HortDirection::East => current_index += 1,
            HortDirection::West => current_index -= 1,
        };

        let len = self.options.len();
        if current_index >= len as i32 {
            current_index = 0;
        } else if current_index < 0 {
            current_index = len as i32 - 1;
        }
        self.current_index = current_index as u32;
    }
}

impl<T: Clone> HorizontalSelector<T> {
    pub fn get_current(&self) -> Option<T> {
        self.options.get(self.current_index as usize).cloned()
    }
}

#[derive(Component)]
struct JobNameDisplay;
#[derive(Component)]
struct JobDescriptionDisplay;

fn display_job_info_horizontal_selector(
    query: Query<(&HorizontalSelector<UnitJob>, &Children), Changed<HorizontalSelector<UnitJob>>>,
    // TODO: Performance? Should I put some marker on here?
    mut name_query: Query<&mut Text, With<JobNameDisplay>>,
    mut desc_query: Query<&mut Text, (With<JobDescriptionDisplay>, Without<JobNameDisplay>)>,
) {
    for (selector, children) in query {
        if let Some(value) = selector.get_current() {
            for child in children {
                if let Ok(mut text) = name_query.get_mut(*child) {
                    text.0 = value.name();
                } else if let Ok(mut text) = desc_query.get_mut(*child) {
                    text.0 = value.description();
                }
            }
        }
    }
}

fn handle_unload_unit(
    mut commands: Commands,
    mut state: ResMut<JoinedPlayers>,
    input_query: Query<(
        &player::Player,
        &leafwing_input_manager::prelude::ActionState<player::PlayerInputAction>,
    )>,
    ui: Query<
        &GameMenuController,
        (
            With<UnitPreviewScreen>,
            With<ActiveMenu>,
            Without<JoinGameMenuPlayerReady>,
        ),
    >,
    sounds: Res<SoundManager>,
) {
    for controller in ui {
        for (player, input) in input_query {
            if !controller.players.contains(player) {
                continue;
            }

            if input.just_pressed(&player::PlayerInputAction::Deselect) {
                if let Some(player_data) = state.0.get_mut(player) {
                    info!(
                        "Player Unit State {:?} -> {:?}",
                        player_data.unit_state,
                        LoadedUnitState::NoUnit
                    );
                    player_data.unit_state = LoadedUnitState::NoUnit;
                }

                sounds.play_sound(&mut commands, UiSound::Cancel);
            }
        }
    }
}

fn highlight_button_on_join_game_added(
    added: On<Add, JoinGameMenuPlayerReady>,
    unit_preview_menu: Query<&HasReadyButton>,
    mut background_color: Query<&mut BackgroundColor, With<ReadyButtonMarker>>,
) {
    if let Some(mut background_color) = unit_preview_menu
        .get(added.entity)
        .ok()
        .map(|t| background_color.get_mut(t.entity).ok())
        .flatten()
    {
        background_color.0 = Color::linear_rgb(0.0, 0.5, 0.0);
    }
}

fn highlight_button_on_join_game_removed(
    remove: On<Remove, JoinGameMenuPlayerReady>,
    unit_preview_menu: Query<&HasReadyButton>,
    mut background_color: Query<&mut BackgroundColor, With<ReadyButtonMarker>>,
) {
    if let Some(mut background_color) = unit_preview_menu
        .get(remove.entity)
        .ok()
        .map(|t| background_color.get_mut(t.entity).ok())
        .flatten()
    {
        background_color.0 = Color::BLACK
    }
}

#[derive(Component)]
struct SaveFileColorText;

fn display_colors_for_horizontal_selector(
    query: Query<
        (
            &HorizontalSelector<SaveFileColor>,
            &mut BackgroundColor,
            &Children,
        ),
        Changed<HorizontalSelector<SaveFileColor>>,
    >,
    mut name_query: Query<&mut Text, With<SaveFileColorText>>,
) {
    for (selector, mut color, children) in query {
        if let Some(value) = selector.get_current() {
            color.0 = value.color();

            for child in children {
                if let Ok(mut text) = name_query.get_mut(*child) {
                    text.0 = format!("<- {} ->", value.name());
                }
            }
        }
    }
}

/// TODO: I feel like I'm abusing the GameMenuGrid a bit here and this
/// feels really inefficient
fn handle_horizontal_selection<T: Send + Sync + 'static>(
    mut commands: Commands,
    sounds: Res<SoundManager>,
    query: Query<(&GameMenuController, &GameMenuGrid, &GameMenuLatch), With<ActiveMenu>>,
    // I could put the latch here and then just have one system be in charge of updating the latch,
    // and others could read it?
    input_query: Query<(
        &player::Player,
        &leafwing_input_manager::prelude::ActionState<player::PlayerInputAction>,
    )>,
    mut hort_selector: Query<&mut HorizontalSelector<T>>,
) {
    for (controller, menu, latch) in query {
        let Some(mut hort_selector) = menu
            .get_active_menu_option()
            .and_then(|t| hort_selector.get_mut(*t).ok())
        else {
            continue;
        };

        for (player, action_state) in input_query {
            if !controller.players.contains(player) {
                continue;
            }

            // Don't update the latch here, as menu_cursor_navigation owns the latch
            if let Some(dir) = check_latch_on_axis_move(action_state, latch) {
                if dir == IVec2::X {
                    hort_selector.apply_index(HortDirection::East);
                    sounds.play_sound(&mut commands, UiSound::MoveCursor);
                } else if dir == -IVec2::X {
                    hort_selector.apply_index(HortDirection::West);
                    sounds.play_sound(&mut commands, UiSound::MoveCursor);
                }
            }

            if action_state.just_pressed(&player::PlayerInputAction::MoveCursorLeft) {
                hort_selector.apply_index(HortDirection::West);
                sounds.play_sound(&mut commands, UiSound::MoveCursor);
            }

            if action_state.just_pressed(&player::PlayerInputAction::MoveCursorRight) {
                hort_selector.apply_index(HortDirection::East);
                sounds.play_sound(&mut commands, UiSound::MoveCursor);
            }
        }
    }
}

#[derive(Component)]
enum UiCommands {
    FocusTextInput(Entity),
    OpenNestedScreen(Entity),
    OpenLoadCharacterScreen,
    CreateCharacter(CreateCharacterCommand),
    LoadCharacter(SaveFileKey),
    ErasePkvData,
    PlayerReadyForBattle(Player, UnitSaveV1),
}

fn handle_create_character_command(
    save_files: &mut SaveFiles,
    pkv: &mut PkvStore,
    text_input_query: Query<&TextInputValue>,
    job_selector: Query<&HorizontalSelector<UnitJob>>,
    color_selector: Query<&HorizontalSelector<SaveFileColor>>,
    create_character_submission: &CreateCharacterCommand,
) -> anyhow::Result<UnitSaveV1> {
    let Some(name) = text_input_query
        .get(create_character_submission.text_input_entity)
        .ok()
    else {
        anyhow::bail!("CreateCharacterForm Misconfigured. No Name Input")
    };

    let Some(hs) = job_selector
        .get(create_character_submission.job_selector_entity)
        .ok()
    else {
        anyhow::bail!("CreateCharacterForm Misconfigured. No Horizontal Selector for Job");
    };

    let Some(job) = hs.get_current() else {
        anyhow::bail!("CreateCharacterForm Misconfigured: Horizontal Selector out of range");
    };

    let Some(color) = color_selector
        .get(create_character_submission.color_selector_entity)
        .ok()
        .and_then(|t| t.get_current())
    else {
        anyhow::bail!("CreateCharacterForm Misconfigured: No SaveFileColor");
    };

    if name.0.is_empty() {
        anyhow::bail!("Name can't be empty!");
    }

    save_files.cursor = save_files.cursor.overflowing_add(1).0;
    let key = SaveFileKey {
        uid: save_files.cursor,
        name: name.0.clone(),
        color,
    };
    save_files.save_file_keys.push(key.clone());

    let unit_save = UnitSaveV1 {
        save_file_key: key.clone(),
        job,
    };

    // This clone is a bit expensive just to pass, I could return just the key in return type and require
    // the caller to pull from the DB, but probably fine for now.
    pkv.set(key.pkv_key(), &UnitSave::from(unit_save.clone()))
        .context("Failed saving unit to PKV store")?;

    Ok(unit_save)
}

pub struct CreateCharacterCommand {
    text_input_entity: Entity,
    job_selector_entity: Entity,
    color_selector_entity: Entity,
}
