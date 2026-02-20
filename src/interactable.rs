//! Houses the different definitions of interactable entities on the Grid.

use bevy::prelude::*;

use crate::{
    assets::FontResource,
    battle_menu::{BattleMenuAction, BattlePlayerUI, UnitMenuAction, battle_ui_button},
    grid::GridPosition,
    menu::menu_navigation::{GameMenuGrid, MenuGridPosition},
    player::Player,
    unit::{
        Unit, UnitAction, UnitActionCompletedMessage, UnitExecuteAction, UnitExecuteActionMessage,
    },
};

/// A marker component for items on the Grid that are "Interactable"
#[derive(Component, Default)]
pub struct Interactable;

/// An interactable might exist on the Grid, but not be able to be interacted with
/// for some reason. If you have this component, you can be interacted with!
///
/// (Think closed treasure chests, a teleporter that doesn't work yet cause you haven't
/// cleared le conditions, etc)
#[derive(Component)]
pub struct InteractionEnabled;

/// A label for the Interaction to be displayed on the Menu for the player.
#[derive(Component)]
pub struct InteractionMenuLabel {
    pub label: &'static str,
}

/// A marker component to say that this is button is tied to a specific interactable.
#[derive(Component)]
pub struct InteractionButton {
    menu_position: MenuGridPosition,
}

/// An example interactable for now.
#[derive(Component, Debug)]
#[require(Interactable, InteractionMenuLabel {
    label: "Open Chest"
})]
pub struct TreasureChest;

/// Another example interactable
#[derive(Component, Debug)]
#[require(Interactable, InteractionMenuLabel {
    label: "Pickup Item"
})]
pub struct ObtainableItem {
    pub(crate) item_id: String,
}

/// A component that stores on the StandardBattleMenu the presence of a
/// interaction action. Used to determine if the menu needs to be updated or not.
#[derive(Component)]
pub struct HasInteractionAction {
    interaction_entity: Entity,
}

/// Top level system that handles interactions when a UnitExecuteActionMessage is received.
///
/// We expect this to fan out to the different types of interactions that can occur.
pub fn handle_interactions(
    mut commands: Commands,
    mut message_reader: MessageReader<UnitExecuteActionMessage>,
    mut message_writer: MessageWriter<UnitActionCompletedMessage>,
    query: Query<(Option<&ObtainableItem>, Option<&TreasureChest>), With<Interactable>>,
) {
    for message in message_reader.read() {
        let UnitExecuteAction::Interact {
            interactable_entity,
        } = message.action
        else {
            continue;
        };

        let Some(interaction_type) = query.get(interactable_entity).ok() else {
            error!(
                "No interactable component for interactable_entity from message: {:?}",
                interactable_entity
            );
            continue;
        };

        // I imagine we will probably have each of these in it's own query.
        // This is kind of just to showcase how we can use this.
        match interaction_type {
            (Some(ObtainableItem { item_id }), None) => {
                info!("Got Item: {:?}", item_id);
            }
            (None, Some(t)) => {
                info!("Opened Treasure Chest: {:?}", t);
            }
            otherwise => {
                error!("Invalid pair for interaction type: {:?}", otherwise);
            }
        }

        commands
            .entity(interactable_entity)
            .remove::<InteractionEnabled>();

        // TODO: We probably want to trigger some side effect above that for the given thing and
        // play some set of animations or adds stuff to the players inventory, etc, before sending this message.
        message_writer.write(UnitActionCompletedMessage {
            unit: message.entity,
            action: UnitAction::Interact,
        });
    }
}

/// Update the Player UIs set of options if they are currently standing on an
/// interactable
///
/// TODO: This seems like it can't be performant lol.
pub fn update_player_ui_available_options(
    mut commands: Commands,
    fonts: Res<FontResource>,
    controlled_unit: Query<(&Player, &GridPosition), With<Unit>>,
    interactables: Query<
        (Entity, &InteractionMenuLabel, &GridPosition),
        (With<InteractionEnabled>, With<Interactable>),
    >,
    mut ui: Query<
        (
            Entity,
            &Player,
            &mut GameMenuGrid,
            &Children,
            Option<&HasInteractionAction>,
        ),
        With<BattlePlayerUI>,
    >,
    interaction_buttons: Query<(Entity, &InteractionButton)>,
) {
    for (p, pos) in controlled_unit {
        // TODO: Querying for all interactables every GridPosition change is quite expensive I imagine? Could add interactables to grid cache.
        let interactable_at_position = interactables
            .iter()
            .find(|t| t.2 == pos)
            .map(|t| (t.0, t.1));
        for (ui_e, ui_player, mut grid, children, has_interaction_action) in ui.iter_mut() {
            if ui_player != p {
                continue;
            }

            // So if the player is on the thing, I want to add a menu option.
            // If not, I want to remove my interactable menu option
            match interactable_at_position {
                Some((interactable_e, menu_label)) => {
                    if let Some(existing_interaction) = has_interaction_action {
                        if existing_interaction.interaction_entity == interactable_e {
                            continue;
                        }

                        for child in children {
                            if let Ok((e, interaction_button)) = interaction_buttons.get(*child) {
                                commands.entity(e).despawn();
                                let _ = grid.remove_button(&interaction_button.menu_position);
                            }
                        }
                    }

                    let button = commands.spawn_empty().id();

                    let menu_position = grid.push_button_to_stack(button);
                    commands.entity(button).insert((
                        InteractionButton { menu_position },
                        battle_ui_button(
                            &fonts,
                            BattleMenuAction::Action(UnitMenuAction::Interact(interactable_e)),
                            menu_label.label,
                        ),
                    ));

                    commands
                        .entity(ui_e)
                        .add_child(button)
                        .insert(HasInteractionAction {
                            interaction_entity: interactable_e,
                        });
                }
                None => {
                    if has_interaction_action.is_none() {
                        continue;
                    }
                    for child in children {
                        if let Ok((e, interaction_button)) = interaction_buttons.get(*child) {
                            commands.entity(e).despawn();
                            let _ = grid.remove_button(&interaction_button.menu_position);
                        }
                    }
                    commands.entity(ui_e).remove::<HasInteractionAction>();
                }
            }
        }
    }
}
