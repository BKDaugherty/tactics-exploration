use bevy::prelude::*;

use crate::{
    animation::{AnimationMarker, AnimationMarkerMessage},
    unit::Unit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttackPhase {
    /// The attacker is preparing for the hit
    Windup,
    ///
    PostWindup,
    /// The attacker has attempted to hit the defender
    Impact,
    /// The impact of the attack has been resolved by the attack impact system
    PostImpact,
    /// The attack is complete.
    Done,
}

pub struct AttackOutcome {
    // Whether or not the defender was hit
    pub defender_reaction: DefenderReaction,
    // TODO: Unify this with DefenderReaction probably
    pub damage: u32,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum DefenderReaction {
    Dodge,
    TakeHit,
}

#[derive(Component)]
pub struct AttackExecution {
    pub attacker: Entity,
    pub defender: Entity,
    pub phase: AttackPhase,
    pub animation_phase: AttackPhase,
    pub outcome: AttackOutcome,
}

#[derive(Component)]
pub struct AttackIntent {
    pub attacker: Entity,
    pub defender: Entity,
}

/// Given an AttackIntent by a Unit, process it
/// and spawn an AttackExecution for the engine to drive animations and
/// changes to the game.
///
/// Note that we expect this system to do all of the actual calculating of
/// what happened in the attack
pub fn attack_intent_system(mut commands: Commands, intent_query: Query<(Entity, &AttackIntent)>) {
    for (e, intent) in intent_query {
        let mut tracker = commands.entity(e);
        tracker.remove::<AttackIntent>();

        // TODO: For now we just assume everything hits and does 1 "damage"
        tracker.insert(AttackExecution {
            attacker: intent.attacker,
            defender: intent.defender,
            phase: AttackPhase::Windup,
            animation_phase: AttackPhase::Windup,
            outcome: AttackOutcome {
                defender_reaction: DefenderReaction::TakeHit,
                damage: 4,
            },
        });
    }
}

/// Respond to AnimationMarkerEvents to drive
/// forward the AttackExecution's Phase
///
/// TODO: If I just had the systems pull from the AnimationMarkerEvents themselves, I think I wouldn't need
/// to add these separate phases for the two systems, as each system gets a unique message I think.
pub fn advance_attack_phase_based_on_attack_animation_markers(
    mut marker_events: MessageReader<AnimationMarkerMessage>,
    mut attacks: Query<&mut AttackExecution>,
) {
    for ev in marker_events.read() {
        for mut attack in &mut attacks {
            if ev.entity == attack.attacker {
                match ev.marker {
                    AnimationMarker::HitFrame => {
                        if attack.phase == AttackPhase::Windup {
                            attack.phase = AttackPhase::Impact;
                            attack.animation_phase = AttackPhase::Impact;
                        }
                    }
                    AnimationMarker::Complete => {
                        attack.phase = AttackPhase::Done;
                        attack.animation_phase = AttackPhase::Done;
                    }
                }
            }
        }
    }
}

/// Drives an AttackExecution from Impact -> PostImpact, applying any
/// effects necessary for the Attack.
pub fn attack_impact_system(
    mut attacks: Query<&mut AttackExecution>,
    mut unit_query: Query<&mut Unit>,
) {
    for mut attack in &mut attacks {
        if attack.phase == AttackPhase::Impact {
            if attack.outcome.defender_reaction == DefenderReaction::TakeHit {
                if let Some(mut defending_unit) = unit_query.get_mut(attack.defender).ok() {
                    defending_unit.stats.health = defending_unit
                        .stats
                        .health
                        .saturating_sub(attack.outcome.damage);
                };

                attack.phase = AttackPhase::PostImpact;
            }
        }
    }
}

/// Cleanup AttackExecutions after we know they've been fully handled
pub fn attack_execution_despawner(
    mut commands: Commands,
    attacks: Query<(Entity, &AttackExecution)>,
) {
    for (e, attack) in attacks {
        if attack.phase == AttackPhase::Done {
            commands.entity(e).despawn();
        }
    }
}
