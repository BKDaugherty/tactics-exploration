//! The player module handles player input, controlling units, and moving around the player's cursor.
//!

use std::collections::HashMap;

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::{
    combat::skills::SkillId,
    grid::GridPosition,
    save_game::UnitSaveV1,
    unit::{AttackOption, ValidMove},
};

// TODO: Probably want this to be more like "PlayerId(u32)"
// Although we probably could just make it 1, 2, 3, 4...
#[derive(Component, Reflect, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum Player {
    One,
    Two,
    Three,
    Four,
    /// This is a catch all player that exists before other players. Meta, I know
    PrePlayer,
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    pub input_manager: InputMap<PlayerInputAction>,
}

impl PlayerBundle {
    pub fn new(player: Player) -> Self {
        let input_map = player.get_keyboard_input_map();
        Self {
            player,
            input_manager: input_map,
        }
    }
}

/// TODO: Support gamepads
impl Player {
    pub fn get_keyboard_input_map(&self) -> InputMap<PlayerInputAction> {
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
            Player::Two | Player::Three | Player::Four => InputMap::new([
                (PlayerInputAction::MoveCursorUp, KeyCode::ArrowUp),
                (PlayerInputAction::MoveCursorDown, KeyCode::ArrowDown),
                (PlayerInputAction::MoveCursorLeft, KeyCode::ArrowLeft),
                (PlayerInputAction::MoveCursorRight, KeyCode::ArrowRight),
                (PlayerInputAction::Select, KeyCode::Enter),
                (PlayerInputAction::Deselect, KeyCode::ShiftRight),
            ]),

            Player::PrePlayer => {
                let mut base_map = InputMap::new([
                    (PlayerInputAction::MoveCursorUp, KeyCode::KeyW),
                    (PlayerInputAction::MoveCursorDown, KeyCode::KeyS),
                    (PlayerInputAction::MoveCursorLeft, KeyCode::KeyA),
                    (PlayerInputAction::MoveCursorRight, KeyCode::KeyD),
                    (PlayerInputAction::Select, KeyCode::Space),
                    (PlayerInputAction::Deselect, KeyCode::ShiftLeft),
                    (PlayerInputAction::ZoomIn, KeyCode::KeyQ),
                    (PlayerInputAction::ZoomOut, KeyCode::KeyE),
                ]);

                base_map.insert_multiple([
                    (PlayerInputAction::MoveCursorUp, GamepadButton::DPadUp),
                    (PlayerInputAction::MoveCursorDown, GamepadButton::DPadDown),
                    (PlayerInputAction::MoveCursorLeft, GamepadButton::DPadLeft),
                    (PlayerInputAction::MoveCursorRight, GamepadButton::DPadRight),
                    (PlayerInputAction::Select, GamepadButton::South),
                    (PlayerInputAction::Deselect, GamepadButton::East),
                ]);

                base_map.insert_dual_axis(PlayerInputAction::MoveCursor, GamepadStick::LEFT);
                base_map
            }
        }
    }

    pub fn get_input_map_with_gamepad(entity: Entity) -> InputMap<PlayerInputAction> {
        InputMap::new([
            (PlayerInputAction::MoveCursorUp, GamepadButton::DPadUp),
            (PlayerInputAction::MoveCursorDown, GamepadButton::DPadDown),
            (PlayerInputAction::MoveCursorLeft, GamepadButton::DPadLeft),
            (PlayerInputAction::MoveCursorRight, GamepadButton::DPadRight),
            (PlayerInputAction::Select, GamepadButton::South),
            (PlayerInputAction::Deselect, GamepadButton::East),
        ])
        .with_gamepad(entity)
        .with_dual_axis(PlayerInputAction::MoveCursor, GamepadStick::LEFT)
    }
}

#[derive(Actionlike, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum PlayerInputAction {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    #[actionlike(DualAxis)]
    MoveCursor,
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
    LookingForTargetWithAttack(Entity, HashMap<GridPosition, AttackOption>, SkillId),
}

#[derive(Debug, Default)]
pub struct PlayerState {
    pub cursor_state: PlayerCursorState,
}

/// I'm not that attached to this yet.
#[derive(Resource)]
pub struct RegisteredBattlePlayers {
    pub players: HashMap<Player, UnitSaveV1>,
}
