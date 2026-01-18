use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::Parser;
use rand::prelude::*;
use rand_pcg::Pcg64;
use rand_seeder::{Seeder, SipHasher};
use tactics_exploration::{
    animation::Direction,
    assets::BATTLE_TACTICS_TILESHEET,
    camera::setup_camera,
    grid::GridPosition,
    map_generation::{
        BattleMapOptions, BridgeTileType, GrassTileType, LayerId, MapData, MapParams, MapResource,
        TileType, WaterTileType, build_tilemap_from_map, setup_map_data_from_params,
    },
};

fn spawn_map(mut commands: Commands, res: Res<MapResource>, asset_server: Res<AssetServer>) {
    let asset = asset_server.load(BATTLE_TACTICS_TILESHEET);
    build_tilemap_from_map(&mut commands, asset, &res.data);
}

fn main() {
    let options = BattleMapOptions::parse();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(MapParams { options })
        .add_plugins(TilemapPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(
            Startup,
            (setup_camera, setup_map_data_from_params, spawn_map).chain(),
        )
        .run();
}
