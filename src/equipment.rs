use std::collections::HashMap;

use anyhow::Context;
use bevy::prelude::*;

use crate::{
    animation::{
        AnimationFollower,
        animation_db::{
            AnimatedSpriteId, AnimationDB, registered_sprite_ids::TT_WEAPON_ANIMATED_SPRITE_ID,
        },
    },
    assets::sprite_db::{SpriteDB, SpriteId, TinyTacticsSprites},
    combat::skills::{ATTACK_SKILL_ID, SkillId},
    gameplay_effects::{ActiveEffects, Effect, EffectData, EffectMetadata, StatModification},
    unit::TINY_TACTICS_ANCHOR,
    unit_stats::StatsDirty,
};

#[derive(Debug, Clone)]
pub struct WeaponData {
    pub range: u32,
    pub attack_skill: SkillId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquippableSlot {
    BothHands,
    Primary,
    Offhand,
    Head,
    Body,
    Gloves,
    Feet,
}

#[derive(Debug)]
pub enum WeaponRestrictions {
    OneHanded,
    TwoHanded,
}

#[allow(dead_code)]
#[derive(Component, Debug, Clone)]
pub struct EquippableItem {
    item_name: String,
    /// The slot that this item can be equipped on
    slot: EquippableSlot,
    modifiers: Vec<StatModification>,
    item_id: ItemId,
    /// Should the SpriteDB maintain this reference?
    sprite_id: SpriteId,
    animated_sprite_id: AnimatedSpriteId,
    weapon_data: Option<WeaponData>,
}

/// The equipment for a unit
///
/// It's expected that all equipped items will be child entities
/// of the Unit.
#[derive(Component, Default)]
pub struct UnitEquipment {
    equipment_slots: HashMap<EquippableSlot, (Entity, EquippableItem)>,
}

impl UnitEquipment {
    fn clear_space_for_slot(&mut self, slot: EquippableSlot) -> Vec<Entity> {
        let unequipped_items = if slot == EquippableSlot::BothHands {
            vec![
                self.equipment_slots.remove(&slot),
                self.equipment_slots.remove(&EquippableSlot::Offhand),
                self.equipment_slots.remove(&EquippableSlot::Primary),
            ]
        } else if slot == EquippableSlot::Primary || slot == EquippableSlot::Offhand {
            vec![
                self.equipment_slots.remove(&EquippableSlot::BothHands),
                self.equipment_slots.remove(&slot),
            ]
        } else {
            vec![self.equipment_slots.remove(&slot)]
        };

        unequipped_items
            .into_iter()
            .filter_map(|t| t.map(|t| t.0))
            .collect()
    }

    pub fn equip_item(&mut self, item: EquippableItem, item_e: Entity) -> Vec<Entity> {
        let unequipped = self.clear_space_for_slot(item.slot);

        if let Some(t) = self.equipment_slots.insert(item.slot, (item_e, item)) {
            error!("Cleared before adding, but found {:?}", t);
        }

        unequipped
    }

    /// Get the WeaponData that the Unit has, if any
    ///
    /// Assumes that weapons can only be held in specified slots, and that specified slots
    /// have priority. IE does not allow for second hand to have different data than primary
    pub fn weapon_data(&self) -> Option<WeaponData> {
        for slot in [
            EquippableSlot::BothHands,
            EquippableSlot::Primary,
            EquippableSlot::Offhand,
        ] {
            if let Some(weapon_data) = self
                .equipment_slots
                .get(&slot)
                .map(|t| t.1.weapon_data.clone())
                .flatten()
            {
                return Some(weapon_data);
            }
        }

        None
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

#[derive(Resource)]
pub struct ItemDB {
    pub equippable_items: HashMap<ItemId, EquippableItem>,
}

pub fn build_item_db() -> ItemDB {
    let equippable_items = HashMap::from([
        (
            ItemId(1),
            EquippableItem {
                item_name: "Iron Axe".to_string(),
                item_id: ItemId(1),
                slot: EquippableSlot::Primary,
                modifiers: Vec::new(),
                sprite_id: TinyTacticsSprites::IronAxe.into(),
                animated_sprite_id: TT_WEAPON_ANIMATED_SPRITE_ID,
                weapon_data: Some(WeaponData {
                    range: 1,
                    attack_skill: ATTACK_SKILL_ID,
                }),
            },
        ),
        (
            ItemId(2),
            EquippableItem {
                item_name: "Bow".to_string(),
                item_id: ItemId(2),
                slot: EquippableSlot::BothHands,
                modifiers: Vec::new(),
                sprite_id: TinyTacticsSprites::IronAxe.into(),
                animated_sprite_id: TT_WEAPON_ANIMATED_SPRITE_ID,
                weapon_data: Some(WeaponData {
                    range: 4,
                    attack_skill: SkillId(4),
                }),
            },
        ),
    ]);

    ItemDB { equippable_items }
}

pub fn setup_item_db(mut commands: Commands) {
    commands.insert_resource(build_item_db());
}

pub fn unequip_items_on_unit(
    commands: &mut Commands,
    equipment: &mut UnitEquipment,
    effects: &mut ActiveEffects,
    unit: Entity,
    slot: EquippableSlot,
) -> anyhow::Result<()> {
    for equipment_e in equipment.clear_space_for_slot(slot) {
        effects.effects.retain(|t| {
            if let Some(source) = t.metadata.source {
                source != equipment_e
            } else {
                true
            }
        });

        commands.entity(equipment_e).despawn();
    }

    commands.entity(unit).insert(StatsDirty);

    Ok(())
}

/// Equip an item on a unit
pub fn equip_item_on_unit(
    commands: &mut Commands,
    sprite_db: &SpriteDB,
    anim_db: &AnimationDB,
    unit_equipment: &mut UnitEquipment,
    unit_effects: &mut ActiveEffects,
    unit_e: Entity,
    item: EquippableItem,
) -> anyhow::Result<()> {
    unequip_items_on_unit(
        commands,
        unit_equipment,
        unit_effects,
        unit_e,
        item.slot.clone(),
    )
    .with_context(|| format!("Unequipping existing items at slot {:?}", item.slot))?;

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
    let item_e = commands
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
            AnimationFollower {
                leader: unit_e,
                animated_sprite_id: item.animated_sprite_id,
            },
            TINY_TACTICS_ANCHOR,
        ))
        .id();

    for modifier in &item.modifiers {
        unit_effects.effects.push(Effect {
            metadata: EffectMetadata {
                target: unit_e,
                source: Some(item_e),
            },
            data: EffectData {
                effect_type: crate::gameplay_effects::EffectType::StatBuff(modifier.clone()),
                duration: crate::gameplay_effects::EffectDuration::Permanent,
            },
        })
    }

    unit_equipment.equip_item(item, item_e);
    commands.entity(unit_e).add_child(item_e);

    Ok(())
}
