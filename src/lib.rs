pub mod animation;
pub mod assets;
pub mod battle;
pub mod battle_menu;
pub mod battle_phase;
mod bevy_ecs_tilemap_example;
pub mod camera;
pub mod combat;
pub mod grid;
pub mod grid_cursor;
pub mod main_menu;
pub mod menu;
pub mod player;
pub mod unit;

use bevy::prelude::*;

/// The state of the Game
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum GameState {
    #[default]
    MainMenu,
    Battle,
}
