use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::Parser;
use tactics_exploration::{
    assets::BATTLE_TACTICS_TILESHEET,
    camera::setup_camera,
    map_generation::{
        BattleMapOptions, MapParams, MapResource, build_tilemap_from_map,
        setup_map_data_from_params,
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
