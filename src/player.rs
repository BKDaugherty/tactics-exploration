//! The player module handles player input, controlling units, and moving around the player's cursor.
//!

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::{
    grid::GridPosition,
    unit::{AttackOption, ValidMove},
};

// TODO: Probably want this to be more like "PlayerId(u32)"
// Although we probably could just make it 1, 2, 3, 4...
#[derive(Component, Reflect, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum Player {
    One,
    Two,
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    input_manager: InputMap<PlayerInputAction>,
}

impl PlayerBundle {
    pub fn new(player: Player) -> Self {
        let input_map = player.get_input_map();
        Self {
            player,
            input_manager: input_map,
        }
    }
}

/// TODO: Support gamepads
impl Player {
    fn get_input_map(&self) -> InputMap<PlayerInputAction> {
        match self {
            Player::One => {
                let mut input_map = InputMap::new([
                    (PlayerInputAction::MoveCursorUp, KeyCode::KeyW),
                    (PlayerInputAction::MoveCursorDown, KeyCode::KeyS),
                    (PlayerInputAction::MoveCursorLeft, KeyCode::KeyA),
                    (PlayerInputAction::MoveCursorRight, KeyCode::KeyD),
                    (PlayerInputAction::Select, KeyCode::Space),
                    (PlayerInputAction::Deselect, KeyCode::ShiftLeft),
                    (PlayerInputAction::ZoomIn, KeyCode::KeyQ),
                    (PlayerInputAction::ZoomOut, KeyCode::KeyE),
                ]);

                input_map.insert_multiple([
                    (PlayerInputAction::MoveCursorUp, GamepadButton::DPadUp),
                    (PlayerInputAction::MoveCursorDown, GamepadButton::DPadDown),
                    (PlayerInputAction::MoveCursorLeft, GamepadButton::DPadLeft),
                    (PlayerInputAction::MoveCursorRight, GamepadButton::DPadRight),
                    (PlayerInputAction::Select, GamepadButton::South),
                    (PlayerInputAction::Deselect, GamepadButton::East),
                ]);

                input_map
            }
            Player::Two => InputMap::new([
                (PlayerInputAction::MoveCursorUp, KeyCode::ArrowUp),
                (PlayerInputAction::MoveCursorDown, KeyCode::ArrowDown),
                (PlayerInputAction::MoveCursorLeft, KeyCode::ArrowLeft),
                (PlayerInputAction::MoveCursorRight, KeyCode::ArrowRight),
                (PlayerInputAction::Select, KeyCode::Enter),
                (PlayerInputAction::Deselect, KeyCode::ShiftRight),
            ]),
        }
    }
}

#[derive(Actionlike, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum PlayerInputAction {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    Select,
    Deselect,
    ZoomIn,
    ZoomOut,
}

// TODO:  Is this really how I want to track this?
#[derive(Resource)]
pub struct PlayerGameStates {
    pub player_state: HashMap<Player, PlayerState>,
}

/// The current state of the player's cursor
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum PlayerCursorState {
    #[default]
    Idle,
    /// Moving Entity from source position
    MovingUnit(Entity, GridPosition, HashMap<GridPosition, ValidMove>),
    LookingForTargetWithAttack(Entity, HashMap<GridPosition, AttackOption>),
}

#[derive(Debug, Default)]
pub struct PlayerState {
    pub cursor_state: PlayerCursorState,
}

// TODO: This should be replaced with some system to ask
// players to join the game by pressing bumpers or something?
//
// In general I need to solve my input problem of not having a default handler?
pub fn spawn_coop_players(mut commands: Commands) {
    for player in [Player::One, Player::Two] {
        commands.spawn((
            Name::new(format!("Player {:?}", player)),
            PlayerBundle::new(player),
        ));
    }
}

/// I'm not that attached to this yet.
#[derive(Resource)]
pub struct RegisteredPlayers {
    pub players: HashSet<Player>,
}
