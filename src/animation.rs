//! A module for talking about and coordinating animation data.

use std::collections::HashMap;

use bevy::prelude::*;
pub use tinytactics::Direction;

use crate::{
    animation::{
        combat::ATTACK_FRAME_DURATION,
        tinytactics::{Character, WeaponType},
    },
    assets::BATTLE_TACTICS_TILESHEET,
    grid::{GridManagerResource, GridMovement, GridVec},
    unit::Unit,
};

#[derive(Component, Debug, Clone)]
pub struct FacingDirection(pub Direction);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationType {
    Idle,
    Attacking,
}

/// We need some way to decide whether or not we should
/// play the animation or not
#[derive(PartialEq, Eq, PartialOrd, Copy, Clone, Hash, Debug)]
pub enum AnimationPriority {
    Idle,
    Reaction,
    Combat,
}

#[derive(Debug, Clone, Copy)]
pub enum AnimationMarker {
    /// The frame at which the animation "hit" the target.
    ///
    /// Our combat system expects this to be emitted in order to advance the "AttackPhase"
    HitFrame,

    /// The frame at which the animation is "complete"
    ///
    /// Typically this is only used when the game "care's" about the animation being complete.
    /// This wouldn't typically be given for an "Idle" animation (at least for now)
    Complete,
}

#[derive(Debug, Message)]
pub struct AnimationMarkerMessage {
    pub entity: Entity,
    pub marker: AnimationMarker,
}

#[derive(Component)]
pub struct AnimationFollower {
    pub leader: Entity,
}

pub fn unit_animation_tick_system(
    time: Res<Time>,
    animation_data: Res<TinytacticsAssets>,
    mut query: Query<
        (
            Entity,
            &FacingDirection,
            &mut UnitAnimationPlayer,
            &mut Sprite,
        ),
        Without<AnimationFollower>,
    >,
    mut follower_query: Query<
        (&AnimationFollower, &mut Sprite, &mut Visibility),
        With<AnimationFollower>,
    >,
    mut marker_events: MessageWriter<AnimationMarkerMessage>,
) {
    for (entity, dir, mut player, mut sprite) in &mut query {
        // Get the current animation, if any
        let Some(anim) = &mut player.current_animation else {
            continue;
        };

        let key = UnitAnimationKey {
            kind: anim.id,
            direction: dir.0.animation_direction(),
        };

        let Some(clip_data) = animation_data.unit_animation_data.unit_animations.get(&key) else {
            warn!("No animation data found for running clip");
            continue;
        };

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.frame += 1;

            // Send event before bounds checking to allow for using the len(frames) as a "Complete" marker
            if let Some(marker) = clip_data.inner.animation_offset_markers.get(&anim.frame) {
                marker_events.write(AnimationMarkerMessage {
                    entity,
                    marker: *marker,
                });
            }

            if anim.frame >= clip_data.inner.frame_count {
                player.current_animation = None;
                continue;
            }
        }

        if let Some(texture_atlas) = sprite.texture_atlas.as_mut() {
            let target_frame = anim.frame + clip_data.start_index;
            texture_atlas.index = target_frame;
            sprite.flip_x = dir.0.should_flip_across_y();
        }
    }

    // TODO: This system feels really hyperspecific for overlays based on Attack
    // I'd love to make these a littel better
    for (follower, mut sprite, mut vis) in follower_query.iter_mut() {
        if let Some((_, facing_direction, player, _)) = query.get(follower.leader).ok() {
            let Some(anim) = &player.current_animation else {
                *vis = Visibility::Hidden;
                continue;
            };

            let Some(weapon_animation) =
                animation_data
                    .unit_animation_data
                    .weapon_animations
                    .get(&UnitAnimationKey {
                        kind: anim.id,
                        direction: facing_direction.0.animation_direction(),
                    })
            else {
                *vis = Visibility::Hidden;
                continue;
            };

            let Some(texture_atlas) = sprite.texture_atlas.as_mut() else {
                warn!("No texture atlas for Weapon Sprite Follower");
                continue;
            };

            texture_atlas.index = anim.frame + weapon_animation.start_index;
            sprite.flip_x = facing_direction.0.should_flip_across_y();

            *vis = Visibility::Visible;
        }
    }
}

/// The set of systems and data associated with Combat Animations
pub mod combat {
    use super::*;
    use crate::combat::{AttackExecution, AttackIntent, AttackPhase, DefenderReaction};

    pub const ATTACK_FRAME_DURATION: f32 = 1.0 / 8.;
    pub const HURT_BY_ATTACK_FRAME_DURATION: f32 = ATTACK_FRAME_DURATION * 2.;

    pub fn apply_animation_on_attack_phase(
        mut attacks: Query<&mut AttackExecution>,
        mut anims: Query<&mut UnitAnimationPlayer>,
    ) {
        for mut attack in attacks.iter_mut() {
            match attack.animation_phase {
                crate::combat::AttackPhase::Windup => {
                    if let Some(mut attacker) = anims.get_mut(attack.attacker).ok() {
                        attacker.play(AnimToPlay {
                            id: UnitAnimationKind::Attack,
                            frame_duration: ATTACK_FRAME_DURATION,
                        });
                    }
                    attack.animation_phase = crate::combat::AttackPhase::PostWindup;
                }
                crate::combat::AttackPhase::Impact => {
                    let anim = match attack.outcome.defender_reaction {
                        DefenderReaction::TakeHit => UnitAnimationKind::TakeDamage,
                        _ => {
                            warn!("We only have a TakeDamage animation!");
                            UnitAnimationKind::TakeDamage
                        }
                    };
                    if let Some(mut defender) = anims.get_mut(attack.defender).ok() {
                        defender.play(AnimToPlay {
                            frame_duration: HURT_BY_ATTACK_FRAME_DURATION,
                            id: anim,
                        });
                    }
                    attack.animation_phase = AttackPhase::PostImpact;
                }
                _ => {}
            }
        }
    }

    /// How do I ensure that this runs before I despawn the AttackIntent?
    pub fn update_facing_direction_on_attack(
        grid_resource_manager: Res<GridManagerResource>,
        query: Query<(Entity, &AttackIntent)>,
        mut facing_query: Query<&mut FacingDirection>,
    ) {
        for (_, a) in query.iter() {
            let grid = &(grid_resource_manager.grid_manager);
            let a_pos = grid.get_by_id(&a.attacker);
            let t_pos = grid.get_by_id(&a.defender);

            match (a_pos, t_pos) {
                (Some(attacker_position), Some(target_position)) => {
                    let Some(mut facing) = facing_query.get_mut(a.attacker).ok() else {
                        continue;
                    };

                    let y = target_position.y as i32 - attacker_position.y as i32;
                    let x = target_position.x as i32 - attacker_position.x as i32;

                    let x_dir = if x >= 0 { Direction::NE } else { Direction::SW };
                    let y_dir = if y >= 0 { Direction::SE } else { Direction::NW };

                    *facing = FacingDirection(if x.abs() > y.abs() { x_dir } else { y_dir })
                }
                _ => {
                    continue;
                }
            }
        }
    }
}

pub fn idle_animation_system(
    res: Res<TinytacticsAssets>,
    mut query: Query<(
        &Unit,
        &FacingDirection,
        &mut UnitAnimationPlayer,
        Option<&GridMovement>,
    )>,
) {
    for (unit, dir, mut anim_player, moving) in &mut query {
        let anim_kind_to_play = match (unit.downed(), unit.critical_health(), moving) {
            (true, _, _) => UnitAnimationKind::IdleDead,
            (false, true, None) => UnitAnimationKind::IdleHurt,
            (false, true, Some(..)) => UnitAnimationKind::IdleWalk,
            (false, false, _) => UnitAnimationKind::IdleWalk,
        };

        let Some(inner) = res
            .unit_animation_data
            .unit_animations
            .get(&UnitAnimationKey {
                kind: anim_kind_to_play,
                direction: dir.0.animation_direction(),
            })
        else {
            return;
        };

        let anim_to_play = AnimToPlay {
            id: anim_kind_to_play,
            frame_duration: inner.inner.frame_duration,
        };

        match &anim_player.current_animation {
            Some(anim) => {
                if anim.id != anim_to_play.id {
                    anim_player.play(anim_to_play.clone())
                }
            }
            None => anim_player.play(anim_to_play.clone()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitAnimationKind {
    IdleWalk,
    IdleHurt,
    IdleDead,
    Charge,
    Attack,
    TakeDamage,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitAnimationKey {
    pub kind: UnitAnimationKind,
    pub direction: Direction,
}

impl UnitAnimationKind {
    fn priority(&self) -> AnimationPriority {
        match self {
            UnitAnimationKind::IdleWalk => AnimationPriority::Idle,
            UnitAnimationKind::IdleHurt => AnimationPriority::Idle,
            UnitAnimationKind::IdleDead => AnimationPriority::Idle,
            UnitAnimationKind::Charge => AnimationPriority::Combat,
            UnitAnimationKind::Attack => AnimationPriority::Combat,
            UnitAnimationKind::TakeDamage => AnimationPriority::Reaction,
        }
    }
}

#[derive(Debug)]
pub struct PlayingAnimation {
    id: UnitAnimationKind,
    frame: usize,
    timer: Timer,
}

#[derive(Clone, Debug)]
pub struct AnimToPlay {
    id: UnitAnimationKind,
    frame_duration: f32,
}

#[derive(Component, Debug)]
pub struct UnitAnimationPlayer {
    current_animation: Option<PlayingAnimation>,
}

impl UnitAnimationPlayer {
    pub fn new() -> Self {
        Self {
            current_animation: None,
        }
    }

    pub fn play(&mut self, anim: AnimToPlay) {
        if self.preempts(&anim) && !self.is_already_running(&anim) {
            self.current_animation = Some(PlayingAnimation {
                id: anim.id,
                frame: 0,
                timer: Timer::from_seconds(anim.frame_duration, TimerMode::Repeating),
            })
        }
    }

    fn is_already_running(&self, anim: &AnimToPlay) -> bool {
        self.current_animation
            .as_ref()
            .map(|t| t.id == anim.id)
            .unwrap_or_default()
    }

    fn preempts(&self, anim: &AnimToPlay) -> bool {
        self.current_priority()
            .map(|t| anim.id.priority() >= t)
            .unwrap_or(true)
    }

    pub fn current_priority(&self) -> Option<AnimationPriority> {
        self.current_animation.as_ref().map(|a| a.id.priority())
    }
}

#[derive(Component, Debug)]
pub struct AnimationState(pub AnimationType);

#[derive(Asset, TypePath, Debug)]
pub struct UnitAnimations {
    pub unit_animations: HashMap<UnitAnimationKey, UnitAnimationData>,
    pub weapon_animations: HashMap<UnitAnimationKey, UnitAnimationData>,
}

pub fn generate_animations(
    kind: UnitAnimationKind,
    data: UnitAnimationDataInner,
    direction_to_start: &[(Direction, usize); 2],
) -> Vec<(UnitAnimationKey, UnitAnimationData)> {
    direction_to_start
        .into_iter()
        .map(|(k, v)| {
            (
                UnitAnimationKey {
                    kind,
                    direction: *k,
                },
                UnitAnimationData {
                    start_index: *v,
                    inner: data.clone(),
                },
            )
        })
        .collect()
}

pub fn weapon_animations() -> HashMap<UnitAnimationKey, UnitAnimationData> {
    let attack_data = UnitAnimationDataInner {
        frame_count: 4,
        frame_duration: ATTACK_FRAME_DURATION,
        animation_offset_markers: HashMap::new(),
    };
    let attack_start_indices = [(Direction::NE, 0), (Direction::SE, 4)];
    let attack_anims = generate_animations(
        UnitAnimationKind::Attack,
        attack_data,
        &attack_start_indices,
    );
    attack_anims.into_iter().collect()
}

pub fn unit_animations() -> HashMap<UnitAnimationKey, UnitAnimationData> {
    let idle_data = UnitAnimationDataInner {
        frame_count: 8,
        frame_duration: (1.0 / 8.),
        animation_offset_markers: HashMap::new(),
    };

    let idle_start_indices = [(Direction::NE, 0), (Direction::SE, 8)];

    let attack_data = UnitAnimationDataInner {
        frame_count: 4,
        frame_duration: ATTACK_FRAME_DURATION,
        animation_offset_markers: HashMap::from([
            (2, AnimationMarker::HitFrame),
            (4, AnimationMarker::Complete),
        ]),
    };

    let attack_start_indices = [(Direction::NE, 16), (Direction::SE, 20)];

    let attack_anims = generate_animations(
        UnitAnimationKind::Attack,
        attack_data,
        &attack_start_indices,
    );

    let take_damage_indices = [(Direction::NE, 40), (Direction::SE, 44)];

    let take_damage_anim_data = UnitAnimationDataInner {
        frame_count: 1,
        frame_duration: (1.0 / 4.),
        animation_offset_markers: HashMap::new(),
    };

    let take_damage_anims = generate_animations(
        UnitAnimationKind::TakeDamage,
        take_damage_anim_data,
        &take_damage_indices,
    );

    let hurt_idle_indices = [(Direction::NE, 48), (Direction::SE, 52)];

    let hurt_idle_anim_data = UnitAnimationDataInner {
        frame_count: 1,
        frame_duration: 1.0,
        animation_offset_markers: HashMap::new(),
    };

    let hurt_idle_anims = generate_animations(
        UnitAnimationKind::IdleHurt,
        hurt_idle_anim_data,
        &hurt_idle_indices,
    );

    let death_idle_indices = [(Direction::NE, 56), (Direction::SE, 60)];

    let death_idle_anim_data = UnitAnimationDataInner {
        frame_count: 1,
        frame_duration: (1.0),
        animation_offset_markers: HashMap::new(),
    };

    let death_idle_anims = generate_animations(
        UnitAnimationKind::IdleDead,
        death_idle_anim_data,
        &death_idle_indices,
    );

    let idle_anims =
        generate_animations(UnitAnimationKind::IdleWalk, idle_data, &idle_start_indices);

    let mut all_anims = Vec::new();

    all_anims.extend(idle_anims);
    all_anims.extend(attack_anims);
    all_anims.extend(take_damage_anims);
    all_anims.extend(hurt_idle_anims);
    all_anims.extend(death_idle_anims);

    all_anims.into_iter().collect()
}

#[derive(Debug)]
pub struct UnitAnimationData {
    pub start_index: usize,
    pub inner: UnitAnimationDataInner,
}

#[derive(Debug, Clone)]
pub struct UnitAnimationDataInner {
    pub frame_duration: f32,
    pub frame_count: usize,
    pub animation_offset_markers: HashMap<usize, AnimationMarker>,
}

// Create a Texture Atlas from a tinytactics spritesheet
#[derive(Resource)]
pub struct TinytacticsAssets {
    pub fighter_spritesheet: Handle<Image>,
    pub mage_spritesheet: Handle<Image>,
    pub cleric_spritesheet: Handle<Image>,
    pub iron_axe_spritesheet: Handle<Image>,
    pub scepter_spritesheet: Handle<Image>,
    pub tile_spritesheet: Handle<Image>,
    /// Probably could do one of these for all characters for now
    pub unit_layout: Handle<TextureAtlasLayout>,
    pub weapon_layout: Handle<TextureAtlasLayout>,
    pub tile_layout: Handle<TextureAtlasLayout>,
    pub animation_data: Handle<tinytactics::AnimationAsset>,
    pub unit_animation_data: UnitAnimations,
}

pub fn startup_load_tinytactics_assets(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) {
    let fighter_spritesheet = asset_server.load(tinytactics::spritesheet_path(Character::Fighter));
    let mage_spritesheet = asset_server.load(tinytactics::spritesheet_path(Character::Mage));
    let cleric_spritesheet = asset_server.load(tinytactics::spritesheet_path(Character::Cleric));
    let iron_axe_spritesheet =
        asset_server.load(tinytactics::weapon_spritesheet_path(WeaponType::IronAxe));
    let scepter_spritesheet =
        asset_server.load(tinytactics::weapon_spritesheet_path(WeaponType::Scepter));
    let weapon_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(
            tinytactics::FRAME_SIZE_X + 16,
            tinytactics::FRAME_SIZE_Y + 16,
        ),
        4,
        2,
        None,
        None,
    ));
    let layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(tinytactics::FRAME_SIZE_X, tinytactics::FRAME_SIZE_Y),
        4,
        16,
        None,
        None,
    ));
    let tile_spritesheet = asset_server.load(BATTLE_TACTICS_TILESHEET);
    let tile_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(tinytactics::FRAME_SIZE_X, tinytactics::FRAME_SIZE_Y),
        16,
        13,
        None,
        None,
    ));

    let animation_data = asset_server.load(tinytactics::spritesheet_data_path(Character::Fighter));
    let unit_animations = unit_animations();
    let weapon_animations = weapon_animations();

    commands.insert_resource(TinytacticsAssets {
        fighter_spritesheet,
        mage_spritesheet,
        cleric_spritesheet,
        unit_layout: layout,
        animation_data,
        scepter_spritesheet,
        unit_animation_data: UnitAnimations {
            unit_animations,
            weapon_animations,
        },
        iron_axe_spritesheet,
        weapon_layout,
        tile_layout,
        tile_spritesheet,
    })
}

/// TODO: how should I do different durations for different animations?
#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

pub fn update_facing_direction_on_movement(
    mut query: Query<(&GridMovement, &mut FacingDirection), Changed<GridMovement>>,
) {
    for (movement, mut facing_direction) in query.iter_mut() {
        if let Some(next_pos) = movement.waypoints.get(movement.current_waypoint_index + 1) {
            if let Some(current_pos) = movement.waypoints.get(movement.current_waypoint_index) {
                let delta = GridVec {
                    x: next_pos.x as i32 - current_pos.x as i32,
                    y: next_pos.y as i32 - current_pos.y as i32,
                };

                // Convert delta to direction
                let new_direction = match (delta.x, delta.y) {
                    (0, 1) => Direction::SE,
                    (-1, 0) => Direction::SW,
                    (1, 0) => Direction::NE,
                    (0, -1) => Direction::NW,
                    _ => facing_direction.0,
                };

                if !(facing_direction.0 == new_direction) {
                    facing_direction.0 = new_direction;
                }
            }
        }
    }
}

/// Mod for handling specifics about tinytactics assets
pub mod tinytactics {
    use bevy::prelude::*;
    use std::{path::PathBuf, str::FromStr};

    use image::{ImageBuffer, Rgba};

    /// These are hardcoded from observation, but probably could
    /// be derived if we end up with more complex spritesheets or
    /// want to use this system for our own stuff as we start
    /// creating our own art :shrug:
    pub const FRAME_SIZE_X: u32 = 32;
    pub const FRAME_SIZE_Y: u32 = 32;
    pub const SPRITESHEET_GRID_X: u32 = 4;
    pub const SPRITESHEET_GRID_Y: u32 = 32;

    impl From<Action> for super::AnimationType {
        fn from(value: Action) -> Self {
            match value {
                Action::Walking => Self::Idle,
                Action::Attack => todo!(),
                Action::Release => todo!(),
                Action::Charging => todo!(),
                Action::Damage => todo!(),
                Action::Weak => todo!(),
                Action::Dead => todo!(),
            }
        }
    }

    /// Assumes index is zero indexed.
    pub fn spritesheet_coords_to_index(coord: (u32, u32)) -> u32 {
        let (x, y) = coord;
        y * SPRITESHEET_GRID_X + x
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    pub struct AnimationData {
        action: Action,
        direction: Direction,
        frame_count: u32,
        frame_indices: Vec<(u32, u32)>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, Asset, TypePath)]
    pub struct AnimationAsset {
        pub character: Character,
        pub data: Vec<AnimationData>,
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        Hash,
        PartialEq,
        Eq,
        Ord,
        PartialOrd,
        serde::Serialize,
        serde::Deserialize,
    )]
    pub enum Character {
        Fighter,
        Mage,
        Cleric,
    }

    impl std::fmt::Display for Character {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Character::Fighter => write!(f, "fighter"),
                Character::Mage => write!(f, "mage"),
                Character::Cleric => write!(f, "cleric"),
            }
        }
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        Hash,
        PartialEq,
        Eq,
        Ord,
        PartialOrd,
        serde::Serialize,
        serde::Deserialize,
    )]
    pub enum Action {
        Walking,
        Attack,
        Release,
        Charging,
        Damage,
        Weak,
        Dead,
    }

    impl std::fmt::Display for Action {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Action::Attack => write!(f, "attack"),
                Action::Charging => write!(f, "charging"),
                Action::Damage => write!(f, "damage"),
                Action::Dead => write!(f, "dead"),
                Action::Release => write!(f, "release"),
                Action::Walking => write!(f, "walking"),
                Action::Weak => write!(f, "weak"),
            }
        }
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        Hash,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        serde::Serialize,
        serde::Deserialize,
    )]

    pub enum Direction {
        NE,
        NW,
        SE,
        SW,
    }

    impl Direction {
        pub fn flip_across_y(&self) -> Direction {
            match self {
                Direction::NE => Direction::NW,
                Direction::NW => Direction::NE,
                Direction::SE => Direction::SW,
                Direction::SW => Direction::SE,
            }
        }

        // TODO: Should I add a different type here to
        // better constrain the Domain?
        pub fn animation_direction(&self) -> Direction {
            match self {
                Direction::NE => Direction::NE,
                Direction::NW => Direction::NE,
                Direction::SE => Direction::SE,
                Direction::SW => Direction::SE,
            }
        }

        pub fn should_flip_across_y(&self) -> bool {
            match self {
                Direction::NE => false,
                Direction::SE => false,
                Direction::SW => true,
                Direction::NW => true,
            }
        }
    }

    impl std::fmt::Display for Direction {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Direction::SE => write!(f, "SE"),
                Direction::NE => write!(f, "NE"),
                Direction::NW => write!(f, "NW"),
                Direction::SW => write!(f, "SW"),
            }
        }
    }

    pub const FILE_PREFIX: &str = "assets/unit_assets/tinytactics_battlekiti_v1_0/";
    pub const UNIT_DATE_MADE: &str = "20240427";
    pub const WEAPON_DATE_MADE: &str = "20240429";

    pub fn sprite_filename(character: Character, action: Action, dir: Direction) -> PathBuf {
        PathBuf::from_str(&format!(
            "{FILE_PREFIX}{UNIT_DATE_MADE}{}-{}{}.png",
            character.to_string(),
            action.to_string(),
            dir.to_string()
        ))
        .expect("Should be valid path")
    }

    pub fn spritesheet_data_path(character: Character) -> PathBuf {
        PathBuf::from_str(&format!(
            "unit_assets/spritesheets/{}_animation_data.json",
            character
        ))
        .expect("Must be valid path")
    }

    pub fn spritesheet_path(character: Character) -> PathBuf {
        PathBuf::from_str(&format!(
            "unit_assets/spritesheets/{}_spritesheet.png",
            character
        ))
        .expect("Must be valid path")
    }

    pub fn calculate_animation_data(
        action: Action,
        direction: Direction,
        current_height: u32,
        image: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    ) -> AnimationData {
        let y_offset = current_height / FRAME_SIZE_Y;
        let vert_index_count = image.height() / FRAME_SIZE_Y;
        let hort_index_count = image.width() / FRAME_SIZE_X;
        let mut frame_indices = Vec::new();

        for y in y_offset..(y_offset + vert_index_count) {
            for x in 0..hort_index_count {
                frame_indices.push((x, y));
            }
        }

        AnimationData {
            action,
            direction,
            frame_count: hort_index_count * vert_index_count,
            frame_indices,
        }
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        Hash,
        PartialEq,
        Eq,
        Ord,
        PartialOrd,
        serde::Serialize,
        serde::Deserialize,
    )]
    pub enum WeaponType {
        Hatchet,
        IronAxe,
        IronSword,
        Scepter,
        WoodenStaff,
        WoodenSword,
    }

    impl WeaponType {
        pub fn variants() -> Vec<WeaponType> {
            vec![
                WeaponType::Hatchet,
                WeaponType::IronAxe,
                WeaponType::IronSword,
                WeaponType::Scepter,
                WeaponType::WoodenStaff,
                WeaponType::WoodenSword,
            ]
        }
    }

    impl std::fmt::Display for WeaponType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                WeaponType::Hatchet => write!(f, "Hatchet"),
                WeaponType::IronAxe => write!(f, "IronAxe"),
                WeaponType::IronSword => write!(f, "IronSword"),
                WeaponType::Scepter => write!(f, "Scepter"),
                WeaponType::WoodenStaff => write!(f, "WoodenStaff"),
                WeaponType::WoodenSword => write!(f, "WoodenSword"),
            }
        }
    }

    pub fn weapon_attack_sprite_filename(weapon: WeaponType, dir: Direction) -> PathBuf {
        PathBuf::from_str(&format!(
            "{FILE_PREFIX}{WEAPON_DATE_MADE}weapons-{}attack{}.png",
            weapon.to_string(),
            dir.to_string()
        ))
        .expect("Should be valid path")
    }

    pub fn weapon_spritesheet_path(weapon: WeaponType) -> PathBuf {
        PathBuf::from_str(&format!(
            "unit_assets/spritesheets/{}_spritesheet.png",
            weapon
        ))
        .expect("Should be valid path")
    }
}
