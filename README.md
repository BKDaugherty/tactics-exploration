# Tactics Exploration

Hey! My favorite game of all time is Final Fantasy Tactics. I've always wanted to build a multiplayer version, and I don't have a job right now so why not!

## Setup

### Assets

This project uses `git-lfs` to track assets. You'll likely need to install `git-lfs` to get started.

### Building the project

This project is written in Rust! Use [rustup](https://rustup.rs/) to setup Rust.

From the root of the project, run `cargo build` which will take a fair bit of time on the first go, as it needs to compile all of bevy. 

After that, you should be able to run `cargo run` to run the project with a fair bit of speed.

### Testing the Project

`cargo test` is your friend!

### Building for "other targets"

#### WebAssembly

Check out this guide: https://bevy-cheatbook.github.io/platforms/wasm.html

One time installation:
```
rustup target install wasm32-unknown-unknown
cargo install wasm-server-runner
```

Then run `cargo web` which is an alias configured in `.cargo/config.toml`

#### Steam Deck

#### Building

Use this: https://github.com/paul-hansen/bevy_steamos_docker?tab=readme-ov-file
Used docker cli version 29.1.3

#### Steam Deck Configuration

Had to enable sshd via systemd directly on the steam deck since I
couldn't get the steamoskit thing to work.

Once you have that, it's easy to copy the release files directly to a
folder, create a desktop shortcut, and then from there you can add the
"non-steam" content so that you can see the game in playmode.
