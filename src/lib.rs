// Goal 2: Render an object in a place on the Grid!

use std::collections::HashMap;
use anyhow::Context;



use bevy::prelude::*;

pub mod grid;
pub mod player;

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

/// A unit! Units can't share spaces (for now I guess)
/// 
/// The base controllable entity that can exist and do things on the map
/// Units would have stats, skills, etc?
#[derive(Component, Debug)]
struct Unit {
    stats: Stats,
    // effect_modifiers: ()
    // equipment?
}

#[derive(Debug)]
struct Stats {
    max_health: u32,
    strength: u32,
    // TBD if it makes sense for this to be here.
    health: u32,
}

struct Obstructed {}

fn can_unit_stand_here(
    grid_manager: grid::GridManager,
    grid_position: grid::GridPosition,
    // Component needs to have a GridPosition, and Ground, and can't have a Unit!
    stand_query: Query<(&grid::GridPosition, &Ground, Option<&Unit>)>,
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