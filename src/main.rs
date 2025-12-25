use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use leafwing_input_manager::plugin::InputManagerPlugin;
use tactics_exploration::GameState;
use tactics_exploration::assets::setup_fonts;
use tactics_exploration::battle::battle_plugin;
use tactics_exploration::camera::setup_camera;
use tactics_exploration::main_menu::main_menu_plugin;
use tactics_exploration::player::{PlayerInputAction, spawn_coop_players};

fn main() {
    App::new()
        // Add Bevy's default plugins
        .add_plugins(DefaultPlugins)
        // TODO: Dev Mode
        .add_plugins(EguiPlugin::default())
        // .add_plugins(WorldInspectorPlugin::new())
        // Add the bevy_ecs_tiled plugin
        // bevy_ecs_tilemap::TilemapPlugin will be added automatically if needed
        // .add_plugins(TiledDebugPluginGroup)
        .init_state::<GameState>()
        .add_systems(PreStartup, setup_fonts)
        .add_systems(Startup, (setup_camera, spawn_coop_players, boot_game))
        .add_plugins(InputManagerPlugin::<PlayerInputAction>::default())
        .add_plugins(main_menu_plugin)
        .add_plugins(battle_plugin)
        .run();
}

fn boot_game(mut game_state: ResMut<NextState<GameState>>) {
    game_state.set(GameState::MainMenu)
}
