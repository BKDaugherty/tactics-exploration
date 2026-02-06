use crate::unit::StatType;
use bevy::prelude::*;
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum EffectType {
    StatBuff(StatModification),
    StatusInfliction(StatusTag),
}

#[derive(Clone, Debug)]
pub enum AppliesTo<T: Debug + Clone> {
    Specific(T),
    All(Vec<T>),
    Any(Vec<T>),
    Always,
}

impl<T: PartialEq + Eq + Debug + Clone> AppliesTo<T> {
    #[allow(dead_code)]
    fn check_applies(&self, t: &T) -> bool {
        match &self {
            AppliesTo::Specific(check) => check == t,
            AppliesTo::All(items) => items.iter().all(|v| v == t),
            AppliesTo::Any(items) => items.iter().any(|v| v == t),
            AppliesTo::Always => true,
        }
    }
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
    pub attribute_type: StatType,
    pub operator: Operator,
    pub value: f32,
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
                self.effects.push(effect);
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
                    info!(
                        "[DEBUG] Status {:?} applied with duration {:?}",
                        status_tag, effect.data.duration
                    );
                    self.effects.push(effect);
                }
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
