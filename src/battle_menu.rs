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
    menu::{
        menu_navigation::{ActiveMenu, GameMenuController, GameMenuGrid},
        ui_consts::{SELECTABLE_BUTTON_BACKGROUND, UI_MENU_BACKGROUND, UI_TEXT_COLOR},
    },
    player::{self, Player, PlayerInputAction},
    unit::{StatType, Unit, UnitDerivedStats},
};

/// A marker component for the "Standard Battle UI", or the first menu of the Player's battle menu
#[derive(Component)]
pub struct BattlePlayerUI {}

/// Component for the containere that holds the "owning" player's unit information for this UI box.
pub struct PlayerOwnedUnitUi {}

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
/// It's expected that the BattleUiContainer will house all of the
/// battle menus for a specific player
#[derive(Component)]
pub struct BattleUiContainer {
    standard: Entity,
    pub(crate) skills_menu: Entity,
    pub(crate) filtered_skills_menu: Entity,
    pub(crate) map_viewer: Entity,
}

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
    ViewMap,
}

/// A terminal node in the BattleMenu. Turned into a `UnitCommand` and sent out
/// as an Event.
#[derive(Component, PartialEq, Eq, Clone)]
pub enum UnitMenuAction {
    Move,
    Attack,
    UseSkill(skills::SkillId),
    Wait,
    Interact(Entity),
}

#[derive(Component)]
pub struct UnitViewerItem;

#[derive(Component)]
pub struct UnitViewerScreen {
    name: Entity,
    info_container: Entity,
}

#[derive(Component)]
pub struct UnitViewHealthText;

#[derive(Component)]
pub struct UnitViewNameText;

#[derive(Component)]
pub struct MenuAwaitingInputFromCursor;

/// Functions for spawning the battle UI itself
///
/// Includes the definition of the Battle Menus, PlayerUI, and the ObjectiveUI.
pub mod battle_menu_ui_definition {
    use std::collections::HashMap;

    use crate::{
        menu::{
            menu_navigation::GameMenuLatch,
            ui_consts::{UI_MENU_BACKGROUND, UI_TEXT_COLOR},
        },
        player::RegisteredBattlePlayers,
    };

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
                    align_items: AlignItems::FlexStart,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::top(percent(5)).with_left(percent(5)),
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
                BackgroundColor(UI_MENU_BACKGROUND),
                ObjectiveUi {},
                BorderRadius::all(percent(20)),
                children![(
                    // Lol
                    Text("Objective:    Defeat all Enemies".to_string()),
                    TextColor(UI_TEXT_COLOR),
                    ObjectiveText {},
                    TextFont {
                        font: fonts.pixelify_sans_regular.clone(),
                        ..Default::default()
                    }
                )],
            ))
            .id();

        commands.entity(ui_top_space).add_child(objective_ui);
    }

    #[derive(Component)]
    pub struct PlayerBattleMenu;

    fn build_battle_grid_ui(
        commands: &mut Commands,
        fonts: &FontResource,
        registered_players: &RegisteredBattlePlayers,
    ) {
        let top_level_battle_ui = commands
            .spawn((
                Name::new("BattleUI"),
                Node {
                    display: Display::Flex,
                    align_self: AlignSelf::FlexEnd,
                    width: percent(100),
                    height: percent(40),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceEvenly,
                    justify_items: JustifyItems::Center,
                    align_items: AlignItems::Center,
                    align_content: AlignContent::SpaceEvenly,
                    ..Default::default()
                },
                BattleEntity {},
            ))
            .id();

        for player in registered_players.save_files.keys().cloned() {
            let player_ui_container = commands
                .spawn((
                    Name::new(format!("PlayerUiContainer {:?}", player)),
                    Node {
                        display: Display::Flex,
                        width: percent(25),
                        height: percent(100),
                        padding: UiRect::bottom(percent(2)),
                        justify_content: JustifyContent::SpaceEvenly,
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BorderRadius::all(percent(20)),
                ))
                .id();

            let font_style = TextFont {
                font: fonts.pixelify_sans_regular.clone(),
                font_size: 24.,
                font_smoothing: bevy::text::FontSmoothing::None,
                ..Default::default()
            };

            let health_text = commands
                .spawn((
                    Text::new("Health"),
                    PlayerUiHealthText {},
                    font_style.clone(),
                    TextColor(UI_TEXT_COLOR),
                ))
                .id();
            let name_text = commands
                .spawn((
                    Text::new("Unit Name"),
                    PlayerUiNameText {},
                    font_style
                        .clone()
                        .with_font(fonts.pixelify_sans_regular.clone()),
                    TextColor(UI_TEXT_COLOR),
                ))
                .id();
            let ap_text = commands
                .spawn((
                    Text::new("AP"),
                    PlayerUiApText {},
                    font_style.clone(),
                    TextColor(UI_TEXT_COLOR),
                ))
                .id();
            let move_text = commands
                .spawn((
                    Text::new("Move"),
                    PlayerUiMoveText {},
                    font_style.clone(),
                    TextColor(UI_TEXT_COLOR),
                ))
                .id();

            // Build Player Unit UI Info
            let player_ui_info = commands
                .spawn((
                    BackgroundColor(UI_MENU_BACKGROUND),
                    Node {
                        display: Display::Flex,
                        height: percent(20),
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceEvenly,
                        justify_items: JustifyItems::Center,
                        align_items: AlignItems::Center,
                        align_content: AlignContent::SpaceEvenly,
                        padding: UiRect {
                            left: percent(5),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    player,
                    PlayerUiInfo {},
                    ControlledUnitUiEntities {
                        move_text,
                        health_text,
                        ap_text,
                        name_text,
                    },
                    BorderRadius::all(percent(20)),
                ))
                .id();

            let action_category_menu = commands
                .spawn((
                    Name::new("ActionCategoryMenu"),
                    Node {
                        display: Display::None,
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::FlexStart,
                        padding: UiRect::all(percent(1)),
                        ..Default::default()
                    },
                    BackgroundColor(UI_MENU_BACKGROUND),
                    BorderRadius::all(percent(20)),
                    player,
                    SkillMenu {},
                    GameMenuLatch::default(),
                    PlayerBattleMenu,
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

            let view_map_button = commands
                .spawn(battle_ui_button(
                    fonts,
                    BattleMenuAction::ViewMap,
                    "View Map",
                ))
                .id();

            let mut menu = GameMenuGrid::new_vertical();
            menu.push_buttons_to_stack(&[move_button, skills_button, wait_button, view_map_button]);

            let standard_battle_menu_container = commands
                .spawn((
                    Name::new("StandardBattleMenuScreen"),
                    Node {
                        display: Display::None,
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::FlexStart,
                        padding: UiRect::all(percent(1)),
                        ..Default::default()
                    },
                    GameMenuController {
                        players: HashSet::from([player]),
                    },
                    GameMenuLatch::default(),
                    menu,
                    BattlePlayerUI {},
                    BorderRadius::all(percent(20)),
                    player,
                    BackgroundColor(UI_MENU_BACKGROUND),
                    PlayerBattleMenu,
                    ActiveMenu {},
                ))
                .id();

            let action_menu = commands
                .spawn((
                    Name::new("ActionMenuScreen"),
                    Node {
                        display: Display::None,
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::FlexStart,
                        padding: UiRect::all(percent(1)),
                        ..Default::default()
                    },
                    BackgroundColor(UI_MENU_BACKGROUND),
                    BorderRadius::all(percent(20)),
                    player,
                    SkillsFilteredByCategoryMenu {},
                    GameMenuLatch::default(),
                    PlayerBattleMenu,
                ))
                .id();

            let unit_view_name_text = commands
                .spawn((
                    Text::new("Viewed Unit Name"),
                    font_style
                        .clone()
                        .with_font(fonts.pixelify_sans_regular.clone()),
                    TextColor(UI_TEXT_COLOR),
                    UnitViewerItem,
                ))
                .id();

            let view_map_info_container = commands
                .spawn((
                    Name::new("ViewMapInfo"),
                    Node {
                        display: Display::Flex,
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceEvenly,
                        justify_items: JustifyItems::Center,
                        align_items: AlignItems::FlexStart,
                        align_content: AlignContent::Center,
                        padding: UiRect::all(percent(2)),
                        ..Default::default()
                    },
                    Visibility::Inherited,
                    UnitViewerItem,
                ))
                .id();
            let view_map_container = commands
                .spawn((
                    Name::new("ViewMapScreen"),
                    Node {
                        display: Display::None,
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::FlexStart,
                        ..Default::default()
                    },
                    BackgroundColor(UI_MENU_BACKGROUND),
                    BorderRadius::all(percent(20)),
                    player,
                    UnitViewerScreen {
                        name: unit_view_name_text,
                        info_container: view_map_info_container,
                    },
                    GameMenuLatch::default(),
                    PlayerBattleMenu,
                ))
                .id();

            commands
                .entity(view_map_info_container)
                .add_child(unit_view_name_text);
            commands
                .entity(view_map_container)
                .add_child(view_map_info_container);

            commands
                .entity(standard_battle_menu_container)
                .add_children(&[move_button, skills_button, wait_button, view_map_button]);

            // Build Battle UI
            let battle_menu_container = commands
                .spawn((
                    Name::new("PlayerBattleMenu"),
                    Node {
                        display: Display::Flex,
                        width: percent(100),
                        height: percent(100),
                        ..Default::default()
                    },
                    BattleUiContainer {
                        standard: standard_battle_menu_container,
                        skills_menu: action_category_menu,
                        filtered_skills_menu: action_menu,
                        map_viewer: view_map_container,
                    },
                    player,
                ))
                .id();

            commands.entity(battle_menu_container).add_children(&[
                standard_battle_menu_container,
                action_category_menu,
                action_menu,
                view_map_container,
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
                .entity(top_level_battle_ui)
                .add_child(player_ui_container);
        }
    }
}

// Returns an opaque Button Bundle to spawn for a BattleUiButton
pub fn battle_ui_button(fonts: &FontResource, action: BattleMenuAction, text: &str) -> impl Bundle {
    (
        BackgroundColor(SELECTABLE_BUTTON_BACKGROUND),
        BorderRadius::all(percent(20)),
        Button,
        Node {
            width: percent(80),
            height: percent(30),
            justify_content: JustifyContent::FlexStart,
            justify_items: JustifyItems::Center,
            align_items: AlignItems::Center,
            align_self: AlignSelf::Center,
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
                font: fonts.pixelify_sans_regular.clone(),
                font_smoothing: bevy::text::FontSmoothing::None,
                ..Default::default()
            },
            TextColor(UI_TEXT_COLOR)
        )],
    )
}

/// Marker component for the text that represents how much AP is left in the PlayerCursorInformationUI
#[derive(Component)]
pub struct PlayerUiApText {}

/// Marker component for the text that represents how much Move is left in the PlayerCursorInformationUI
#[derive(Component)]
pub struct PlayerUiMoveText {}

#[derive(Component)]
pub struct ControlledUnitUiEntities {
    name_text: Entity,
    health_text: Entity,
    ap_text: Entity,
    move_text: Entity,
}

// We want this to update anytime the Unit's resources change or
// if the controlled unit changes. I'm not sure how to express that with Bevy's
// filters.
pub fn update_controlled_ui_info(
    player_unit_ui: Query<(&player::Player, &ControlledUnitUiEntities)>,
    unit_query: Query<
        (&Unit, &UnitPhaseResources, &Player, &UnitDerivedStats),
        Or<(
            Changed<Unit>,
            Changed<UnitDerivedStats>,
            Changed<UnitPhaseResources>,
        )>,
    >,
    // So would this block any other queries updating text in the Game?
    mut text: Query<&mut Text>,
) {
    for (player, controlled_ui) in player_unit_ui {
        for (unit, resources, unit_player, unit_stats) in unit_query {
            if player != unit_player {
                continue;
            }

            if let Some(mut text_item) = text.get_mut(controlled_ui.name_text).ok() {
                text_item.0 = unit.name.clone();
            }

            if let Some(mut text_item) = text.get_mut(controlled_ui.health_text).ok() {
                text_item.0 = format!(
                    "HP: {} / {}",
                    unit_stats.stats.stat(StatType::Health).0 as u32,
                    unit_stats.stats.stat(StatType::MaxHealth).0 as u32,
                );
            }

            if let Some(mut text_item) = text.get_mut(controlled_ui.move_text).ok() {
                text_item.0 = format!("Move: {}", resources.movement_points_left_in_phase);
            }

            if let Some(mut text_item) = text.get_mut(controlled_ui.ap_text).ok() {
                text_item.0 = format!("AP: {}", resources.action_points_left_in_phase);
            }
        }
    }
}

/// Systems that update information on the Player Cursor Information UI
pub mod player_info_ui_systems {
    use super::*;

    /// Updates the UnitViewerScreen pane based on the current position of the player's cursor.
    ///
    /// This pane should be updated by
    pub fn update_unit_viewer_ui(
        grid_manager: Res<grid::GridManagerResource>,
        // TODO: Can I do (Changed<GridPosition> or Changed<Unit>) in two diff queries?
        cursor_query: Query<(&player::Player, &grid::GridPosition), With<Cursor>>,
        unit_query: Query<(&Unit, Option<&UnitPhaseResources>)>,
        player_unit_viewer: Query<(&player::Player, &UnitViewerScreen)>,
        mut vis_mutator: Query<&mut Visibility, With<UnitViewerItem>>,
        mut text_query: Query<&mut Text, With<UnitViewerItem>>,
    ) {
        for (cursor_player, grid_pos) in cursor_query.iter() {
            for (ui_player, unit_viewer_screen) in player_unit_viewer {
                if cursor_player != ui_player {
                    continue;
                }

                let Some((unit, _phase_resources)) = grid_manager
                    .grid_manager
                    .get_by_position(grid_pos)
                    .map(|t| t.iter().filter_map(|t| unit_query.get(*t).ok()).next())
                    .flatten()
                else {
                    // Nothing for the viewer to see. Set the internal Viewer Vis to 0?

                    if let Some(mut viewer_container_vis) =
                        vis_mutator.get_mut(unit_viewer_screen.info_container).ok()
                    {
                        *viewer_container_vis = Visibility::Hidden
                    }

                    continue;
                };

                if let Some(mut viewer_container_vis) =
                    vis_mutator.get_mut(unit_viewer_screen.info_container).ok()
                {
                    *viewer_container_vis = Visibility::Inherited
                }

                if let Some(mut text_item) = text_query.get_mut(unit_viewer_screen.name).ok() {
                    text_item.0 = unit.name.clone();
                }
            }
        }
    }
}

/// Systems that update a given player's "BattleUI"
///
/// This UI allows the user to pick what they want a unit to do.
pub mod player_battle_ui_systems {

    use crate::{
        assets::sounds::{SoundManagerParam, UiSound},
        battle_menu::battle_menu_ui_definition::PlayerBattleMenu,
        combat::skills::{ATTACK_SKILL_ID, SkillDBResource, UnitSkills},
        grid::GridPosition,
        grid_cursor::LockedOn,
        menu::NestedDynamicMenu,
        unit::UnitActionCompletedMessage,
    };

    use super::*;

    /// An ActiveBattleMenu component
    ///
    /// Primarily used to communicate to the menu what unit was selected.
    #[derive(Component, Clone)]
    pub struct ActiveBattleMenu {
        pub(crate) selected_unit: Entity,
    }

    /// Close the player battle menus, by removing ActiveMenu
    pub fn close_player_battle_menus(mut commands: Commands, query: Query<&BattleUiContainer>) {
        for battle_ui_container in query {
            commands
                .entity(battle_ui_container.standard)
                .remove::<ActiveMenu>();
        }
    }

    /// If the player has selected a terminal node in the BattleUi, but then clicks back
    /// we use this handler to reactivate the battle menu, without clearing the previous state.
    pub fn reactivate_ui_on_back_message(
        mut commands: Commands,
        mut message_reader: MessageReader<UnitSelectionBackMessage>,
        mut battle_container_ui: Query<
            (&player::Player, &Children),
            (With<BattleUiContainer>, Without<BattlePlayerUI>),
        >,
        nested_dynamic: Query<Option<&NestedDynamicMenu>>,
    ) {
        for message in message_reader.read() {
            let Some((_, children)) = battle_container_ui
                .iter_mut()
                .find(|(p, _)| **p == message.player)
            else {
                warn!(
                    "No BattleUiContainer found for player: {:?}",
                    message.player
                );
                continue;
            };

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

    /// This function relies on the Player only "Controlling" one unit.
    pub fn on_unit_completed_action_reopen_battle_menu(
        mut commands: Commands,
        mut reader: MessageReader<UnitActionCompletedMessage>,
        grid_manager: Res<GridManagerResource>,
        player_query: Query<&Player, With<Unit>>,
        battle_ui_container_query: Query<(&Player, &BattleUiContainer)>,
        mut battle_ui_query: Query<(Entity, &Player, &mut GameMenuGrid), With<BattlePlayerUI>>,
        mut cursor_query: Query<(Entity, &Player, &mut GridPosition), With<Cursor>>,
    ) {
        for m in reader.read() {
            info!("Unit Action Completed: {:?}", m);

            let Some(player) = player_query.get(m.unit).ok() else {
                continue;
            };

            // The unit is controlled by a player and just finished an action, re-open
            // the player's menu.
            for (menu, menu_player, mut menu_grid) in battle_ui_query.iter_mut() {
                if menu_player != player {
                    continue;
                }

                commands.entity(menu).insert((
                    ActiveMenu {},
                    ActiveBattleMenu {
                        selected_unit: m.unit,
                    },
                ));

                menu_grid.reset_menu_option();
            }

            for (cursor, cursor_player, mut pos) in cursor_query.iter_mut() {
                if player != cursor_player {
                    continue;
                }

                if let Some(unit_pos) = grid_manager.grid_manager.get_by_id(&m.unit) {
                    *pos = unit_pos;
                }

                commands.entity(cursor).insert(LockedOn {});
            }

            for (p, ui_container) in battle_ui_container_query {
                if p != player {
                    continue;
                }
                clean_stale_menu(&mut commands, ui_container.skills_menu, true);
                clean_stale_menu(&mut commands, ui_container.filtered_skills_menu, true);
                clean_stale_menu(&mut commands, ui_container.map_viewer, false);
            }
        }
    }

    /// Activate the Battle UI
    pub fn activate_battle_ui(
        mut commands: Commands,
        mut unit_selected: MessageReader<UnitSelectionMessage>,
        _grid_manager: Res<GridManagerResource>,
        mut player_battle_menu: Query<
            (Entity, &player::Player, &mut GameMenuGrid),
            With<BattlePlayerUI>,
        >,
    ) {
        for message in unit_selected.read() {
            for (player_grid_menu, player, mut menu) in player_battle_menu.iter_mut() {
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
            }
        }
    }

    /// Utility function for cleaning up a stale skill menu
    pub fn clean_stale_menu(commands: &mut Commands, menu_e: Entity, despawn_children: bool) {
        let mut skill_menu = commands.entity(menu_e);
        if despawn_children {
            skill_menu.despawn_children();
        }
        skill_menu.remove::<(
            GameMenuGrid,
            ActiveBattleMenu,
            NestedDynamicMenu,
            ActiveMenu,
        )>();
    }

    /// At the moment a player has exactly one Unit on the board.
    /// This system links the UI that the player will get to that entity.
    pub fn set_active_battle_menu(
        mut commands: Commands,
        player_units: Query<(Entity, &Player), With<Unit>>,
        battle_menus: Query<(Entity, &Player), With<BattlePlayerUI>>,
    ) {
        for (e, player) in player_units {
            for (battle_menu_e, battle_player) in battle_menus {
                if player != battle_player {
                    continue;
                }

                commands
                    .entity(battle_menu_e)
                    .insert((ActiveBattleMenu { selected_unit: e }, ActiveMenu {}));
            }
        }
    }

    /// Clear out potentially stale skill systems when the Battle UI is activated
    ///
    /// TODO: I don't love that this uses UnitSelectionMessage, as opposed to
    /// having a specific event coming off the BattleUI.
    pub fn clear_stale_battle_menus_on_activate(
        mut commands: Commands,
        mut unit_selected: MessageReader<UnitSelectionMessage>,
        battle_ui_query: Query<(&Player, &BattleUiContainer)>,
    ) {
        for message in unit_selected.read() {
            for (p, ui_container) in battle_ui_query {
                if *p != message.player {
                    continue;
                }
                clean_stale_menu(&mut commands, ui_container.skills_menu, true);
                clean_stale_menu(&mut commands, ui_container.filtered_skills_menu, true);
                clean_stale_menu(&mut commands, ui_container.map_viewer, false);
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
        hand_me_downs: SkillMenuHandMeDowns,
    ) {
        let mut skill_menu_e = commands.entity(skill_menu_entity);
        skill_menu_e.add_children(buttons.as_slice());
        let mut menu = GameMenuGrid::new_vertical();
        menu.push_buttons_to_stack(buttons.as_slice());
        skill_menu_e.insert((hand_me_downs, menu, ActiveMenu {}));
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
        battle_ui_container_query: Query<(&Player, &BattleUiContainer)>,
        mut active_player_battle_menu: Query<
            (
                Entity,
                &ActiveBattleMenu,
                &mut GameMenuGrid,
                &GameMenuController,
                Option<&NestedDynamicMenu>,
            ),
            With<ActiveMenu>,
        >,
        unit_menu_query: Query<&BattleMenuAction>,
        unit_info_query: Query<(&UnitSkills, &UnitPhaseResources)>,
        mut battle_command_writer: MessageWriter<UnitUiCommandMessage>,
        sounds: SoundManagerParam,
    ) {
        for (player, input_actions) in player_input_query.iter() {
            let Some((battle_menu_e, battle_menu, menu, controller, nested)) =
                active_player_battle_menu
                    .iter_mut()
                    .find(|(_, _, _, controller, _)| controller.players.contains(player))
            else {
                continue;
            };

            let Some((_, battle_ui_container)) =
                battle_ui_container_query.iter().find(|(p, _)| *p == player)
            else {
                error!("No BattleUI Container found for player: {:?}", player);
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

                let Some((_, unit_resources)) = unit_info_query.get(battle_menu.selected_unit).ok()
                else {
                    warn!("No controlled unit for battle menu");
                    continue;
                };

                // Spawn a ChildMenu with Menu Options? Set that as the ActiveMenu with a Reference for this Player?
                // So then a Cancel goes back to
                match menu_option {
                    BattleMenuAction::Action(action) => {
                        // Check if the Unit can take this action or not!
                        match action {
                            UnitMenuAction::Move => {
                                if unit_resources.movement_points_left_in_phase == 0 {
                                    sounds.play_ui_sound(&mut commands, UiSound::Error);
                                    info!(
                                        "It'd be nice if the player was told why they can't move!"
                                    );
                                    continue;
                                }
                            }
                            // TODO: Calculate the AP cost for the skill, don't assume it's just 1.
                            UnitMenuAction::UseSkill(skill_id) => {
                                if unit_resources.action_points_left_in_phase == 0 {
                                    sounds.play_ui_sound(&mut commands, UiSound::Error);
                                    info!(
                                        "It'd be nice if the player was told why they can't use this skill! We are assuming {:?} has AP 1",
                                        skill_id
                                    );
                                    continue;
                                }
                            }
                            _ => {}
                        };

                        sounds.play_ui_sound(&mut commands, UiSound::Select);
                        battle_command_writer.write(UnitUiCommandMessage {
                            player: *player,
                            command: match action {
                                UnitMenuAction::Move => UnitCommand::Move,
                                UnitMenuAction::Attack => UnitCommand::Attack,
                                UnitMenuAction::Wait => UnitCommand::Wait,
                                UnitMenuAction::UseSkill(skill_id) => {
                                    UnitCommand::UseSkill(*skill_id)
                                }
                                UnitMenuAction::Interact(e) => UnitCommand::Interact(*e),
                            },
                            unit: battle_menu.selected_unit,
                        });
                        commands.entity(battle_menu_e).remove::<ActiveMenu>();
                    }
                    BattleMenuAction::OpenSkillMenu => {
                        // Should only be one Menu per player
                        let skill_menu = battle_ui_container.skills_menu;

                        let Some((unit_skills, _)) =
                            unit_info_query.get(battle_menu.selected_unit).ok()
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
                            SkillMenuHandMeDowns {
                                battle_menu: battle_menu.to_owned(),
                                controller: controller.to_owned(),
                                nested: NestedDynamicMenu {
                                    parent: battle_menu_e,
                                },
                            },
                        );

                        sounds.play_ui_sound(&mut commands, UiSound::Select);
                        commands.entity(battle_menu_e).remove::<ActiveMenu>();
                    }
                    BattleMenuAction::OpenSkillsFilteredByCategoryMenu(selected_category) => {
                        let skill_menu_category = battle_ui_container.filtered_skills_menu;
                        let Some((unit_skills, _)) =
                            unit_info_query.get(battle_menu.selected_unit).ok()
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
                            SkillMenuHandMeDowns {
                                battle_menu: battle_menu.to_owned(),
                                controller: controller.to_owned(),
                                nested: NestedDynamicMenu {
                                    parent: battle_menu_e,
                                },
                            },
                        );

                        sounds.play_ui_sound(&mut commands, UiSound::Select);
                        commands.entity(battle_menu_e).remove::<ActiveMenu>();
                    }
                    BattleMenuAction::ViewMap => {
                        sounds.play_ui_sound(&mut commands, UiSound::Select);
                        commands.entity(battle_menu_e).remove::<ActiveMenu>();
                        commands.entity(battle_ui_container.map_viewer).insert((
                            battle_menu.to_owned(),
                            NestedDynamicMenu {
                                parent: battle_menu_e,
                            },
                            controller.to_owned(),
                            ActiveMenu {},
                        ));
                        battle_command_writer.write(UnitUiCommandMessage {
                            player: *player,
                            command: UnitCommand::ViewMap,
                            unit: battle_menu.selected_unit,
                        });
                    }
                }
            } else if input_actions.just_pressed(&PlayerInputAction::Deselect) {
                if let Some(dynamic_menu) = nested {
                    sounds.play_ui_sound(&mut commands, UiSound::Cancel);
                    let parent = dynamic_menu.parent;
                    clean_stale_menu(&mut commands, battle_menu_e, true);
                    commands.entity(parent).insert(ActiveMenu {});
                }
            }
        }
    }

    //
}
