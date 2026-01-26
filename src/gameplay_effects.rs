use bevy::prelude::*;

use std::collections::HashSet;
use std::fmt::Debug;

use crate::{combat::skills::AttackModifier, unit::StatType};

/// Looking at GAS from Unreal as a motivator for this

// Choices
//
// Should I have different DamageTypes for different elements?
// Yes.
// - DamageType - Neutral, Fire, Shadow, Ice, Thunder
//
// Do I want skills to be able to choose what stats influence them?
// For both the defender and the attacker?
//
// Seems pretty fun I think. Let's do that with some scale?

// So an attack can then include...
// Vec<(DamageType, DamageScale)>

#[derive(Clone, Debug)]
pub struct ModifiyingProportion {
    modifier: AttackModifier,
    proportion: f32,
}

#[derive(Clone, Debug)]
/// Probably needs to replace `DamagingSkill`
pub struct Damage {
    pub base_damage: f32,
    pub damage_type: DamageType,
    pub offensive_scalar: Vec<ModifiyingProportion>,
    pub defensive_scalar: Vec<ModifiyingProportion>,
    pub combat_tags: HashSet<CombatTag>,
}

#[derive(Clone, Debug)]
pub struct HitChance {
    amount: f32,
    offensive_modifier: ModifiyingProportion,
    defensive_modifier: ModifiyingProportion,
}

#[derive(Clone, Debug)]
pub enum EffectType {
    StatBuff(StatModification),
    StatusInfliction(StatusTag),
    AffectsDamage(DamageEffect),
}

#[derive(Clone, Debug)]
pub enum AppliesTo<T: Debug + Clone> {
    Specific(T),
    All(Vec<T>),
    Any(Vec<T>),
    Always,
}

impl<T: PartialEq + Eq + Debug + Clone> AppliesTo<T> {
    fn check_applies(&self, t: &T) -> bool {
        match &self {
            AppliesTo::Specific(check) => check == t,
            AppliesTo::All(items) => items.iter().all(|v| v == t),
            AppliesTo::Any(items) => items.iter().any(|v| v == t),
            AppliesTo::Always => true,
        }
    }
}

// Increase Fire Damage by 20%
// Conditional: CombatTag::Damage
// AppliesTo: AppliesTo::Specific(DamageType::Fire)
// value: 1.2
// operator: Mul

#[derive(Debug, Clone)]
pub struct DamageEffect {
    applies_to: AppliesTo<DamageType>,
    value: f32,
    operator: Operator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DamageType {
    Neutral,
    Fire,
    Ice,
    Thunder,
    Holy,
    Shadow,
}

/// Maybe should be a bitset, but for now let's just use a HashSet
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum CombatTag {
    Damage,
    Healing,
    // Sort of types of damage
    Melee,
    Ranged,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum StatusTag {
    /// The target is poisoned
    Poisoned,
    /// The target is stunned
    Stunned,
}

#[derive(Clone, Debug)]
pub enum SkillParameter {
    Range,
}

#[derive(Clone, Debug)]
pub enum EffectDuration {
    /// The effect should last for this many turns
    TurnCount(u8),
    /// The effect can be applied this many times
    Consumable(u8),
    /// Permanent in the sense that it's on as long as it's "on"
    ///
    /// This can be used for passive skills / equipment
    Permanent,
}

#[derive(Clone, Debug)]
pub struct EffectData {
    pub effect_type: EffectType,
    pub duration: EffectDuration,
}

#[derive(Clone, Debug, Component)]
pub struct Effect {
    pub metadata: EffectMetadata,
    pub data: EffectData,
}

#[derive(Clone, Debug)]
pub struct EffectMetadata {
    pub target: Entity,
    pub source: Option<Entity>,
}

#[derive(Clone, Debug)]
pub enum AttributeType {
    Stat(StatType),
}

#[derive(Clone, Debug)]
pub enum Operator {
    Add,
    Mul,
}

#[derive(Clone, Debug)]
pub struct StatModification {
    attribute_type: StatType,
    operator: Operator,
    value: f32,
}

#[derive(Clone, Debug, Component)]
pub struct ActiveEffects {
    /// The ActiveEffects associated with this entity
    ///
    /// When effects are added, removed or consumed, it's
    /// assumed that these will be updated as well.
    pub effects: Vec<Effect>,
}

impl ActiveEffects {
    /// Is this the right layer? The UI would want to know why I can't move.
    pub fn prevent_move(&self) -> bool {
        self.has_status(StatusTag::Stunned)
    }

    pub fn prevent_action(&self) -> bool {
        self.has_status(StatusTag::Stunned)
    }

    pub fn apply_effect(&mut self, effect: Effect) {
        match effect.data.effect_type {
            EffectType::StatBuff(..) => {
                error!("Stat Buffs aren't implemented, plz don't apply them");
            }
            EffectType::StatusInfliction(status_tag) => {
                let mut doesnt_already_have_status = true;
                for existing_effect in self.effects.iter_mut() {
                    if let EffectType::StatusInfliction(existing_status_tag) =
                        existing_effect.data.effect_type
                        && status_tag == existing_status_tag
                    {
                        warn!(
                            "The target already had {:?} - updating duration",
                            existing_status_tag
                        );
                        doesnt_already_have_status = false;

                        // TODO: Probably should choose max here
                        existing_effect.data.duration = effect.data.duration.clone();
                    }
                }
                if doesnt_already_have_status {
                    self.effects.push(effect);
                }
            }
            EffectType::AffectsDamage(..) => {
                error!("Affect Damage Effects aren't implemented, plz don't apply them")
            }
        }
    }

    fn has_status(&self, tag: StatusTag) -> bool {
        self.has_any_status(Vec::from([tag]))
    }

    fn has_any_status(&self, tags: Vec<StatusTag>) -> bool {
        self.effects.iter().any(|t| {
            if let EffectType::StatusInfliction(found_tag) = t.data.effect_type {
                tags.contains(&found_tag)
            } else {
                false
            }
        })
    }

    pub fn statuses(&self) -> Vec<StatusTag> {
        self.effects
            .iter()
            .filter_map(|t| {
                if let EffectType::StatusInfliction(t) = t.data.effect_type {
                    Some(t)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn stat_buffs(&self) -> Vec<&StatModification> {
        self.effects
            .iter()
            .filter_map(|t| {
                if let EffectType::StatBuff(t) = &t.data.effect_type {
                    Some(t)
                } else {
                    None
                }
            })
            .collect()
    }
}
