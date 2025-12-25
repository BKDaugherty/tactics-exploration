//! This module handles various UI things associated with an in battle character

use std::collections::HashSet;

use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    assets::FontResource,
    battle::{BattleEntity, UnitCommand, UnitSelectionMessage, UnitUiCommandMessage},
    battle_phase::UnitPhaseResources,
    grid::{self, GridManagerResource},
    grid_cursor::Cursor,
    menu::{
        menu_navigation::{ActiveMenu, GameMenuController, GameMenuGrid},
        ui_consts::NORMAL_MENU_BUTTON_COLOR,
    },
    player::{self, Player, PlayerInputAction},
    unit::Unit,
};

pub const UI_BACKGROUND: Color = Color::linear_rgba(0.2, 0.2, 0.2, 0.7);

#[derive(Component)]
pub struct BattlePlayerUI {}

#[derive(Component)]
pub struct PlayerUiInfo {}

#[derive(Component)]
pub struct PlayerUiHealthText {}

#[derive(Component)]
pub struct PlayerUiNameText {}

#[derive(Component)]
pub enum UnitMenuAction {
    Move,
    Attack,
    Wait,
}

#[derive(Component)]
pub struct ObjectiveUi {}
#[derive(Component)]
pub struct ObjectiveText {}

pub fn build_top_ui(commands: &mut Commands, fonts: &FontResource) {
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
                width: percent(15),
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            BackgroundColor(UI_BACKGROUND),
            ObjectiveUi {},
            BorderRadius::all(percent(20)),
            children![(
                Text("Objective Text".to_string()),
                ObjectiveText {},
                TextFont {
                    font: fonts.fine_fantasy.clone(),
                    ..Default::default()
                }
            )],
        ))
        .id();

    commands.entity(ui_top_space).add_child(objective_ui);
}

pub fn battle_ui_setup(mut commands: Commands, fonts: Res<FontResource>) {
    let bottom_ui = commands
        .spawn((Node {
            align_self: AlignSelf::FlexEnd,
            height: percent(20),
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceEvenly,
            ..Default::default()
        },))
        .id();

    let player_ui_1 = build_player_ui(&mut commands, &fonts, Player::One);
    let player_ui_2 = build_player_ui(&mut commands, &fonts, Player::Two);

    commands
        .entity(bottom_ui)
        .add_children(&[player_ui_1, player_ui_2]);

    let mut battle_ui_node = commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        BattleEntity {},
    ));

    battle_ui_node.add_child(bottom_ui);
    build_top_ui(&mut commands, &fonts);
}

fn player_ui_info_style() -> Node {
    Node {
        height: percent(100),
        width: percent(35),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::FlexStart,
        padding: UiRect {
            left: percent(5),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn build_player_ui_info(commands: &mut Commands, fonts: &FontResource, player: Player) -> Entity {
    let player_ui_info = commands
        .spawn((
            PlayerUiInfo {},
            player_ui_info_style(),
            Visibility::Hidden,
            player.clone(),
        ))
        .id();

    let font_style = TextFont {
        font: fonts.fine_fantasy.clone(),
        ..Default::default()
    };

    let health_text = commands
        .spawn((
            Text::new("Health"),
            PlayerUiHealthText {},
            font_style.clone(),
        ))
        .id();
    let name_text = commands
        .spawn((
            Text::new("Unit Name"),
            PlayerUiNameText {},
            font_style.clone(),
        ))
        .id();
    let ap_text = commands
        .spawn((Text::new("AP"), PlayerUiApText {}, font_style.clone()))
        .id();
    let move_text = commands
        .spawn((Text::new("Move"), PlayerUiMoveText {}, font_style.clone()))
        .id();

    commands
        .entity(player_ui_info)
        .add_children(&[name_text, health_text, ap_text, move_text]);

    player_ui_info
}

fn battle_menu_button_font(font: Handle<Font>) -> TextFont {
    let button_text_font = TextFont {
        font_size: 15.0,
        font,
        ..Default::default()
    };
    button_text_font
}

fn player_ui_button_style() -> Node {
    Node {
        width: percent(70),
        height: percent(20),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(percent(2)),
        ..Default::default()
    }
}

fn build_battle_menu(commands: &mut Commands, fonts: &FontResource, player: Player) -> Entity {
    let player_ui_battle_menu_style = Node {
        height: percent(100),
        width: percent(65),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::SpaceEvenly,
        align_items: AlignItems::Center,
        ..Default::default()
    };

    let move_button = commands
        .spawn((
            BorderColor::all(NORMAL_MENU_BUTTON_COLOR),
            BorderRadius::all(percent(25)),
            Button,
            player_ui_button_style(),
            player,
            BackgroundColor(NORMAL_MENU_BUTTON_COLOR),
            UnitMenuAction::Move,
            children![(
                Text::new("Move"),
                battle_menu_button_font(fonts.fine_fantasy.clone()),
                TextColor(Color::srgb(0.9, 0.9, 0.9))
            )],
        ))
        .id();

    let attack_button = commands
        .spawn((
            BorderColor::all(NORMAL_MENU_BUTTON_COLOR),
            BorderRadius::all(percent(25)),
            Button,
            player_ui_button_style(),
            player,
            BackgroundColor(NORMAL_MENU_BUTTON_COLOR),
            UnitMenuAction::Attack,
            children![(
                Text::new("Attack"),
                battle_menu_button_font(fonts.fine_fantasy.clone()),
                TextColor(Color::srgb(0.9, 0.9, 0.9))
            )],
        ))
        .id();

    let wait_button = commands
        .spawn((
            BorderColor::all(NORMAL_MENU_BUTTON_COLOR),
            BorderRadius::all(percent(25)),
            Button,
            player_ui_button_style(),
            player,
            BackgroundColor(NORMAL_MENU_BUTTON_COLOR),
            UnitMenuAction::Wait,
            children![(
                Text::new("Wait"),
                battle_menu_button_font(fonts.fine_fantasy.clone()),
                TextColor(Color::srgb(0.9, 0.9, 0.9))
            )],
        ))
        .id();

    let mut menu = GameMenuGrid::new_vertical();
    menu.push_button_to_stack(move_button);
    menu.push_button_to_stack(attack_button);
    menu.push_button_to_stack(wait_button);

    let player_ui_battle_menu = commands
        .spawn((
            Name::new(format!("Player {:?}'s Battle UI", player)),
            player_ui_battle_menu_style.clone(),
            GameMenuController {
                players: HashSet::from([player]),
            },
            menu,
            BackgroundColor(UI_BACKGROUND),
            BattlePlayerUI {},
            Visibility::Hidden,
            BorderRadius::right(percent(25)),
            player,
        ))
        .id();
    commands
        .entity(player_ui_battle_menu)
        .add_children(&[move_button, attack_button, wait_button]);

    player_ui_battle_menu
}

fn build_player_ui(commands: &mut Commands, fonts: &FontResource, player: Player) -> Entity {
    let player_ui_node = Node {
        height: percent(100),
        width: percent(30),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        margin: UiRect {
            left: percent(5),
            right: percent(5),
            ..Default::default()
        },
        ..Default::default()
    };

    let player_ui_node = commands
        .spawn((
            Name::new(format!("Player {:?} UI", player)),
            BackgroundColor(UI_BACKGROUND),
            player_ui_node.clone(),
            BorderRadius::all(percent(25)),
        ))
        .id();

    let player_ui_info = build_player_ui_info(commands, fonts, player);
    let player_ui_battle_menu = build_battle_menu(commands, fonts, player);

    commands
        .entity(player_ui_node)
        .add_children(&[player_ui_info, player_ui_battle_menu]);

    player_ui_node
}

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

#[derive(Component)]
pub struct PlayerUiApText {}

#[derive(Component)]
pub struct PlayerUiMoveText {}

// TODO: Can I do Changed<GridPosition> or Changed<Unit> in two diff queries?
pub fn update_player_ui_info(
    grid_manager: Res<grid::GridManagerResource>,
    cursor_query: Query<(&player::Player, &grid::GridPosition), With<Cursor>>,
    unit_query: Query<(&Unit, &UnitPhaseResources)>,
    // TODO: This is terrible. I could make this one Text box, or
    // could do Option<Component> and have one query and then
    // do some match / runtime stuff? Def a little silly.
    mut player_unit_ui: Query<(&player::Player, &mut Visibility, &Children), With<PlayerUiInfo>>,
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
                if let Some(mut text) = player_ui_health_text.get_mut(*child).ok() {
                    text.0 = PlayerUiHealthText::derive_text(&unit);
                } else if let Some(mut text) = player_ui_name_text.get_mut(*child).ok() {
                    text.0 = PlayerUiNameText::derive_text(&unit);
                } else if let Some(mut text) = player_ui_move_text.get_mut(*child).ok() {
                    text.0 = format!("Move: {}", phase_resources.movement_points_left_in_phase);
                } else if let Some(mut text) = player_ui_ap_text.get_mut(*child).ok() {
                    text.0 = format!("AP: {}", phase_resources.action_points_left_in_phase);
                }
            }
        }
    }
}

#[derive(Component)]
pub struct ActiveBattleMenu {
    selected_unit: Entity,
}

/// Likely will want to have this spawn the set of options based
/// on the Unit
pub fn activate_battle_ui(
    mut commands: Commands,
    mut unit_selected: MessageReader<UnitSelectionMessage>,
    _grid_manager: Res<GridManagerResource>,
    mut player_battle_menu: Query<
        (Entity, &player::Player, &mut Visibility, &mut GameMenuGrid),
        With<BattlePlayerUI>,
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
            *vis = Visibility::Visible;
        }
    }
}

// TODO: Should I support touching with le mouse?
pub fn handle_battle_ui_interactions(
    mut commands: Commands,
    player_input_query: Query<(&Player, &ActionState<PlayerInputAction>)>,
    mut player_battle_menu: Query<
        (
            Entity,
            &ActiveBattleMenu,
            &mut GameMenuGrid,
            &GameMenuController,
            &mut Visibility,
        ),
        With<ActiveMenu>,
    >,
    unit_menu_query: Query<&UnitMenuAction>,
    mut battle_command_writer: MessageWriter<UnitUiCommandMessage>,
) {
    for (player, input_actions) in player_input_query.iter() {
        for (battle_menu_e, battle_menu, menu, controller, mut visibility) in
            player_battle_menu.iter_mut()
        {
            if !controller.players.contains(player) {
                continue;
            }

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

                battle_command_writer.write(UnitUiCommandMessage {
                    player: *player,
                    command: match menu_option {
                        UnitMenuAction::Move => UnitCommand::Move,
                        UnitMenuAction::Attack => UnitCommand::Attack,
                        UnitMenuAction::Wait => UnitCommand::Wait,
                    },
                    unit: battle_menu.selected_unit,
                });

                *visibility = Visibility::Hidden;
                commands.entity(battle_menu_e).remove::<ActiveMenu>();
            } else if input_actions.just_pressed(&PlayerInputAction::Deselect) {
                // Turn off the Battle Menu, unlock cursor
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

mod unused_experiments {
    use super::*;

    pub fn update_text_from_unit_under_cursor<T: TextFromUnit>(
        grid_manager: Res<grid::GridManagerResource>,
        cursor_query: Query<
            (&player::Player, &grid::GridPosition),
            (With<Cursor>, Changed<grid::GridPosition>),
        >,
        unit_query: Query<&Unit>,
        mut text_query: Query<(&player::Player, &mut Text), With<T>>,
    ) {
        for (cursor_player, grid_pos) in cursor_query.iter() {
            let Some(entities) = grid_manager.grid_manager.get_by_position(grid_pos) else {
                continue;
            };

            let unit = entities
                .iter()
                .filter_map(|t| unit_query.get(*t).ok())
                .next();

            // If there is a unit, we need to update the now visible UI
            let Some(unit) = unit else {
                continue;
            };

            for (ui_player, mut text) in text_query.iter_mut() {
                if cursor_player != ui_player {
                    continue;
                }

                text.0 = T::derive_text(unit);
            }
        }
    }

    pub fn update_player_ui_visibility(
        grid_manager: Res<grid::GridManagerResource>,
        cursor_query: Query<
            (&player::Player, &grid::GridPosition),
            (With<Cursor>, Changed<grid::GridPosition>),
        >,
        unit_query: Query<&Unit>,
        mut player_ui_unit_visible: Query<(&player::Player, &mut Visibility), With<PlayerUiInfo>>,
    ) {
        for (cursor_player, grid_pos) in cursor_query.iter() {
            let Some(entities) = grid_manager.grid_manager.get_by_position(grid_pos) else {
                continue;
            };

            let unit = entities
                .iter()
                .filter_map(|t| unit_query.get(*t).ok())
                .next();

            for (ui_player, mut unit_ui_repr) in player_ui_unit_visible.iter_mut() {
                if cursor_player != ui_player {
                    continue;
                }

                match unit {
                    Some(..) => *unit_ui_repr = Visibility::Visible,
                    None => *unit_ui_repr = Visibility::Hidden,
                };
            }
        }
    }
}
