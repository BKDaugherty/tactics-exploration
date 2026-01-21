use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::Parser;
use tactics_exploration::{
    camera::setup_camera,
    map_generation::{BattleMapOptions, MapParams},
};

fn main() {
    let options = BattleMapOptions::parse();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(MapParams { options })
        .add_plugins(TilemapPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, (setup_camera).chain())
        .run();
}
