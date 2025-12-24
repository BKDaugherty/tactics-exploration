//! Managing your tactics game with a Player Phase / Enemy Phase? Use this!
//!
//! Remember a lil yagni never hurt anyone though. For now tries not to be too generic
//! and just assumes there's only a Player / Enemy Phase.

use bevy::prelude::*;

use crate::{battle::Enemy, player::Player, unit::Unit};

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

#[derive(Clone)]
pub enum PhaseMessageType {
    PhaseBegin(PlayerEnemyPhase),
}

#[derive(Message)]
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
    query: Query<(&UnitPhaseResources, &Unit), With<T::Marker>>,
) {
    if phase_manager.current_phase != T::OWNED_PHASE
        || phase_manager.phase_state != PhaseState::Running
    {
        return;
    }

    if query
        .iter()
        .all(|(resources, unit)| !resources.can_act() || unit.downed())
    {
        let next_phase = T::OWNED_PHASE.next();
        phase_manager.current_phase = next_phase;
        info!("Advancing To Next Phase: {:?}", next_phase);
        phase_manager.phase_state = PhaseState::Initializing;
        message_writer.write(PhaseMessage(PhaseMessageType::PhaseBegin(next_phase)));
    }
}

pub fn refresh_units_at_beginning_of_phase<T: PhaseSystem<PlayerEnemyPhase>>(
    mut phase_manager: ResMut<PhaseManager>,
    mut message_reader: MessageReader<PhaseMessage>,
    mut query: Query<(&Unit, &mut UnitPhaseResources), With<T::Marker>>,
) {
    for message in message_reader.read() {
        let PhaseMessageType::PhaseBegin(phase) = message.0;

        if phase == T::OWNED_PHASE && phase_manager.phase_state == PhaseState::Initializing {
            for (unit, mut phase_resources) in query.iter_mut() {
                phase_resources.action_points_left_in_phase = 1;
                phase_resources.movement_points_left_in_phase = unit.stats.movement;
                phase_resources.waited = false;
            }

            // TODO: Should this actually be where the "PhaseBegin" event is emitted for external systems?
            phase_manager.phase_state = PhaseState::Running;
        }
    }
}
