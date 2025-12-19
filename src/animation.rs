//! A module for talking about and coordinating animation data.

use std::collections::HashMap;

use bevy::prelude::*;
pub use tinytactics::Direction;

use crate::{
    animation::tinytactics::Character,
    grid::{GridMovement, GridVec},
};

#[derive(Component, Debug, Clone)]
pub struct FacingDirection(pub Direction);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationType {
    Idle,
}

#[derive(Component, Debug)]
pub struct AnimationState(pub AnimationType);

#[derive(Asset, TypePath, Debug)]
pub struct UnitAnimations {
    pub idle: HashMap<Direction, UnitAnimationData>,
}

// TODO: Load from _animation_data.json (maybe just preparse into this format)
pub const UNIT_IDLE_ANIMATIONS: [(Direction, UnitAnimationData); 4] = [
    (
        Direction::NE,
        UnitAnimationData {
            start_index: 0,
            end_index: 7,
            duration: 1.0,
        },
    ),
    (
        Direction::NW,
        UnitAnimationData {
            start_index: 8,
            end_index: 15,
            duration: 1.0,
        },
    ),
    (
        Direction::SE,
        UnitAnimationData {
            start_index: 16,
            end_index: 23,
            duration: 1.0,
        },
    ),
    (
        Direction::SW,
        UnitAnimationData {
            start_index: 24,
            end_index: 31,
            duration: 1.0,
        },
    ),
];

#[derive(Debug)]
pub struct UnitAnimationData {
    pub start_index: usize,
    pub end_index: usize,
    pub duration: f32,
}

// Create a Texture Atlas from a tinytactics spritesheet
#[derive(Resource)]
pub struct TinytacticsAssets {
    pub fighter_spritesheet: Handle<Image>,
    pub mage_spritesheet: Handle<Image>,
    /// Probably could do one of these for all characters for now
    pub layout: Handle<TextureAtlasLayout>,
    pub animation_data: Handle<tinytactics::AnimationAsset>,
    pub unit_animation_data: UnitAnimations,
}

pub fn startup_load_tinytactics_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let fighter_spritesheet = asset_server.load(tinytactics::spritesheet_path(Character::Fighter));
    let mage_spritesheet = asset_server.load(tinytactics::spritesheet_path(Character::Mage));
    let layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(tinytactics::FRAME_SIZE_X, tinytactics::FRAME_SIZE_Y),
        4,
        32,
        None,
        None,
    ));

    let animation_data = asset_server.load(tinytactics::spritesheet_data_path(Character::Fighter));
    let unit_animations = UnitAnimations {
        idle: HashMap::from(UNIT_IDLE_ANIMATIONS),
    };

    commands.insert_resource(TinytacticsAssets {
        fighter_spritesheet,
        mage_spritesheet,
        layout,
        animation_data,
        unit_animation_data: unit_animations,
    })
}

/// TODO: how should I do different durations for different animations?
#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

pub fn update_facing_direction(
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

pub fn update_sprite_on_animation_change(
    tinytactics_assets: Res<TinytacticsAssets>,
    mut query: Query<
        (
            &AnimationState,
            &FacingDirection,
            &mut AnimationTimer,
            &mut Sprite,
        ),
        Or<(Changed<AnimationState>, Changed<FacingDirection>)>,
    >,
) {
    for (state, facing_direction, mut timer, mut sprite) in &mut query {
        // Create new timer based on state? timer.0 = Timer::new()
        let anim_data = match state.0 {
            AnimationType::Idle => tinytactics_assets
                .unit_animation_data
                .idle
                .get(&facing_direction.0),
        };

        if let Some(anim_data) = anim_data {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = anim_data.start_index;
            }

            // Reset the timer
            timer.0 = Timer::from_seconds(
                anim_data.duration / (anim_data.end_index - anim_data.start_index + 1) as f32,
                TimerMode::Repeating,
            )
        }
    }
}

pub fn animate_sprite(
    time: Res<Time>,
    tinytactics_assets: Res<TinytacticsAssets>,
    mut query: Query<(
        &AnimationState,
        &FacingDirection,
        &mut AnimationTimer,
        &mut Sprite,
    )>,
) {
    for (state, facing_direction, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());

        let unit_animation_data = match state.0 {
            AnimationType::Idle => tinytactics_assets
                .unit_animation_data
                .idle
                .get(&facing_direction.0),
        };

        let Some(unit_animation_data) = unit_animation_data else {
            warn!("No animation data found for unit");
            continue;
        };

        if timer.just_finished()
            && let Some(atlas) = &mut sprite.texture_atlas
        {
            atlas.index = if atlas.index == unit_animation_data.end_index {
                unit_animation_data.start_index
            } else {
                atlas.index + 1
            };
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
        pub fn flip_y(&self) -> Direction {
            match self {
                Direction::NE => Direction::NW,
                Direction::NW => Direction::NE,
                Direction::SE => Direction::SW,
                Direction::SW => Direction::SE,
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
    pub const DATE_MADE: &str = "20240427";

    pub fn sprite_filename(character: Character, action: Action, dir: Direction) -> PathBuf {
        PathBuf::from_str(&format!(
            "{FILE_PREFIX}{DATE_MADE}{}-{}{}.png",
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
}
