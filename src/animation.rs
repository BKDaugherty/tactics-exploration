//! A module for talking about and coordinating animation data.

use std::collections::HashMap;

use bevy::prelude::*;
pub use tinytactics::Direction;

use crate::{
    animation::{
        animation_db::{
            AnimatedSpriteId, AnimationDB, AnimationKey, AnimationStartIndexKey,
            FollowerAnimationKey, RegisteredAnimationId,
        },
        combat::ATTACK_FRAME_DURATION,
        tinytactics::{Character, WeaponType},
    },
    assets::BATTLE_TACTICS_TILESHEET,
    combat::{CombatAnimationId, UnitIsAttacking},
    grid::{GridManagerResource, GridMovement, GridVec},
    unit_stats::UnitDerivedStats,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub id: Option<AnimationId>,
}

#[derive(Component)]
pub struct AnimationFollower {
    pub leader: Entity,
    pub animated_sprite_id: AnimatedSpriteId,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AnimationId {
    Combat(CombatAnimationId),
}

// Problem, I need a general animation_tick system
// But if the FacingDirection changes, I'll want to update the
// AnimationId

// So the AnimationPlayer could carry the AnimationInformation associated?
// And then playing an animation could involve looking up the relevant information?

// AnimationKind
//

// So maybe I have a database of...
// Animation -> TextureAtlas start index?

// Specific Animation

/// One per registered TextureAtlas basically

// AnimationDB
// - Provides lookups for a registered AnimationKey to Start Index
//
//
// - AnimationKey -> StartIndex

// AnimationClip
// - AnimationData
// - StartIndex (relative to the TextureAtlas)

// AnimationData
// - Frame Count
// - AnimationOffsetMarkers (relative to frames in atlas)

pub fn animation_follower_system(
    anim_db: Res<AnimationDB>,
    anim_query: Query<(Option<&FacingDirection>, &UnitAnimationPlayer)>,
    mut follower_query: Query<
        (&AnimationFollower, &mut Sprite, &mut Visibility),
        With<AnimationFollower>,
    >,
) {
    for (follower, mut sprite, mut vis) in follower_query.iter_mut() {
        if let Ok((facing_direction, player)) = anim_query.get(follower.leader) {
            let Some(anim) = &player.current_animation else {
                *vis = Visibility::Hidden;
                continue;
            };

            let Some(follower_start_index) = anim_db.get_follower_animation_start_index(
                &FollowerAnimationKey {
                    follower_id: follower.animated_sprite_id,
                    followee_key: AnimationKey {
                        animated_sprite_id: player.animated_sprite_id,
                        animation_id: anim.id,
                    },
                },
                facing_direction.cloned().map(|t| t.0.animation_direction()),
            ) else {
                *vis = Visibility::Hidden;
                continue;
            };

            let Some(texture_atlas) = sprite.texture_atlas.as_mut() else {
                warn!("No texture atlas for Weapon Sprite Follower");
                continue;
            };

            texture_atlas.index = anim.frame + *follower_start_index as usize;

            if let Some(fd) = facing_direction {
                sprite.flip_x = fd.0.should_flip_across_y();
            }

            *vis = Visibility::Visible;
        }
    }
}

pub fn animation_tick_system(
    time: Res<Time>,
    anim_db: Res<AnimationDB>,
    mut query: Query<
        (
            Entity,
            Option<&FacingDirection>,
            &mut UnitAnimationPlayer,
            &mut Sprite,
        ),
        Without<AnimationFollower>,
    >,
    mut marker_events: MessageWriter<AnimationMarkerMessage>,
) {
    for (entity, dir, mut player, mut sprite) in &mut query {
        let animated_sprite_id = player.animated_sprite_id;

        // Get the current animation, if any
        let Some(anim) = &mut player.current_animation else {
            continue;
        };

        let key = AnimationKey {
            animated_sprite_id,
            animation_id: anim.id,
        };

        let (Some(clip_data), Some(clip_start_index)) = (
            anim_db.get_data(&key),
            anim_db.get_start_index(&AnimationStartIndexKey {
                facing_direction: dir.map(|t| t.0.animation_direction()),
                key: key.clone(),
            }),
        ) else {
            warn!("No animation data found for running clip");
            continue;
        };

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.frame += 1;

            // Send event before bounds checking to allow for using the len(frames) as a "Complete" marker
            if let Some(marker) = clip_data.animation_offset_markers.get(&anim.frame) {
                marker_events.write(AnimationMarkerMessage {
                    entity,
                    marker: *marker,
                    id: anim.animation_id.clone(),
                });
            }

            if anim.frame >= clip_data.frame_count {
                player.current_animation = None;
                continue;
            }
        }

        if let Some(texture_atlas) = sprite.texture_atlas.as_mut() {
            let target_frame = anim.frame + *clip_start_index as usize;
            texture_atlas.index = target_frame;

            if let Some(dir) = dir {
                sprite.flip_x = dir.0.should_flip_across_y();
            }
        }
    }
}

/// The set of systems and data associated with Combat Animations
pub mod combat {
    use super::*;
    use crate::combat::AttackExecution;

    pub const ATTACK_FRAME_DURATION: f32 = 1.0 / 8.;
    pub const HURT_BY_ATTACK_FRAME_DURATION: f32 = ATTACK_FRAME_DURATION * 2.;

    /// How do I ensure that this runs before I despawn the AttackIntent?
    pub fn update_facing_direction_on_attack(
        grid_resource_manager: Res<GridManagerResource>,
        query: Query<(Entity, &AttackExecution)>,
        mut facing_query: Query<&mut FacingDirection>,
    ) {
        for (_, a) in query.iter() {
            let grid = &(grid_resource_manager.grid_manager);
            let Some(attacker) = a.attacker else {
                continue;
            };
            let a_pos = grid.get_by_id(&attacker);
            let t_pos = grid.get_by_id(&a.defender);

            match (a_pos, t_pos) {
                (Some(attacker_position), Some(target_position)) => {
                    let Some(mut facing) = facing_query.get_mut(attacker).ok() else {
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
    res: Res<AnimationDB>,
    mut query: Query<
        (
            &UnitDerivedStats,
            &mut UnitAnimationPlayer,
            Option<&GridMovement>,
        ),
        Without<UnitIsAttacking>,
    >,
) {
    for (unit_stats, mut anim_player, moving) in &mut query {
        let anim_kind_to_play = match (unit_stats.downed(), unit_stats.critical_health(), moving) {
            (true, _, _) => UnitAnimationKind::IdleDead,
            (false, true, None) => UnitAnimationKind::IdleHurt,
            (false, true, Some(..)) => UnitAnimationKind::IdleWalk,
            (false, false, _) => UnitAnimationKind::IdleWalk,
        };

        let Some(inner) = res.get_data(&AnimationKey {
            animated_sprite_id: anim_player.animated_sprite_id,
            animation_id: anim_kind_to_play.into(),
        }) else {
            return;
        };

        let anim_to_play = AnimToPlay {
            id: anim_kind_to_play.into(),
            frame_duration: inner.frame_duration,
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
#[repr(u32)]
pub enum UnitAnimationKind {
    IdleWalk = 1,
    IdleHurt = 2,
    IdleDead = 3,
    Charge = 4,
    Attack = 5,
    TakeDamage = 6,
    Release = 7,
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
            UnitAnimationKind::Release => AnimationPriority::Combat,
            UnitAnimationKind::TakeDamage => AnimationPriority::Reaction,
        }
    }
}

#[derive(Debug)]
pub struct PlayingAnimation {
    pub animation_id: Option<AnimationId>,
    pub id: RegisteredAnimationId,
    pub frame: usize,
    pub timer: Timer,
}

#[derive(Clone, Debug)]
pub struct AnimToPlay {
    pub id: RegisteredAnimationId,
    pub frame_duration: f32,
}

#[derive(Component, Debug)]
pub struct UnitAnimationPlayer {
    current_animation: Option<PlayingAnimation>,
    pub animated_sprite_id: AnimatedSpriteId,
}

impl UnitAnimationPlayer {
    pub fn new(animated_sprite_id: AnimatedSpriteId) -> Self {
        Self {
            current_animation: None,
            animated_sprite_id,
        }
    }

    /// Sometimes you want to spawn an AnimationPlayer with an animation already running!
    pub fn new_with_animation(
        animated_sprite_id: AnimatedSpriteId,
        current_animation: PlayingAnimation,
    ) -> Self {
        Self {
            animated_sprite_id,
            current_animation: Some(current_animation),
        }
    }

    // TODO: As we start to rely on animations more, we probably can't accept this.
    // I think we should ensure that we queue animations that aren't Idle priority.
    //
    // Then when we finish an animation, we can pop from the queue?
    pub fn play(&mut self, anim: AnimToPlay) {
        self.play_with_maybe_id(anim, None);
    }

    pub fn play_with_id(&mut self, anim: AnimToPlay, animation_id: AnimationId) {
        self.play_with_maybe_id(anim, Some(animation_id));
    }

    pub fn play_with_maybe_id(&mut self, anim: AnimToPlay, animation_id: Option<AnimationId>) {
        if self.preempts(&anim) && !self.is_already_running(&anim) {
            self.current_animation = Some(PlayingAnimation {
                id: anim.id,
                frame: 0,
                timer: Timer::from_seconds(anim.frame_duration, TimerMode::Repeating),
                animation_id,
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
            .map(|t| anim.id.priority >= t)
            .unwrap_or(true)
    }

    pub fn current_priority(&self) -> Option<AnimationPriority> {
        self.current_animation.as_ref().map(|a| a.id.priority)
    }
}

#[derive(Component, Debug)]
pub struct AnimationState(pub AnimationType);

#[derive(Asset, TypePath, Debug)]
pub struct UnitAnimations {
    pub animation_db: AnimationDB,
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
    pub tt_unit_layout: Handle<TextureAtlasLayout>,
    pub weapon_layout: Handle<TextureAtlasLayout>,
    pub tile_layout: Handle<TextureAtlasLayout>,
    pub animation_data: Handle<tinytactics::AnimationAsset>,
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

    // TODO: Use AnimationData to populate le db?
    let animation_data = asset_server.load(tinytactics::spritesheet_data_path(Character::Fighter));

    commands.insert_resource(TinytacticsAssets {
        fighter_spritesheet,
        mage_spritesheet,
        cleric_spritesheet,
        tt_unit_layout: layout,
        animation_data,
        scepter_spritesheet,
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
        if let Some(next_pos) = movement.waypoints.get(movement.current_waypoint_index + 1)
            && let Some(current_pos) = movement.waypoints.get(movement.current_waypoint_index)
        {
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

            if facing_direction.0 != new_direction {
                facing_direction.0 = new_direction;
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
            character, action, dir
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
            weapon, dir
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

pub mod animation_db {
    use registered_sprite_ids::UNIT_DEMO_SPRITE_ID;

    use crate::animation::animation_db::registered_sprite_ids::{
        FLAME_VFX_ANIMATED_SPRITE_ID, POISON_VFX_ANIMATED_SPRITE_ID, TT_UNIT_ANIMATED_SPRITE_ID,
        TT_WEAPON_ANIMATED_SPRITE_ID, build_animated_sprite_to_atlas_layout,
    };

    use super::*;

    pub fn load_animation_data(
        mut commands: Commands,
        mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    ) {
        let mut animation_db =
            build_animation_db().expect("Must be able to build static animation data");

        animation_db.initialize_atlas_map(&mut texture_atlas_layouts);
        commands.insert_resource(animation_db);
    }

    #[derive(Debug, PartialEq, Eq, Hash)]
    pub struct FollowerAnimationKey {
        pub(crate) follower_id: AnimatedSpriteId,
        pub(crate) followee_key: AnimationKey,
    }

    // TODO: Build this from json?
    fn build_animation_db() -> anyhow::Result<AnimationDB> {
        let mut db = AnimationDB::new();
        db.register_animation(
            "weapon_attack",
            AnimationKey {
                animated_sprite_id: TT_WEAPON_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::Attack.into(),
            },
            UnitAnimationDataInner {
                frame_count: 4,
                frame_duration: ATTACK_FRAME_DURATION,
                animation_offset_markers: HashMap::new(),
            },
            &[(Some(Direction::NE), 0), (Some(Direction::SE), 4)],
        )?
        .register_animation(
            "unit_attack",
            AnimationKey {
                animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::Attack.into(),
            },
            UnitAnimationDataInner {
                frame_count: 4,
                frame_duration: ATTACK_FRAME_DURATION,
                animation_offset_markers: HashMap::from([
                    (2, AnimationMarker::HitFrame),
                    (4, AnimationMarker::Complete),
                ]),
            },
            &[(Some(Direction::NE), 16), (Some(Direction::SE), 20)],
        )?
        .register_animation(
            "unit_idle_walk",
            AnimationKey {
                animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::IdleWalk.into(),
            },
            UnitAnimationDataInner {
                frame_count: 8,
                frame_duration: (1.0 / 8.),
                animation_offset_markers: HashMap::new(),
            },
            &[(Some(Direction::NE), 0), (Some(Direction::SE), 8)],
        )?
        .register_animation(
            "unit_take_damage",
            AnimationKey {
                animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::TakeDamage.into(),
            },
            UnitAnimationDataInner {
                frame_count: 1,
                frame_duration: (1.0 / 4.),
                animation_offset_markers: HashMap::new(),
            },
            &[(Some(Direction::NE), 40), (Some(Direction::SE), 44)],
        )?
        .register_animation(
            "unit_idle_hurt",
            AnimationKey {
                animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::IdleHurt.into(),
            },
            UnitAnimationDataInner {
                frame_count: 1,
                frame_duration: 1.0,
                animation_offset_markers: HashMap::new(),
            },
            &[(Some(Direction::NE), 48), (Some(Direction::SE), 52)],
        )?
        .register_animation(
            "unit_idle_dead",
            AnimationKey {
                animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::IdleDead.into(),
            },
            UnitAnimationDataInner {
                frame_count: 1,
                frame_duration: (1.0),
                animation_offset_markers: HashMap::new(),
            },
            &[(Some(Direction::NE), 56), (Some(Direction::SE), 60)],
        )?
        .register_animation(
            "unit_charge",
            AnimationKey {
                animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::Charge.into(),
            },
            UnitAnimationDataInner {
                frame_count: 1,
                frame_duration: (1.0),
                animation_offset_markers: HashMap::from([(1, AnimationMarker::Complete)]),
            },
            // TODO: Fix Spritesheet
            &[(Some(Direction::NE), 36), (Some(Direction::SE), 32)],
        )?
        .register_animation(
            "unit_release",
            AnimationKey {
                animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::Release.into(),
            },
            UnitAnimationDataInner {
                frame_count: 1,
                frame_duration: (1.0),
                animation_offset_markers: HashMap::from([(1, AnimationMarker::Complete)]),
            },
            &[(Some(Direction::NE), 24), (Some(Direction::SE), 28)],
        )?
        .register_follower(
            FollowerAnimationKey {
                follower_id: TT_WEAPON_ANIMATED_SPRITE_ID,
                followee_key: AnimationKey {
                    animated_sprite_id: TT_UNIT_ANIMATED_SPRITE_ID,
                    animation_id: UnitAnimationKind::Attack.into(),
                },
            },
            AnimationKey {
                animated_sprite_id: TT_WEAPON_ANIMATED_SPRITE_ID,
                animation_id: UnitAnimationKind::Attack.into(),
            },
        )?
        .register_animation(
            "flame_explosion",
            AnimationKey {
                animated_sprite_id: FLAME_VFX_ANIMATED_SPRITE_ID,
                // Duplicated rn between here and in Skill definition
                animation_id: RegisteredAnimationId {
                    id: 1,
                    priority: AnimationPriority::Combat,
                },
            },
            UnitAnimationDataInner {
                frame_duration: (1.0 / 9.),
                frame_count: 18,
                animation_offset_markers: HashMap::from([
                    (9, AnimationMarker::HitFrame),
                    (18, AnimationMarker::Complete),
                ]),
            },
            &[(None, 0)],
        )?
        .register_animation(
            "poison_effect",
            AnimationKey {
                animated_sprite_id: POISON_VFX_ANIMATED_SPRITE_ID,
                animation_id: RegisteredAnimationId {
                    id: 1,
                    priority: AnimationPriority::Combat,
                },
            },
            UnitAnimationDataInner {
                frame_duration: (1.0 / 16.),
                frame_count: 16,
                animation_offset_markers: HashMap::from([
                    (10, AnimationMarker::HitFrame),
                    (16, AnimationMarker::Complete),
                ]),
            },
            &[(None, 0)],
        )?
        .register_animation(
            "demo_unit_idle",
            AnimationKey {
                animated_sprite_id: UNIT_DEMO_SPRITE_ID,
                animation_id: UnitAnimationKind::IdleWalk.into(),
            },
            UnitAnimationDataInner {
                frame_duration: 1.0,
                frame_count: 1,
                animation_offset_markers: HashMap::new(),
            },
            &[(None, 0), (Some(Direction::SE), 0)],
        )?;

        Ok(db)
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AnimatedSpriteId(pub u32);

    /// TODO: Might need to encode priority of the animation in this struct too?
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct RegisteredAnimationId {
        pub id: u8,
        pub priority: AnimationPriority,
    }

    impl From<UnitAnimationKind> for RegisteredAnimationId {
        fn from(value: UnitAnimationKind) -> Self {
            RegisteredAnimationId {
                id: value as u8,
                priority: value.priority(),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]

    pub struct AnimationKey {
        pub animated_sprite_id: AnimatedSpriteId,
        /// Local ID to a specific animation for the animated_sprite_id
        pub animation_id: RegisteredAnimationId,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]

    pub struct AnimationStartIndexKey {
        pub facing_direction: Option<Direction>,
        pub key: AnimationKey,
    }

    #[derive(Debug, Resource)]
    pub struct AnimationDB {
        index_key_to_start_frame: HashMap<AnimationStartIndexKey, u8>,
        animation_data: HashMap<AnimationKey, UnitAnimationDataInner>,
        follower_map: HashMap<FollowerAnimationKey, AnimationKey>,
        atlas_map: HashMap<AnimatedSpriteId, Handle<TextureAtlasLayout>>,
    }

    impl AnimationDB {
        fn new() -> Self {
            Self {
                index_key_to_start_frame: HashMap::new(),
                animation_data: HashMap::new(),
                follower_map: HashMap::new(),
                atlas_map: HashMap::new(),
            }
        }

        fn register_animation(
            &mut self,
            _name: &str,
            key: AnimationKey,
            data: UnitAnimationDataInner,
            offsets: &[(Option<Direction>, u8)],
        ) -> anyhow::Result<&mut Self> {
            if let Some(t) = self.animation_data.insert(key.clone(), data) {
                anyhow::bail!("Data already existed for key {:?}, data: {:?}", key, t);
            };

            for (direction, offset) in offsets {
                self.index_key_to_start_frame.insert(
                    AnimationStartIndexKey {
                        facing_direction: *direction,
                        key: key.clone(),
                    },
                    *offset,
                );
            }
            Ok(self)
        }

        fn register_follower(
            &mut self,
            f_key: FollowerAnimationKey,
            key: AnimationKey,
        ) -> anyhow::Result<&mut Self> {
            if let Some(t) = self.follower_map.insert(f_key, key) {
                anyhow::bail!("Follower already existed: {:?}", t);
            };
            Ok(self)
        }

        pub fn get_data(&self, key: &AnimationKey) -> Option<&UnitAnimationDataInner> {
            self.animation_data.get(key)
        }

        pub fn get_start_index(&self, key: &AnimationStartIndexKey) -> Option<&u8> {
            self.index_key_to_start_frame.get(key)
        }

        pub fn get_follower_animation_start_index(
            &self,
            key: &FollowerAnimationKey,
            facing_direction: Option<Direction>,
        ) -> Option<&u8> {
            let key = self.follower_map.get(key);

            key.cloned().and_then(|t| {
                self.index_key_to_start_frame.get(&AnimationStartIndexKey {
                    facing_direction,
                    key: t,
                })
            })
        }

        pub fn get_atlas(&self, key: &AnimatedSpriteId) -> Option<Handle<TextureAtlasLayout>> {
            self.atlas_map.get(key).cloned()
        }

        pub fn initialize_atlas_map(
            &mut self,
            texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
        ) {
            let map = build_animated_sprite_to_atlas_layout();

            for (id, layout) in map {
                let handle = texture_atlas_layouts.add(layout);
                self.atlas_map.insert(id, handle.clone());
            }
        }
    }

    pub mod registered_sprite_ids {
        use std::collections::HashMap;

        use bevy::prelude::*;

        use crate::animation::tinytactics;

        use super::AnimatedSpriteId;

        pub const TT_UNIT_ANIMATED_SPRITE_ID: AnimatedSpriteId = AnimatedSpriteId(1);
        pub const TT_WEAPON_ANIMATED_SPRITE_ID: AnimatedSpriteId = AnimatedSpriteId(2);
        pub const BATTLE_TACTICS_TILESHEET: AnimatedSpriteId = AnimatedSpriteId(3);
        pub const FLAME_VFX_ANIMATED_SPRITE_ID: AnimatedSpriteId = AnimatedSpriteId(4);
        pub const POISON_VFX_ANIMATED_SPRITE_ID: AnimatedSpriteId = AnimatedSpriteId(5);
        pub const UNIT_DEMO_SPRITE_ID: AnimatedSpriteId = AnimatedSpriteId(6);

        pub fn build_animated_sprite_to_atlas_layout()
        -> HashMap<AnimatedSpriteId, TextureAtlasLayout> {
            HashMap::from([
                (
                    TT_UNIT_ANIMATED_SPRITE_ID,
                    TextureAtlasLayout::from_grid(
                        UVec2::new(tinytactics::FRAME_SIZE_X, tinytactics::FRAME_SIZE_Y),
                        4,
                        16,
                        None,
                        None,
                    ),
                ),
                (
                    TT_WEAPON_ANIMATED_SPRITE_ID,
                    TextureAtlasLayout::from_grid(
                        UVec2::new(
                            tinytactics::FRAME_SIZE_X + 16,
                            tinytactics::FRAME_SIZE_Y + 16,
                        ),
                        4,
                        2,
                        None,
                        None,
                    ),
                ),
                (
                    BATTLE_TACTICS_TILESHEET,
                    TextureAtlasLayout::from_grid(
                        UVec2::new(tinytactics::FRAME_SIZE_X, tinytactics::FRAME_SIZE_Y),
                        16,
                        13,
                        None,
                        None,
                    ),
                ),
                (
                    FLAME_VFX_ANIMATED_SPRITE_ID,
                    TextureAtlasLayout::from_grid(UVec2::new(48, 48), 18, 1, None, None),
                ),
                (
                    POISON_VFX_ANIMATED_SPRITE_ID,
                    TextureAtlasLayout::from_grid(UVec2::new(32, 32), 16, 1, None, None),
                ),
                (
                    UNIT_DEMO_SPRITE_ID,
                    TextureAtlasLayout::from_grid(UVec2::new(32, 32), 1, 1, None, None),
                ),
            ])
        }
    }
}
