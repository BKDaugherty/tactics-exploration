//! Managing your tactics game with a Player Phase / Enemy Phase? Use this!
//!
//! Remember a lil yagni never hurt anyone though. For now tries not to be too generic
//! and just assumes there's only a Player / Enemy Phase.

use bevy::prelude::*;

use crate::{
    battle::Enemy,
    battle_phase::phase_ui::{BattlePhaseMessageComplete, ShowBattleBannerMessage},
    combat::{
        AttackExecution, CombatTimeline,
        skills::{SkillDBResource, SkillId},
    },
    gameplay_effects::{ActiveEffects, EffectDuration, StatusTag},
    grid::GridPosition,
    player::Player,
    unit::{CombatActionMarker, StatType, Unit, UnitDerivedStats},
};

/// The Phase Manager keeps track of the current phase globally for the battle.
#[derive(Resource)]
pub struct PhaseManager {
    pub current_phase: PlayerEnemyPhase,
    pub phase_state: PhaseState,
    pub turn_count: u32,
}

/// Basically a boolean gate to avoid fast ending a turn
/// and coordinating between our systems
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseState {
    Initializing,
    HandlingStartOfPhaseEffects,
    Running,
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, Debug)]
pub enum PlayerEnemyPhase {
    Player,
    Enemy,
}

impl PlayerEnemyPhase {
    pub fn next(&self) -> Self {
        match self {
            PlayerEnemyPhase::Player => PlayerEnemyPhase::Enemy,
            PlayerEnemyPhase::Enemy => PlayerEnemyPhase::Player,
        }
    }
}

pub fn is_running_player_phase(pm: Option<Res<PhaseManager>>) -> bool {
    pm.map(|pm| {
        pm.current_phase == PlayerEnemyPhase::Player && pm.phase_state == PhaseState::Running
    })
    .unwrap_or_default()
}

pub fn is_running_enemy_phase(pm: Option<Res<PhaseManager>>) -> bool {
    pm.map(|pm| {
        pm.current_phase == PlayerEnemyPhase::Enemy && pm.phase_state == PhaseState::Running
    })
    .unwrap_or_default()
}

pub fn is_enemy_phase(pm: Option<Res<PhaseManager>>) -> bool {
    pm.map(|pm| pm.current_phase == PlayerEnemyPhase::Enemy)
        .unwrap_or_default()
}

#[derive(Clone, Debug)]
pub enum PhaseMessageType {
    PhaseBegin(PlayerEnemyPhase),
}

#[derive(Message, Debug)]
pub struct PhaseMessage(pub PhaseMessageType);

#[derive(Component, Debug, Reflect, Default)]
pub struct UnitPhaseResources {
    pub movement_points_left_in_phase: u32,
    pub action_points_left_in_phase: u32,
    pub waited: bool,
}

impl UnitPhaseResources {
    pub fn can_act(&self) -> bool {
        if self.waited {
            return false;
        }

        self.movement_points_left_in_phase > 0 || self.action_points_left_in_phase > 0
    }
}

pub trait PhaseSystem<T> {
    type Marker: Component;
    const OWNED_PHASE: T;
}

impl PhaseSystem<PlayerEnemyPhase> for Player {
    type Marker = Self;
    const OWNED_PHASE: PlayerEnemyPhase = PlayerEnemyPhase::Player;
}

impl PhaseSystem<PlayerEnemyPhase> for Enemy {
    type Marker = Self;
    const OWNED_PHASE: PlayerEnemyPhase = PlayerEnemyPhase::Enemy;
}

pub fn init_phase_system(
    mut commands: Commands,
    mut phase_message_writer: MessageWriter<PhaseMessage>,
) {
    commands.insert_resource(PhaseManager {
        turn_count: 0,
        phase_state: PhaseState::Initializing,
        current_phase: PlayerEnemyPhase::Player,
    });

    // Will this get picked up by the
    phase_message_writer.write(PhaseMessage(PhaseMessageType::PhaseBegin(
        PlayerEnemyPhase::Player,
    )));
}

pub fn check_should_advance_phase<T: PhaseSystem<PlayerEnemyPhase>>(
    mut phase_manager: ResMut<PhaseManager>,
    mut message_writer: MessageWriter<PhaseMessage>,
    query: Query<(&UnitPhaseResources, &UnitDerivedStats), With<T::Marker>>,
    wait_for_no_attacks_ongoing: Query<Entity, With<CombatActionMarker>>,
) {
    if phase_manager.current_phase != T::OWNED_PHASE
        || phase_manager.phase_state != PhaseState::Running
        || !wait_for_no_attacks_ongoing.is_empty()
    {
        return;
    }

    if query
        .iter()
        .all(|(resources, derived_stats)| !resources.can_act() || derived_stats.downed())
    {
        let next_phase = T::OWNED_PHASE.next();
        phase_manager.current_phase = next_phase;
        info!("Advancing To Next Phase: {:?}", next_phase);
        phase_manager.phase_state = PhaseState::Initializing;
        message_writer.write(PhaseMessage(PhaseMessageType::PhaseBegin(next_phase)));
    }
}

/// Prepares for Phase
pub fn prepare_for_phase<T: PhaseSystem<PlayerEnemyPhase>>(
    phase_manager: ResMut<PhaseManager>,
    mut message_reader: MessageReader<PhaseMessage>,
    mut query: Query<(&UnitDerivedStats, &mut UnitPhaseResources), With<T::Marker>>,
    mut battle_phase_change_writer: MessageWriter<ShowBattleBannerMessage>,
) {
    for message in message_reader.read() {
        let PhaseMessageType::PhaseBegin(phase) = message.0;

        if phase == T::OWNED_PHASE && phase_manager.phase_state == PhaseState::Initializing {
            for (unit, mut phase_resources) in query.iter_mut() {
                phase_resources.action_points_left_in_phase = 1;
                phase_resources.movement_points_left_in_phase =
                    unit.stats.stat(StatType::Movement).0 as u32;
                phase_resources.waited = false;
            }

            battle_phase_change_writer.write(ShowBattleBannerMessage {
                message: phase_ui::BattleBannerMessage::PhaseBegin(T::OWNED_PHASE),
            });
        }
    }
}

// TODO: It feels like I should apply poison damage here right?
pub fn decrement_turn_count_effects_on_turn_start<T: PhaseSystem<PlayerEnemyPhase>>(
    mut message_reader: MessageReader<TurnStartMessage>,
    mut query: Query<&mut ActiveEffects, With<T::Marker>>,
) {
    for message in message_reader.read() {
        if message.phase == T::OWNED_PHASE {
            for mut active_effects in query.iter_mut() {
                for effect in active_effects.effects.iter_mut() {
                    let EffectDuration::TurnCount(turn_count) = &mut effect.data.duration else {
                        continue;
                    };

                    *turn_count = turn_count.saturating_sub(1);
                }

                // TODO: Do we need an event here?
                active_effects.effects.retain(|t| {
                    if let EffectDuration::TurnCount(turn_count) = t.data.duration {
                        return turn_count != 0;
                    } else {
                        true
                    }
                });
            }
        }
    }
}

pub fn check_for_active_effect_damage_on_turn_start<T: PhaseSystem<PlayerEnemyPhase>>(
    mut commands: Commands,
    mut message_reader: MessageReader<StartOfPhaseEffectsMessage>,
    skill_db: Res<SkillDBResource>,
    query: Query<(Entity, &ActiveEffects, &GridPosition), With<T::Marker>>,
) {
    for message in message_reader.read() {
        if message.phase != T::OWNED_PHASE {
            continue;
        }

        // Handle Poison Damage
        let poison_skill = skill_db.skill_db.get_skill(&SkillId(7));
        for (e, active_effect, grid_position) in query {
            if active_effect.statuses().contains(&StatusTag::Poisoned) {
                let mut poison_damage_e = commands.spawn(PoisonDamageEntity);
                let Ok(poison_timeline) = CombatTimeline::build_without_attacker(
                    poison_damage_e.id(),
                    poison_skill.clone(),
                    e,
                    grid_position,
                ) else {
                    error!("Failed to build Poison Damage Timeline!");
                    continue;
                };
                poison_damage_e.insert((
                    AttackExecution {
                        attacker: None,
                        defender: e,
                        combat_timeline: poison_timeline,
                        skill: poison_skill.clone(),
                    },
                    CombatActionMarker,
                    StartOfPhaseEffect,
                ));
            }
        }
    }
}

#[derive(Component)]
struct PoisonDamageEntity;

#[derive(Message)]
pub struct TurnStartMessage {
    phase: PlayerEnemyPhase,
}

#[derive(Message)]
pub struct StartOfPhaseEffectsMessage {
    phase: PlayerEnemyPhase,
}

/// A start of phase effect that needs to be handled
/// before moving to the "running" portion of the phase
#[derive(Component)]
pub struct StartOfPhaseEffect;

pub fn advance_after_start_of_phase_effects(
    mut phase_manager: ResMut<PhaseManager>,
    query: Query<Entity, With<StartOfPhaseEffect>>,
    mut message_writer: MessageWriter<TurnStartMessage>,
) {
    if phase_manager.phase_state == PhaseState::HandlingStartOfPhaseEffects {
        if query.is_empty() {
            phase_manager.phase_state = PhaseState::Running;
            message_writer.write(TurnStartMessage {
                phase: phase_manager.current_phase,
            });
        }
    }
}

/// Advances the phase after the BattleBanner has been displayed
pub fn start_phase(
    mut phase_manager: ResMut<PhaseManager>,
    mut message_reader: MessageReader<BattlePhaseMessageComplete>,
    mut message_writer: MessageWriter<StartOfPhaseEffectsMessage>,
) {
    for _message in message_reader.read() {
        if phase_manager.phase_state == PhaseState::Initializing {
            phase_manager.phase_state = PhaseState::HandlingStartOfPhaseEffects;
        }

        message_writer.write(StartOfPhaseEffectsMessage {
            phase: phase_manager.current_phase,
        });
    }
}

pub mod phase_ui {
    use bevy::prelude::*;

    use crate::{
        GameState, assets::FontResource, battle::BattleEntity, battle_phase::PlayerEnemyPhase,
    };

    #[derive(Debug)]
    pub enum BattleBannerMessage {
        PhaseBegin(PlayerEnemyPhase),
    }

    #[derive(Message, Debug)]
    pub struct ShowBattleBannerMessage {
        pub message: BattleBannerMessage,
    }

    #[derive(Message)]
    pub struct BattlePhaseMessageComplete {}

    #[derive(Component)]
    pub struct BattleBanner;

    #[derive(Component)]
    pub struct BannerAnimation {
        timer: Timer,
        state: BannerAnimState,
    }

    enum BannerAnimState {
        Entering,
        Holding,
        Exiting,
    }

    fn spawn_phase_ui(
        commands: &mut Commands,
        fonts: &Res<FontResource>,
        event: &ShowBattleBannerMessage,
    ) {
        let container = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BattleBanner,
                BannerAnimation {
                    timer: Timer::from_seconds(0.4, TimerMode::Once),
                    state: BannerAnimState::Entering,
                },
                BattleEntity {},
                DespawnOnExit(GameState::Battle),
            ))
            .id();

        let blue = Color::linear_rgba(0.0, 0.0, 1.0, 1.0);
        let red = Color::linear_rgba(1.0, 0.0, 0.0, 1.0);

        let (color, text) = match &event.message {
            BattleBannerMessage::PhaseBegin(phase) => match phase {
                PlayerEnemyPhase::Player => (blue, "PLAYER PHASE"),
                PlayerEnemyPhase::Enemy => (red, "ENEMY PHASE"),
            },
        };

        let banner = commands
            .spawn((
                Node {
                    width: percent(80),
                    height: percent(20),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                BorderRadius::all(percent(20)),
                BackgroundColor(Color::linear_rgba(0.7, 0.7, 0.7, 0.8)),
                children![(
                    TextColor(color),
                    Text::new(text),
                    TextFont {
                        font_size: 60.,
                        font: fonts.badge.clone(),
                        ..Default::default()
                    },
                )],
            ))
            .id();

        commands.entity(container).add_child(banner);
    }

    pub fn spawn_banner_system(
        mut commands: Commands,
        font_res: Res<FontResource>,
        mut events: MessageReader<ShowBattleBannerMessage>,
    ) {
        for event in events.read() {
            spawn_phase_ui(&mut commands, &font_res, event);
        }
    }

    pub fn banner_animation_system(
        time: Res<Time>,
        mut commands: Commands,
        mut query: Query<(Entity, &mut BannerAnimation), With<BattleBanner>>,
        mut writer: MessageWriter<BattlePhaseMessageComplete>,
    ) {
        for (entity, mut anim) in &mut query {
            anim.timer.tick(time.delta());
            if anim.timer.just_finished() {
                match anim.state {
                    BannerAnimState::Entering => {
                        anim.state = BannerAnimState::Holding;
                        anim.timer = Timer::from_seconds(0.6, TimerMode::Once);
                    }
                    BannerAnimState::Holding => {
                        anim.state = BannerAnimState::Exiting;
                        anim.timer = Timer::from_seconds(0.4, TimerMode::Once);
                    }
                    BannerAnimState::Exiting => {
                        commands.entity(entity).despawn();
                        writer.write(BattlePhaseMessageComplete {});
                    }
                }
            }
        }
    }
}
