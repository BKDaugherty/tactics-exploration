use image::{ImageBuffer, Rgba};
use std::{collections::BTreeMap, fs::OpenOptions, path::Path};

use tactics_exploration::animation::tinytactics::*;

pub fn generate_animations_for_weapons() -> anyhow::Result<()> {
    let mut image_data = BTreeMap::new();
    for weapon_type in WeaponType::variants() {
        for direction in [Direction::NE, Direction::SE] {
            let image_path = weapon_attack_sprite_filename(weapon_type, direction);
            let img_buffer = image::open(image_path)?.to_rgba8();
            image_data.insert((weapon_type, direction), img_buffer);
        }
    }

    let new_image_width = image_data
        .values()
        .map(|t| t.width())
        .max()
        .expect("Loaded images should have width");

    // Assume they all have the same height
    let new_image_height: u32 = image_data
        .iter()
        .filter(|((w, _), _)| *w == WeaponType::IronAxe)
        .map(|(_, v)| v.height())
        .sum::<u32>();

    for weapon in WeaponType::variants() {
        let keys_of_weapon: Vec<&(WeaponType, Direction)> =
            image_data.keys().filter(|(w, _)| *w == weapon).collect();

        let mut output_img =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::new(new_image_width, new_image_height);
        let mut height = 0;

        for (w, d) in keys_of_weapon.into_iter().cloned() {
            let image: &ImageBuffer<Rgba<u8>, Vec<u8>> =
                image_data.get(&(w, d)).expect("Must have image");
            image::imageops::replace(&mut output_img, image, 0, height.into());
            height += image.height();
        }
        output_img.save(Path::new("assets").join(weapon_spritesheet_path(weapon)))?;
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut image_data = BTreeMap::new();
    for character in [Character::Cleric, Character::Fighter, Character::Mage] {
        for direction in [Direction::NE, Direction::SE] {
            for action in [
                Action::Walking,
                Action::Attack,
                Action::Charging,
                Action::Damage,
                Action::Dead,
                Action::Release,
                Action::Weak,
            ] {
                let image_path = sprite_filename(character, action, direction);
                let img_buffer = image::open(image_path)?.to_rgba8();
                image_data.insert((character, action, direction), img_buffer);
            }
        }
    }

    let new_image_width = image_data
        .values()
        .map(|t| t.width())
        .max()
        .expect("Loaded images should have width");

    // Assume they all have the same height
    let new_image_height: u32 = image_data
        .iter()
        .filter(|((c, _, _), _)| *c == Character::Mage)
        .map(|(_, v)| v.height())
        .sum::<u32>();

    for character in [Character::Cleric, Character::Fighter, Character::Mage] {
        let keys_of_character: Vec<&(Character, Action, Direction)> = image_data
            .keys()
            .filter(|(c, _, _)| *c == character)
            .collect();
        let mut output_img =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::new(new_image_width, new_image_height);

        let mut animation_data = Vec::new();
        let mut height = 0;
        for (c, action, direction) in keys_of_character.into_iter().cloned() {
            let image: &ImageBuffer<Rgba<u8>, Vec<u8>> = image_data
                .get(&(c, action, direction))
                .expect("Must have image");
            image::imageops::replace(&mut output_img, image, 0, height.into());

            animation_data.push(calculate_animation_data(action, direction, height, image));

            height += image.height();
        }
        let animation_data_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(Path::new("assets").join(spritesheet_data_path(character)))?;

        serde_json::to_writer(
            animation_data_file,
            &AnimationAsset {
                character,
                data: animation_data,
            },
        )?;

        output_img.save(Path::new("assets").join(spritesheet_path(character)))?;
    }
    generate_animations_for_weapons()?;

    Ok(())
}
