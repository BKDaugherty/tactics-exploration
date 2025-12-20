use bevy::{
    color::palettes::css::CRIMSON,
    input_focus::{
        InputDispatchPlugin, InputFocusVisible, directional_navigation::DirectionalNavigationPlugin,
    },
    prelude::*,
};

use crate::GameState;

#[derive(Component)]
pub struct OnMainMenuScreen {}

pub fn main_menu_plugin(app: &mut App) {
    app.add_plugins((InputDispatchPlugin, DirectionalNavigationPlugin))
        // This resource is canonically used to track whether or not to render a focus indicator
        // It starts as false, but we set it to true here as we would like to see the focus indicator
        .insert_resource(InputFocusVisible(true))
        .add_systems(OnEnter(GameState::MainMenu), main_menu_setup)
        .add_systems(Update, menu_action.run_if(in_state(GameState::MainMenu)));
}

#[derive(Component)]
enum MenuButtonAction {
    PlayDemo,
    Quit,
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

fn main_menu_setup(mut commands: Commands) {
    // Common style for all buttons on the screen
    let button_node = Node {
        width: px(300),
        height: px(65),
        margin: UiRect::all(px(20)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    let button_text_font = TextFont {
        font_size: 33.0,
        ..default()
    };

    commands.spawn((
        DespawnOnExit(GameState::MainMenu),
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        OnMainMenuScreen {},
        children![(
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(CRIMSON.into()),
            children![
                // Display the game name
                (
                    Text::new("Tactics Exploration"),
                    TextFont {
                        font_size: 67.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Node {
                        margin: UiRect::all(px(50)),
                        ..default()
                    },
                ),
                // Display three buttons for each action available from the main menu:
                // - new game
                // - settings
                // - quit
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::PlayDemo,
                    children![(
                        Text::new("Play Demo"),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR),
                    ),]
                ),
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Quit,
                    children![(
                        Text::new("Quit"),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR),
                    ),]
                ),
            ]
        )],
    ));
}

fn menu_action(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut app_exit_writer: MessageWriter<AppExit>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for (interaction, menu_button_action) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match menu_button_action {
                MenuButtonAction::Quit => {
                    app_exit_writer.write(AppExit::Success);
                }
                MenuButtonAction::PlayDemo => {
                    game_state.set(GameState::Battle);
                }
            }
        }
    }
}

// UI Navigation
// - https://github.com/bevyengine/bevy/blob/main/examples/ui/auto_directional_navigation.rs
// - https://github.com/bevyengine/bevy/blob/main/examples/ui/directional_navigation.rs
pub mod ui_navigation {}
