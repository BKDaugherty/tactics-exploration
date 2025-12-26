use clap::Parser;

/// Tactics Exploration is a Bevy Game!
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Whether to enable God Mode or not during battle.
    ///
    /// Also enables the Inspector
    #[arg(long, env = "TACTICS_EXPLORATION_GOD_MODE")]
    pub god_mode: bool,
}
