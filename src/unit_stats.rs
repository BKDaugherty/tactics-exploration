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

impl From<f32> for StatValue {
    fn from(value: f32) -> Self {
        StatValue(value)
    }
}

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

pub mod experience {
    use bevy::prelude::*;
    use std::collections::BTreeMap;

    use crate::{
        unit::{UnitAction, UnitActionCompletedMessage},
        unit_stats::{StatType, StatValue, StatsDirty, UnitBaseStats, growths::StatGrowths},
    };

    /// It'd be fun to make specific actions give more or less xp dependent on results
    /// but I've always been frustrated with games that let u just farm xp. Maybe this should be
    /// more of a contribution tracker and xp should be doled out at end of fight?
    pub fn give_flat_xp_after_attack_action_complete(
        mut unit_action_completed: MessageReader<UnitActionCompletedMessage>,
        mut xp_query: Query<&mut UnitLevelManager>,
        mut level_up_writer: MessageWriter<LevelUpMessage>,
    ) {
        const ACTION_XP: f32 = 50.;
        for message in unit_action_completed.read() {
            if message.action != UnitAction::Attack {
                continue;
            }

            let Some(mut level_manager) = xp_query.get_mut(message.unit).ok() else {
                continue;
            };

            let events = level_manager.accept_experience(ACTION_XP);
            info!(
                "Unit {:?} at level {:?} gained {:?} XP producing level ups: {:?}",
                message.unit, level_manager.current_level, ACTION_XP, events
            );
            for event in events {
                level_up_writer.write(LevelUpMessage {
                    entity: message.unit,
                    level_up: event,
                });
            }
        }
    }

    #[derive(Debug, Component)]
    pub struct UnitLevelManager {
        current_level: u32,
        experience: f32,
        growths: StatGrowths,
    }

    impl UnitLevelManager {
        pub fn new(growths: StatGrowths) -> Self {
            Self {
                growths,
                current_level: 1,
                experience: 0.0,
            }
        }
    }

    impl UnitLevelManager {
        pub fn accept_experience(&mut self, exp: f32) -> Vec<LevelUp> {
            info!(
                "Accepting exp, current: {:?}, value: {:?}",
                self.experience, exp
            );
            self.experience += exp;
            let level_ups = (self.experience / 100.).floor() as u32;
            self.experience = self.experience % 100.;

            info!("Resolved XP: {:?} {:?}", self.experience, level_ups);

            let mut level_up_messages = Vec::new();

            for _ in 1..=level_ups {
                level_up_messages.push(LevelUp {
                    growths: self.growths.get_growths_for_level_up(),
                });
            }

            level_up_messages
        }
    }

    #[derive(Debug)]
    pub struct LevelUp {
        growths: BTreeMap<StatType, f32>,
    }

    #[derive(Message, Debug)]
    pub struct LevelUpMessage {
        entity: Entity,
        level_up: LevelUp,
    }

    pub fn apply_level_up_to_stats(
        mut commands: Commands,
        mut reader: MessageReader<LevelUpMessage>,
        mut unit_query: Query<(&mut UnitBaseStats, &mut UnitLevelManager)>,
    ) {
        for m in reader.read() {
            let Some((mut stats, mut level)) = unit_query.get_mut(m.entity).ok() else {
                error!("Invalid Entity got a level up: {:?}", m.entity);
                continue;
            };

            info!(
                "Unit {:?} at level {:?} leveled up!",
                m.entity, level.current_level
            );
            // TODO: Should I send a "StatChangeRequest here?"
            for (stat, growth_value) in &m.level_up.growths {
                let current = stats.stats.stat(*stat);
                stats
                    .stats
                    .with_stat(*stat, StatValue(current.0 + growth_value));
            }

            level.current_level += 1;

            commands.entity(m.entity).insert(StatsDirty);
        }
    }
}

pub mod growths {
    use bevy::prelude::*;

    use std::collections::BTreeMap;

    use rand::Rng;
    use rand_distr::Normal;
    use rand_pcg::Pcg64;
    use rand_seeder::Seeder;

    use crate::unit_stats::StatType;

    #[derive(Debug)]
    pub struct StatGrowths {
        pub growths: BTreeMap<StatType, Box<dyn StatGrowth>>,
    }

    impl StatGrowths {
        pub fn get_growths_for_level_up(&mut self) -> BTreeMap<StatType, f32> {
            let mut values = BTreeMap::new();
            for (stat_type, growth) in self.growths.iter_mut() {
                if let Some(grown_value) = growth.proc() {
                    values.insert(*stat_type, grown_value);
                }
            }
            values
        }
    }

    #[derive(Debug)]
    pub struct StatGrowthClampedNormalRounded {
        rng: Pcg64,
        chance: f32,
        distribution: Normal<f32>,
        min: f32,
        max: f32,
    }

    impl StatGrowthClampedNormalRounded {
        pub fn new(
            seed: String,
            chance: f32,
            distribution: Normal<f32>,
            min: f32,
            max: f32,
        ) -> Self {
            Self {
                rng: Seeder::from(seed).into_rng(),
                chance,
                distribution,
                min,
                max,
            }
        }
    }

    pub trait StatGrowth: Sync + Send + 'static + std::fmt::Debug {
        fn proc(&mut self) -> Option<f32>;
    }

    impl StatGrowth for StatGrowthClampedNormalRounded {
        fn proc(&mut self) -> Option<f32> {
            if self.rng.random::<f32>() < self.chance {
                let value = self
                    .rng
                    .sample(&self.distribution)
                    .clamp(self.min, self.max)
                    .round();
                Some(value)
            } else {
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use rand_distr::Normal;

        use crate::unit_stats::growths::{StatGrowth, StatGrowthClampedNormalRounded};

        #[test]
        fn test_growths() -> anyhow::Result<()> {
            let mut growth = StatGrowthClampedNormalRounded::new(
                "Hello Seed".to_string(),
                1.0,
                Normal::new(3.0, 2.0)?,
                2.0,
                6.0,
            );

            let value = growth.proc().expect("Chance is 1.0");
            // Value captured from le seed
            assert_eq!(value, 3.0);

            Ok(())
        }
    }
}
