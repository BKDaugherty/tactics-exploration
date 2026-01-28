use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    combat::UnitHealthChangedEvent,
    gameplay_effects::{ActiveEffects, Operator},
};

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Reflect, Hash)]
pub enum StatType {
    // Not super sure if I want health here, but maybe fine for now?
    Health,
    MaxHealth,
    Movement,
    Strength,
    Magic,
    Defense,
    Resistance,
    Speed,
    Skill,
}

impl StatType {
    pub const VARIANTS: &[StatType] = &[
        StatType::Health,
        StatType::MaxHealth,
        StatType::Movement,
        StatType::Strength,
        StatType::Magic,
        StatType::Defense,
        StatType::Resistance,
        StatType::Speed,
        StatType::Skill,
    ];

    pub fn abbreviation(&self) -> &'static str {
        match &self {
            StatType::Strength => "STR",
            StatType::Magic => "MAG",
            StatType::Defense => "DEF",
            StatType::Resistance => "RES",
            StatType::Speed => "SPD",
            StatType::Skill => "SKL",
            StatType::MaxHealth => "MAX HP",
            StatType::Movement => "MOVE",
            StatType::Health => "HP",
        }
    }
}

#[derive(PartialEq, Clone, Copy, Default, Debug)]
pub struct StatValue(pub f32);

#[derive(Clone, Default, Debug)]
pub struct StatContainer {
    stats: HashMap<StatType, StatValue>,
}

impl StatContainer {
    pub fn new() -> StatContainer {
        let mut stats = HashMap::new();
        for variant in StatType::VARIANTS {
            stats.insert(*variant, StatValue(0.));
        }
        StatContainer { stats }
    }

    /// Access a stat of a particular type
    pub fn stat(&self, stat_type: StatType) -> StatValue {
        self.stats
            .get(&stat_type)
            .expect("The game depends on a StatContainer always having a value for all stats")
            .to_owned()
    }

    /// Update a stat of a particular type
    pub fn with_stat(&mut self, stat_type: StatType, value: StatValue) -> &mut Self {
        let _ = self.stats.insert(stat_type, value);
        self
    }
}

pub fn derive_stats(
    mut commands: Commands,
    unit_query: Query<
        (
            Entity,
            &UnitBaseStats,
            &mut UnitDerivedStats,
            Option<&ActiveEffects>,
        ),
        With<StatsDirty>,
    >,
) {
    for (e, base_stats, mut derived, active_effects) in unit_query {
        let stat_modifications = active_effects.map(|t| t.stat_buffs()).unwrap_or_default();
        for stat in StatType::VARIANTS {
            let mut base = base_stats.stats.stat(*stat);
            for modification in &stat_modifications {
                if modification.attribute_type != *stat {
                    continue;
                }

                // TODO: Probably need to apply all adds first and then do mul?
                match modification.operator {
                    Operator::Add => base.0 += modification.value,
                    Operator::Mul => base.0 *= modification.value,
                };
            }

            derived.stats.with_stat(*stat, base);
        }
        commands.entity(e).remove::<StatsDirty>();
    }
}

#[derive(Component)]
pub struct UnitBaseStats {
    pub stats: StatContainer,
}

#[derive(Component)]
pub struct UnitDerivedStats {
    pub stats: StatContainer,
}

impl UnitDerivedStats {
    /// Whether or not the unit is at 0 health
    pub fn downed(&self) -> bool {
        self.stats.stat(StatType::Health) == StatValue(0.)
    }

    // Not downed, but less than 30% of max health is "critical"
    pub fn critical_health(&self) -> bool {
        !self.downed()
            && (self.stats.stat(StatType::Health).0 / self.stats.stat(StatType::MaxHealth).0) <= 0.3
    }
}

#[derive(Debug, Message)]
pub struct UnitStatChangeRequest {
    pub entity: Entity,
    pub stat: StatType,
    pub stat_change: StatValue,
}

#[derive(Component)]
pub struct StatsDirty;

pub fn handle_stat_changes(
    mut reader: MessageReader<UnitStatChangeRequest>,
    mut query: Query<(&mut UnitBaseStats, &mut UnitDerivedStats)>,
    mut health_changed_writer: MessageWriter<UnitHealthChangedEvent>,
) {
    for message in reader.read() {
        let Some((mut base_stats, mut derived_stats)) = query.get_mut(message.entity).ok() else {
            error!(
                "No stats found for Unit. Could not process request: {:?}",
                message
            );
            continue;
        };

        let current = base_stats.stats.stat(message.stat);
        let next = StatValue(current.0 + message.stat_change.0);
        info!(
            "Unit {:?} {:?} {:?} -> {:?}",
            message.entity, message.stat, current, next
        );

        // TODO: Special handling for this could be encoded for other types
        // that are capped by another stat?
        let next = if message.stat == StatType::Health {
            StatValue(f32::max(
                f32::min(next.0, derived_stats.stats.stat(StatType::MaxHealth).0),
                0.,
            ))
        } else {
            StatValue(f32::max(0., next.0))
        };

        if message.stat == StatType::Health {
            let difference = next.0 - current.0;
            health_changed_writer.write(UnitHealthChangedEvent {
                unit: message.entity,
                health_changed: difference.round() as i32,
            });
        }

        // TODO: Recalculate UnitDerivedStats using any ActiveEffects once those exist, or maybe
        // the other thing we could do is just insert a Component here?
        base_stats.stats.with_stat(message.stat, next);
        derived_stats.stats.with_stat(message.stat, next);
    }
}
