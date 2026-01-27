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

Add something like this to `~/.ssh/config`
```
Host deck
  HostName <RUN ip addr on deck to get your ip>
  User deck
  Port 22
  IdentityFile ~/.ssh/<YOUR_IDENTITY_FILE>
```
Then you can just scp contents over

## Asset Attributions!

I'm using a fair bit of community assets here and there. This section is dedicated to attributions for those! Thank you all so much! It'd be so much worse making a game without all of these!

### Sprites

I'm using a collection of different sprites.

For tiles and some characters for now, I'm using the TinyTactics Battle Kit [here](https://tiopalada.itch.io/tiny-tactics-battle-kit-i) by Tiopalada. See the license [here](assets/unit_assets/tinytactics_battlekiti_v1_0/license.html)

#### VFX
- Fire Effect Explosion I got [here](https://pimen.itch.io/fire-spell-effect-02)
- Acid Effect I got [here](https://pimen.itch.io/acid-spell-effect)

### Fonts

In a few places, I'm using the tinyRPGFontKit by Tiopalada. Check out the license [here](assets/font_assets/tinyRPGFontKit01_v1_2/license.html) and the pack [here](https://tiopalada.itch.io/tiny-rpg-font-kit-i).

I'm also using the pixelify_sans font which is licensed [here](ssets/font_assets/pixelify-sans/OFL.txt)

### Sound Assets

#### SFX
I'm using JD Sherberts Pixel UI Sfx Pack! You can see the license [here](assets/sound_assets/jdsherbert-pixel-ui-sfx-pack-free/LICENSE-1.pdf)


I'm using some sfx sounds from Leohpaz! Check it out [here](https://leohpaz.itch.io/rpg-essentials-sfx-free)

#### Battle Music

I recorded this actually!