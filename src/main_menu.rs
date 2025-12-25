use std::collections::HashSet;

use bevy::{input_focus::InputDispatchPlugin, prelude::*};

use crate::{
    GameState,
    assets::FontResource,
    menu::{
        menu_navigation::{self, ActiveMenu, handle_menu_cursor_navigation, highlight_menu_option},
        ui_consts::NORMAL_MENU_BUTTON_COLOR,
    },
    player::Player,
};

#[derive(Component)]
pub struct OnMainMenuScreen {}

pub fn main_menu_plugin(app: &mut App) {
    app.add_plugins(InputDispatchPlugin)
        .add_systems(OnEnter(GameState::MainMenu), main_menu_setup)
        .add_systems(
            Update,
            (handle_menu_cursor_navigation, highlight_menu_option)
                .run_if(in_state(GameState::MainMenu)),
        )
        .add_observer(main_menu_action);
}

#[derive(Component)]
enum MainMenuButtonAction {
    PlayDemo,
    Quit,
}

const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

fn main_menu_setup(mut commands: Commands, font_resource: Res<FontResource>) {
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

    let mut menu_column = commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::linear_rgb(0.2, 0.2, 0.2).into()),
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
    ));

    menu_column.add_children(&[play_button, quit_button]);
    let menu_column_id = menu_column.id();

    let mut menu = commands.spawn((
        DespawnOnExit(GameState::MainMenu),
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        OnMainMenuScreen {},
    ));

    menu.add_child(menu_column_id);

    let mut main_menu_grid = menu_navigation::GameMenuGrid::new_vertical();
    main_menu_grid.push_button_to_stack(play_button);
    main_menu_grid.push_button_to_stack(quit_button);

    commands.spawn((
        main_menu_grid,
        menu_navigation::GameMenuController {
            players: HashSet::from([Player::One, Player::Two]),
        },
        ActiveMenu {},
    ));
}

fn main_menu_action(
    mut click: On<Pointer<Click>>,
    menu_button: Query<&MainMenuButtonAction, With<Button>>,
    mut app_exit_writer: MessageWriter<AppExit>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    let button_entity = click.entity;
    if let Some(menu_button_action) = menu_button.get(button_entity).ok() {
        click.propagate(false);
        match menu_button_action {
            MainMenuButtonAction::Quit => {
                app_exit_writer.write(AppExit::Success);
            }
            MainMenuButtonAction::PlayDemo => {
                game_state.set(GameState::Battle);
            }
        }
    }
}
