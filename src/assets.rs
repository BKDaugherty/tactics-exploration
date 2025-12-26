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
    let badge = asset_loader.load("font_assets/tinyRPGFontKit01_v1_2/TinyRpg-BadgeFont.ttf");
    let fine_fantasy =
        asset_loader.load("font_assets/tinyRPGFontKit01_v1_2/TinyRpg-FineFantasyStrategies.ttf");
    commands.insert_resource(FontResource {
        fine_fantasy,
        badge,
    });
}
