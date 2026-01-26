use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::assets::FontResource;
use crate::assets::sounds::AudioContext;
use crate::assets::sounds::AudioCue;
use crate::assets::sounds::AudioEventMessage;
use crate::assets::sounds::ImpactInteractionRole;
use crate::gameplay_effects::ActiveEffects;
use crate::gameplay_effects::Effect;
use crate::gameplay_effects::EffectMetadata;
use crate::unit::StatType;
use crate::unit::StatValue;
use crate::unit::UnitBaseStats;
use crate::unit::UnitDerivedStats;
use crate::{
    animation::{
        AnimToPlay, AnimationId, AnimationMarker, AnimationMarkerMessage, PlayingAnimation,
        UnitAnimationKind, UnitAnimationPlayer,
        animation_db::{AnimationDB, AnimationKey, AnimationStartIndexKey},
        combat::HURT_BY_ATTACK_FRAME_DURATION,
    },
    assets::sprite_db::SpriteDB,
    battle_phase::UnitPhaseResources,
    combat::skills::{
        CastingData, Skill, SkillAction, SkillActionType, SkillAnimationId, SkillDBResource,
        SkillEvent, SkillId,
    },
    grid::{GridPosition, init_grid_to_world_transform},
    projectile::{ProjectileArrived, spawn_arrow},
    unit::{TINY_TACTICS_ANCHOR, Unit, UnitAction, UnitActionCompletedMessage},
};

#[derive(Component)]
pub struct AttackExecution {
    pub attacker: Option<Entity>,
    pub defender: Entity,
    pub skill: Skill,
    pub combat_timeline: CombatTimeline,
}

#[derive(Debug)]
pub enum AttackExecutionTrigger {
    AnimationMarker(CombatAnimationId, AnimationMarker),
    ProjectileImpactEvent(CombatAnimationId),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct CombatStageId(pub u32);

#[derive(Debug)]
enum CombatStage {
    UnitAttack(Entity, CombatAnimationId, UnitAnimationKind),
    Cast(GridPosition, CastingData),
    Impact(Option<Entity>, Entity, Vec<SkillAction>, SkillId),
}

pub struct CombatTimeline {
    current_stage: CombatStageId,
    stages: BTreeMap<CombatStageId, CombatStage>,
    conditions_to_advance: BTreeMap<CombatStageId, Vec<AttackExecutionTrigger>>,
    audio_events_to_emit_on_stage_complete: BTreeMap<CombatStageId, Vec<AudioCue>>,
}

impl CombatTimeline {
    fn new() -> Self {
        Self {
            current_stage: CombatStageId(0),
            stages: BTreeMap::new(),
            conditions_to_advance: BTreeMap::from([(CombatStageId(0), Vec::new())]),
            audio_events_to_emit_on_stage_complete: BTreeMap::new(),
        }
    }

    fn parse_triggers(ae_entity: Entity, trigger: &SkillEvent) -> Vec<AttackExecutionTrigger> {
        match trigger {
            SkillEvent::AnimationMarker(skill_animation_id, animation_marker) => {
                vec![AttackExecutionTrigger::AnimationMarker(
                    CombatAnimationId::new(ae_entity, *skill_animation_id),
                    *animation_marker,
                )]
            }
            SkillEvent::ProjectileImpact(skill_animation_id) => {
                vec![AttackExecutionTrigger::ProjectileImpactEvent(
                    CombatAnimationId::new(ae_entity, *skill_animation_id),
                )]
            }
        }
    }

    /// Assumes that there's no attacker for this skill variant
    ///
    /// Returns an error if it needs an attacker to do something.
    pub fn build_without_attacker(
        ae_entity: Entity,
        skill: Skill,
        defender_e: Entity,
        defender_grid_pos: &GridPosition,
    ) -> anyhow::Result<Self> {
        let mut timeline = CombatTimeline::new();
        let mut stage_id = timeline.current_stage;
        stage_id.0 += 1;
        for skill_stage in &skill.animation_data {
            let stage = match &skill_stage.stage {
                skills::SkillStageAction::Cast(casting_data) => {
                    CombatStage::Cast(*defender_grid_pos, casting_data.clone())
                }
                skills::SkillStageAction::Impact(action_indices) => {
                    let mut actions = Vec::new();
                    for action_index in action_indices {
                        if let Some(action) = skill.actions.get(action_index.0) {
                            actions.push(action.clone());
                        }
                    }
                    CombatStage::Impact(None, defender_e, actions, skill.skill_id)
                }
                otherwise => {
                    anyhow::bail!(
                        "Unsupported skill stage passed to build_without_attacker: {:?}",
                        otherwise
                    );
                }
            };

            let triggers = Self::parse_triggers(ae_entity, &skill_stage.advancing_event);
            timeline.stages.insert(stage_id, stage);
            timeline.conditions_to_advance.insert(stage_id, triggers);
            stage_id.0 += 1;
        }

        Ok(timeline)
    }
}

#[derive(Component, Clone, Debug)]
pub struct AttackIntent {
    pub attacker: Entity,
    pub defender: Entity,
    pub skill: SkillId,
}

/// Assumes everything is gonna hit for now
fn calculate_damage(
    attacker: Option<&UnitDerivedStats>,
    defender: &UnitDerivedStats,
    skill_actions: &Vec<SkillAction>,
) -> i32 {
    let (mut damage, mut healing) = (0, 0);
    for action in skill_actions {
        if let SkillActionType::DamagingSkill { scaled_damage } = &action.action_type {
            let bonus_attack = attacker
                .map(|t| {
                    scaled_damage
                        .offensive_modifier
                        .as_ref()
                        .map(|modifier| t.stats.stat(modifier.stat))
                })
                .flatten()
                .unwrap_or_default()
                .0 as u32;
            let defense = scaled_damage
                .defensive_modifier
                .as_ref()
                .map(|ref t| defender.stats.stat(t.stat))
                .unwrap_or_default()
                .0 as u32;
            damage += scaled_damage.power + bonus_attack - defense;
        }
    }
    for action in skill_actions {
        if let SkillActionType::HealingSkill { scaled_damage } = &action.action_type {
            let bonus_attack = attacker
                .map(|t| {
                    scaled_damage
                        .offensive_modifier
                        .as_ref()
                        .map(|modifier| t.stats.stat(modifier.stat))
                })
                .flatten()
                .unwrap_or_default()
                .0 as u32;
            let defense = scaled_damage
                .defensive_modifier
                .as_ref()
                .map(|ref t| defender.stats.stat(t.stat))
                .unwrap_or_default()
                .0 as u32;
            healing += scaled_damage.power + bonus_attack - defense;
        }
    }
    healing as i32 - damage as i32
}

#[derive(Message)]
pub struct CombatStageComplete {
    attack_execution: Entity,
    stage_id: CombatStageId,
}

pub fn listen_for_combat_conditions(
    mut animation_markers: MessageReader<AnimationMarkerMessage>,
    mut projectiles: MessageReader<ProjectileArrived>,
    mut ae: Query<&mut AttackExecution>,
) {
    for m in animation_markers.read() {
        let Some(AnimationId::Combat(ref combat_id)) = m.id else {
            continue;
        };

        let Some(mut ae) = ae.get_mut(combat_id.ae_id).ok() else {
            error!("Stale AE Referenced? {:?}", combat_id);
            continue;
        };

        info!("Received Marker Event: {:?}", m);

        info!(
            "Before conditions: {:?}",
            ae.combat_timeline.conditions_to_advance
        );
        // lol well this is just incredibly inefficient
        for (_, conditions) in ae.combat_timeline.conditions_to_advance.iter_mut() {
            conditions.retain(|t| {
                if let AttackExecutionTrigger::AnimationMarker(cid, anim_marker) = t {
                    !(cid == combat_id && *anim_marker == m.marker)
                } else {
                    true
                }
            })
        }

        info!(
            "After Conditions: {:?}",
            ae.combat_timeline.conditions_to_advance
        );
    }

    // TODO: Deduplicate so there's one sink for these messages
    for m in projectiles.read() {
        let Some(mut ae) = ae.get_mut(m.ae_entity).ok() else {
            error!("Stale AE Referenced? {:?}", m.ae_entity);
            continue;
        };

        for (_, conditions) in ae.combat_timeline.conditions_to_advance.iter_mut() {
            conditions.retain(|t| {
                if let AttackExecutionTrigger::ProjectileImpactEvent(cid) = t {
                    !(*cid == m.combat_anim_id)
                } else {
                    true
                }
            })
        }
    }
}

pub fn check_combat_timeline_should_advance(
    mut ae: Query<(Entity, &mut AttackExecution)>,
    mut message_writer: MessageWriter<CombatStageComplete>,
    mut audio_event_writer: MessageWriter<AudioEventMessage>,
) {
    for (ae_entity, mut ae) in ae.iter_mut() {
        let combat_timeline = &ae.combat_timeline;
        let Some(conditions_to_advance) = combat_timeline
            .conditions_to_advance
            .get(&combat_timeline.current_stage)
        else {
            error!(
                "Combat Stage for AE has no conditions, shouldn't it have been handled by listen_for_combat_conditions?: {:?}",
                ae_entity
            );
            continue;
        };

        if conditions_to_advance.is_empty() {
            message_writer.write(CombatStageComplete {
                attack_execution: ae_entity,
                stage_id: ae.combat_timeline.current_stage,
            });

            if let Some(events) = ae
                .combat_timeline
                .audio_events_to_emit_on_stage_complete
                .get(&ae.combat_timeline.current_stage)
            {
                for event in events {
                    audio_event_writer.write(
                        // Should source be the attacker or the AE?
                        AudioEventMessage {
                            source: ae_entity,
                            cue: *event,
                            audio_context: AudioContext {
                                skill_id: Some(ae.skill.skill_id),
                            },
                        },
                    );
                }
            }

            info!(
                "Advancing from {:?} to {:?}",
                ae.combat_timeline.current_stage,
                ae.combat_timeline.current_stage.0 + 1
            );
            ae.combat_timeline.current_stage.0 += 1;
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct CombatAnimationId {
    pub ae_id: Entity,
    pub skill_animation_id: SkillAnimationId,
}

impl CombatAnimationId {
    pub fn new(ae_id: Entity, skill_anim: SkillAnimationId) -> Self {
        Self {
            ae_id,
            skill_animation_id: skill_anim,
        }
    }
}

/// This is basically a collision?
#[derive(Message)]
pub struct ImpactEvent {
    attacker: Option<Entity>,
    /// TBD, maybe a GridPosition instead to calculate whether or not the hit should happen?
    defender: Entity,
    // grid_position: GridPosition,
    skill_actions: Vec<SkillAction>,
    skill_id: SkillId,
    attack_execution: Entity,
}

#[derive(Component)]
pub struct VFXMarker {}

/// This seems like a good thing to put in an observer maybe?
pub fn cleanup_vfx_on_animation_complete(
    mut commands: Commands,
    mut messages: MessageReader<AnimationMarkerMessage>,
    query: Query<Entity, With<VFXMarker>>,
) {
    for message in messages.read() {
        if let Ok(e) = query.get(message.entity)
            && message.marker == AnimationMarker::Complete
        {
            commands.entity(e).despawn();
        }
    }
}

pub fn spawn_tile_sprite_vfx_on_grid_pos(
    commands: &mut Commands,
    anim_db: &AnimationDB,
    sprite_db: &SpriteDB,
    ae_entity: Entity,
    casting_data: &CastingData,
    grid_pos: &GridPosition,
) -> anyhow::Result<Entity> {
    let CastingData::TileSprite(sprite_id, anim_key, skill_anim_id) = casting_data else {
        anyhow::bail!("Must call spawn_tile_sprite_vfx with CastingData::TileSprite");
    };

    let Some(cast_image) = sprite_db.sprite_id_to_handle.get(sprite_id) else {
        anyhow::bail!("No image found for SpriteId {:?}", sprite_id);
    };

    let Some(atlas) = anim_db.get_atlas(&anim_key.animated_sprite_id) else {
        anyhow::bail!(
            "No Texture Atlas Layout found for Animated Sprite Id: {:?}",
            anim_key.animated_sprite_id
        );
    };

    let Some(start_index) = anim_db.get_start_index(&AnimationStartIndexKey {
        facing_direction: None,
        key: anim_key.clone(),
    }) else {
        anyhow::bail!(
            "No Texture Atlas Layout found for Animated Sprite Id: {:?}",
            anim_key.animated_sprite_id
        );
    };

    let Some(anim_data) = anim_db.get_data(anim_key) else {
        anyhow::bail!("no animation data found for vfx {:?}", anim_key);
    };

    let e = commands
        .spawn((
            Sprite {
                image: cast_image.clone(),
                texture_atlas: Some(TextureAtlas {
                    index: (*start_index).into(),
                    layout: atlas,
                }),
                color: Color::WHITE,
                custom_size: Some(Vec2::splat(32.)),
                ..Default::default()
            },
            UnitAnimationPlayer::new_with_animation(
                anim_key.animated_sprite_id,
                PlayingAnimation {
                    animation_id: Some(AnimationId::Combat(CombatAnimationId::new(
                        ae_entity,
                        *skill_anim_id,
                    ))),
                    id: anim_key.animation_id,
                    frame: 0,
                    timer: Timer::from_seconds(anim_data.frame_duration, TimerMode::Repeating),
                },
            ),
            *grid_pos,
            TINY_TACTICS_ANCHOR,
            VFXMarker {},
        ))
        .id();

    Ok(e)
}

pub fn handle_combat_stage_enter(
    mut commands: Commands,
    mut messages: MessageReader<CombatStageComplete>,
    mut impact_event: MessageWriter<ImpactEvent>,
    mut ae_query: Query<&mut AttackExecution>,
    mut animation_player: Query<&mut UnitAnimationPlayer>,
    // GridPosition Query?
    grid_position_query: Query<&GridPosition>,
    // Things needed for VFX Spawning
    anim_db: Res<AnimationDB>,
    sprite_db: Res<SpriteDB>,
) {
    for message in messages.read() {
        let Some(ae) = ae_query.get_mut(message.attack_execution).ok() else {
            error!("CombatStage passed an invalid entity for Attack Execution");
            continue;
        };

        if message.stage_id.0 != ae.combat_timeline.current_stage.0 - 1 {
            error!(
                "Stale CombatStageComplete message received? {:?}, {:?}",
                message.stage_id, ae.combat_timeline.current_stage
            );
            continue;
        }

        if let Some(stage) = ae
            .combat_timeline
            .stages
            .get(&ae.combat_timeline.current_stage)
        {
            match stage {
                CombatStage::UnitAttack(attacker, combat_id, unit_animation_kind) => {
                    let Some(mut player) = animation_player.get_mut(*attacker).ok() else {
                        error!("No Animation Player on Entity performing Unit Attack");
                        continue;
                    };

                    let Some(anim_data) = anim_db.get_data(&AnimationKey {
                        animated_sprite_id: player.animated_sprite_id,
                        animation_id: (*unit_animation_kind).into(),
                    }) else {
                        error!(
                            "No animation data for unit combat attack: {:?}",
                            unit_animation_kind
                        );
                        continue;
                    };

                    // Need a better API here right?
                    player.play_with_id(
                        AnimToPlay {
                            id: (*unit_animation_kind).into(),
                            frame_duration: anim_data.frame_duration,
                        },
                        AnimationId::Combat(combat_id.clone()),
                    );
                }
                CombatStage::Cast(defender_pos, casting_data) => {
                    // Should we use a separate system for this and send a SpawnVFXMessage?
                    match casting_data {
                        CastingData::TileSprite(..) => {
                            if let Err(e) = spawn_tile_sprite_vfx_on_grid_pos(
                                &mut commands,
                                &anim_db,
                                &sprite_db,
                                message.attack_execution,
                                casting_data,
                                defender_pos,
                            ) {
                                error!("Failed to spawn tile sprite vfx: {:?}", e);
                                continue;
                            }
                        }
                        CastingData::Projectile(sprite, skill_anim_id) => {
                            let combat_anim_id =
                                CombatAnimationId::new(message.attack_execution, *skill_anim_id);

                            let Some(image) = sprite_db.sprite_id_to_handle.get(sprite) else {
                                error!("No image for Projectile Sprite");
                                continue;
                            };

                            let Some(start_grid_pos) = ae
                                .attacker
                                .map(|t| grid_position_query.get(t).ok())
                                .flatten()
                            else {
                                error!("No attacker, or no grid position for spawned projectile!");
                                continue;
                            };

                            let start = init_grid_to_world_transform(start_grid_pos);
                            let end = init_grid_to_world_transform(defender_pos);

                            spawn_arrow(
                                &mut commands,
                                combat_anim_id,
                                message.attack_execution,
                                start.translation,
                                end.translation,
                                image.clone(),
                            );
                        }
                    }
                }
                CombatStage::Impact(entity, entity1, items, skill_id) => {
                    // TODO: Counterattacks should write back to the
                    // CombatStage.
                    impact_event.write(ImpactEvent {
                        attacker: *entity,
                        defender: *entity1,
                        skill_actions: items.clone(),
                        skill_id: *skill_id,
                        attack_execution: message.attack_execution,
                    });
                }
            }
        } else {
            commands
                .entity(message.attack_execution)
                .remove::<AttackExecution>()
                .insert(AttackResolved {
                    attacker: ae.attacker,
                });
        }
    }
}

#[derive(Component)]
pub struct AttackResolved {
    attacker: Option<Entity>,
}

fn build_timeline_for_skill(
    ae_entity: Entity,
    attack_intent: &AttackIntent,
    skill: &Skill,
    defender_grid_pos: &GridPosition,
) -> CombatTimeline {
    let mut timeline = CombatTimeline::new();
    let mut stage_id = timeline.current_stage;
    stage_id.0 += 1;
    for skill_stage in &skill.animation_data {
        let stage = match &skill_stage.stage {
            skills::SkillStageAction::UnitAttack(skill_animation_id, unit_animation_kind) => {
                CombatStage::UnitAttack(
                    attack_intent.attacker,
                    CombatAnimationId::new(ae_entity, *skill_animation_id),
                    *unit_animation_kind,
                )
            }
            skills::SkillStageAction::Cast(casting_data) => {
                // TODO: Probably need the target Grid space?
                CombatStage::Cast(*defender_grid_pos, casting_data.clone())
            }
            skills::SkillStageAction::Impact(items) => {
                let mut actions: Vec<SkillAction> = Vec::new();
                for item in items {
                    let Some(action) = skill.actions.get(item.0) else {
                        error!(
                            "Skill {:?} seems to be misdefined as it does not have {:?}",
                            skill, item
                        );
                        continue;
                    };
                    actions.push(action.to_owned());
                }
                CombatStage::Impact(
                    Some(attack_intent.attacker),
                    attack_intent.defender,
                    actions,
                    skill.skill_id,
                )
            }
        };

        let triggers = CombatTimeline::parse_triggers(ae_entity, &skill_stage.advancing_event);

        timeline.stages.insert(stage_id, stage);

        timeline.conditions_to_advance.insert(stage_id, triggers);

        stage_id.0 += 1;
    }

    timeline.audio_events_to_emit_on_stage_complete = skill
        .audio_cues_to_emit
        .to_emit_on_exit
        .clone()
        .into_iter()
        .map(|(k, v)| (k.into(), v))
        .collect();

    timeline
}

/// A marker component for tracking that a given unit is attacking
#[derive(Component)]
pub struct UnitIsAttacking {
    ae_entity: Entity,
}

#[derive(Message)]
pub struct UnitHealthChangedEvent {
    pub unit: Entity,
    pub health_changed: i32,
}

/// Given an AttackIntent by a Unit, process it
/// and spawn an AttackExecution for the engine to drive animations and
/// changes to the game.
///
pub fn attack_intent_system(
    mut commands: Commands,
    skill_db: Res<SkillDBResource>,
    intent_query: Query<(Entity, &AttackIntent)>,
    unit_query: Query<(&Unit, &GridPosition)>,
    mut attacker_resource_query: Query<&mut UnitPhaseResources>,
) {
    for (e, intent) in intent_query {
        commands
            .entity(intent.attacker)
            .insert(UnitIsAttacking { ae_entity: e });

        let mut tracker = commands.entity(e);
        tracker.remove::<AttackIntent>();

        let Some((attacker, attacker_grid_pos)) = unit_query.get(intent.attacker).ok() else {
            error!("Attack Intent originated from an Attacker that no longer exists?");
            continue;
        };

        let Some((defender, defender_grid_pos)) = unit_query.get(intent.defender).ok() else {
            error!("Attack Intent is attacking a defender that no longer exists?");
            continue;
        };

        let skill = skill_db.skill_db.get_skill(&intent.skill);
        let combat_timeline = build_timeline_for_skill(e, intent, skill, defender_grid_pos);

        let Some(mut attacker_resources) = attacker_resource_query.get_mut(intent.attacker).ok()
        else {
            error!("Attacker has no resources!");
            continue;
        };

        attacker_resources.action_points_left_in_phase = attacker_resources
            .action_points_left_in_phase
            .saturating_sub(skill.cost.ap as u32);

        // TODO: Create the concept of an AttackPreview, and ask the player for confirmation.
        tracker.insert(AttackExecution {
            attacker: Some(intent.attacker),
            defender: intent.defender,
            skill: skill.to_owned(),
            combat_timeline,
        });
    }
}

#[derive(Component)]
pub struct DamageText;

pub fn despawn_after_timer_completed<Marker: Component>(
    mut commands: Commands,
    time: Res<Time>,
    query: Query<(Entity, &mut DespawnTimer), With<Marker>>,
) {
    let delta = time.delta();
    for (e, mut t) in query {
        t.timer.tick(delta);

        if t.timer.is_finished() {
            commands.entity(e).despawn()
        }
    }
}

#[derive(Component)]
pub struct DespawnTimer {
    pub timer: Timer,
}

pub fn spawn_damage_text(
    mut commands: Commands,
    mut message_reader: MessageReader<UnitHealthChangedEvent>,
    fonts: Res<FontResource>,
) {
    for message in message_reader.read() {
        let health_changed_text = if message.health_changed > 0 {
            (
                Text2d(format!("+ {} HP", message.health_changed.abs())),
                TextColor(Color::linear_rgb(0.0, 1.0, 0.0)),
                TextFont {
                    font: fonts.pixelify_sans_regular.clone(),
                    font_size: 12.,
                    font_smoothing: bevy::text::FontSmoothing::None,
                    ..Default::default()
                },
                DamageText,
                DespawnTimer {
                    timer: Timer::from_seconds(0.5, TimerMode::Once),
                },
                Transform::from_translation(Vec3::new(0., 36., 0.)),
                TextBackgroundColor(Color::WHITE.with_alpha(0.5)),
            )
        } else if message.health_changed < 0 {
            (
                Text2d(format!("- {} HP", message.health_changed.abs())),
                TextColor(Color::linear_rgb(1.0, 0.0, 0.0)),
                TextFont {
                    font: fonts.pixelify_sans_regular.clone(),
                    font_size: 12.,
                    font_smoothing: bevy::text::FontSmoothing::None,
                    ..Default::default()
                },
                DamageText,
                DespawnTimer {
                    timer: Timer::from_seconds(0.5, TimerMode::Once),
                },
                Transform::from_translation(Vec3::new(0., 36., 0.)),
                TextBackgroundColor(Color::WHITE.with_alpha(0.5)),
            )
        } else {
            error!("0 damage reached spawn_damage_text");
            continue;
        };
        commands
            .entity(message.unit)
            .with_child(health_changed_text);
    }
}

#[derive(Debug, Message)]
pub struct UnitStatChangeRequest {
    entity: Entity,
    stat: StatType,
    stat_change: StatValue,
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

// TODO: Should Attacker be Optional here?
//
// TODO: Stats how to handle Health changes here? Should that be an update to
// UnitBaseStats? Or just a direct update to UnitDerivedStats?
//
// How would you "re-derive" stats? You need "permanent" mutations to apply to base stats I think...
pub fn impact_event_handler(
    mut impact_events: MessageReader<ImpactEvent>,
    mut unit_query: Query<(
        &UnitDerivedStats,
        Option<&mut UnitAnimationPlayer>,
        &mut ActiveEffects,
    )>,
    mut stat_change_request: MessageWriter<UnitStatChangeRequest>,
    mut audio_writer: MessageWriter<AudioEventMessage>,
) {
    for impact in impact_events.read() {
        let attacker = impact
            .attacker
            .map(|t| unit_query.get(t).ok().map(|(attacker, _, _)| attacker))
            .flatten();

        let Some((defender_derived, _, _)) = unit_query.get(impact.defender).ok() else {
            continue;
        };

        let damage = calculate_damage(attacker, defender_derived, &impact.skill_actions);

        if let Ok((defender_derived_stats, mut animation_player, _)) =
            unit_query.get_mut(impact.defender)
        {
            if damage < 0 {
                stat_change_request.write(UnitStatChangeRequest {
                    entity: impact.defender,
                    stat: StatType::Health,
                    stat_change: StatValue(damage as f32),
                });

                if let Some(animation_player) = animation_player.as_mut() {
                    animation_player.play(AnimToPlay {
                        id: UnitAnimationKind::TakeDamage.into(),
                        frame_duration: HURT_BY_ATTACK_FRAME_DURATION,
                    });
                }

                audio_writer.write(AudioEventMessage {
                    source: impact.attack_execution,
                    cue: AudioCue::Hit,
                    audio_context: AudioContext {
                        skill_id: Some(impact.skill_id),
                    },
                });
            }

            if damage > 0 {
                stat_change_request.write(UnitStatChangeRequest {
                    entity: impact.defender,
                    stat: StatType::Health,
                    stat_change: StatValue(damage as f32),
                });

                if let Some(animation_player) = animation_player.as_mut() {
                    animation_player.play(AnimToPlay {
                        id: UnitAnimationKind::TakeDamage.into(),
                        frame_duration: HURT_BY_ATTACK_FRAME_DURATION,
                    });
                }

                audio_writer.write(AudioEventMessage {
                    source: impact.attack_execution,
                    cue: AudioCue::Healed,
                    audio_context: AudioContext {
                        skill_id: Some(impact.skill_id),
                    },
                });
            }
        }

        if let Ok((_, _, mut defender_effects)) = unit_query.get_mut(impact.defender) {
            for action in &impact.skill_actions {
                let SkillActionType::ApplyEffects { effects } = &action.action_type else {
                    continue;
                };

                // Attach a Gameplay effect to the unit
                for effect in effects {
                    defender_effects.apply_effect(Effect {
                        metadata: EffectMetadata {
                            source: impact.attacker,
                            target: impact.defender,
                        },
                        data: effect.clone(),
                    });
                }
            }
        }
    }
}

/// Cleanup AttackExecutions after we know they've been fully handled
pub fn attack_execution_despawner(
    mut commands: Commands,
    attacks: Query<(Entity, &AttackResolved)>,
    mut action_completed_message: MessageWriter<UnitActionCompletedMessage>,
) {
    for (e, attack) in attacks {
        if let Some(attacker) = attack.attacker {
            action_completed_message.write(UnitActionCompletedMessage {
                unit: attacker,
                action: UnitAction::Attack,
            });

            commands.entity(attacker).remove::<UnitIsAttacking>();
        }

        info!(
            "Unit Action Completed, removing attack anim and despawning tracker for: {:?}",
            attack.attacker
        );

        commands.entity(e).despawn();
    }
}

pub mod skills {
    use anyhow::Context;
    use bevy::reflect::Reflect;
    use std::collections::{HashMap, HashSet};

    use crate::{
        animation::{
            AnimationMarker, UnitAnimationKind,
            animation_db::{
                AnimatedSpriteId, AnimationKey, RegisteredAnimationId,
                registered_sprite_ids::POISON_VFX_ANIMATED_SPRITE_ID,
            },
        },
        assets::{
            sounds::{AudioCue, AudioProfile, CombatSound, SkillSound},
            sprite_db::SpriteId,
        },
        combat::CombatStageId,
        gameplay_effects::{EffectData, StatusTag},
        unit::StatType,
    };

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
    pub struct SkillCategoryId(pub u32);

    #[derive(
        Clone, Copy, PartialEq, Eq, Debug, Hash, Reflect, serde::Serialize, serde::Deserialize,
    )]
    pub struct SkillId(pub u32);

    #[derive(Clone, Debug)]
    pub struct AttackModifier {
        pub(crate) stat: StatType,
    }

    #[derive(Debug, Clone)]
    pub struct DamagingSkill {
        /// The base power of this portion of the skill
        pub power: u32,
        /// The modifer that the offender uses for this skill
        pub offensive_modifier: Option<AttackModifier>,
        /// The modifier that the defender uses against this skill
        pub defensive_modifier: Option<AttackModifier>,
    }

    /// TODO: How do I represent changes in potency of damage / hit
    /// for different spatial applications?
    ///
    /// How would I represent something like divine ruination?
    ///
    /// Should I use this to represent whether you can hit a friend or foe?
    ///
    /// Splash {x, y} + TargetInRange (Where x and y are relative to the unit?)
    /// Line {dist, range_modifier?}
    /// Surround {radius?}
    ///
    #[derive(Debug, Clone)]
    pub enum Targeting {
        /// The skill simply is directed at one target, within a range of
        /// tiles from the caster.
        ///
        /// Should the inner value be paired with a modifier and f32?
        TargetInRange(u32),
    }

    pub enum TargetType {
        Any,
    }

    /// Buffs and Debuffs
    ///
    /// Idk
    // #[derive(Debug, Clone)]
    // pub struct Effect {
    //     pub name: String,
    //     pub duration: EffectDuration,
    // }

    // #[derive(Debug, Clone)]
    // pub enum EffectDuration {
    //     TurnCount(u32),
    //     ForTheRun,
    //     /// Lol idk when this would be applied but seems funny
    //     Permanent,
    // }

    /// The cost of the skill in UnitResources?
    ///
    /// Maybe this should be a general Effect that's given on the skill?
    /// IE: What if someone casts...
    /// - AP Drain?
    /// - Mana Drain?
    /// - Movement Drain?
    #[derive(Debug, Clone)]
    pub struct SkillCost {
        /// Amount of AP it costs to use the skill
        pub ap: u8,
    }

    #[derive(Debug, Clone)]
    pub enum SkillActionType {
        DamagingSkill { scaled_damage: DamagingSkill },
        HealingSkill { scaled_damage: DamagingSkill },
        ApplyEffects { effects: Vec<EffectData> },
    }

    #[derive(Debug, Clone)]
    pub struct SkillAction {
        /// Accuracy of the skill from 0 - 1
        ///
        /// TODO: Should accuracy be paired with an AttackModifier?
        pub base_accuracy: f32,
        pub action_type: SkillActionType,
    }

    #[derive(Debug, Clone)]
    pub enum CastingData {
        Projectile(SpriteId, SkillAnimationId),
        TileSprite(SpriteId, AnimationKey, SkillAnimationId),
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
    pub struct SkillActionIndex(pub usize);

    #[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
    pub struct SkillAnimationId(pub u32);

    #[derive(Debug, Clone)]
    pub enum SkillStageAction {
        /// The unit animation to play for the unit doing the skill
        // TODO: This should probably be removed.
        UnitAttack(SkillAnimationId, UnitAnimationKind),
        /// The cast to do
        Cast(CastingData),
        /// Skill actions that should be applied at this stage
        Impact(Vec<SkillActionIndex>),
    }

    #[derive(Debug, Clone)]
    pub enum SkillEvent {
        AnimationMarker(SkillAnimationId, AnimationMarker),
        ProjectileImpact(SkillAnimationId),
    }

    #[derive(Debug, Clone)]
    pub struct SkillStage {
        pub stage: SkillStageAction,
        pub advancing_event: SkillEvent,
    }

    #[derive(PartialEq, Eq, Clone, Copy, Default, Hash, PartialOrd, Ord, Debug)]
    pub struct SkillStageIndex(pub u32);

    impl From<SkillStageIndex> for CombatStageId {
        fn from(value: SkillStageIndex) -> Self {
            CombatStageId(value.0)
        }
    }

    #[derive(Default, Clone, Debug)]
    pub struct SkillAudioCues {
        /// Emit this on exit of the skill stage.
        ///
        /// Use SkillStageIndex 0 for on start of 1.
        pub to_emit_on_exit: HashMap<SkillStageIndex, Vec<AudioCue>>,
    }

    /// A representation of a Skill used in combat by a player.
    #[derive(Debug, Clone)]
    pub struct Skill {
        /// The Id of the skill
        pub skill_id: SkillId,

        /// The name of the skill
        pub name: String,
        /// Damaging portions of the skill, if any
        /// I think I want to use this for healing too, so maybe come up with a better name
        pub actions: Vec<SkillAction>,

        /// Uh, haven't really thought about this yet, mostly a placeholder for now
        pub targeting: Targeting,

        /// Animation Data
        pub animation_data: Vec<SkillStage>,

        /// The base cost of the skill
        ///
        /// Should this just be an effect on Self?
        /// If so I might need to change Targeting?
        pub cost: SkillCost,

        /// Cues that should be emitted when this skill is processed.
        pub audio_cues_to_emit: SkillAudioCues,

        /// Not so sure that this should be generic across all types,
        /// but let's leave it like this for now.
        pub audio_profile: AudioProfile,
    }

    #[derive(bevy::prelude::Resource)]
    pub struct SkillDBResource {
        pub skill_db: SkillDB,
    }

    pub struct SkillDB {
        skills: HashMap<SkillId, Skill>,
        skills_by_category: HashMap<SkillCategoryId, Vec<SkillId>>,
        category_by_skills: HashMap<SkillId, SkillCategoryId>,
        skill_categories: HashMap<SkillCategoryId, SkillCategory>,
    }

    impl SkillDB {
        fn new() -> Self {
            // TBD if all of these are necessary but...
            Self {
                skills: HashMap::new(),
                skills_by_category: HashMap::new(),
                category_by_skills: HashMap::new(),
                skill_categories: HashMap::new(),
            }
        }

        /// Since skills should always be registered, we take the liberty that if you somehow
        /// got a Skill ID, there isn't a way it can't be registered. So the game crashes!
        ///
        /// TBD if that's a good decision or not, but it cleans up a fair bit of callsites from the
        /// same silly (Oh no this doesn't have a skill, let me do something basically game breaking)
        /// logic. #failfast
        ///
        /// I'm questioning this decision tho lol (esp. since Units would need to store their learned skills?)
        pub fn get_skill(&self, skill_id: &SkillId) -> &Skill {
            self.skills
                .get(skill_id)
                .unwrap_or_else(|| panic!("SkillID should be registered: {:?}", skill_id))
        }

        pub fn get_category(&self, category_id: &SkillCategoryId) -> &SkillCategory {
            self.skill_categories
                .get(category_id)
                .unwrap_or_else(|| panic!("SkillCategory should be registered: {:?}", category_id))
        }

        pub fn get_category_for_skill(&self, skill_id: &SkillId) -> &SkillCategoryId {
            self.category_by_skills.get(skill_id).unwrap_or_else(|| {
                panic!("Skill should be registered with a category: {:?}", skill_id)
            })
        }

        fn register_category(
            &mut self,
            skill_category: SkillCategoryId,
            category: SkillCategory,
        ) -> anyhow::Result<&mut Self> {
            if self
                .skill_categories
                .insert(skill_category, category)
                .is_some()
            {
                return Err(anyhow::anyhow!(
                    "Skill Category {:?} already existed",
                    skill_category
                ));
            }

            if self
                .skills_by_category
                .insert(skill_category, Vec::new())
                .is_some()
            {
                return Err(anyhow::anyhow!(
                    "Skill Category {:?} already existed",
                    skill_category
                ));
            }

            Ok(self)
        }

        fn register_skill(
            &mut self,
            skill_category: SkillCategoryId,
            skill_id: SkillId,
            skill: Skill,
        ) -> anyhow::Result<&mut Self> {
            if self.skills.insert(skill_id, skill).is_some() {
                return Err(anyhow::anyhow!("Skill {:?} already exists", skill_id));
            }
            let category_container = self
                .skills_by_category
                .get_mut(&skill_category)
                .with_context(|| {
                    format!(
                        "Skill Category for skill {:?} should be registered first",
                        skill_id
                    )
                })?;
            category_container.push(skill_id);
            if self
                .category_by_skills
                .insert(skill_id, skill_category)
                .is_some()
            {
                return Err(anyhow::anyhow!(format!(
                    "Reverse skill map already exists for {:?}, {:?}?",
                    skill_id, skill_category
                )));
            }

            Ok(self)
        }
    }

    pub struct SkillCategory {
        pub name: String,
    }

    /// While Units mostly have skills, we separate this into it's own component
    /// because we can, and vibes.
    ///
    /// In some ways, it's nice that a system focused on a Unit doesn't need to know the Unit's stats,
    /// but can know it's skills.
    ///
    /// Tbh I'm still figuring out the composition balance here.
    #[derive(Clone, Debug, bevy::prelude::Component)]
    pub struct UnitSkills {
        /// Could split this out by category? IDK.
        ///
        /// TODO: Worth separating out the learned from
        /// equipped into different structs for when we serialize a
        /// unit back to it's "Camp" state?
        pub learned_skills: HashSet<SkillId>,
        pub equipped_skill_categories: Vec<SkillCategoryId>,
        // pub primary_category: SkillCategoryId,
        // pub secondary_category: SkillCategoryId,
    }

    pub fn setup_skill_system(mut commands: bevy::prelude::Commands) {
        let skill_db = build_skill_table().expect("Should be able to build the skill DB");
        commands.insert_resource(SkillDBResource { skill_db });
    }

    /// Can't decide if everyone should get this, or not
    ///
    /// But for now it's a special snowflake
    pub const ATTACK_SKILL_ID: SkillId = SkillId(1);

    // What do I gain from skills not being a part of the code itself?
    // - Makes modding easy I guess
    // How do I ensure skills are always valid if they are being passed around the codebase?

    /// Probably should be loaded from json or ron or something
    pub fn build_skill_table() -> anyhow::Result<SkillDB> {
        let mut skill_db = SkillDB::new();
        skill_db
            .register_category(
                SkillCategoryId(0),
                SkillCategory {
                    name: "Base".to_string(),
                },
            )?
            .register_skill(
                SkillCategoryId(0),
                ATTACK_SKILL_ID,
                Skill {
                    skill_id: ATTACK_SKILL_ID,
                    name: "Attack".to_owned(),
                    actions: Vec::from([SkillAction {
                        base_accuracy: 1.0,
                        action_type: SkillActionType::DamagingSkill {
                            scaled_damage: DamagingSkill {
                                power: 3,
                                offensive_modifier: Some(AttackModifier {
                                    stat: StatType::Strength,
                                }),
                                defensive_modifier: Some(AttackModifier {
                                    stat: StatType::Defense,
                                }),
                            },
                        },
                    }]),
                    targeting: Targeting::TargetInRange(1),
                    animation_data: vec![
                        SkillStage {
                            stage: SkillStageAction::UnitAttack(
                                SkillAnimationId(1),
                                UnitAnimationKind::Attack,
                            ),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(1),
                                AnimationMarker::HitFrame,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![SkillActionIndex(0)]),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(1),
                                AnimationMarker::Complete,
                            ),
                        },
                    ],
                    cost: SkillCost { ap: 1 },
                    audio_profile: AudioProfile::default(),
                    audio_cues_to_emit: SkillAudioCues::default(),
                },
            )?
            .register_category(
                SkillCategoryId(1),
                SkillCategory {
                    name: "Primal Arts".to_owned(),
                },
            )?
            .register_category(
                SkillCategoryId(2),
                SkillCategory {
                    name: "Items".to_string(),
                },
            )?
            .register_category(
                SkillCategoryId(3),
                SkillCategory {
                    name: "Dev Category".to_string(),
                },
            )?
            .register_category(
                SkillCategoryId(4),
                SkillCategory {
                    name: "Gallantry".to_string(),
                },
            )?
            .register_category(
                SkillCategoryId(5),
                SkillCategory {
                    name: "Marksmanship".to_string(),
                },
            )?
            .register_category(
                SkillCategoryId(6),
                SkillCategory {
                    name: "Sword Arts".to_string(),
                },
            )?
            .register_skill(
                SkillCategoryId(1),
                SkillId(2),
                Skill {
                    skill_id: SkillId(2),
                    name: "Flame".to_owned(),
                    actions: Vec::from([SkillAction {
                        base_accuracy: 0.8,
                        action_type: SkillActionType::DamagingSkill {
                            scaled_damage: DamagingSkill {
                                power: 2,
                                offensive_modifier: Some(AttackModifier {
                                    stat: StatType::Magic,
                                }),
                                defensive_modifier: Some(AttackModifier {
                                    stat: StatType::Resistance,
                                }),
                            },
                        },
                    }]),
                    targeting: Targeting::TargetInRange(3),
                    animation_data: vec![
                        SkillStage {
                            stage: SkillStageAction::UnitAttack(
                                SkillAnimationId(1),
                                UnitAnimationKind::Charge,
                            ),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(1),
                                AnimationMarker::Complete,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::UnitAttack(
                                SkillAnimationId(2),
                                UnitAnimationKind::Release,
                            ),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(2),
                                AnimationMarker::Complete,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::Cast(CastingData::TileSprite(
                                SpriteId(5),
                                AnimationKey {
                                    animated_sprite_id: AnimatedSpriteId(4),
                                    animation_id: RegisteredAnimationId {
                                        id: 1,
                                        priority: crate::animation::AnimationPriority::Combat,
                                    },
                                },
                                SkillAnimationId(3),
                            )),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(3),
                                AnimationMarker::HitFrame,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![
                                SkillActionIndex(0),
                                SkillActionIndex(1),
                            ]),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(3),
                                AnimationMarker::Complete,
                            ),
                        },
                    ],
                    cost: SkillCost { ap: 1 },
                    // TODO: We could consider just having the SkillStageAction
                    // drive what is happening for Impact?
                    audio_cues_to_emit: SkillAudioCues {
                        to_emit_on_exit: HashMap::from([(
                            SkillStageIndex(3),
                            vec![AudioCue::Impact],
                        )]),
                    },
                    audio_profile: AudioProfile {
                        on_cue: HashMap::from([(
                            AudioCue::Impact,
                            vec![CombatSound::Skill(SkillSound::FlameExplosion)],
                        )]),
                    },
                },
            )?
            .register_skill(
                SkillCategoryId(6),
                SkillId(3),
                Skill {
                    skill_id: SkillId(3),
                    name: "Hit em Twice".to_owned(),
                    actions: Vec::from([
                        SkillAction {
                            base_accuracy: 1.0,
                            action_type: SkillActionType::DamagingSkill {
                                scaled_damage: DamagingSkill {
                                    power: 3,
                                    offensive_modifier: Some(AttackModifier {
                                        stat: StatType::Strength,
                                    }),
                                    defensive_modifier: Some(AttackModifier {
                                        stat: StatType::Defense,
                                    }),
                                },
                            },
                        },
                        SkillAction {
                            base_accuracy: 1.0,
                            action_type: SkillActionType::DamagingSkill {
                                scaled_damage: DamagingSkill {
                                    power: 5,
                                    offensive_modifier: Some(AttackModifier {
                                        stat: StatType::Strength,
                                    }),
                                    defensive_modifier: Some(AttackModifier {
                                        stat: StatType::Defense,
                                    }),
                                },
                            },
                        },
                    ]),
                    targeting: Targeting::TargetInRange(1),
                    animation_data: vec![
                        SkillStage {
                            stage: SkillStageAction::UnitAttack(
                                SkillAnimationId(1),
                                UnitAnimationKind::Attack,
                            ),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(1),
                                AnimationMarker::HitFrame,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![SkillActionIndex(0)]),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(1),
                                AnimationMarker::Complete,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::UnitAttack(
                                SkillAnimationId(2),
                                UnitAnimationKind::Attack,
                            ),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(2),
                                AnimationMarker::HitFrame,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![SkillActionIndex(1)]),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(2),
                                AnimationMarker::Complete,
                            ),
                        },
                    ],
                    cost: SkillCost { ap: 1 },
                    audio_cues_to_emit: SkillAudioCues::default(),
                    audio_profile: AudioProfile::default(),
                },
            )?
            .register_skill(
                SkillCategoryId(3),
                SkillId(4),
                Skill {
                    skill_id: SkillId(4),
                    name: "Lob Attack".to_owned(),
                    actions: Vec::from([SkillAction {
                        base_accuracy: 1.0,
                        action_type: SkillActionType::DamagingSkill {
                            scaled_damage: DamagingSkill {
                                power: 3,
                                offensive_modifier: Some(AttackModifier {
                                    stat: StatType::Strength,
                                }),
                                defensive_modifier: Some(AttackModifier {
                                    stat: StatType::Defense,
                                }),
                            },
                        },
                    }]),
                    targeting: Targeting::TargetInRange(5),
                    animation_data: vec![
                        SkillStage {
                            stage: SkillStageAction::Cast(CastingData::Projectile(
                                SpriteId(6),
                                SkillAnimationId(1),
                            )),
                            advancing_event: SkillEvent::ProjectileImpact(SkillAnimationId(1)),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![SkillActionIndex(0)]),
                            advancing_event: SkillEvent::ProjectileImpact(SkillAnimationId(1)),
                        },
                    ],
                    cost: SkillCost { ap: 1 },
                    audio_cues_to_emit: SkillAudioCues::default(),
                    audio_profile: AudioProfile::default(),
                },
            )?
            .register_skill(
                SkillCategoryId(5),
                SkillId(5),
                Skill {
                    skill_id: SkillId(5),
                    name: "Poison Shot".to_owned(),
                    actions: Vec::from([
                        SkillAction {
                            base_accuracy: 1.0,
                            action_type: SkillActionType::DamagingSkill {
                                scaled_damage: DamagingSkill {
                                    power: 1,
                                    offensive_modifier: Some(AttackModifier {
                                        stat: StatType::Strength,
                                    }),
                                    defensive_modifier: Some(AttackModifier {
                                        stat: StatType::Defense,
                                    }),
                                },
                            },
                        },
                        SkillAction {
                            base_accuracy: 1.0,
                            action_type: SkillActionType::ApplyEffects {
                                effects: Vec::from([EffectData {
                                    effect_type:
                                        crate::gameplay_effects::EffectType::StatusInfliction(
                                            StatusTag::Poisoned,
                                        ),
                                    duration: crate::gameplay_effects::EffectDuration::TurnCount(3),
                                }]),
                            },
                        },
                    ]),
                    targeting: Targeting::TargetInRange(5),
                    animation_data: vec![
                        SkillStage {
                            stage: SkillStageAction::Cast(CastingData::Projectile(
                                SpriteId(6),
                                SkillAnimationId(1),
                            )),
                            advancing_event: SkillEvent::ProjectileImpact(SkillAnimationId(1)),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![
                                SkillActionIndex(0),
                                SkillActionIndex(1),
                            ]),
                            advancing_event: SkillEvent::ProjectileImpact(SkillAnimationId(1)),
                        },
                    ],
                    cost: SkillCost { ap: 1 },
                    audio_cues_to_emit: SkillAudioCues::default(),
                    audio_profile: AudioProfile::default(),
                },
            )?
            .register_skill(
                SkillCategoryId(5),
                SkillId(6),
                Skill {
                    skill_id: SkillId(6),
                    name: "Stun Shot".to_owned(),
                    actions: Vec::from([
                        SkillAction {
                            base_accuracy: 1.0,
                            action_type: SkillActionType::DamagingSkill {
                                scaled_damage: DamagingSkill {
                                    power: 1,
                                    offensive_modifier: Some(AttackModifier {
                                        stat: StatType::Strength,
                                    }),
                                    defensive_modifier: Some(AttackModifier {
                                        stat: StatType::Defense,
                                    }),
                                },
                            },
                        },
                        SkillAction {
                            base_accuracy: 1.0,
                            action_type: SkillActionType::ApplyEffects {
                                effects: Vec::from([EffectData {
                                    effect_type:
                                        crate::gameplay_effects::EffectType::StatusInfliction(
                                            StatusTag::Stunned,
                                        ),
                                    duration: crate::gameplay_effects::EffectDuration::TurnCount(3),
                                }]),
                            },
                        },
                    ]),
                    targeting: Targeting::TargetInRange(5),
                    animation_data: vec![
                        SkillStage {
                            stage: SkillStageAction::Cast(CastingData::Projectile(
                                SpriteId(6),
                                SkillAnimationId(1),
                            )),
                            advancing_event: SkillEvent::ProjectileImpact(SkillAnimationId(1)),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![SkillActionIndex(0)]),
                            advancing_event: SkillEvent::ProjectileImpact(SkillAnimationId(1)),
                        },
                    ],
                    cost: SkillCost { ap: 1 },
                    audio_cues_to_emit: SkillAudioCues::default(),
                    audio_profile: AudioProfile::default(),
                },
            )?
            // Uh, not a skill necessarily, but this pretty much gets what I want for free so...
            .register_skill(
                SkillCategoryId(3),
                SkillId(7),
                Skill {
                    skill_id: SkillId(7),
                    name: "Take Poison Damage".to_owned(),
                    actions: Vec::from([SkillAction {
                        base_accuracy: 1.0,
                        action_type: SkillActionType::DamagingSkill {
                            scaled_damage: DamagingSkill {
                                power: 2,
                                offensive_modifier: None,
                                defensive_modifier: None,
                            },
                        },
                    }]),
                    targeting: Targeting::TargetInRange(0),
                    animation_data: vec![
                        SkillStage {
                            stage: SkillStageAction::Cast(CastingData::TileSprite(
                                SpriteId(7),
                                AnimationKey {
                                    animated_sprite_id: POISON_VFX_ANIMATED_SPRITE_ID,
                                    animation_id: RegisteredAnimationId {
                                        id: 1,
                                        priority: crate::animation::AnimationPriority::Combat,
                                    },
                                },
                                SkillAnimationId(1),
                            )),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(1),
                                AnimationMarker::HitFrame,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![SkillActionIndex(0)]),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(1),
                                AnimationMarker::Complete,
                            ),
                        },
                    ],
                    cost: SkillCost { ap: 1 },
                    audio_cues_to_emit: SkillAudioCues::default(),
                    audio_profile: AudioProfile::default(),
                },
            )?
            .register_skill(
                SkillCategoryId(1),
                SkillId(8),
                Skill {
                    skill_id: SkillId(8),
                    name: "Heal".to_owned(),
                    actions: Vec::from([SkillAction {
                        base_accuracy: 1.0,
                        action_type: SkillActionType::HealingSkill {
                            scaled_damage: DamagingSkill {
                                power: 5,
                                offensive_modifier: None,
                                defensive_modifier: None,
                            },
                        },
                    }]),
                    targeting: Targeting::TargetInRange(0),
                    animation_data: vec![
                        // TODO: Using flare damage for now lol
                        SkillStage {
                            stage: SkillStageAction::UnitAttack(
                                SkillAnimationId(1),
                                UnitAnimationKind::Charge,
                            ),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(1),
                                AnimationMarker::Complete,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::UnitAttack(
                                SkillAnimationId(2),
                                UnitAnimationKind::Release,
                            ),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(2),
                                AnimationMarker::Complete,
                            ),
                        },
                        SkillStage {
                            stage: SkillStageAction::Impact(vec![SkillActionIndex(0)]),
                            advancing_event: SkillEvent::AnimationMarker(
                                SkillAnimationId(2),
                                AnimationMarker::Complete,
                            ),
                        },
                    ],
                    cost: SkillCost { ap: 1 },
                    audio_cues_to_emit: SkillAudioCues::default(),
                    audio_profile: AudioProfile::default(),
                },
            )?;

        // TODO: Validate SkillDB once we load it from an external source.

        Ok(skill_db)
    }

    #[cfg(test)]
    mod test {
        use crate::combat::skills::build_skill_table;

        #[test]
        fn test_build_skill_system() {
            build_skill_table().expect("Should be able to build skill table");
        }
    }
}
