//! The player module handles player input, controlling units, and moving around the player's cursor.
//! 

use std::collections::HashMap;

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::grid::GridPosition;

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
            Player::One => InputMap::new(
                [(PlayerInputAction::MoveCursorUp , KeyCode::KeyW),
                 (PlayerInputAction::MoveCursorDown, KeyCode::KeyS),
                 (PlayerInputAction::MoveCursorLeft, KeyCode::KeyA),
                 (PlayerInputAction::MoveCursorRight, KeyCode::KeyD),
                 (PlayerInputAction::Select, KeyCode::Space),
                 (PlayerInputAction::Deselect, KeyCode::ShiftLeft),
                 (PlayerInputAction::ZoomIn, KeyCode::KeyQ),
                 (PlayerInputAction::ZoomOut, KeyCode::KeyE),
                 (PlayerInputAction::DeleteOverlayRemoveMe, KeyCode::Backspace)]
            ),
            Player::Two => InputMap::new(
                [(PlayerInputAction::MoveCursorUp, KeyCode::ArrowUp),
                 (PlayerInputAction::MoveCursorDown, KeyCode::ArrowDown),
                 (PlayerInputAction::MoveCursorLeft, KeyCode::ArrowLeft),
                 (PlayerInputAction::MoveCursorRight, KeyCode::ArrowRight),
                 (PlayerInputAction::Select, KeyCode::Enter),
                 (PlayerInputAction::Deselect, KeyCode::ShiftRight)]
            ),
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
    /// Temporary actions for testing overlay generation/removal
    CreateOverlayRemoveMe,
    DeleteOverlayRemoveMe,
}

// TODO:  Is this really how I want to track this?
#[derive(Resource, Reflect)]
pub struct PlayerGameStates {
    pub player_state: HashMap<Player, PlayerState>,
}

/// The current state of the player's cursor
#[derive(Debug, PartialEq, Eq, Reflect, Clone)]
pub enum PlayerCursorState {
    Idle,
    /// Moving Entity from source position
    MovingUnit(Entity, GridPosition, Vec<GridPosition>),
}

impl Default for PlayerCursorState {
    fn default() -> Self {
        PlayerCursorState::Idle
    }
}

#[derive(Debug, Default, Reflect)]
pub struct PlayerState { 
    pub cursor_state: PlayerCursorState
}

