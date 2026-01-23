pub mod ui_consts {
    use bevy::color::Color;

    // pub const NORMAL_MENU_BUTTON_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);
    // pub const FOCUSED_BORDER_BUTTON_COLOR: Color = Color::srgb(1.0, 1.0, 1.0);

    /// Base UI Background
    /// rgb(5.5%,8.2%,22.7%)
    /// #0e153a
    pub const UI_MENU_BACKGROUND: Color = Color::linear_rgba(0.055, 0.082, 0.227, 1.0);
    /// Highlighted Button
    /// #464E7F
    /// rgb(27.5%,30.6%,49.8%)
    pub const HIGHLIGHTED_BUTTON_BACKGROUND: Color = Color::linear_rgba(0.275, 0.306, 0.498, 1.0);
    pub const SELECTABLE_BUTTON_BACKGROUND: Color = Color::linear_rgba(0.15, 0.14, 0.26, 1.0);

    /// Borders between the different UI components
    /// #949393
    pub const UI_BORDER_COLOR: Color = Color::linear_rgb(0.58, 0.576, 0.576);
    pub const UI_TEXT_COLOR: Color = Color::WHITE;

    /// Confirmed button
    /// #70A649
    pub const UI_CONFIRMED_BUTTON_COLOR: Color = Color::linear_rgb(0.439, 0.651, 0.286);

    pub const UI_BUTTON_BACKGROUND: Color = Color::linear_rgba(0.74, 0.69, 0.62, 1.0);
    pub const UI_HEADER_BACKGROUND: Color = Color::linear_rgba(0.64, 0.59, 0.52, 1.0);
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
    use leafwing_input_manager::prelude::ActionState;

    use crate::{
        assets::sounds::{SoundManager, SoundSettings, UiSound},
        menu::ui_consts::{HIGHLIGHTED_BUTTON_BACKGROUND, SELECTABLE_BUTTON_BACKGROUND},
        player::{self, Player},
    };

    use std::{
        collections::{HashMap, HashSet},
        time::Duration,
    };

    #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Component, Default, Reflect)]
    pub struct MenuGridPosition {
        pub x: u8,
        pub y: u8,
    }

    #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Default)]
    pub struct MenuVec {
        x: i8,
        y: i8,
    }

    #[derive(Debug, Component, Reflect)]
    pub struct GameMenuGrid {
        active_position: MenuGridPosition,
        buttons: HashMap<MenuGridPosition, Entity>,
        column_heights: HashMap<u8, u8>,
        width: u8,
    }

    pub fn check_latch_on_axis_move(
        action_state: &ActionState<player::PlayerInputAction>,
        latch: &GameMenuLatch,
    ) -> Option<IVec2> {
        let mut delta = IVec2::ZERO;
        let axis = action_state.axis_pair(&player::PlayerInputAction::MoveCursor);

        const DEADZONE: f32 = 0.3;
        let dir = if axis.y > DEADZONE {
            IVec2::Y
        } else if axis.y < -DEADZONE {
            -IVec2::Y
        } else if axis.x > DEADZONE {
            IVec2::X
        } else if axis.x < -DEADZONE {
            -IVec2::X
        } else {
            IVec2::ZERO
        };

        if latch.latch != dir {
            delta.x += dir.x;
            // Y Direction is inverted in menu logic
            delta.y += dir.y;
            Some(delta)
        } else {
            None
        }
    }

    impl GameMenuGrid {
        pub fn new_vertical() -> Self {
            Self {
                width: 1,
                column_heights: HashMap::from([(1, 0)]),
                buttons: HashMap::default(),
                // This is an invalid position at the start...
                active_position: MenuGridPosition { x: 1, y: 1 },
            }
        }

        /// Returns true if the MenuVec resulted in a change to the original position
        pub fn apply_menu_vec_to_cursor(&mut self, menu_vec: MenuVec) -> bool {
            let mut x = self.active_position.x as i8 + menu_vec.x;
            let mut y = self.active_position.y as i8 + menu_vec.y;

            if x <= 0 {
                x = self.width as i8;
            } else if x > self.width as i8 {
                x = 1;
            }

            let height_new = self.column_heights.get(&(x as u8));

            // TODO: Just make this return Err? Would the caller ever really care? I think
            // the right thing to do here is probably panic since this should never be possible?
            //
            // Let's err instead cause I'm a coward.
            let Some(height_new) = height_new else {
                error!("No registered height for destination. Skipping application of MenuVec");
                return false;
            };

            if y > *height_new as i8 {
                y = 1;
            } else if y <= 0 {
                y = *height_new as i8;
            }

            let new_pos = MenuGridPosition {
                x: x as u8,
                y: y as u8,
            };

            let changed = self.active_position != new_pos;
            self.active_position = new_pos;
            changed
        }

        pub fn get_active_menu_option(&self) -> Option<&Entity> {
            self.buttons.get(&self.active_position)
        }

        pub fn reset_menu_option(&mut self) {
            self.active_position = MenuGridPosition { x: 1, y: 1 };
        }

        /// Pushes a button the default stack of the Game Menu Grid.
        pub fn push_button_to_stack(&mut self, button_entity: Entity) -> MenuGridPosition {
            match self.add_button_to_column(1, button_entity) {
                Ok(pos) => pos,
                Err(e) => panic!("Failed to push button to base stack: {:?}", e),
            }
        }

        pub fn remove_button(&mut self, position: &MenuGridPosition) -> anyhow::Result<()> {
            if let Some(..) = self.buttons.remove(position) {
                if let Some(y) = self.column_heights.get_mut(&position.x) {
                    *y = y.saturating_sub(1);
                }
            }
            Ok(())
        }

        /// Pushes buttons to the default stack of the Game Menu Grid.
        pub fn push_buttons_to_stack(&mut self, buttons: &[Entity]) {
            for button in buttons {
                self.push_button_to_stack(*button);
            }
        }

        fn add_button_to_column(
            &mut self,
            col: u8,
            button_entity: Entity,
        ) -> anyhow::Result<MenuGridPosition> {
            if col > self.width {
                return Err(anyhow::anyhow!(
                    "Tried to insert column, greater than width {:?}",
                    self.width
                ));
            }
            let height = self.column_heights.entry(col).or_insert(0);
            *height += 1;
            let pos = MenuGridPosition { x: col, y: *height };

            let _ = self.buttons.insert(pos, button_entity);
            Ok(pos)
        }
    }

    #[derive(Component, Clone, Reflect)]
    pub struct GameMenuController {
        /// The Vec of players that can control the Game Menu
        pub players: HashSet<Player>,
    }

    #[derive(Component, Clone, Default)]
    pub struct GameMenuLatch {
        pub latch: IVec2,
    }

    pub fn handle_menu_cursor_navigation(
        mut commands: Commands,
        sounds: Res<SoundManager>,
        sound_settings: Res<SoundSettings>,
        input_query: Query<(
            &player::Player,
            &leafwing_input_manager::prelude::ActionState<player::PlayerInputAction>,
        )>,
        mut menu_query: Query<
            (
                &mut GameMenuGrid,
                &GameMenuController,
                Option<&mut GameMenuLatch>,
            ),
            With<ActiveMenu>,
        >,
    ) {
        for (player, input_action_state) in input_query {
            for (mut game_menu, controller, menu_latch) in menu_query.iter_mut() {
                if !controller.players.contains(player) {
                    continue;
                }

                let mut delta = MenuVec::default();

                if let Some(mut menu_latch) = menu_latch
                    && let Some(axis_delta) =
                        check_latch_on_axis_move(input_action_state, &menu_latch)
                {
                    menu_latch.latch = axis_delta;
                    delta.x += axis_delta.x as i8;
                    // Y Direction is inverted in menu logic
                    delta.y -= axis_delta.y as i8;
                }

                if input_action_state.just_pressed(&player::PlayerInputAction::MoveCursorUp) {
                    delta.y -= 1;
                }
                if input_action_state.just_pressed(&player::PlayerInputAction::MoveCursorDown) {
                    delta.y += 1;
                }

                if delta != MenuVec::default() {
                    let changed = game_menu.apply_menu_vec_to_cursor(delta);
                    if changed {
                        sounds.play_ui_sound(&mut commands, &sound_settings, UiSound::MoveCursor);
                    }
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
    //
    // You probably need to be able to inject some form of theme into this for it to work okay
    pub fn highlight_menu_option(
        menu_query: Query<&GameMenuGrid, With<ActiveMenu>>,
        mut background_color_query: Query<(Entity, &mut BackgroundColor)>,
    ) {
        for menu in menu_query.iter() {
            let mut buttons: Vec<&Entity> = menu.buttons.values().collect();
            if let Some(active_button) = menu.get_active_menu_option() {
                buttons.retain(|e| *e != active_button);
                if let Ok((_, mut background_color)) =
                    background_color_query.get_mut(*active_button)
                {
                    background_color.0 = HIGHLIGHTED_BUTTON_BACKGROUND;
                }
            }

            for button in buttons {
                if let Ok((_, mut background_color)) = background_color_query.get_mut(*button) {
                    background_color.0 = SELECTABLE_BUTTON_BACKGROUND
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

pub mod menu_horizontal_selector {
    use bevy::prelude::*;

    use crate::{
        assets::sounds::{SoundManager, SoundSettings, UiSound},
        menu::menu_navigation::{
            ActiveMenu, GameMenuController, GameMenuGrid, GameMenuLatch, check_latch_on_axis_move,
        },
        player,
    };

    #[derive(Component)]
    pub struct HorizontalSelector<T> {
        options: Vec<T>,
        current_index: u32,
    }

    pub enum HortDirection {
        East,
        West,
    }

    impl<T: Clone> HorizontalSelector<T> {
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

        pub fn new(options: &[T]) -> Self {
            Self {
                options: Vec::from(options),
                current_index: 0,
            }
        }

        pub fn get_current(&self) -> Option<T> {
            self.options.get(self.current_index as usize).cloned()
        }
    }

    impl<T: PartialEq + std::fmt::Debug> HorizontalSelector<T> {
        /// TODO: Silent failures should feel bad
        pub fn set_index(&mut self, v: T) {
            let index = self.options.iter().position(|t| t == &v);
            if let Some(index) = index {
                self.current_index = index as u32;
            } else {
                error!(
                    "Silent failure makes u sad: {:?} not found in {:?}",
                    v, self.options
                );
            }
        }
    }

    /// TODO: I feel like I'm abusing the GameMenuGrid a bit here and this
    /// feels really inefficient
    ///
    /// I think the right thing to do here will be to
    ///
    /// Note that we could also consider removing the generic here and
    /// tracking just the index in this component and then having a paired component in the bundle
    /// that houses the options.
    pub fn handle_horizontal_selection<T: Send + Sync + 'static + Clone>(
        mut commands: Commands,
        sounds: Res<SoundManager>,
        sound_settings: Res<SoundSettings>,
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
                        sounds.play_ui_sound(&mut commands, &sound_settings, UiSound::MoveCursor);
                    } else if dir == -IVec2::X {
                        hort_selector.apply_index(HortDirection::West);
                        sounds.play_ui_sound(&mut commands, &sound_settings, UiSound::MoveCursor);
                    }
                }

                if action_state.just_pressed(&player::PlayerInputAction::MoveCursorLeft) {
                    hort_selector.apply_index(HortDirection::West);
                    sounds.play_ui_sound(&mut commands, &sound_settings, UiSound::MoveCursor);
                }

                if action_state.just_pressed(&player::PlayerInputAction::MoveCursorRight) {
                    hort_selector.apply_index(HortDirection::East);
                    sounds.play_ui_sound(&mut commands, &sound_settings, UiSound::MoveCursor);
                }
            }
        }
    }
}

use bevy::{ecs::query::QueryFilter, prelude::*};
use leafwing_input_manager::prelude::ActionState;

use crate::{
    assets::sounds::{SoundManager, SoundSettings, UiSound},
    menu::menu_navigation::{ActiveMenu, GameMenuController},
    player::{Player, PlayerInputAction},
};

/// Marker component for whether or not this menu has an open "child" menu.
///
/// While our level of nesting in the BattleUI is currently fixed, this
/// marker component gives us a way of referencing our parent, and let's things in the
/// battle ui system be fairly general.
#[derive(Component)]
pub struct NestedDynamicMenu {
    pub parent: Entity,
}

pub fn deselect_nested_menu(
    mut commands: Commands,
    sounds: Res<SoundManager>,
    sound_settings: Res<SoundSettings>,
    menu: Query<(Entity, &NestedDynamicMenu, &GameMenuController), With<ActiveMenu>>,
    player_input_query: Query<(&Player, &ActionState<PlayerInputAction>)>,
) {
    // I have this in so many places. We could have the UI just store the entity of the
    // controlling player (s) instead of an enum when applicable
    //
    // Maybe like an owned menu component or something
    for (player, action) in player_input_query {
        if action.just_pressed(&PlayerInputAction::Deselect) {
            for (menu_e, nested, controller) in menu {
                if !controller.players.contains(player) {
                    continue;
                }

                commands.entity(menu_e).remove::<ActiveMenu>();
                commands.entity(nested.parent).insert(ActiveMenu {});
                sounds.play_ui_sound(&mut commands, &sound_settings, UiSound::CloseMenu);
            }
        }
    }
}

pub fn show_active_game_menu_only<Inactive: QueryFilter, Active: QueryFilter>(
    mut inactive_menu: Query<&mut Node, Inactive>,
    mut active_menu: Query<&mut Node, Active>,
) {
    for mut node in active_menu.iter_mut() {
        node.display = Display::Flex;
    }

    for mut node in inactive_menu.iter_mut() {
        node.display = Display::None;
    }
}
