//! Some constants associated with specific assets.
//! "assets" itself is omitted

pub const CURSOR32_PATH: &str = "utility_assets/cursor.png";
pub const CURSOR_PATH: &str = "utility_assets/cursor-16.png";
pub const OVERLAY32_PATH: &str = "utility_assets/iso_color.png";
pub const OVERLAY_PATH: &str = "utility_assets/iso_color-16.png";
pub const BATTLE_TACTICS_TILESHEET: &str =
    "map_assets/tinytactics-32-map/20240420tinyTacticsTileset00.png";

pub const GRADIENT_PATH: &str = "utility_assets/gradient.png";

pub const EXAMPLE_MAP_PATH: &str = "map_assets/example-map/example-map.tmx";
pub const EXAMPLE_MAP_2_PATH: &str = "map_assets/tinytactics-32-map/example-map-tiny-tactics.tmx";

use bevy::prelude::*;

#[derive(Resource)]
pub struct FontResource {
    pub fine_fantasy: Handle<Font>,
    pub badge: Handle<Font>,
}

pub fn setup_fonts(mut commands: Commands, asset_loader: Res<AssetServer>) {
    let badge = asset_loader.load("font_assets/tinyRPGFontKit01_v1_2/TinyRPG-BadgeFont.ttf");
    let fine_fantasy =
        asset_loader.load("font_assets/tinyRPGFontKit01_v1_2/TinyRPG-FineFantasyStrategies.ttf");
    commands.insert_resource(FontResource {
        fine_fantasy,
        badge,
    });
}

/// Skills need to be able to reference in data format
/// what asset they spawn for VFX. For now, this can be tracked in a "DB".
pub mod sprite_db {
    use std::collections::HashMap;

    use super::*;

    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Reflect,
    )]
    pub struct SpriteId(pub u32);

    #[derive(Debug, Resource)]
    pub struct SpriteDB {
        pub sprite_id_to_handle: HashMap<SpriteId, Handle<Image>>,
    }

    impl SpriteDB {
        fn new() -> Self {
            Self {
                sprite_id_to_handle: HashMap::new(),
            }
        }
    }

    /// Utility enum to track the TinyTactics specific sprites for now while
    /// they are still in use.
    #[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, Hash)]
    pub enum TinyTacticsSprites {
        TtMapSheet,
        Fighter,
        Mage,
        Cleric,
        IronAxe,
        Scepter,
    }

    impl From<TinyTacticsSprites> for SpriteId {
        fn from(value: TinyTacticsSprites) -> Self {
            match value {
                TinyTacticsSprites::TtMapSheet => SpriteId(1),
                TinyTacticsSprites::Fighter => SpriteId(2),
                TinyTacticsSprites::Mage => SpriteId(3),
                TinyTacticsSprites::Cleric => SpriteId(4),
                TinyTacticsSprites::IronAxe => SpriteId(7),
                TinyTacticsSprites::Scepter => SpriteId(8),
            }
        }
    }

    fn build_sprite_map() -> HashMap<SpriteId, String> {
        HashMap::from([
            (
                TinyTacticsSprites::TtMapSheet.into(),
                BATTLE_TACTICS_TILESHEET.to_string(),
            ),
            (
                TinyTacticsSprites::Fighter.into(),
                "unit_assets/spritesheets/fighter_spritesheet.png".to_string(),
            ),
            (
                TinyTacticsSprites::Mage.into(),
                "unit_assets/spritesheets/mage_spritesheet.png".to_string(),
            ),
            (
                TinyTacticsSprites::Cleric.into(),
                "unit_assets/spritesheets/cleric_spritesheet.png".to_string(),
            ),
            (
                TinyTacticsSprites::IronAxe.into(),
                "unit_assets/spritesheets/IronAxe_spritesheet.png".to_string(),
            ),
            (
                TinyTacticsSprites::Scepter.into(),
                "unit_assets/spritesheets/Scepter_spritesheet.png".to_string(),
            ),
            (
                SpriteId(5),
                "misc_assets/fire_effect_2/explosion_2_spritesheet.png".to_string(),
            ),
            (SpriteId(6), "misc_assets/arrow.png".to_string()),
        ])
    }

    pub fn build_sprite_db(mut commands: Commands, asset_server: Res<AssetServer>) {
        let map = build_sprite_map();
        let mut db = SpriteDB::new();
        for (id, path) in map {
            let handle = asset_server.load(path);
            db.sprite_id_to_handle.insert(id, handle);
        }

        commands.insert_resource(db);
    }
}
