use std::collections::HashMap;

use anyhow::Context;
use bevy::prelude::*;

use crate::{
    animation::{
        AnimationFollower,
        animation_db::{AnimatedSpriteId, AnimationDB},
    },
    assets::sprite_db::{SpriteDB, SpriteId},
    unit::Unit,
};

#[derive(Debug)]
pub enum WeaponType {
    /// Melee weapons use the base "Attack" skill?
    Melee {},
    /// Projectile weapons use ProjectileAttack maybe with a custom
    /// sprite?
    Projectile {},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponEquippableSlot {
    BothHands,
    Primary,
    Offhand,
}

#[derive(Debug)]
pub struct Weapon {
    range: u32,
    weapon_type: WeaponType,
}

#[derive(Debug)]
pub enum WeaponRestrictions {
    OneHanded,
    TwoHanded,
}

// Not sure I will actually use this, just helps me think about
// how to spawn it
pub type EquippedArmorBundle = (ArmorItem, AnimationFollower, Sprite);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArmorEquippableSlot {
    Head,
    Body,
    Gloves,
    Feet,
}

#[derive(Component, Debug, Clone)]
pub struct ArmorItem {
    item_name: String,
    /// The slot that this item can be equipped on
    slot: ArmorEquippableSlot,
    item_id: ItemId,
    /// Should the SpriteDB maintain this reference?
    sprite_id: SpriteId,
    animated_sprite_id: AnimatedSpriteId,
}

/// The equipment for a unit
///
/// It's expected that all equipped items will be child entities
/// of the Unit.
#[derive(Component)]
pub struct UnitEquipment {
    pub slots: HashMap<ArmorEquippableSlot, Entity>,
}

#[derive(Debug)]
pub enum Item {
    Weapon(Weapon),
    Armor(ArmorItem),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(u32);

#[derive(Resource)]
pub struct ItemDB {
    armor_db: HashMap<ItemId, Item>,
}

/// Equip an item on a unit
pub fn equip_item_on_unit(
    commands: &mut Commands,
    sprite_db: &SpriteDB,
    anim_db: &AnimationDB,
    mut unit_query: Query<(&Unit, &mut UnitEquipment)>,
    unit_e: Entity,
    item: ArmorItem,
) -> anyhow::Result<()> {
    let (_unit, mut unit_equipment) = unit_query
        .get_mut(unit_e)
        .with_context(|| format!("No Unit or UnitEquipment on given entity {:?}", unit_e))?;

    if let Some(already_equipped_item) = unit_equipment.slots.get(&item.slot) {
        anyhow::bail!(
            "Unit already has item entity {:?} in slot {:?}",
            already_equipped_item,
            item.slot
        )
    }

    let image = sprite_db
        .sprite_id_to_handle
        .get(&item.sprite_id)
        .with_context(|| {
            format!(
                "No Sprite registered for equipped item {:?} with sprite id: {:?}",
                item.item_id, item.sprite_id
            )
        })?;

    let texture_atlas = anim_db.get_atlas(&item.animated_sprite_id);

    let armor_e = commands
        .spawn((
            Sprite {
                image: image.clone(),
                texture_atlas: texture_atlas.map(|layout| TextureAtlas {
                    layout,
                    // TODO: Timing here might be odd
                    index: 0,
                }),
                ..Default::default()
            },
            item.clone(),
        ))
        .id();

    commands.entity(unit_e).add_child(armor_e);

    let _ = unit_equipment.slots.insert(item.slot, armor_e);

    Ok(())
}
