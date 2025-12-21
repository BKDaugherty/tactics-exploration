pub mod ui_consts {
    use bevy::color::Color;

    pub const NORMAL_MENU_BUTTON_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);
    pub const FOCUSED_BORDER_BUTTON_COLOR: Color = Color::srgb(1.0, 1.0, 1.0);
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
        prelude::*,
    };

    use crate::{
        menu::ui_consts::{FOCUSED_BORDER_BUTTON_COLOR, NORMAL_MENU_BUTTON_COLOR},
        player::{self, Player},
    };

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

        pub fn get_active_menu_option(&self) -> Option<&Entity> {
            self.buttons.get(&self.active_position)
        }

        pub fn reset_menu_option(&mut self) {
            self.active_position = MenuGridPosition { x: 1, y: 1 };
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
        mut menu_query: Query<(&mut GameMenuGrid, &GameMenuController), With<ActiveMenu>>,
    ) {
        for (player, input_action_state) in input_query {
            for (mut game_menu, controller) in menu_query.iter_mut() {
                if !controller.players.contains(player) {
                    continue;
                }

                let mut delta = MenuVec::default();
                if input_action_state.just_pressed(&player::PlayerInputAction::MoveCursorUp) {
                    delta.y -= 1;
                }
                if input_action_state.just_pressed(&player::PlayerInputAction::MoveCursorDown) {
                    delta.y += 1;
                }

                if delta != MenuVec::default() {
                    game_menu.apply_menu_vec_to_cursor(delta);
                }

                if input_action_state.just_pressed(&player::PlayerInputAction::Select)
                    && let Some(entity) = game_menu.get_active_menu_option()
                {
                    click_entity_with_fake_mouse(&mut commands, *entity);
                }
            }
        }
    }

    #[derive(Component)]
    pub struct ActiveMenu {}

    // Highlight the current menu option for each player
    pub fn highlight_menu_option(
        menu_query: Query<&GameMenuGrid, With<ActiveMenu>>,
        mut border_color_query: Query<(Entity, &mut BorderColor)>,
    ) {
        for menu in menu_query.iter() {
            let mut buttons: Vec<&Entity> = menu.buttons.values().collect();
            if let Some(active_button) = menu.get_active_menu_option() {
                buttons.retain(|e| *e != active_button);
                if let Some((_, mut border_color)) = border_color_query.get_mut(*active_button).ok()
                {
                    *border_color = BorderColor::all(FOCUSED_BORDER_BUTTON_COLOR)
                }
            }

            for button in buttons {
                if let Some((_, mut border_color)) = border_color_query.get_mut(*button).ok() {
                    *border_color = BorderColor::all(NORMAL_MENU_BUTTON_COLOR)
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
