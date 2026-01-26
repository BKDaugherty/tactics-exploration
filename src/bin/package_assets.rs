use std::{fs, path::Path};

use anyhow::Context;
use tactics_exploration::assets::active_assets::get_used_asset_paths;

fn main() -> anyhow::Result<()> {
    let source = Path::new("assets/");
    let to = Path::new("out/assets");

    eprintln!("Removing {:?}", to);
    if let Err(e) = fs::remove_dir_all(to) {
        eprintln!("Failed to remove {:?}: {:?}", to, e);
    };

    for path in get_used_asset_paths()? {
        let original = source.join(&path);
        let destination = to.join(&path);

        std::fs::create_dir_all(
            destination
                .parent()
                .context("Given asset doesn't have a parent?")?,
        )?;

        eprintln!("Copying {:?} to {:?}", original, destination);
        std::fs::copy(original, destination)?;
    }

    Ok(())
}
