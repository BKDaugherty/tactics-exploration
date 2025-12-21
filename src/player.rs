//! The player module handles player input, controlling units, and moving around the player's cursor.
//!

use std::collections::HashMap;

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::{grid::GridPosition, unit::ValidMove};

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
            Player::One => InputMap::new([
                (PlayerInputAction::MoveCursorUp, KeyCode::KeyW),
                (PlayerInputAction::MoveCursorDown, KeyCode::KeyS),
                (PlayerInputAction::MoveCursorLeft, KeyCode::KeyA),
                (PlayerInputAction::MoveCursorRight, KeyCode::KeyD),
                (PlayerInputAction::Select, KeyCode::Space),
                (PlayerInputAction::Deselect, KeyCode::ShiftLeft),
                (PlayerInputAction::ZoomIn, KeyCode::KeyQ),
                (PlayerInputAction::ZoomOut, KeyCode::KeyE),
            ]),
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
}

#[derive(Debug, Default)]
pub struct PlayerState {
    pub cursor_state: PlayerCursorState,
}

// TODO: This should be replaced with some system to ask
// players to join the game by pressing bumpers or something
pub fn spawn_coop_players(mut commands: Commands) {
    commands.spawn((Name::new("Player One"), PlayerBundle::new(Player::One)));
    commands.spawn((Name::new("Player Two"), PlayerBundle::new(Player::Two)));
}
