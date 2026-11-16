//! Sound effects. Game and interface code send `Sfx` events; one system plays them when sound is on.
//!
//! Sound can be switched off three ways: the "Sound" button or the V key in game (remembered
//! between sessions), or building without the `audio` feature, which leaves out audio entirely.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use bevy_civ::save::{read_setting, write_setting};
use bevy_civ::unit::Domain;

#[derive(Event, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sfx {
    /// An interface button.
    Click,
    /// A unit or city picked on the map, opening its report.
    Select,
    /// Troops ordered to move.
    March,
    /// A ship ordered to move.
    Sail,
    /// An aircraft ordered to move.
    Fly,
    /// A fight on the map.
    Battle,
}

impl Sfx {
    /// The movement sound for a kind of unit.
    pub fn moving(domain: Domain) -> Self {
        match domain {
            Domain::Land => Sfx::March,
            Domain::Sea => Sfx::Sail,
            Domain::Air => Sfx::Fly,
        }
    }

    #[cfg(feature = "audio")]
    const ALL: [Sfx; 6] = [Sfx::Click, Sfx::Select, Sfx::March, Sfx::Sail, Sfx::Fly, Sfx::Battle];

    #[cfg(feature = "audio")]
    fn file(self) -> &'static str {
        match self {
            Sfx::Click => "sounds/click.wav",
            Sfx::Select => "sounds/select.wav",
            Sfx::March => "sounds/march.wav",
            Sfx::Sail => "sounds/sail.wav",
            Sfx::Fly => "sounds/fly.wav",
            Sfx::Battle => "sounds/battle.wav",
        }
    }
}

const SETTINGS_KEY: &str = "sound.json";

/// The player's sound preference, saved whenever it changes.
#[derive(Resource, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct SoundSettings {
    pub enabled: bool,
    /// 0.0 to 1.0.
    pub volume: f32,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self { enabled: true, volume: 0.6 }
    }
}

impl SoundSettings {
    fn load() -> Self {
        read_setting(SETTINGS_KEY).and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default()
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        if let Ok(json) = serde_json::to_string(self) {
            write_setting(SETTINGS_KEY, &json);
        }
    }

    /// Whether this build can make sound at all.
    pub const AVAILABLE: bool = cfg!(feature = "audio");
}

pub struct SfxPlugin;

impl Plugin for SfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Sfx>().insert_resource(SoundSettings::load());
        #[cfg(feature = "audio")]
        app.add_systems(Startup, load_sounds).add_systems(Last, play_sounds);
    }
}

#[cfg(feature = "audio")]
#[derive(Resource)]
struct Sounds(std::collections::HashMap<Sfx, Handle<AudioSource>>);

#[cfg(feature = "audio")]
fn load_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(Sounds(Sfx::ALL.into_iter().map(|sfx| (sfx, assets.load(sfx.file()))).collect()));
}

/// Plays each requested sound once per frame, however many times it was asked for.
#[cfg(feature = "audio")]
fn play_sounds(mut events: EventReader<Sfx>, settings: Res<SoundSettings>, sounds: Option<Res<Sounds>>, mut commands: Commands) {
    let Some(sounds) = sounds.filter(|_| settings.enabled && settings.volume > 0.0) else {
        events.clear();
        return;
    };
    let mut played = Vec::new();
    for sfx in events.read() {
        if played.contains(sfx) {
            continue;
        }
        played.push(*sfx);
        if let Some(handle) = sounds.0.get(sfx) {
            commands.spawn((
                AudioPlayer(handle.clone()),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(settings.volume)),
            ));
        }
    }
}
