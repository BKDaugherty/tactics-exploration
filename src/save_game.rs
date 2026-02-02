use bevy::prelude::*;

use crate::unit::jobs::UnitJob;

#[derive(
    Debug, serde::Serialize, serde::Deserialize, Reflect, Clone, PartialEq, Eq, Hash, Component,
)]
pub struct SaveFileKey {
    pub uid: u32,
    pub name: String,
    pub color: SaveFileColor,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Resource, Default, Reflect)]
pub struct SaveFiles {
    pub save_file_keys: Vec<SaveFileKey>,
    pub cursor: u32,
}

impl SaveFileKey {
    pub fn pkv_key(&self) -> String {
        format!("unit-save-{}", self.uid)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Reflect, Clone)]
pub struct UnitSaveV1 {
    pub save_file_key: SaveFileKey,
    pub job: UnitJob,
}

impl From<UnitSaveV1> for UnitSave {
    fn from(value: UnitSaveV1) -> Self {
        UnitSave::V1(value)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Reflect, Clone)]
#[serde(tag = "version")]
pub enum UnitSave {
    V1(UnitSaveV1),
}

pub fn upgrade_save_file_to_latest(save_file: UnitSave) -> anyhow::Result<UnitSaveV1> {
    let UnitSave::V1(v1) = save_file;
    Ok(v1)
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Reflect, PartialEq, Eq, Hash)]
pub enum SaveFileColor {
    Blue,
    Green,
    Red,
}

impl SaveFileColor {
    pub fn name(&self) -> String {
        match self {
            SaveFileColor::Blue => "Blue".to_string(),
            SaveFileColor::Green => "Green".to_string(),
            SaveFileColor::Red => "Red".to_string(),
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SaveFileColor::Blue => Color::linear_rgb(0.0, 0.0, 0.7),
            SaveFileColor::Green => Color::linear_rgb(0.0, 0.7, 0.0),
            SaveFileColor::Red => Color::linear_rgb(0.7, 0.0, 0.0),
        }
    }
}
