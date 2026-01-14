use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pkv::{PersistentResourceAppExtensions, PkvStore};
use clap::Parser;
use leafwing_input_manager::plugin::InputManagerPlugin;
use tactics_exploration::GameState;
use tactics_exploration::animation::animation_db::load_animation_data;
use tactics_exploration::args::Cli;
use tactics_exploration::assets::setup_fonts;
use tactics_exploration::assets::sounds::{Music, SoundManager, setup_sounds};
use tactics_exploration::assets::sprite_db::build_sprite_db;
use tactics_exploration::battle::{battle_plugin, god_mode_plugin};
use tactics_exploration::camera::setup_camera;
use tactics_exploration::join_game_menu::join_game_plugin;
use tactics_exploration::main_menu::main_menu_plugin;
use tactics_exploration::player::{Player, PlayerBundle, PlayerInputAction};
use tactics_exploration::save_game::SaveFiles;

fn main() {
    let options = Cli::parse();

    let mut app = App::new();
    let mut runner = &mut app;

    runner = runner
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(PkvStore::new("bkdaugherty", "tactics-exploration"))
        .init_persistent_resource::<SaveFiles>()
        .init_state::<GameState>()
        .add_systems(
            Startup,
            (
                setup_camera,
                setup_sounds,
                boot_game,
                setup_fonts,
                load_animation_data,
                build_sprite_db,
                start_music.after(setup_sounds),
            ),
        )
        .add_plugins(InputManagerPlugin::<PlayerInputAction>::default())
        .add_plugins(join_game_plugin)
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

fn start_music(mut commands: Commands, sounds: Res<SoundManager>) {
    sounds.start_music(&mut commands, Music::BattleMusic);
}

fn boot_game(mut commands: Commands, mut game_state: ResMut<NextState<GameState>>) {
    // Spawn the "PrePlayer" only once!
    commands.spawn(PlayerBundle::new(Player::PrePlayer));
    game_state.set(GameState::MainMenu)
}
