// Goal 2: Render an object in a place on the Grid!

use std::collections::HashMap;
use anyhow::Context;

use bevy::prelude::*;
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



struct Obstructed {}

fn can_unit_stand_here(
    grid_manager: grid::GridManager,
    grid_position: grid::GridPosition,
    // Component needs to have a GridPosition, and Ground, and can't have a Unit!
    stand_query: Query<(&grid::GridPosition, &Ground, Option<&unit::Unit>)>,
) -> anyhow::Result<bool> {
    let entities = grid_manager.get_by_position(&grid_position).context("Invalid position given to grid manager!")?;
    for entity in entities {
        let (_, _, has_unit) = stand_query.get(*entity)?;
        if has_unit.is_some() {
            return Ok(false)
        }
    }

    if entities.len() != 0 {
        return Ok(true)
    } else {
        anyhow::bail!("No valid ground positions found in stand query?");
    }
}



#[cfg(test)]
mod test {
    #[test]
    fn my_test() {
        assert!(1 == 1);
    }
}