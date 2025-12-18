use image::{ImageBuffer, Rgba};
use std::{collections::BTreeMap, path::PathBuf, str::FromStr};

pub const FRAME_SIZE_X: u32 = 32;
pub const FRAME_SIZE_Y: u32 = 32;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Ord, PartialOrd)]
enum Character {
    Fighter,
    Mage,
    Cleric,
}

impl std::fmt::Display for Character {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Character::Fighter => write!(f, "fighter"),
            Character::Mage => write!(f, "mage"),
            Character::Cleric => write!(f, "cleric"),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Ord, PartialOrd)]
enum Action {
    Walking,
    Attack,
    Release,
    Charging,
    Damage,
    Weak,
    Dead,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Attack => write!(f, "attack"),
            Action::Charging => write!(f, "charging"),
            Action::Damage => write!(f, "damage"),
            Action::Dead => write!(f, "dead"),
            Action::Release => write!(f, "release"),
            Action::Walking => write!(f, "walking"),
            Action::Weak => write!(f, "weak"),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Direction {
    NE,
    NW,
    SE,
    SW,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::SE => write!(f, "SE"),
            Direction::NE => write!(f, "NE"),
            Direction::NW => write!(f, "NW"),
            Direction::SW => write!(f, "SW"),
        }
    }
}

pub const FILE_PREFIX: &str = "assets/unit_assets/tinytactics_battlekiti_v1_0/";
pub const DATE_MADE: &str = "20240427";

fn sprite_filename(character: Character, action: Action, dir: Direction) -> PathBuf {
    PathBuf::from_str(&format!(
        "{FILE_PREFIX}{DATE_MADE}{}-{}{}.png",
        character.to_string(),
        action.to_string(),
        dir.to_string()
    ))
    .expect("Should be valid path")
}

// Quick example of concatenating images for spritesheet
fn main() -> Result<(), image::ImageError> {
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
        .sum::<u32>()
        * 2;

    for character in [Character::Cleric, Character::Fighter, Character::Mage] {
        let keys_of_character: Vec<&(Character, Action, Direction)> = image_data
            .keys()
            .filter(|(c, _, _)| *c == character)
            .collect();
        let mut output_img =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::new(new_image_width, new_image_height as u32);

        // TODO: Need to store indices, for the annoying images that decided to be two rows lol
        let mut height = 0;
        for (c, d, a) in keys_of_character.into_iter().cloned() {
            let image: &ImageBuffer<Rgba<u8>, Vec<u8>> =
                image_data.get(&(c, d, a)).expect("Must have image");
            image::imageops::replace(&mut output_img, image, 0, height.into());

            height += image.height();

            let flipped = image::imageops::flip_horizontal(image);
            image::imageops::replace(&mut output_img, &flipped, 0, height.into());

            height += image.height();
        }
        output_img.save(format!(
            "assets/unit_assets/spritesheets/{}_spritesheet.png",
            character
        ))?;
    }
    Ok(())
}
