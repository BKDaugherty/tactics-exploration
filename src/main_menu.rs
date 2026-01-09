use std::collections::HashSet;

use bevy::{input_focus::InputDispatchPlugin, prelude::*};

use crate::{
    GameState,
    assets::{
        FontResource,
        sounds::{SoundManager, SoundSettings, UiSound},
    },
    menu::{
        NestedDynamicMenu, deselect_nested_menu,
        menu_horizontal_selector::{HorizontalSelector, handle_horizontal_selection},
        menu_navigation::{
            self, ActiveMenu, GameMenuGrid, GameMenuLatch, handle_menu_cursor_navigation,
            highlight_menu_option,
        },
        show_active_game_menu_only,
        ui_consts::NORMAL_MENU_BUTTON_COLOR,
    },
    player::Player,
};

pub fn main_menu_plugin(app: &mut App) {
    app.add_plugins(InputDispatchPlugin)
        .add_systems(
            OnEnter(GameState::MainMenu),
            (main_menu_setup, main_menu_initialized).chain(),
        )
        .add_systems(
            Update,
            (
                handle_menu_cursor_navigation,
                highlight_menu_option,
                deselect_nested_menu,
                show_active_game_menu_only::<
                    (With<MainMenuMarker>, Without<ActiveMenu>),
                    (With<MainMenuMarker>, With<ActiveMenu>),
                >,
                display_volume_text::<MusicVolumeSelector>,
                display_volume_text::<SfxVolumeSelector>,
                display_volume_text::<GlobalVolumeSelector>,
                handle_horizontal_selection::<f64>,
            )
                .run_if(in_state(GameState::MainMenu)),
        )
        .add_observer(main_menu_action);
}

pub struct SaveSettingsSubmit {
    global_volume_selector: Entity,
    music_volume_selector: Entity,
    sfx_volume_selector: Entity,
}

#[derive(Component)]
enum MainMenuButtonAction {
    PlayDemo,
    OpenSettings,
    // TODO: Maybe pull this out into its own thing?
    SaveSettings(SaveSettingsSubmit),
    Quit,
}

#[derive(Component)]
pub struct MainMenuMarker;

const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

fn main_menu_initialized(mut game_state: ResMut<NextState<GameState>>) {
    game_state.set(GameState::MainMenu);
}

#[derive(Component)]
pub struct GlobalVolumeSelector;

#[derive(Component)]
pub struct MusicVolumeSelector;

#[derive(Component)]
pub struct SfxVolumeSelector;

trait VolumeSelector: Component {
    const NAME: &str;

    fn text(v: f64) -> String {
        format!("{}: <- {}% ->", Self::NAME, v * 100.)
    }
}

impl VolumeSelector for GlobalVolumeSelector {
    const NAME: &str = "Global Volume";
}

impl VolumeSelector for MusicVolumeSelector {
    const NAME: &str = "Music Volume";
}

impl VolumeSelector for SfxVolumeSelector {
    const NAME: &str = "Sfx Volume";
}

// TODO: Do these actually need to be generic types?
fn display_volume_text<T: VolumeSelector>(
    query: Query<
        (&HorizontalSelector<f64>, &Children),
        (With<T>, Changed<HorizontalSelector<f64>>),
    >,
    mut display_query: Query<&mut Text, With<T>>,
) {
    for (selector, children) in query {
        if let Some(value) = selector.get_current() {
            for child in children {
                if let Ok(mut text) = display_query.get_mut(*child) {
                    text.0 = T::text(value);
                }
            }
        }
    }
}

fn build_settings_menu(
    commands: &mut Commands,
    font_resource: &FontResource,
    sound_settings: &SoundSettings,
) -> Entity {
    let button_node = Node {
        width: percent(60),
        height: percent(20),
        margin: UiRect::all(percent(0.5)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(percent(0.5)),
        ..default()
    };

    let button_text_font = TextFont {
        font_size: 25.0,
        font: font_resource.fine_fantasy.clone(),
        ..default()
    };

    let volume_options = Vec::from(&[0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0]);
    let mut selector = HorizontalSelector::new(&volume_options);
    selector.set_index(sound_settings.global_volume);
    let global_volume_selector = commands
        .spawn((
            Button,
            button_node.clone(),
            GlobalVolumeSelector,
            selector,
            children![(
                Text::default(),
                GlobalVolumeSelector,
                button_text_font.clone()
            )],
        ))
        .id();

    let mut selector = HorizontalSelector::new(&volume_options);
    selector.set_index(sound_settings.music_volume);
    let music_volume_selector = commands
        .spawn((
            Button,
            button_node.clone(),
            MusicVolumeSelector,
            selector,
            children![(
                Text::default(),
                MusicVolumeSelector,
                button_text_font.clone()
            )],
        ))
        .id();

    let mut selector = HorizontalSelector::new(&volume_options);
    selector.set_index(sound_settings.sfx_volume);
    let sfx_volume_selector = commands
        .spawn((
            Button,
            button_node.clone(),
            SfxVolumeSelector,
            selector,
            children![(Text::default(), SfxVolumeSelector, button_text_font.clone())],
        ))
        .id();

    let mut settings_grid = GameMenuGrid::new_vertical();
    let save_settings_button = commands
        .spawn((
            Button,
            BorderRadius::all(percent(20)),
            button_node.clone(),
            BackgroundColor(NORMAL_MENU_BUTTON_COLOR),
            MainMenuButtonAction::SaveSettings(SaveSettingsSubmit {
                global_volume_selector,
                music_volume_selector,
                sfx_volume_selector,
            }),
            children![(
                Text::new("Apply"),
                button_text_font.clone(),
                TextColor(TEXT_COLOR),
            ),],
        ))
        .id();

    settings_grid.push_buttons_to_stack(&[
        global_volume_selector,
        music_volume_selector,
        sfx_volume_selector,
        save_settings_button,
    ]);

    let settings_column = commands
        .spawn((
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                width: percent(40),
                height: percent(85),
                ..default()
            },
            BackgroundColor(Color::linear_rgb(0.2, 0.2, 0.2)),
            BorderRadius::all(percent(20)),
            children![
                // Display the game name
                (
                    Text::new("Settings"),
                    TextFont {
                        font_size: 40.0,
                        font: font_resource.badge.clone(),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Node {
                        margin: UiRect::all(percent(7.5)),
                        ..default()
                    },
                )
            ],
            settings_grid,
            menu_navigation::GameMenuController {
                players: HashSet::from([Player::PrePlayer]),
            },
            GameMenuLatch::default(),
            MainMenuMarker,
        ))
        .add_children(&[
            global_volume_selector,
            music_volume_selector,
            sfx_volume_selector,
            save_settings_button,
        ])
        .id();
    settings_column
}

fn main_menu_setup(mut commands: Commands, font_resource: Res<FontResource>) {
    let menu_screen = commands
        .spawn((
            DespawnOnExit(GameState::MainMenu),
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();

    // Common style for all buttons on the screen
    let button_node = Node {
        width: px(300),
        height: px(65),
        margin: UiRect::all(px(20)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(px(4)),
        ..default()
    };

    let button_text_font = TextFont {
        font_size: 33.0,
        font: font_resource.fine_fantasy.clone(),
        ..default()
    };

    let play_button = commands
        .spawn((
            Button,
            BorderRadius::all(percent(20)),
            button_node.clone(),
            BackgroundColor(NORMAL_MENU_BUTTON_COLOR),
            MainMenuButtonAction::PlayDemo,
            children![(
                Text::new("Play Demo"),
                button_text_font.clone(),
                TextColor(TEXT_COLOR),
            ),],
        ))
        .id();

    let settings_button = commands
        .spawn((
            Button,
            BorderRadius::all(percent(20)),
            button_node.clone(),
            BackgroundColor(NORMAL_MENU_BUTTON_COLOR),
            MainMenuButtonAction::OpenSettings,
            children![(
                Text::new("Settings"),
                button_text_font.clone(),
                TextColor(TEXT_COLOR),
            ),],
        ))
        .id();

    let quit_button = commands
        .spawn((
            Button,
            button_node.clone(),
            BorderRadius::all(percent(20)),
            BackgroundColor(NORMAL_MENU_BUTTON_COLOR),
            MainMenuButtonAction::Quit,
            children![(
                Text::new("Quit"),
                button_text_font.clone(),
                TextColor(TEXT_COLOR),
            ),],
        ))
        .id();

    let mut main_menu_grid = menu_navigation::GameMenuGrid::new_vertical();
    main_menu_grid.push_buttons_to_stack(&[play_button, settings_button, quit_button]);

    let mut main_menu_column = commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::linear_rgb(0.2, 0.2, 0.2)),
        BorderRadius::all(percent(20)),
        children![
            // Display the game name
            (
                Text::new("Tactics Exploration"),
                TextFont {
                    font_size: 67.0,
                    font: font_resource.badge.clone(),
                    ..default()
                },
                TextColor(TEXT_COLOR),
                Node {
                    margin: UiRect::all(px(50)),
                    ..default()
                },
            )
        ],
        main_menu_grid,
        menu_navigation::GameMenuController {
            players: HashSet::from([Player::PrePlayer]),
        },
        ActiveMenu {},
        GameMenuLatch::default(),
        MainMenuMarker,
    ));

    main_menu_column.add_children(&[play_button, settings_button, quit_button]);
    let menu_column_id = main_menu_column.id();

    let mut menu_screen = commands.entity(menu_screen);
    menu_screen.add_children(&[menu_column_id]);
}

fn main_menu_action(
    mut click: On<Pointer<Click>>,
    mut commands: Commands,
    menu_button: Query<&MainMenuButtonAction, With<Button>>,
    sounds: Res<SoundManager>,
    mut app_exit_writer: MessageWriter<AppExit>,
    mut game_state: ResMut<NextState<GameState>>,
    parent_query: Query<&ChildOf>,
    setting_query: Query<&HorizontalSelector<f64>>,
    fonts: Res<FontResource>,
    mut sound_settings: ResMut<SoundSettings>,
    global_volume: ResMut<GlobalVolume>,
) {
    let button_entity = click.entity;
    if let Ok(menu_button_action) = menu_button.get(button_entity) {
        sounds.play_sound(&mut commands, &sound_settings, UiSound::Select);

        click.propagate(false);
        match menu_button_action {
            MainMenuButtonAction::Quit => {
                app_exit_writer.write(AppExit::Success);
            }
            MainMenuButtonAction::PlayDemo => {
                info!("Got signal to JoinGame!");
                game_state.set(GameState::JoinGame);
            }
            MainMenuButtonAction::OpenSettings => {
                let Some(ui) = parent_query.get(button_entity).ok() else {
                    error!("No UI parent for OpenSettings Button?");
                    return;
                };
                let main_menu_column = ui.parent();

                let Some(menu_screen) = parent_query.get(main_menu_column).ok() else {
                    error!("No parent for MainMenu column?");
                    return;
                };

                commands.entity(main_menu_column).remove::<ActiveMenu>();
                let settings = build_settings_menu(&mut commands, &fonts, &sound_settings);
                commands.entity(settings).insert((
                    ActiveMenu {},
                    NestedDynamicMenu {
                        parent: main_menu_column,
                    },
                ));

                commands.entity(menu_screen.parent()).add_child(settings);
            }
            MainMenuButtonAction::SaveSettings(SaveSettingsSubmit {
                global_volume_selector,
                music_volume_selector,
                sfx_volume_selector,
            }) => {
                let Some(global_volume) = setting_query
                    .get(*global_volume_selector)
                    .ok()
                    .map(|t| t.get_current())
                    .flatten()
                else {
                    error!("No Global Volume!");
                    return;
                };

                let Some(music_volume) = setting_query
                    .get(*music_volume_selector)
                    .ok()
                    .map(|t| t.get_current())
                    .flatten()
                else {
                    error!("No Global Volume!");
                    return;
                };

                let Some(sfx_volume) = setting_query
                    .get(*sfx_volume_selector)
                    .ok()
                    .map(|t| t.get_current())
                    .flatten()
                else {
                    error!("No Global Volume!");
                    return;
                };

                sound_settings.global_volume = global_volume;
                sound_settings.music_volume = music_volume;
                sound_settings.sfx_volume = sfx_volume;

                info!("Updated Sound Settings: {:?}", sound_settings)
            }
        }
    }
}
