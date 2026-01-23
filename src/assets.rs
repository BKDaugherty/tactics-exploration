//! Some constants associated with specific assets.
//! "assets" itself is omitted

pub const CURSOR32_PATH: &str = "utility_assets/cursor.png";
pub const CURSOR_PATH: &str = "utility_assets/cursor-16.png";
pub const OVERLAY32_PATH: &str = "utility_assets/iso_color.png";
pub const OVERLAY_PATH: &str = "utility_assets/iso_color-16.png";
pub const BATTLE_TACTICS_TILESHEET: &str =
    "map_assets/tinytactics-32-map/20240420tinyTacticsTileset00.png";

pub const GRADIENT_PATH: &str = "utility_assets/gradient.png";

pub const EXAMPLE_MAP_PATH: &str = "map_assets/example-map/example-map.tmx";
pub const EXAMPLE_MAP_2_PATH: &str = "map_assets/tinytactics-32-map/example-map-tiny-tactics.tmx";

use bevy::prelude::*;

#[derive(Resource)]
pub struct FontResource {
    pub fine_fantasy: Handle<Font>,
    pub badge: Handle<Font>,
    // Bevy doesn't support variable fonts
    // https://github.com/bevyengine/bevy/issues/19854
    pub pixelify_sans_regular: Handle<Font>,
    pub pixelify_sans_medium: Handle<Font>,
    pub pixelify_sans_bold: Handle<Font>,
    pub pixelify_sans_semi_bold: Handle<Font>,
}

pub fn setup_fonts(mut commands: Commands, asset_loader: Res<AssetServer>) {
    let badge = asset_loader.load("font_assets/tinyRPGFontKit01_v1_2/TinyRPG-BadgeFont.ttf");
    let fine_fantasy =
        asset_loader.load("font_assets/tinyRPGFontKit01_v1_2/TinyRPG-FineFantasyStrategies.ttf");
    let pixelify_sans_regular =
        asset_loader.load("font_assets/pixelify-sans/static/PixelifySans-Regular.ttf");
    let pixelify_sans_bold =
        asset_loader.load("font_assets/pixelify-sans/static/PixelifySans-Bold.ttf");
    let pixelify_sans_medium =
        asset_loader.load("font_assets/pixelify-sans/static/PixelifySans-Medium.ttf");
    let pixelify_sans_semi_bold =
        asset_loader.load("font_assets/pixelify-sans/static/PixelifySans-SemiBold.ttf");
    commands.insert_resource(FontResource {
        fine_fantasy,
        badge,
        pixelify_sans_regular,
        pixelify_sans_medium,
        pixelify_sans_bold,
        pixelify_sans_semi_bold,
    });
}

/// Skills need to be able to reference in data format
/// what asset they spawn for VFX. For now, this can be tracked in a "DB".
pub mod sprite_db {
    use std::collections::HashMap;

    use crate::unit::jobs::UnitJob;

    use super::*;

    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Reflect,
    )]
    pub struct SpriteId(pub u32);

    #[derive(Debug, Resource)]
    pub struct SpriteDB {
        pub sprite_id_to_handle: HashMap<SpriteId, Handle<Image>>,
    }

    impl SpriteDB {
        fn new() -> Self {
            Self {
                sprite_id_to_handle: HashMap::new(),
            }
        }
    }

    /// Utility enum to track the TinyTactics specific sprites for now while
    /// they are still in use.
    #[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, Hash)]
    pub enum TinyTacticsSprites {
        TtMapSheet,
        Fighter,
        Mage,
        Cleric,
        IronAxe,
        Scepter,
    }

    impl From<TinyTacticsSprites> for SpriteId {
        fn from(value: TinyTacticsSprites) -> Self {
            match value {
                TinyTacticsSprites::TtMapSheet => SpriteId(1),
                TinyTacticsSprites::Fighter => SpriteId(2),
                TinyTacticsSprites::Mage => SpriteId(3),
                TinyTacticsSprites::Cleric => SpriteId(4),
                TinyTacticsSprites::IronAxe => SpriteId(7),
                TinyTacticsSprites::Scepter => SpriteId(8),
            }
        }
    }

    fn build_sprite_map() -> HashMap<SpriteId, String> {
        HashMap::from([
            (
                TinyTacticsSprites::TtMapSheet.into(),
                BATTLE_TACTICS_TILESHEET.to_string(),
            ),
            (
                TinyTacticsSprites::Fighter.into(),
                "unit_assets/spritesheets/fighter_spritesheet.png".to_string(),
            ),
            (
                TinyTacticsSprites::Mage.into(),
                "unit_assets/spritesheets/mage_spritesheet.png".to_string(),
            ),
            (
                TinyTacticsSprites::Cleric.into(),
                "unit_assets/spritesheets/cleric_spritesheet.png".to_string(),
            ),
            (
                TinyTacticsSprites::IronAxe.into(),
                "unit_assets/spritesheets/IronAxe_spritesheet.png".to_string(),
            ),
            (
                TinyTacticsSprites::Scepter.into(),
                "unit_assets/spritesheets/Scepter_spritesheet.png".to_string(),
            ),
            (
                SpriteId(5),
                "misc_assets/fire_effect_2/explosion_2_spritesheet.png".to_string(),
            ),
            (SpriteId(6), "misc_assets/arrow.png".to_string()),
            (
                SpriteId(7),
                "misc_assets/acid-vfx/acid-vfx-1.png".to_string(),
            ),
            (
                UnitJob::Knight.demo_sprite_id(),
                "unit_assets/spritesheets/cgcarter/knight.png".to_string(),
            ),
            (
                UnitJob::Mage.demo_sprite_id(),
                "unit_assets/spritesheets/cgcarter/mage.png".to_string(),
            ),
            (
                UnitJob::Archer.demo_sprite_id(),
                "unit_assets/spritesheets/cgcarter/archer.png".to_string(),
            ),
            (
                UnitJob::Mercenary.demo_sprite_id(),
                "unit_assets/spritesheets/cgcarter/mercenary.png".to_string(),
            ),
        ])
    }

    pub fn build_sprite_db(mut commands: Commands, asset_server: Res<AssetServer>) {
        let map = build_sprite_map();
        let mut db = SpriteDB::new();
        for (id, path) in map {
            let handle = asset_server.load(path);
            db.sprite_id_to_handle.insert(id, handle);
        }

        commands.insert_resource(db);
    }
}

pub mod sounds {
    use std::collections::HashMap;

    use anyhow::Context;
    use bevy::{audio::Volume, ecs::system::SystemParam, prelude::*};

    use crate::{
        assets::sounds::{
            jdsherbert_pixel_ui_sfx::{
                CANCEL_SOUND_PATH, CLOSE_MENU_PATH, ERROR_SOUND_PATH, MOVE_CURSOR_SOUND_PATH,
                OPEN_MENU_PATH, SELECT_SOUND_PATH,
            },
            music::BATTLE_MUSIC_PATH,
            rpg_essentials::FLAME_EXPLOSION_PATH,
        },
        combat::skills::SkillId,
    };

    /// JD Sherbert holding down the fort on these ui sounds
    pub mod jdsherbert_pixel_ui_sfx {
        pub const OPEN_MENU_PATH: &str = "sound_assets/jdsherbert-pixel-ui-sfx-pack-free/Stereo/ogg/JDSherbert - Pixel UI SFX Pack - Popup Open 1 (Sine).ogg";
        pub const CLOSE_MENU_PATH: &str = "sound_assets/jdsherbert-pixel-ui-sfx-pack-free/Stereo/ogg/JDSherbert - Pixel UI SFX Pack - Popup Close 1 (Sine).ogg";
        pub const SELECT_SOUND_PATH: &str = "sound_assets/jdsherbert-pixel-ui-sfx-pack-free/Stereo/ogg/JDSherbert - Pixel UI SFX Pack - Select 2 (Sine).ogg";
        pub const CANCEL_SOUND_PATH: &str = CLOSE_MENU_PATH;
        pub const ERROR_SOUND_PATH: &str = "sound_assets/jdsherbert-pixel-ui-sfx-pack-free/Stereo/ogg/JDSherbert - Pixel UI SFX Pack - Error 1 (Sine).ogg";
        pub const MOVE_CURSOR_SOUND_PATH: &str = "sound_assets/jdsherbert-pixel-ui-sfx-pack-free/Stereo/ogg/JDSherbert - Pixel UI SFX Pack - Cursor 2 (Sine).ogg";
    }

    pub mod rpg_essentials {
        pub const FLAME_EXPLOSION_PATH: &str =
            "sound_assets/rpg_essentials/04_Fire_explosion_04_medium.ogg";
    }

    pub mod music {
        pub const BATTLE_MUSIC_PATH: &str = "sound_assets/music/BattleMusic.ogg";
    }

    #[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
    pub enum Music {
        BattleMusic,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
    pub enum UiSound {
        OpenMenu,
        CloseMenu,
        Select,
        Cancel,
        Error,
        MoveCursor,
    }

    #[derive(Resource, Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct SoundSettings {
        pub global_volume: f64,
        pub sfx_volume: f64,
        pub music_volume: f64,
    }

    impl Default for SoundSettings {
        fn default() -> Self {
            Self {
                global_volume: 1.0,
                sfx_volume: 1.0,
                music_volume: 1.0,
            }
        }
    }

    /// TBD if this should be an enum or just an id
    #[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
    pub enum SkillSound {
        FlameExplosion,
    }

    #[derive(Resource)]
    /// Resource holding all of our UI SFX
    pub struct SoundManager {
        ui_sounds: HashMap<UiSound, Handle<AudioSource>>,
        music: HashMap<Music, Handle<AudioSource>>,
        combat_sounds: HashMap<CombatSound, Handle<AudioSource>>,
    }

    /// Container type (also will include voice, and weapon I guess? Maybe also footstep?)
    #[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
    pub enum CombatSound {
        Skill(SkillSound),
    }

    #[derive(Component)]
    pub struct BackgroundMusicPlayer;

    impl SoundManager {
        pub fn initialize(asset_server: &AssetServer) -> Self {
            Self {
                ui_sounds: HashMap::from([
                    (UiSound::OpenMenu, asset_server.load(OPEN_MENU_PATH)),
                    (UiSound::CloseMenu, asset_server.load(CLOSE_MENU_PATH)),
                    (UiSound::Select, asset_server.load(SELECT_SOUND_PATH)),
                    (UiSound::Cancel, asset_server.load(CANCEL_SOUND_PATH)),
                    (UiSound::Error, asset_server.load(ERROR_SOUND_PATH)),
                    (
                        UiSound::MoveCursor,
                        asset_server.load(MOVE_CURSOR_SOUND_PATH),
                    ),
                ]),
                music: HashMap::from([(Music::BattleMusic, asset_server.load(BATTLE_MUSIC_PATH))]),
                combat_sounds: HashMap::from([(
                    CombatSound::Skill(SkillSound::FlameExplosion),
                    asset_server.load(FLAME_EXPLOSION_PATH),
                )]),
            }
        }

        /// Back at it again with they dynamic, but static set of resources
        ///
        /// Same choice here.
        pub fn get_ui_sound(&self, sound: UiSound) -> Handle<AudioSource> {
            self.ui_sounds
                .get(&sound)
                .cloned()
                .with_context(|| {
                    format!(
                        "Why do we have an enum for a sound if it doesn't exist: {:?}",
                        sound
                    )
                })
                .unwrap()
        }

        pub fn get_music(&self, music: Music) -> Handle<AudioSource> {
            self.music
                .get(&music)
                .cloned()
                .with_context(|| {
                    format!(
                        "Why do we have an enum for music if it doesn't exist: {:?}",
                        music
                    )
                })
                .unwrap()
        }

        pub fn play_ui_sound(
            &self,
            commands: &mut Commands,
            settings: &SoundSettings,
            sound: UiSound,
        ) {
            commands.spawn((
                AudioPlayer::new(self.get_ui_sound(sound)),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(
                    (settings.global_volume * settings.sfx_volume) as f32,
                )),
            ));
        }

        pub fn get_combat_sound(&self, sound: CombatSound) -> Handle<AudioSource> {
            self.combat_sounds.get(&sound).expect("Why have an enum for Combat sounds if you aren't going to have a sound for an enum discriminant").clone()
        }

        pub fn play_combat_sound(
            &self,
            commands: &mut Commands,
            settings: &SoundSettings,
            sound: CombatSound,
        ) {
            commands.spawn((
                AudioPlayer::new(self.get_combat_sound(sound)),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(
                    (settings.global_volume * settings.sfx_volume) as f32,
                )),
            ));
        }

        pub fn start_music(
            &self,
            commands: &mut Commands,
            sound_settings: &SoundSettings,
            music: Music,
        ) {
            commands.spawn((
                AudioPlayer::new(self.get_music(music)),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(
                    (sound_settings.global_volume * sound_settings.music_volume) as f32,
                )),
                BackgroundMusicPlayer,
            ));
        }
    }

    pub fn setup_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.insert_resource(SoundManager::initialize(&asset_server));
    }

    pub fn apply_volume_settings(
        sound_settings: Res<SoundSettings>,
        mut global_volume: ResMut<GlobalVolume>,
        mut audio_query: Query<&mut AudioSink, With<BackgroundMusicPlayer>>,
    ) {
        global_volume.volume = Volume::Linear(sound_settings.global_volume as f32);

        for mut sink in audio_query.iter_mut() {
            sink.set_volume(Volume::Linear(
                (sound_settings.global_volume * sound_settings.music_volume) as f32,
            ));
        }
    }

    #[derive(SystemParam)]
    pub struct SoundManagerParam<'w> {
        settings: Res<'w, SoundSettings>,
        manager: Res<'w, SoundManager>,
    }

    impl<'s> SoundManagerParam<'s> {
        pub fn play_ui_sound(&self, commands: &mut Commands, sound: UiSound) {
            self.manager.play_ui_sound(commands, &self.settings, sound);
        }

        pub fn play_combat_sound(&self, commands: &mut Commands, sound: CombatSound) {
            self.manager
                .play_combat_sound(commands, &self.settings, sound);
        }
    }

    /// AudioCues allow us to generalize a bit for different
    /// weapons / units / skills different semantic moments in our audio pipeline.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum AudioCue {
        /// Started use of skill
        SkillStart,
        SkillRelease,
        /// Unconditional on Impact sound
        Impact,

        /// Emitted by the ImpactEvent system
        Miss,
        Hit,
    }

    /// Context associated with the AudioEvent that was emitted.
    ///
    /// AudioResolvers use this to know if they need to play an event or not.
    #[derive(Clone, PartialEq, Debug)]
    pub struct AudioContext {
        pub skill_id: Option<SkillId>,
    }

    /// So the skill would have this profile registered for audio
    /// tailored to specific skill events
    #[derive(Debug, Default, Clone)]
    pub struct AudioProfile {
        /// TODO: Is AudioCue enough? We can come back later, I don't think it's
        /// going to be. I think this is going to have to not be generic maybe?
        pub on_cue: HashMap<AudioCue, Vec<CombatSound>>,
    }

    /// Sent out to trigger the Audio Resolvers to act
    #[derive(Message)]
    pub struct AudioEventMessage {
        pub source: Entity,
        pub cue: AudioCue,
        pub audio_context: AudioContext,
    }
}

pub mod sound_resolvers {
    use bevy::prelude::*;

    use crate::{
        assets::sounds::{AudioEventMessage, SoundManagerParam},
        combat::skills::SkillDBResource,
    };

    /// Accepts AudioEventMessages with context and plays skill specific sounds
    pub fn resolve_skill_audio_events(
        mut commands: Commands,
        mut messages: MessageReader<AudioEventMessage>,
        skill_db: Res<SkillDBResource>,
        sound_manager: SoundManagerParam,
    ) {
        for message in messages.read() {
            let Some(skill) = message
                .audio_context
                .skill_id
                .as_ref()
                .map(|t| skill_db.skill_db.get_skill(t))
            else {
                continue;
            };

            let Some(sounds) = skill.audio_profile.on_cue.get(&message.cue) else {
                continue;
            };

            for sound in sounds {
                sound_manager.play_combat_sound(&mut commands, *sound);
            }
        }
    }
}
