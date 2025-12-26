use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::Parser;
use leafwing_input_manager::plugin::InputManagerPlugin;
use tactics_exploration::GameState;
use tactics_exploration::args::Cli;
use tactics_exploration::assets::setup_fonts;
use tactics_exploration::battle::{battle_plugin, god_mode_plugin};
use tactics_exploration::camera::setup_camera;
use tactics_exploration::main_menu::main_menu_plugin;
use tactics_exploration::player::{PlayerInputAction, spawn_coop_players};

fn main() {
    let options = Cli::parse();

    let mut app = App::new();
    let mut runner = &mut app;

    runner = runner
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_systems(
            Startup,
            (setup_camera, spawn_coop_players, boot_game, setup_fonts),
        )
        .add_plugins(InputManagerPlugin::<PlayerInputAction>::default())
        .add_plugins(main_menu_plugin)
        .add_plugins(battle_plugin);

    // TODO: I could probably compile this out for the real game?
    if options.god_mode {
        runner = runner
            .add_plugins(god_mode_plugin)
            .add_plugins(EguiPlugin::default())
            .add_plugins(WorldInspectorPlugin::new());
    }

    runner.run();
}

fn boot_game(mut game_state: ResMut<NextState<GameState>>) {
    game_state.set(GameState::MainMenu)
}
