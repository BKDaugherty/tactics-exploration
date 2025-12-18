//! A module for talking about and coordinating animation data.

use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct FacingDirection(pub Direction);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    NorthEast,
    SouthEast,
    NorthWest,
    SouthWest,
}

#[derive(Debug)]
pub struct AnimationData {
    pub start_index: usize,
    pub end_index: usize,
    pub duration: f32,
}

#[derive(Asset, TypePath, Debug)]
pub struct UnitAnimations {
    pub idle: HashMap<Direction, AnimationData>,
}

// Create a Texture Atlas from a tinytactics spritesheet
#[derive(Resource)]
pub struct TinytacticsAssets {
    pub spritesheet: Handle<Image>,
    /// Probably could do one of these for all characters for now
    pub layout: Handle<TextureAtlasLayout>,
}

pub fn startup_load_tinytactics_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let character = tinytactics::Character::Cleric;
    let spritesheet = asset_server.load(tinytactics::spritesheet_path(character));
    let layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(tinytactics::FRAME_SIZE_X, tinytactics::FRAME_SIZE_Y),
        32,
        4,
        None,
        None,
    ));

    commands.insert_resource(TinytacticsAssets {
        spritesheet,
        layout,
    })
}

/// Mod for handling specifics about tinytactics assets
pub mod tinytactics {
    use std::{path::PathBuf, str::FromStr};

    use image::{ImageBuffer, Rgba};

    pub const FRAME_SIZE_X: u32 = 32;
    pub const FRAME_SIZE_Y: u32 = 32;

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    pub struct AnimationData {
        action: Action,
        direction: Direction,
        frame_count: u32,
        frame_indices: Vec<(u32, u32)>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
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
