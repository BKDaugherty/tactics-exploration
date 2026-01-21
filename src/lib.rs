pub mod animation;
pub mod args;
pub mod assets;
pub mod battle;
pub mod battle_menu;
pub mod battle_phase;
pub mod camera;
pub mod combat;
pub mod dungeon;
pub mod enemy;
pub mod equipment;
pub mod gameplay_effects;
pub mod grid;
pub mod grid_cursor;
pub mod interactable;
pub mod join_game_menu;
pub mod main_menu;
pub mod map_generation;
pub mod menu;
pub mod player;
pub mod projectile;
pub mod save_game;
pub mod unit;
pub mod unit_stats;

use bevy::prelude::*;

/// The state of the Game
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States, Reflect)]
pub enum GameState {
    #[default]
    Initializing,
    MainMenu,
    JoinGame,
    Dungeon,
    BattleResolution,
}
