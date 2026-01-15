//! This module handles various UI things associated with an in battle character

use std::collections::HashSet;

use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    assets::FontResource,
    battle::{
        BattleEntity, UnitCommand, UnitSelectionBackMessage, UnitSelectionMessage,
        UnitUiCommandMessage,
    },
    battle_phase::UnitPhaseResources,
    combat::skills,
    grid::{self, GridManagerResource},
    grid_cursor::Cursor,
    menu::menu_navigation::{ActiveMenu, GameMenuController, GameMenuGrid},
    player::{self, Player, PlayerInputAction},
    unit::Unit,
};

pub const UI_BACKGROUND: Color = Color::linear_rgba(0.84, 0.79, 0.72, 1.0);
pub const UI_BUTTON_BACKGROUND: Color = Color::linear_rgba(0.74, 0.69, 0.62, 1.0);
pub const UI_HEADER_BACKGROUND: Color = Color::linear_rgba(0.64, 0.59, 0.52, 1.0);

/// A marker component for the "Standard Battle UI", or the first menu of the Player's battle menu
#[derive(Component)]
pub struct BattlePlayerUI {}

/// A marker component for the container that holds the information about the unit under the player's cursor
#[derive(Component)]
pub struct PlayerUiInfo {}

/// A marker component for the text that displays the units health underneath the player's cursor
#[derive(Component)]
pub struct PlayerUiHealthText {}

/// A marker component for the text that displays the units name underneath the player's cursor
#[derive(Component)]
pub struct PlayerUiNameText {}

/// A marker component for the Objective UI
#[derive(Component)]
pub struct ObjectiveUi {}

/// A marker component for the text in the ObjectiveUI
#[derive(Component)]
pub struct ObjectiveText {}

/// Marker component for the top level BattleUiContainer.
///
/// It's expected that the BattleUiContainer will house the
#[derive(Component)]
pub struct BattleUiContainer {}

/// Marker component for the third tier of the Battle Menu
#[derive(Component)]
pub struct SkillsFilteredByCategoryMenu {}

/// Marker component for the second tier of the Battle Menu
#[derive(Component)]
pub struct SkillMenu {}

/// Marker component for buttons in menus that potentially should get despawned
/// when a menu is removed.
#[derive(Component)]
pub struct BattleButton {}

/// An action to take when the Component is interacted with.
///
/// Typically a part of a button on a Button
#[derive(Component, Clone)]
pub enum BattleMenuAction {
    Action(UnitMenuAction),
    OpenSkillMenu,
    OpenSkillsFilteredByCategoryMenu(skills::SkillCategoryId),
}

/// A terminal node in the BattleMenu. Turned into a `UnitCommand` and sent out
/// as an Event.
#[derive(Component, PartialEq, Eq, Clone)]
pub enum UnitMenuAction {
    Move,
    Attack,
    UseSkill(skills::SkillId),
    Wait,
}

/// Functions for spawning the battle UI itself
///
/// Includes the definition of the Battle Menus, PlayerUI, and the ObjectiveUI.
pub mod battle_menu_ui_definition {
    use std::collections::HashMap;

    use crate::{menu::menu_navigation::GameMenuLatch, player::RegisteredBattlePlayers};

    use super::*;

    /// Setup the Battle UI! Intended to run OnEnter(GameState::Battle)
    pub fn battle_ui_setup(
        mut commands: Commands,
        fonts: Res<FontResource>,
        registered_players: Res<RegisteredBattlePlayers>,
    ) {
        build_battle_grid_ui(&mut commands, &fonts, &registered_players);
        build_top_ui(&mut commands, &fonts);
    }

    fn build_top_ui(commands: &mut Commands, fonts: &FontResource) {
        let ui_top_space = commands
            .spawn((
                Node {
                    align_self: AlignSelf::FlexStart,
                    height: percent(15),
                    width: percent(100),
                    align_items: AlignItems::FlexEnd,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::top(percent(5)).with_right(percent(5)),
                    ..Default::default()
                },
                BattleEntity {},
            ))
            .id();

        let objective_ui = commands
            .spawn((
                Node {
                    height: percent(100),
                    width: percent(25),
                    align_content: AlignContent::Center,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                BackgroundColor(UI_BACKGROUND),
                ObjectiveUi {},
                BorderRadius::all(percent(20)),
                children![(
                    Text("Defeat all Enemies".to_string()),
                    TextColor(Color::BLACK),
                    ObjectiveText {},
                    TextFont {
                        font: fonts.badge.clone(),
                        ..Default::default()
                    }
                )],
            ))
            .id();

        commands.entity(ui_top_space).add_child(objective_ui);
    }

    fn build_battle_grid_ui(
        commands: &mut Commands,
        fonts: &FontResource,
        registered_players: &RegisteredBattlePlayers,
    ) {
        let top_level_battle_grid_container = commands
            .spawn((
                Name::new("BattleUI"),
                Node {
                    display: Display::Grid,
                    width: percent(100),
                    height: percent(100),
                    grid_template_rows: vec![
                        // Top Menu
                        GridTrack::flex(0.2),
                        // Battle Space
                        GridTrack::flex(0.6),
                        // Bottom Menu
                        GridTrack::flex(0.2),
                    ],
                    grid_template_columns: vec![
                        GridTrack::flex(0.02),
                        GridTrack::flex(0.24),
                        GridTrack::flex(0.24),
                        GridTrack::flex(0.24),
                        GridTrack::flex(0.24),
                        GridTrack::flex(0.02),
                    ],
                    ..Default::default()
                },
                BattleEntity {},
            ))
            .id();

        // TODO: Formalize this into some data representation somewhere? Probably dependent on the configuration of players...
        let player_offsets = HashMap::from([
            (Player::One, (3, 2)),
            (Player::Two, (3, 3)),
            (Player::Three, (3, 4)),
            (Player::Four, (3, 5)),
        ]);
        for player in registered_players.players.keys().cloned() {
            let player_offset = player_offsets
                .get(&player)
                .expect("Must be a UI value for the player!");
            let player_ui_container = commands
                .spawn((
                    Name::new("PlayerUiContainer"),
                    Node {
                        display: Display::Grid,
                        width: percent(100),
                        height: percent(100),
                        grid_template_rows: vec![
                            // One row in the Battle Menu for now
                            GridTrack::flex(1.0),
                        ],
                        grid_template_columns: vec![GridTrack::flex(0.3), GridTrack::flex(0.7)],
                        grid_row: GridPlacement::span(1).set_start(player_offset.0),
                        grid_column: GridPlacement::span(1).set_start(player_offset.1),
                        padding: UiRect::bottom(percent(2)),
                        ..Default::default()
                    },
                    BorderRadius::all(percent(5)),
                ))
                .id();

            // Build Player Unit UI Info
            let player_ui_info = commands
                .spawn((
                    PlayerUiInfo {},
                    BackgroundColor(UI_BACKGROUND),
                    Node {
                        height: percent(100),
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        padding: UiRect {
                            left: percent(5),
                            ..Default::default()
                        },
                        grid_column: GridPlacement::span(1).set_start(1),
                        ..Default::default()
                    },
                    Visibility::Hidden,
                    player,
                ))
                .id();

            let font_style = TextFont {
                font: fonts.fine_fantasy.clone(),
                font_size: 24.,
                font_smoothing: bevy::text::FontSmoothing::None,
                ..Default::default()
            };

            let health_text = commands
                .spawn((
                    Text::new("Health"),
                    PlayerUiHealthText {},
                    font_style.clone(),
                    TextColor(Color::BLACK),
                ))
                .id();
            let name_text = commands
                .spawn((
                    Text::new("Unit Name"),
                    PlayerUiNameText {},
                    font_style.clone().with_font(fonts.badge.clone()),
                    TextColor(Color::BLACK),
                ))
                .id();
            let ap_text = commands
                .spawn((
                    Text::new("AP"),
                    PlayerUiApText {},
                    font_style.clone(),
                    TextColor(Color::BLACK),
                ))
                .id();
            let move_text = commands
                .spawn((
                    Text::new("Move"),
                    PlayerUiMoveText {},
                    font_style.clone(),
                    TextColor(Color::BLACK),
                ))
                .id();

            let action_category_menu = commands
                .spawn((
                    Name::new("ActionCategoryMenu"),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::FlexStart,
                        grid_column: GridPlacement::start(2).set_end(4),
                        grid_row: GridPlacement::start(1).set_end(-1),
                        padding: UiRect::left(percent(1)),
                        ..Default::default()
                    },
                    BackgroundColor(UI_BACKGROUND),
                    Visibility::Hidden,
                    BorderRadius::right(percent(5)),
                    player,
                    ZIndex(1),
                    BoxShadow::new(Color::BLACK, percent(-2), px(0), percent(0), percent(2)),
                    SkillMenu {},
                    GameMenuLatch::default(),
                ))
                .id();

            // Build Battle UI
            let battle_menu_container = commands
                .spawn((
                    Name::new("PlayerBattleMenu"),
                    Node {
                        display: Display::Grid,
                        width: percent(100),
                        height: percent(100),
                        grid_column: GridPlacement::span(1).set_start(2),
                        grid_template_columns: vec![RepeatedGridTrack::flex(4, 0.25)],
                        ..Default::default()
                    },
                    Visibility::Hidden,
                    BattleUiContainer {},
                    player,
                ))
                .id();

            let move_button = commands
                .spawn(battle_ui_button(
                    fonts,
                    BattleMenuAction::Action(UnitMenuAction::Move),
                    "Move",
                ))
                .id();

            let skills_button = commands
                .spawn(battle_ui_button(
                    fonts,
                    BattleMenuAction::OpenSkillMenu,
                    "Skills",
                ))
                .id();

            let wait_button = commands
                .spawn(battle_ui_button(
                    fonts,
                    BattleMenuAction::Action(UnitMenuAction::Wait),
                    "Wait",
                ))
                .id();

            let mut menu = GameMenuGrid::new_vertical();
            menu.push_buttons_to_stack(&[move_button, skills_button, wait_button]);

            let standard_battle_menu_container = commands
                .spawn((
                    Name::new("StandardBattleMenuTab"),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::FlexStart,
                        grid_column: GridPlacement::start(1).set_end(3),
                        grid_row: GridPlacement::start(1).set_end(-1),
                        padding: UiRect::left(percent(1)),
                        ..Default::default()
                    },
                    GameMenuController {
                        players: HashSet::from([player]),
                    },
                    GameMenuLatch::default(),
                    menu,
                    BattlePlayerUI {},
                    Visibility::Hidden,
                    BorderRadius::right(percent(5)),
                    player,
                    BackgroundColor(UI_BACKGROUND),
                    BoxShadow::new(Color::BLACK, percent(-0.5), px(0), percent(0), percent(2)),
                ))
                .id();

            let action_menu = commands
                .spawn((
                    Name::new("ActionMenu"),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::FlexStart,
                        grid_column: GridPlacement::start(3).set_end(-1),
                        grid_row: GridPlacement::start(1).set_end(-1),
                        padding: UiRect::left(percent(1)),
                        ..Default::default()
                    },
                    BackgroundColor(UI_BACKGROUND),
                    BoxShadow::new(Color::BLACK, percent(-2), px(0), percent(0), percent(2)),
                    Visibility::Hidden,
                    BorderRadius::right(percent(5)),
                    player,
                    ZIndex(2),
                    SkillsFilteredByCategoryMenu {},
                    GameMenuLatch::default(),
                ))
                .id();

            commands
                .entity(standard_battle_menu_container)
                .add_children(&[move_button, skills_button, wait_button]);

            commands.entity(battle_menu_container).add_children(&[
                standard_battle_menu_container,
                action_category_menu,
                action_menu,
            ]);

            commands.entity(player_ui_info).add_children(&[
                name_text,
                health_text,
                ap_text,
                move_text,
            ]);

            commands
                .entity(player_ui_container)
                .add_children(&[player_ui_info, battle_menu_container]);

            commands
                .entity(top_level_battle_grid_container)
                .add_child(player_ui_container);
        }
    }
}

// Returns an opaque Button Bundle to spawn for a BattleUiButton
fn battle_ui_button(fonts: &FontResource, action: BattleMenuAction, text: &str) -> impl Bundle {
    (
        BackgroundColor(UI_BACKGROUND),
        BorderColor::all(Color::BLACK),
        BorderRadius::all(percent(5)),
        Button,
        Node {
            width: percent(100),
            height: percent(30),
            justify_content: JustifyContent::FlexStart,
            justify_items: JustifyItems::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(percent(0.5)),
            padding: UiRect::left(percent(1)),
            ..Default::default()
        },
        action,
        BattleButton {},
        children![(
            Text::new(text),
            TextFont {
                font_size: 20.0,
                font: fonts.badge.clone(),
                font_smoothing: bevy::text::FontSmoothing::None,
                ..Default::default()
            },
            TextColor(Color::BLACK)
        )],
    )
}

/// Marker component for the text that represents how much AP is left in the PlayerCursorInformationUI
#[derive(Component)]
pub struct PlayerUiApText {}

/// Marker component for the text that represents how much Move is left in the PlayerCursorInformationUI
#[derive(Component)]
pub struct PlayerUiMoveText {}

/// Systems that update information on the Player Cursor Information UI
pub mod player_info_ui_systems {
    use super::*;

    /// Updates the PlayerUiInfo pane based on the current position of the player's cursor.
    pub fn update_player_ui_info(
        grid_manager: Res<grid::GridManagerResource>,
        // TODO: Can I do (Changed<GridPosition> or Changed<Unit>) in two diff queries?
        cursor_query: Query<(&player::Player, &grid::GridPosition), With<Cursor>>,
        unit_query: Query<(&Unit, &UnitPhaseResources)>,
        // TODO: This is terrible. I could make this one Text box, or
        // could do Option<Component> and have one query and then
        // do some match / runtime stuff? Def a little silly.
        mut player_unit_ui: Query<
            (&player::Player, &mut Visibility, &Children),
            With<PlayerUiInfo>,
        >,
        mut player_ui_health_text: Query<
            &mut Text,
            (
                With<PlayerUiHealthText>,
                Without<PlayerUiNameText>,
                Without<PlayerUiApText>,
                Without<PlayerUiMoveText>,
            ),
        >,
        mut player_ui_name_text: Query<
            &mut Text,
            (
                With<PlayerUiNameText>,
                Without<PlayerUiHealthText>,
                Without<PlayerUiApText>,
                Without<PlayerUiMoveText>,
            ),
        >,
        mut player_ui_move_text: Query<
            &mut Text,
            (
                With<PlayerUiApText>,
                Without<PlayerUiHealthText>,
                Without<PlayerUiMoveText>,
                Without<PlayerUiNameText>,
            ),
        >,
        mut player_ui_ap_text: Query<
            &mut Text,
            (
                With<PlayerUiMoveText>,
                Without<PlayerUiHealthText>,
                Without<PlayerUiApText>,
                Without<PlayerUiNameText>,
            ),
        >,
    ) {
        for (cursor_player, grid_pos) in cursor_query.iter() {
            let Some(entities) = grid_manager.grid_manager.get_by_position(grid_pos) else {
                continue;
            };

            let unit = entities
                .iter()
                .filter_map(|t| unit_query.get(*t).ok())
                .next();

            for (ui_player, mut unit_ui_repr, children) in player_unit_ui.iter_mut() {
                if cursor_player != ui_player {
                    continue;
                }

                match unit {
                    Some(..) => *unit_ui_repr = Visibility::Visible,
                    None => *unit_ui_repr = Visibility::Hidden,
                };

                // If there is a unit, we need to update the now visible UI
                let Some((unit, phase_resources)) = unit else {
                    continue;
                };

                for child in children {
                    if let Ok(mut text) = player_ui_health_text.get_mut(*child) {
                        text.0 = PlayerUiHealthText::derive_text(unit);
                    } else if let Ok(mut text) = player_ui_name_text.get_mut(*child) {
                        text.0 = PlayerUiNameText::derive_text(unit);
                    } else if let Ok(mut text) = player_ui_move_text.get_mut(*child) {
                        text.0 = format!("Move: {}", phase_resources.movement_points_left_in_phase);
                    } else if let Ok(mut text) = player_ui_ap_text.get_mut(*child) {
                        text.0 = format!("AP: {}", phase_resources.action_points_left_in_phase);
                    }
                }
            }
        }
    }

    /// Kind of a silly trait I made when I thought it would be better to make these systems
    /// generic instead of all in one system.
    ///
    /// Leaving them around for now in case I change my mind.
    pub trait TextFromUnit: Component {
        fn derive_text(unit: &Unit) -> String;

        fn update(unit: &Unit, to_update: &mut Text) {
            to_update.0 = Self::derive_text(unit);
        }
    }

    impl TextFromUnit for PlayerUiHealthText {
        fn derive_text(unit: &Unit) -> String {
            format!("HP: {}/{}", unit.stats.health, unit.stats.max_health)
        }
    }

    impl TextFromUnit for PlayerUiNameText {
        fn derive_text(unit: &Unit) -> String {
            unit.name.clone()
        }
    }
}

/// Systems that update a given player's "BattleUI"
///
/// This UI allows the user to pick what they want a unit to do.
pub mod player_battle_ui_systems {

    use crate::{
        assets::sounds::{SoundManagerParam, UiSound},
        combat::skills::{ATTACK_SKILL_ID, SkillDBResource, UnitSkills},
        menu::NestedDynamicMenu,
    };

    use super::*;

    /// An ActiveBattleMenu component
    ///
    /// Primarily used to communicate to the menu what unit was selected.
    #[derive(Component, Clone)]
    pub struct ActiveBattleMenu {
        selected_unit: Entity,
    }

    /// If the player has selected a terminal node in the BattleUi, but then clicks back
    /// we use this handler to reactivate the battle menu, without clearing the previous state.
    pub fn reactivate_ui_on_back_message(
        mut commands: Commands,
        mut message_reader: MessageReader<UnitSelectionBackMessage>,
        mut battle_container_ui: Query<
            (&player::Player, &mut Visibility, &Children),
            (With<BattleUiContainer>, Without<BattlePlayerUI>),
        >,
        nested_dynamic: Query<Option<&NestedDynamicMenu>>,
    ) {
        for message in message_reader.read() {
            let Some((_, mut vis, children)) = battle_container_ui
                .iter_mut()
                .find(|(p, _, _)| **p == message.player)
            else {
                warn!(
                    "No BattleUiContainer found for player: {:?}",
                    message.player
                );
                continue;
            };

            *vis = Visibility::Visible;
            let mut target = children.first().cloned();
            for child in children.iter().rev() {
                if nested_dynamic.get(child).ok().flatten().is_some() {
                    target = Some(child);
                    break;
                }
            }

            if let Some(target) = target {
                commands.entity(target).insert(ActiveMenu {});
            }
        }
    }

    /// Activate the Battle UI
    pub fn activate_battle_ui(
        mut commands: Commands,
        mut unit_selected: MessageReader<UnitSelectionMessage>,
        _grid_manager: Res<GridManagerResource>,
        mut player_battle_menu: Query<
            (Entity, &player::Player, &mut Visibility, &mut GameMenuGrid),
            With<BattlePlayerUI>,
        >,
        mut battle_container_ui: Query<
            (&player::Player, &mut Visibility),
            (With<BattleUiContainer>, Without<BattlePlayerUI>),
        >,
    ) {
        for message in unit_selected.read() {
            for (player_grid_menu, player, mut vis, mut menu) in player_battle_menu.iter_mut() {
                if *player != message.player {
                    continue;
                }

                menu.reset_menu_option();

                commands.entity(player_grid_menu).insert((
                    ActiveMenu {},
                    ActiveBattleMenu {
                        selected_unit: message.entity,
                    },
                ));
                *vis = Visibility::Inherited;
            }

            if let Some((_, mut vis)) = battle_container_ui
                .iter_mut()
                .find(|(p, _)| **p == message.player)
            {
                *vis = Visibility::Visible;
            }
        }
    }

    /// Utility function for cleaning up a stale skill menu
    fn clean_stale_menu(commands: &mut Commands, menu_e: Entity, vis: &mut Visibility) {
        let mut skill_menu = commands.entity(menu_e);
        skill_menu.despawn_children();
        skill_menu.remove::<(
            GameMenuGrid,
            ActiveBattleMenu,
            NestedDynamicMenu,
            ActiveMenu,
        )>();
        *vis = Visibility::Hidden;
    }

    /// Clear out potentially stale skill systems when the Battle UI is activated
    ///
    /// TODO: I don't love that this uses UnitSelectionMessage, as opposed to
    /// having a specific event coming off the BattleUI.a
    pub fn clear_stale_battle_menus_on_activate(
        mut commands: Commands,
        mut unit_selected: MessageReader<UnitSelectionMessage>,
        mut skill_menu_query: Query<(Entity, &Player, &mut Visibility), With<SkillMenu>>,
        mut skill_menu_category_query: Query<
            (Entity, &Player, &mut Visibility),
            (With<SkillsFilteredByCategoryMenu>, Without<SkillMenu>),
        >,
    ) {
        for message in unit_selected.read() {
            for (e, p, mut vis) in skill_menu_query.iter_mut() {
                if *p != message.player {
                    continue;
                }
                clean_stale_menu(&mut commands, e, &mut vis);
            }

            for (e, p, mut vis) in skill_menu_category_query.iter_mut() {
                if *p != message.player {
                    continue;
                }
                clean_stale_menu(&mut commands, e, &mut vis);
            }
        }
    }

    /// If we send a command to the Unit, set the UI to invisible so they can make their pick without the UI
    /// getting in their way
    pub fn hide_battle_ui_on_unit_ui_command(
        mut query: Query<(&Player, &mut Visibility), With<BattleUiContainer>>,
        mut unit_command_message: MessageReader<UnitUiCommandMessage>,
    ) {
        for message in unit_command_message.read() {
            if let Some((_, mut vis)) = query.iter_mut().find(|(p, _)| **p == message.player) {
                *vis = Visibility::Hidden;
            }
        }
    }

    /// The set of components that one menu should pass to the next.
    #[derive(Bundle)]
    struct SkillMenuHandMeDowns {
        battle_menu: ActiveBattleMenu,
        controller: GameMenuController,
        nested: NestedDynamicMenu,
    }

    fn initialize_skill_menu(
        commands: &mut Commands,
        skill_menu_entity: Entity,
        buttons: Vec<Entity>,
        vis: &mut Visibility,
        hand_me_downs: SkillMenuHandMeDowns,
    ) {
        let mut skill_menu_e = commands.entity(skill_menu_entity);
        skill_menu_e.add_children(buttons.as_slice());
        let mut menu = GameMenuGrid::new_vertical();
        menu.push_buttons_to_stack(buttons.as_slice());
        skill_menu_e.insert((hand_me_downs, menu, ActiveMenu {}));

        *vis = Visibility::Inherited;
    }

    /// The chonky function that handles most of the logic here.
    ///
    /// Sends out UnitUiCommandMessages, or manages sub menus when a player clicks
    /// select or deselect, based on the current state.
    ///
    /// We use the With<ActiveMenu> tag for operating on only one of our menus at a time,
    /// and the skill_menu_*_querys for giving sub menus focus and visibility. We also maintain a
    /// NestedDynamicMenu primarily for telling if we are the root menu or not.
    ///
    /// TODO: Could split this into one query on ActiveMenu that handles Select / Deselect
    /// and then another that handles what to with a given Action being pressed?
    pub fn handle_battle_ui_interactions(
        mut commands: Commands,
        fonts: Res<FontResource>,
        skill_db: Res<SkillDBResource>,
        player_input_query: Query<(&Player, &ActionState<PlayerInputAction>)>,
        mut player_battle_menu: Query<
            (
                Entity,
                &ActiveBattleMenu,
                &mut GameMenuGrid,
                &GameMenuController,
                &mut Visibility,
                Option<&NestedDynamicMenu>,
            ),
            With<ActiveMenu>,
        >,
        unit_menu_query: Query<&BattleMenuAction>,
        mut skill_menu_query: Query<
            (Entity, &Player, &mut Visibility),
            (
                With<SkillMenu>,
                Without<SkillsFilteredByCategoryMenu>,
                Without<ActiveMenu>,
            ),
        >,
        mut skill_menu_category_query: Query<
            (Entity, &Player, &mut Visibility),
            (
                With<SkillsFilteredByCategoryMenu>,
                Without<SkillMenu>,
                Without<ActiveMenu>,
            ),
        >,
        unit_skills_query: Query<&UnitSkills>,
        mut battle_command_writer: MessageWriter<UnitUiCommandMessage>,
        sounds: SoundManagerParam,
    ) {
        for (player, input_actions) in player_input_query.iter() {
            let Some((battle_menu_e, battle_menu, menu, controller, mut visibility, nested)) =
                player_battle_menu
                    .iter_mut()
                    .find(|(_, _, _, controller, _, _)| controller.players.contains(player))
            else {
                continue;
            };

            // Player confirms
            if input_actions.just_pressed(&PlayerInputAction::Select) {
                let e = menu.get_active_menu_option();
                let Some(button) = e else {
                    warn!("Somehow le gamer clicked a button that doesn't exist!");
                    continue;
                };

                let Some(menu_option) = unit_menu_query.get(*button).ok() else {
                    warn!("Somehow le gamer clicked a button that doesn't have a menu option");
                    continue;
                };

                sounds.play_sound(&mut commands, UiSound::Select);

                // Spawn a ChildMenu with Menu Options? Set that as the ActiveMenu with a Reference for this Player?
                // So then a Cancel goes back to
                match menu_option {
                    BattleMenuAction::Action(action) => {
                        battle_command_writer.write(UnitUiCommandMessage {
                            player: *player,
                            command: match action {
                                UnitMenuAction::Move => UnitCommand::Move,
                                UnitMenuAction::Attack => UnitCommand::Attack,
                                UnitMenuAction::Wait => UnitCommand::Wait,
                                UnitMenuAction::UseSkill(skill_id) => {
                                    UnitCommand::UseSkill(*skill_id)
                                }
                            },
                            unit: battle_menu.selected_unit,
                        });
                        commands.entity(battle_menu_e).remove::<ActiveMenu>();
                    }
                    BattleMenuAction::OpenSkillMenu => {
                        // Should only be one Menu per player
                        let Some((skill_menu, _, mut vis)) =
                            skill_menu_query.iter_mut().find(|(_, p, _)| *p == player)
                        else {
                            continue;
                        };

                        let Some(unit_skills) =
                            unit_skills_query.get(battle_menu.selected_unit).ok()
                        else {
                            error!("No skills found for Unit: {:?}", battle_menu.selected_unit);
                            continue;
                        };

                        let attack_button = commands
                            .spawn(battle_ui_button(
                                &fonts,
                                BattleMenuAction::Action(UnitMenuAction::UseSkill(ATTACK_SKILL_ID)),
                                "Attack",
                            ))
                            .id();

                        let mut buttons = Vec::new();
                        buttons.push(attack_button);

                        for category_id in &unit_skills.equipped_skill_categories {
                            let category = skill_db.skill_db.get_category(category_id);

                            let button_id = commands
                                .spawn(battle_ui_button(
                                    &fonts,
                                    BattleMenuAction::OpenSkillsFilteredByCategoryMenu(
                                        *category_id,
                                    ),
                                    &category.name,
                                ))
                                .id();

                            buttons.push(button_id)
                        }

                        initialize_skill_menu(
                            &mut commands,
                            skill_menu,
                            buttons,
                            &mut vis,
                            SkillMenuHandMeDowns {
                                battle_menu: battle_menu.to_owned(),
                                controller: controller.to_owned(),
                                nested: NestedDynamicMenu {
                                    parent: battle_menu_e,
                                },
                            },
                        );

                        commands.entity(battle_menu_e).remove::<ActiveMenu>();
                    }
                    BattleMenuAction::OpenSkillsFilteredByCategoryMenu(selected_category) => {
                        let Some((skill_menu_category, _, mut vis)) = skill_menu_category_query
                            .iter_mut()
                            .find(|(_, p, _)| *p == player)
                        else {
                            continue;
                        };

                        let Some(unit_skills) =
                            unit_skills_query.get(battle_menu.selected_unit).ok()
                        else {
                            error!("No skills found for Unit: {:?}", battle_menu.selected_unit);
                            continue;
                        };

                        let mut buttons = Vec::new();
                        for skill_id in &unit_skills.learned_skills {
                            if selected_category
                                != skill_db.skill_db.get_category_for_skill(skill_id)
                            {
                                continue;
                            }

                            let skill = skill_db.skill_db.get_skill(skill_id);
                            let button_id = commands
                                .spawn(battle_ui_button(
                                    &fonts,
                                    BattleMenuAction::Action(UnitMenuAction::UseSkill(*skill_id)),
                                    &skill.name,
                                ))
                                .id();

                            buttons.push(button_id)
                        }

                        initialize_skill_menu(
                            &mut commands,
                            skill_menu_category,
                            buttons,
                            &mut vis,
                            SkillMenuHandMeDowns {
                                battle_menu: battle_menu.to_owned(),
                                controller: controller.to_owned(),
                                nested: NestedDynamicMenu {
                                    parent: battle_menu_e,
                                },
                            },
                        );

                        commands.entity(battle_menu_e).remove::<ActiveMenu>();
                    }
                }
            } else if input_actions.just_pressed(&PlayerInputAction::Deselect) {
                if let Some(dynamic_menu) = nested {
                    sounds.play_sound(&mut commands, UiSound::Cancel);
                    let parent = dynamic_menu.parent;
                    clean_stale_menu(&mut commands, battle_menu_e, &mut visibility);
                    commands.entity(parent).insert(ActiveMenu {});
                } else {
                    sounds.play_sound(&mut commands, UiSound::CloseMenu);
                    // Turn off the Battle Menu, and unlock the unit's cursor
                    battle_command_writer.write(UnitUiCommandMessage {
                        player: *player,
                        command: UnitCommand::Cancel,
                        unit: battle_menu.selected_unit,
                    });

                    *visibility = Visibility::Hidden;
                    commands.entity(battle_menu_e).remove::<ActiveMenu>();
                }
            }
        }
    }
}
