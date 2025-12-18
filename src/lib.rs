// Goal 2: Render an object in a place on the Grid!

use std::collections::HashMap;
use anyhow::Context;

use bevy::prelude::*;
pub mod assets;
pub mod grid;
pub mod grid_cursor;
pub mod player;
pub mod unit;


// These locations are valid locations to move to
// Ideally I make these types of entities

/// Something with the "Ground" Component can be stood upon!
/// 
/// Note that other components may conflict with this
#[derive(Component)]
pub struct Ground {}

/// An Interactable component. Can be stood upon!
#[derive(Component)]
struct Interactable {}