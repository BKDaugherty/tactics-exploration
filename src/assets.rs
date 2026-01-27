//! Some constants associated with specific assets.
//! "assets" itself is omitted

pub const CURSOR32_PATH: &str = "utility_assets/cursor.png";
pub const CURSOR_PATH: &str = "utility_assets/cursor-16.png";
pub const OVERLAY32_PATH: &str = "utility_assets/iso_color.png";
pub const OVERLAY_PATH: &str = "utility_assets/iso_color-16.png";
pub const BATTLE_TACTICS_TILESHEET: &str =
    "map_assets/tinytactics-32-map/20240420tinyTacticsTileset00.png";

pub const GRADIENT_PATH: &str = "utility_assets/gradient.png";

use std::path::PathBuf;

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

impl FontResource {
    pub(crate) fn get_all_paths(&self) -> Vec<PathBuf> {
        let FontResource {
            fine_fantasy,
            badge,
            pixelify_sans_regular,
            pixelify_sans_medium,
            pixelify_sans_bold,
            pixelify_sans_semi_bold,
        } = self;
        [
            fine_fantasy,
            badge,
            pixelify_sans_regular,
            pixelify_sans_medium,
            pixelify_sans_bold,
            pixelify_sans_semi_bold,
        ]
        .iter()
        .filter_map(|t| t.path().map(|t| t.path().to_path_buf()))
        .collect()
    }
}

pub const FONT_BADGE_PATH: &str = "font_assets/tinyRPGFontKit01_v1_2/TinyRPG-BadgeFont.ttf";
pub const FONT_FINE_FANTASY_PATH: &str =
    "font_assets/tinyRPGFontKit01_v1_2/TinyRPG-FineFantasyStrategies.ttf";
pub const FONT_PIXELIFY_SANS_REGULAR_PATH: &str =
    "font_assets/pixelify-sans/static/PixelifySans-Regular.ttf";
pub const FONT_PIXELIFY_SANS_BOLD_PATH: &str =
    "font_assets/pixelify-sans/static/PixelifySans-BOLD.ttf";
pub const FONT_PIXELIFY_SANS_MEDIUM_PATH: &str =
    "font_assets/pixelify-sans/static/PixelifySans-Medium.ttf";
pub const FONT_PIXELIFY_SANS_SEMI_BOLD_PATH: &str =
    "font_assets/pixelify-sans/static/PixelifySans-SemiBold.ttf";

pub fn setup_fonts(mut commands: Commands, asset_loader: Res<AssetServer>) {
    let badge = asset_loader.load(FONT_BADGE_PATH);
    let fine_fantasy = asset_loader.load(FONT_FINE_FANTASY_PATH);
    let pixelify_sans_regular = asset_loader.load(FONT_PIXELIFY_SANS_REGULAR_PATH);
    let pixelify_sans_bold = asset_loader.load(FONT_PIXELIFY_SANS_BOLD_PATH);
    let pixelify_sans_medium = asset_loader.load(FONT_PIXELIFY_SANS_MEDIUM_PATH);
    let pixelify_sans_semi_bold = asset_loader.load(FONT_PIXELIFY_SANS_SEMI_BOLD_PATH);
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
    use std::collections::{HashMap, HashSet};

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

    impl TinyTacticsSprites {
        pub fn spritesheet_path(&self) -> &'static str {
            match self {
                Self::TtMapSheet => BATTLE_TACTICS_TILESHEET,
                Self::Fighter => "unit_assets/spritesheets/fighter_spritesheet.png",
                Self::Mage => "unit_assets/spritesheets/mage_spritesheet.png",
                Self::Cleric => "unit_assets/spritesheets/cleric_spritesheet.png",
                Self::IronAxe => "unit_assets/spritesheets/IronAxe_spritesheet.png",
                Self::Scepter => "unit_assets/spritesheets/Scepter_spritesheet.png",
            }
        }
    }

    impl From<TinyTacticsSprites> for SpriteId {
        fn from(value: TinyTacticsSprites) -> Self {
            match value {
                TinyTacticsSprites::TtMapSheet => SpriteId(1),
                TinyTacticsSprites::Fighter => SpriteId(2),
                TinyTacticsSprites::Mage => SpriteId(3),
                TinyTacticsSprites::Cleric => SpriteId(4),
                TinyTacticsSprites::IronAxe => SpriteId(12),
                TinyTacticsSprites::Scepter => SpriteId(13),
            }
        }
    }

    pub(crate) fn build_sprite_map() -> HashMap<SpriteId, String> {
        let data = [
            (
                TinyTacticsSprites::TtMapSheet.into(),
                BATTLE_TACTICS_TILESHEET.to_string(),
            ),
            (
                TinyTacticsSprites::Fighter.into(),
                TinyTacticsSprites::Fighter.spritesheet_path().to_string(),
            ),
            (
                TinyTacticsSprites::Mage.into(),
                TinyTacticsSprites::Mage.spritesheet_path().to_string(),
            ),
            (
                TinyTacticsSprites::Cleric.into(),
                TinyTacticsSprites::Cleric.spritesheet_path().to_string(),
            ),
            (
                TinyTacticsSprites::IronAxe.into(),
                TinyTacticsSprites::IronAxe.spritesheet_path().to_string(),
            ),
            (
                TinyTacticsSprites::Scepter.into(),
                TinyTacticsSprites::Scepter.spritesheet_path().to_string(),
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
        ];
        let len = data.len();
        let map = HashMap::from(data.clone());

        if map.len() != len {
            let mapped = data.map(|t| t.0);
            let mut set = HashSet::new();
            for key in mapped {
                if set.contains(&key) {
                    error!("Key already found in set: {:?}", key);
                } else {
                    set.insert(key);
                }
            }
            panic!("Keys in SpriteMap are not unique");
        }
        map
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
    use std::{collections::HashMap, path::PathBuf};

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
            voice_sounds::BASE_OUCH,
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

    pub mod voice_sounds {
        pub const BASE_OUCH: &str = "sound_assets/voice/base/ouch.ogg";
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
        pub voice_db: HashMap<VoiceId, VoiceProfile>,
    }

    /// Container type (also will include voice, and weapon I guess? Maybe also footstep?)
    #[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
    pub enum CombatSound {
        Skill(SkillSound),
        Voice(VoiceId, VoiceSound),
    }

    #[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
    pub enum VoiceSound {
        Ouch,
        Hiyah,
    }

    #[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
    pub enum VoiceId {
        Base,
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
                combat_sounds: HashMap::from([
                    (
                        CombatSound::Skill(SkillSound::FlameExplosion),
                        asset_server.load(FLAME_EXPLOSION_PATH),
                    ),
                    (
                        CombatSound::Voice(VoiceId::Base, VoiceSound::Ouch),
                        asset_server.load(BASE_OUCH),
                    ),
                ]),
                voice_db: HashMap::from([(VoiceId::Base, default_voice_profile())]),
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

        pub(crate) fn get_all_sound_paths(&self) -> Vec<PathBuf> {
            let mut sounds = Vec::new();

            for source in self.combat_sounds.values() {
                if let Some(path) = source.path() {
                    sounds.push(path.path().to_path_buf());
                }
            }

            for source in self.ui_sounds.values() {
                if let Some(path) = source.path() {
                    sounds.push(path.path().to_path_buf());
                }
            }

            for source in self.music.values() {
                if let Some(path) = source.path() {
                    sounds.push(path.path().to_path_buf());
                }
            }

            sounds
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
        pub manager: Res<'w, SoundManager>,
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
        Healed,
    }

    #[derive(Debug, Clone)]
    pub struct VoiceProfile {
        responses: Vec<VoiceResponse>,
    }

    fn default_voice_profile() -> VoiceProfile {
        VoiceProfile {
            responses: vec![VoiceResponse {
                cue: AudioCue::Hit,
                role: Some(ImpactInteractionRole::Defender),
                sound: VoiceSound::Ouch,
            }],
        }
    }

    #[derive(Debug, Clone)]
    pub struct VoiceResponse {
        cue: AudioCue,
        role: Option<ImpactInteractionRole>,
        // Later we could add a Vec<VoiceSound> and let u choose randomly
        // maybe with some priority set.
        sound: VoiceSound,
    }

    impl VoiceProfile {
        /// Figure out what VoiceSounds should be played based on the AudioEventMessage
        /// for this Profile.
        pub fn resolve(
            &self,
            audio_event: &AudioEventMessage,
            interaction_role: Option<ImpactInteractionRole>,
        ) -> Vec<VoiceSound> {
            let mut options = Vec::new();
            info!(
                "VoiceProfile::resolve: {:?} {:?}",
                audio_event, interaction_role
            );
            for response in &self.responses {
                if audio_event.cue != response.cue {
                    continue;
                }

                if let Some(required_role) = response.role
                    && !interaction_role
                        .map(|t| t == required_role)
                        .unwrap_or_default()
                {
                    continue;
                }

                options.push(response.sound)
            }
            options
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_resolve() {
            let profile = default_voice_profile();
            let lines = profile.resolve(
                &super::AudioEventMessage {
                    source: Entity::from_bits(1),
                    cue: AudioCue::Hit,
                    audio_context: AudioContext { skill_id: None },
                },
                Some(ImpactInteractionRole::Defender),
            );

            assert_eq!(lines, vec![VoiceSound::Ouch]);
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Hash)]
    pub enum ImpactInteractionRole {
        Caster,
        Defender,
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
    #[derive(Message, Debug)]
    pub struct AudioEventMessage {
        pub source: Entity,
        pub cue: AudioCue,
        pub audio_context: AudioContext,
    }
}

pub mod sound_resolvers {
    use bevy::prelude::*;

    use crate::{
        assets::sounds::{AudioEventMessage, SoundManagerParam, VoiceId},
        combat::{AttackExecution, skills::SkillDBResource},
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

    #[derive(Component, Debug)]
    pub struct Voice {
        pub voice_id: VoiceId,
    }

    pub fn resolve_voice_audio_events(
        mut commands: Commands,
        mut messages: MessageReader<AudioEventMessage>,
        attack_execution: Query<&AttackExecution>,
        unit_query: Query<&Voice>,
        sound_manager: SoundManagerParam,
    ) {
        for message in messages.read() {
            let attacker_voice = attack_execution
                .get(message.source)
                .ok()
                .and_then(|t| t.attacker)
                .and_then(|t| unit_query.get(t).ok());

            let defender_voice = attack_execution
                .get(message.source)
                .ok()
                .map(|t| t.defender)
                .and_then(|t| unit_query.get(t).ok());

            info!(
                "Resolving Voice Audio Event: {:?}, {:?}, {:?}",
                attacker_voice, defender_voice, message
            );

            // TODO: Make me a fn
            if let Some(voice) = attacker_voice {
                let Some(profile) = sound_manager.manager.voice_db.get(&voice.voice_id) else {
                    error!("No Profile found for Voice {:?}", voice.voice_id);
                    continue;
                };

                let lines =
                    profile.resolve(message, Some(super::sounds::ImpactInteractionRole::Caster));

                info!(
                    "Resolved Sounds with voice: {:?}, {:?}",
                    voice.voice_id, lines
                );
                for line in lines {
                    sound_manager.play_combat_sound(
                        &mut commands,
                        super::sounds::CombatSound::Voice(voice.voice_id, line),
                    );
                }
            };

            if let Some(voice) = defender_voice {
                let Some(profile) = sound_manager.manager.voice_db.get(&voice.voice_id) else {
                    error!("No Profile found for Voice {:?}", voice.voice_id);
                    continue;
                };

                let lines = profile.resolve(
                    message,
                    Some(super::sounds::ImpactInteractionRole::Defender),
                );

                info!(
                    "Resolved Sounds with voice: {:?}, {:?}",
                    voice.voice_id, lines
                );

                for line in lines {
                    sound_manager.play_combat_sound(
                        &mut commands,
                        super::sounds::CombatSound::Voice(voice.voice_id, line),
                    );
                }
            };
        }
    }
}

/// I keep a fair bit of assets in the assets directory,
/// but not all of them are actually used for the game. This
/// hardcodes a list of "used" assets that will be packaged into the
/// assets directory for a published build of the game.
pub mod active_assets {
    use std::{path::PathBuf, str::FromStr};

    use anyhow::Context;
    use bevy::prelude::*;

    use crate::assets::{
        BATTLE_TACTICS_TILESHEET, CURSOR_PATH, FontResource, GRADIENT_PATH, OVERLAY_PATH,
        setup_fonts,
        sounds::{SoundManager, setup_sounds},
        sprite_db::build_sprite_map,
    };

    pub const MISC_USED_ASSET_PATHS: &[&str] = &[
        CURSOR_PATH,
        OVERLAY_PATH,
        BATTLE_TACTICS_TILESHEET,
        GRADIENT_PATH,
    ];

    pub fn get_used_asset_paths() -> anyhow::Result<Vec<PathBuf>> {
        let mut app = App::new();

        // TODO: It's a little jank that I need to run a bevy app to
        // get all of the "used" assets, but it's also helpful that I
        // don't need to change any of the existing loading functions.
        //
        // Let's think about restructuring these loaders to not need
        // to pull in an AssetServer to specify the paths they actively
        // depend on.
        app.add_systems(Startup, (setup_fonts, setup_sounds));
        app.add_plugins(DefaultPlugins);
        app.world_mut().run_schedule(Startup);

        let w = app.world();

        let font_resource = w
            .get_resource::<FontResource>()
            .context("We just setup the FontResource, and we need it to know what fonts exist")?;
        let sound_manager = w
            .get_resource::<SoundManager>()
            .context("We just setup the SoundManager, and we need it to know what sounds exist")?;

        let mut used = Vec::new();

        used.extend(sound_manager.get_all_sound_paths());
        used.extend(font_resource.get_all_paths());

        for path in MISC_USED_ASSET_PATHS {
            used.push(
                PathBuf::from_str(path).context("Creating pathbuf from misc_used_asset_paths")?,
            );
        }

        for (_, sprite_path) in build_sprite_map() {
            used.push(PathBuf::from(sprite_path));
        }

        Ok(used)
    }
}
