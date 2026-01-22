use std::collections::{HashMap, HashSet};

use anyhow::Context;
use bevy::prelude::*;
use bevy_pkv::PkvStore;

use crate::{
    GameState,
    animation::{
        Direction, UnitAnimationKind,
        animation_db::{
            AnimatedSpriteId, AnimationDB, AnimationKey, AnimationStartIndexKey,
            RegisteredAnimationId,
            registered_sprite_ids::{TT_UNIT_ANIMATED_SPRITE_ID, UNIT_DEMO_SPRITE_ID},
        },
    },
    assets::{
        FontResource,
        sounds::{SoundManager, SoundManagerParam, SoundSettings, UiSound},
        sprite_db::{SpriteDB, SpriteId},
    },
    menu::{
        NestedDynamicMenu,
        menu_horizontal_selector::{HorizontalSelector, handle_horizontal_selection},
        menu_navigation::{
            ActiveMenu, GameMenuController, GameMenuGrid, GameMenuLatch,
            handle_menu_cursor_navigation, highlight_menu_option,
        },
        show_active_game_menu_only,
        ui_consts::{SELECTABLE_BUTTON_BACKGROUND, UI_CONFIRMED_BUTTON_COLOR, UI_MENU_BACKGROUND},
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

type InactiveGameMenuFilter = (
    With<PlayerGameMenu>,
    Without<ActiveMenu>,
    // Don't make the Game Menu disappear if we have JoinGameMenuPlayerReady!
    Without<JoinGameMenuPlayerReady>,
);
type ActiveGameMenuFilter = (With<PlayerGameMenu>, With<ActiveMenu>);

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
                show_active_game_menu_only::<InactiveGameMenuFilter, ActiveGameMenuFilter>,
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

pub fn join_game_menu_setup(mut commands: Commands, fonts: Res<FontResource>) {
    commands.insert_resource(JoinedPlayers::default());
    commands.insert_resource(RegisteredBattlePlayers::default());
    build_ui(&mut commands, &fonts);
}

fn build_ui(commands: &mut Commands, fonts: &FontResource) {
    let screen_space = commands
        .spawn((
            Node {
                height: percent(100),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            DespawnOnExit(GameState::JoinGame),
        ))
        .id();

    let top_banner = commands
        .spawn((
            Node {
                align_self: AlignSelf::FlexStart,
                height: percent(10),
                width: percent(100),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceEvenly,
                justify_items: JustifyItems::Center,
                ..Default::default()
            },
            children![(
                Text("Press \"J\" or LB and RB together to join the game".to_string()),
                TextFont {
                    font: fonts.pixelify_sans_regular.clone(),
                    ..Default::default()
                }
            )],
        ))
        .id();

    let bottom_space = commands
        .spawn((
            Node {
                align_self: AlignSelf::FlexEnd,
                height: percent(90),
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

    commands
        .entity(screen_space)
        .add_children(&[top_banner, bottom_space]);
}

#[derive(Component)]
pub struct PlayerGameMenu;

#[derive(Component)]
pub struct JobImageDisplay;

fn add_player_ui(
    commands: &mut Commands,
    fonts: &FontResource,
    anim_db: &AnimationDB,
    sprite_db: &SpriteDB,
    parent: Entity,
    player: Player,
) -> Entity {
    let font_settings = TextFont {
        font: fonts.pixelify_sans_regular.clone(),
        ..Default::default()
    };
    let player_block_container = commands
        .spawn((
            Node {
                height: percent(100),
                width: percent(24.),
                ..Default::default()
            },
            BackgroundColor(UI_MENU_BACKGROUND),
            BorderRadius::all(percent(20)),
        ))
        .id();

    let name_input_id = commands
        .spawn((
            Button,
            Node {
                width: percent(80),
                height: percent(10),
                border: UiRect::all(percent(0.5)),
                ..default()
            },
            TextInput,
            TextInputTextFont(TextFont {
                font_size: 34.,
                ..font_settings.clone()
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
            BorderRadius::all(percent(20)),
        ))
        .id();

    let placeholder_save = UnitSaveV1 {
        save_file_key: SaveFileKey {
            uid: 0,
            name: "lol".to_string(),
            color: SaveFileColor::Red,
        },
        job: UnitJob::Archer,
    };

    let (image, texture_atlas) =
        get_sprite_resources_for_job(anim_db, sprite_db, &placeholder_save, Direction::SE, true)
            .expect("Failed getting Sprite resources for hardcoded unit job");

    let character_job_selector = commands
        .spawn((
            Button,
            BorderColor::all(Color::NONE),
            Node {
                width: percent(80),
                height: percent(50),
                border: UiRect::all(percent(0.5)),
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            HorizontalSelector::new(&[
                UnitJob::Archer,
                UnitJob::Knight,
                UnitJob::Mercenary,
                UnitJob::Mage,
            ]),
            children![
                (
                    Text("Placeholder".to_string()),
                    JobNameDisplay,
                    font_settings.clone()
                ),
                (
                    JobImageDisplay,
                    Node {
                        width: Val::Px(128.),
                        height: Val::Px(128.),
                        justify_content: JustifyContent::Center,
                        align_content: AlignContent::Center,

                        ..Default::default()
                    },
                    ImageNode::from_atlas_image(image, texture_atlas)
                ),
                (
                    Text("Placeholder".to_string()),
                    JobDescriptionDisplay,
                    font_settings.clone()
                )
            ],
            BorderRadius::all(percent(20)),
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
                width: percent(80),
                height: percent(15),
                border: UiRect::all(percent(0.5)),
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            HorizontalSelector::new(&[
                SaveFileColor::Red,
                SaveFileColor::Blue,
                SaveFileColor::Green,
            ]),
            children![(
                Text("Save Color".to_string()),
                SaveFileColorText,
                font_settings.clone()
            ),],
            BorderRadius::all(percent(20)),
        ))
        .id();

    let create_character_button = commands
        .spawn((
            Button,
            Node {
                width: percent(80),
                height: percent(10),
                justify_content: JustifyContent::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,

                border: UiRect::all(percent(0.5)),
                ..Default::default()
            },
            children![(Text::new("Create Character"), font_settings.clone())],
            BackgroundColor(SELECTABLE_BUTTON_BACKGROUND),
            UiCommands::CreateCharacter(CreateCharacterCommand {
                text_input_entity: name_input_id,
                job_selector_entity: character_job_selector,
                color_selector_entity: character_color_selector,
            }),
            BorderRadius::all(percent(20)),
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
            BackgroundColor(UI_MENU_BACKGROUND),
            BorderRadius::all(percent(20)),
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
                width: percent(80),
                height: percent(20),
                justify_content: JustifyContent::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(percent(0.5)),
                ..Default::default()
            },
            children![(Text::new("New Character"), font_settings.clone())],
            BackgroundColor(SELECTABLE_BUTTON_BACKGROUND),
            BorderColor::all(Color::NONE),
            UiCommands::OpenNestedScreen(new_character_screen),
            BorderRadius::all(percent(20)),
        ))
        .id();

    let load_character_button = commands
        .spawn((
            Button,
            Node {
                width: percent(80),
                height: percent(20),
                justify_content: JustifyContent::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(percent(0.5)),
                ..Default::default()
            },
            children![(Text::new("Load Character"), font_settings.clone())],
            BackgroundColor(SELECTABLE_BUTTON_BACKGROUND),
            BorderColor::all(Color::NONE),
            UiCommands::OpenLoadCharacterScreen,
            BorderRadius::all(percent(20)),
        ))
        .id();

    let delete_all_button = commands
        .spawn((
            Button,
            Node {
                width: percent(80),
                height: percent(20),
                justify_content: JustifyContent::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(percent(0.5)),
                ..Default::default()
            },
            children![(Text::new("(DEV) Delete All Data"), font_settings.clone())],
            BackgroundColor(SELECTABLE_BUTTON_BACKGROUND),
            BorderColor::all(Color::NONE),
            UiCommands::ErasePkvData,
            BorderRadius::all(percent(20)),
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
                align_items: AlignItems::Center,
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
    fonts: &FontResource,
    anim_db: &AnimationDB,
    sprite_db: &SpriteDB,
    joined_players: &mut JoinedPlayers,
    player_ui_parent: Entity,
    controller: PlayerController,
) -> anyhow::Result<()> {
    // TODO: This is super stupid lol.
    let players_count = joined_players.0.len();
    let player = if players_count >= 4 {
        anyhow::bail!("Maximum number of players reached.");
    } else {
        let max_player_id = joined_players
            .0
            .keys()
            .map(|t| t.id())
            .max()
            .unwrap_or_default();

        Player::PlayerId(max_player_id + 1)
    };

    let input_map = match controller {
        PlayerController::Gamepad(entity) => Player::get_input_map_with_gamepad(entity),
        PlayerController::Keyboard => player.get_keyboard_input_map(),
    };

    let e = add_player_ui(
        commands,
        &fonts,
        anim_db,
        sprite_db,
        player_ui_parent,
        player,
    );
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
    fonts: Res<FontResource>,
    anim_db: Res<AnimationDB>,
    sprite_db: Res<SpriteDB>,
    mut joined_players: ResMut<JoinedPlayers>,
    sounds: Res<SoundManager>,
    sound_settings: Res<SoundSettings>,
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
                &fonts,
                &anim_db,
                &sprite_db,
                &mut joined_players,
                players_ui_container.entity(),
                PlayerController::Gamepad(gamepad_entity),
            ) {
                error!("Failed to add player: {:?}", e);
            } else {
                sounds.play_sound(&mut commands, &sound_settings, UiSound::OpenMenu);
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
                &fonts,
                &anim_db,
                &sprite_db,
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

pub struct JoinGameButtonEvent {
    /// The player that pressed the event
    player: Player,
    /// The command that was clicked
    command: UiCommands,
}

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
    character_creator_queries: (
        Query<&TextInputValue>,
        Query<&HorizontalSelector<UnitJob>>,
        Query<&HorizontalSelector<SaveFileColor>>,
    ),
    mut save_files: ResMut<SaveFiles>,
    mut pkv_store: ResMut<PkvStore>,
    anim_db: Res<AnimationDB>,
    sprite_db: Res<SpriteDB>,
    mut registered_players: ResMut<RegisteredBattlePlayers>,
    mut next_state: ResMut<NextState<GameState>>,
    sounds: SoundManagerParam,
    fonts: Res<FontResource>,
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

                sounds.play_sound(&mut commands, UiSound::Select);
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
                            character_creator_queries.0,
                            character_creator_queries.1,
                            character_creator_queries.2,
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

                        let unit_preview_screen = match build_unit_preview_screen(
                            &mut commands,
                            &fonts,
                            &sprite_db,
                            &anim_db,
                            save_info,
                            controlled_ui_block.entity,
                            *player,
                        ) {
                            Ok(screen) => screen,
                            Err(e) => {
                                error!("Failed building unit preview screen: {:?}", e);
                                continue;
                            }
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
                            &fonts,
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
                            &fonts,
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
                                    registered_players.save_files.insert(*k, t.clone());
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

pub fn get_sprite_resources(
    anim_db: &AnimationDB,
    sprite_db: &SpriteDB,
    sprite_id: SpriteId,
    animation_key: AnimationStartIndexKey,
) -> anyhow::Result<(Handle<Image>, TextureAtlas)> {
    let Some(image) = sprite_db.sprite_id_to_handle.get(&sprite_id).cloned() else {
        anyhow::bail!("No image found for SpriteId: {:?}", sprite_id);
    };

    let Some(start_index) = anim_db.get_start_index(&animation_key) else {
        anyhow::bail!("No Start index found for: {:?}", animation_key);
    };

    let Some(texture_atlas_handle) = anim_db.get_atlas(&animation_key.key.animated_sprite_id)
    else {
        anyhow::bail!("No texture atlas found for: {:?}", animation_key);
    };
    Ok((
        image,
        TextureAtlas {
            layout: texture_atlas_handle,
            index: *start_index as usize,
        },
    ))
}

/// TODO: Not stoked on this being derived from Job, but :shrug:
pub fn get_sprite_resources_for_job(
    anim_db: &AnimationDB,
    sprite_db: &SpriteDB,
    unit_save: &UnitSaveV1,
    direction: Direction,
    // Bit of a hack, but we don't want to use caroline's sprites in battle until we
    // have animated versions, but also I don't want two copies of this lookup table
    // since it's all gonna be the same eventually.
    use_caros_sprites: bool,
) -> anyhow::Result<(Handle<Image>, TextureAtlas)> {
    let (sprite_id, animated_sprite_id) = match use_caros_sprites {
        true => (unit_save.job.demo_sprite_id(), UNIT_DEMO_SPRITE_ID),
        false => (unit_save.job.base_sprite_id(), TT_UNIT_ANIMATED_SPRITE_ID),
    };

    let key = AnimationStartIndexKey {
        facing_direction: Some(direction),
        key: AnimationKey {
            animated_sprite_id,
            animation_id: UnitAnimationKind::IdleWalk.into(),
        },
    };

    get_sprite_resources(anim_db, sprite_db, sprite_id, key)
}

#[derive(Component)]
struct UnitPreviewScreen;

fn build_unit_preview_screen(
    commands: &mut Commands,
    fonts: &FontResource,
    sprite_db: &SpriteDB,
    anim_db: &AnimationDB,
    unit_save: UnitSaveV1,
    player_ui_parent: Entity,
    player: Player,
) -> anyhow::Result<Entity> {
    let unit_name = commands
        .spawn((
            Node {
                width: percent(80),
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
            children![(
                Text(unit_save.save_file_key.name.clone()),
                TextFont {
                    font: fonts.pixelify_sans_regular.clone(),
                    ..Default::default()
                }
            )],
            BorderRadius::all(percent(20)),
        ))
        .id();

    let (image, texture_atlas) =
        get_sprite_resources_for_job(anim_db, sprite_db, &unit_save, Direction::SE, true)
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
            BackgroundColor(SELECTABLE_BUTTON_BACKGROUND),
            UiCommands::PlayerReadyForBattle(player, unit_save),
            Node {
                width: percent(80),
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
            children![(
                Text::new("Ready!"),
                TextFont {
                    font: fonts.pixelify_sans_regular.clone(),
                    ..Default::default()
                }
            )],
            BorderRadius::all(percent(20)),
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
            BackgroundColor(UI_MENU_BACKGROUND),
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
            BorderRadius::all(percent(20)),
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
    fonts: &FontResource,
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
            BackgroundColor(UI_MENU_BACKGROUND),
            PlayerGameMenu,
            ActiveMenu {},
            GameMenuController {
                players: HashSet::from([player]),
            },
            GameMenuLatch::default(),
            BorderRadius::all(percent(20)),
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
                    width: percent(80),
                    height: percent(10),
                    justify_items: JustifyItems::Center,
                    justify_content: JustifyContent::SpaceEvenly,
                    align_items: AlignItems::Center,
                    align_content: AlignContent::SpaceEvenly,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                BackgroundColor(save_file_key.color.color()),
                UiCommands::LoadCharacter(save_file_key.clone()),
                children![(
                    Text(save_file_key.name.clone()),
                    TextFont {
                        font: fonts.pixelify_sans_regular.clone(),
                        ..Default::default()
                    }
                )],
                BorderRadius::all(percent(20)),
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
struct JobNameDisplay;
#[derive(Component)]
struct JobDescriptionDisplay;

fn display_job_info_horizontal_selector(
    query: Query<(&HorizontalSelector<UnitJob>, &Children), Changed<HorizontalSelector<UnitJob>>>,
    sprite_db: Res<SpriteDB>,
    // TODO: Performance? Should I put some marker on here?
    mut name_query: Query<&mut Text, With<JobNameDisplay>>,
    mut desc_query: Query<&mut Text, (With<JobDescriptionDisplay>, Without<JobNameDisplay>)>,
    mut image_query: Query<&mut ImageNode, With<JobImageDisplay>>,
) {
    for (selector, children) in query {
        if let Some(value) = selector.get_current() {
            for child in children {
                if let Ok(mut text) = name_query.get_mut(*child) {
                    text.0 = value.name();
                } else if let Ok(mut text) = desc_query.get_mut(*child) {
                    text.0 = value.description();
                } else if let Ok(mut ui_image) = image_query.get_mut(*child) {
                    if let Some(image) = sprite_db.sprite_id_to_handle.get(&value.demo_sprite_id())
                    {
                        ui_image.image = image.clone();
                    }
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
    sound_settings: Res<SoundSettings>,
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

                sounds.play_sound(&mut commands, &sound_settings, UiSound::Cancel);
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
        background_color.0 = UI_CONFIRMED_BUTTON_COLOR;
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
        background_color.0 = SELECTABLE_BUTTON_BACKGROUND;
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
