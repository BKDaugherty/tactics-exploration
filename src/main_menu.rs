use std::collections::HashSet;

use bevy::{color::palettes::css::CRIMSON, input_focus::InputDispatchPlugin, prelude::*};

use crate::{
    GameState,
    main_menu::menu_navigation::{handle_menu_cursor_navigation, highlight_menu_option},
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
enum MenuButtonAction {
    PlayDemo,
    Quit,
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const FOCUSED_BORDER: Color = Color::srgb(1.0, 1.0, 1.0);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

fn main_menu_setup(mut commands: Commands) {
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
        ..default()
    };

    let play_button = commands
        .spawn((
            Button,
            button_node.clone(),
            BackgroundColor(NORMAL_BUTTON),
            MenuButtonAction::PlayDemo,
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
            BackgroundColor(NORMAL_BUTTON),
            MenuButtonAction::Quit,
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
    ));
}

fn main_menu_action(
    mut click: On<Pointer<Click>>,
    menu_button: Query<&MenuButtonAction, With<Button>>,
    mut app_exit_writer: MessageWriter<AppExit>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    let button_entity = click.entity;
    if let Some(menu_button_action) = menu_button.get(button_entity).ok() {
        click.propagate(false);
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

// UI Navigation
// - https://github.com/bevyengine/bevy/blob/main/examples/ui/auto_directional_navigation.rs
// - https://github.com/bevyengine/bevy/blob/main/examples/ui/directional_navigation.rs
pub mod menu_navigation {
    use bevy::{
        camera::NormalizedRenderTarget,
        picking::{
            backend::HitData,
            pointer::{Location, PointerId},
        },
    };

    use crate::player::{self, Player};

    use super::*;
    use std::{
        collections::{HashMap, HashSet},
        time::Duration,
    };

    #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Component, Default, Reflect)]
    pub struct MenuGridPosition {
        x: u8,
        y: u8,
    }

    #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Default)]
    struct MenuVec {
        x: i8,
        y: i8,
    }

    #[derive(Debug, Component, Reflect)]
    pub struct GameMenuGrid {
        width: u8,
        height: u8,
        buttons: HashMap<MenuGridPosition, Entity>,
        active_position: MenuGridPosition,
    }

    impl GameMenuGrid {
        /// TODO: Should I require there to always be 1 button or just vibe?
        pub fn new_vertical() -> Self {
            Self {
                width: 1,
                height: 0,
                buttons: HashMap::default(),
                // This is an invalid position at the start...
                active_position: MenuGridPosition { x: 1, y: 1 },
            }
        }

        pub fn push_button_to_stack(&mut self, button_entity: Entity) {
            self.height += 1;
            let pos = MenuGridPosition {
                x: self.width,
                y: self.height,
            };
            let _ = self.buttons.insert(pos, button_entity);
        }

        fn apply_menu_vec_to_cursor(&mut self, menu_vec: MenuVec) {
            let mut x = self.active_position.x as i8 + menu_vec.x;
            let mut y = self.active_position.y as i8 + menu_vec.y;

            if y > self.height as i8 {
                y = 1;
            } else if y <= 0 {
                y = self.height as i8;
            }

            if x <= 0 {
                x = self.width as i8;
            } else if x >= self.width as i8 {
                x = 1;
            }

            self.active_position = MenuGridPosition {
                x: x as u8,
                y: y as u8,
            };
        }

        fn get_active_menu_option(&self) -> Option<&Entity> {
            self.buttons.get(&self.active_position)
        }
    }

    #[derive(Component)]
    pub struct GameMenuController {
        /// The Vec of players that can control the Game Menu
        pub players: HashSet<Player>,
    }

    pub fn handle_menu_cursor_navigation(
        mut commands: Commands,
        input_query: Query<(
            &player::Player,
            &leafwing_input_manager::prelude::ActionState<player::PlayerInputAction>,
        )>,
        mut menu_query: Query<(&mut GameMenuGrid, &GameMenuController)>,
    ) {
        for (player, input_action_state) in input_query {
            for (mut game_menu, controller) in menu_query.iter_mut() {
                if !controller.players.contains(player) {
                    continue;
                }

                let mut delta = MenuVec::default();
                if input_action_state.just_pressed(&player::PlayerInputAction::MoveCursorUp) {
                    delta.y += 1;
                }
                if input_action_state.just_pressed(&player::PlayerInputAction::MoveCursorDown) {
                    delta.y -= 1;
                }

                if delta != MenuVec::default() {
                    game_menu.apply_menu_vec_to_cursor(delta);
                }

                if input_action_state.just_pressed(&player::PlayerInputAction::Select)
                    && let Some(entity) = game_menu.get_active_menu_option()
                {
                    info!("Clicking our enitty {:?}", entity);
                    click_entity_with_fake_mouse(&mut commands, *entity);
                }
            }
        }
    }

    // Highlight the current menu option for each player
    pub fn highlight_menu_option(
        menu_query: Query<&GameMenuGrid, Changed<GameMenuGrid>>,
        mut border_color_query: Query<(Entity, &mut BorderColor)>,
    ) {
        for menu in menu_query.iter() {
            let mut buttons: Vec<&Entity> = menu.buttons.values().collect();
            if let Some(active_button) = menu.get_active_menu_option() {
                buttons.retain(|e| *e != active_button);
                if let Some((_, mut border_color)) = border_color_query.get_mut(*active_button).ok()
                {
                    *border_color = BorderColor::all(FOCUSED_BORDER)
                }
            }

            for button in buttons {
                if let Some((_, mut border_color)) = border_color_query.get_mut(*button).ok() {
                    *border_color = BorderColor::all(NORMAL_BUTTON)
                }
            }
        }
    }

    fn click_entity_with_fake_mouse(c: &mut Commands, entity: Entity) {
        c.trigger(Pointer::<Click> {
            entity,
            // We're pretending that we're a mouse
            pointer_id: PointerId::Mouse,
            // This field isn't used, so we're just setting it to a placeholder value
            pointer_location: Location {
                target: NormalizedRenderTarget::None {
                    width: 0,
                    height: 0,
                },
                position: Vec2::ZERO,
            },
            event: Click {
                button: PointerButton::Primary,
                // This field isn't used, so we're just setting it to a placeholder value
                hit: HitData {
                    camera: Entity::PLACEHOLDER,
                    depth: 0.0,
                    position: None,
                    normal: None,
                },
                duration: Duration::from_secs_f32(0.1),
            },
        });
    }
}
