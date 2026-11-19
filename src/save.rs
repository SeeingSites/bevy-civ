use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub turn: u32,
    pub gold: i32,
    pub science: i32,
    pub government: String,
    pub score: u32,
    pub player_cities: u32,
    pub researched_techs: u32,
    pub mode: String,
}

const SAVE_KEY: &str = "save.json";

/// Native: a `save.json` file in the working directory.
#[cfg(not(target_arch = "wasm32"))]
mod storage {
    pub fn write(key: &str, json: &str) -> bool {
        std::fs::write(key, json).is_ok()
    }

    pub fn read(key: &str) -> Option<String> {
        std::fs::read_to_string(key).ok()
    }
}

/// Web: the browser's localStorage.
#[cfg(target_arch = "wasm32")]
mod storage {
    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn write(key: &str, json: &str) -> bool {
        local_storage().is_some_and(|s| s.set_item(key, json).is_ok())
    }

    pub fn read(key: &str) -> Option<String> {
        local_storage()?.get_item(key).ok()?
    }
}

/// Returns whether the save was written.
pub fn save_game(data: &SaveData) -> bool {
    match serde_json::to_string_pretty(data) {
        Ok(json) if storage::write(SAVE_KEY, &json) => {
            info!("Game saved to {SAVE_KEY} (turn {})", data.turn);
            true
        }
        _ => {
            warn!("Could not save game");
            false
        }
    }
}

pub fn load_game() -> Option<SaveData> {
    serde_json::from_str(&storage::read(SAVE_KEY)?).ok()
}

pub fn save_exists() -> bool {
    storage::read(SAVE_KEY).is_some()
}

/// Writes a small text file of player preferences (next to the save, or in localStorage on the web).
pub fn write_setting(key: &str, value: &str) -> bool {
    storage::write(key, value)
}

/// Reads a preference written by [`write_setting`].
pub fn read_setting(key: &str) -> Option<String> {
    storage::read(key)
}

/// Serialises a map whose keys are not strings (tiles, pairs of ids) as a list of pairs, which JSON
/// can hold. Use with `#[serde(with = "crate::save::pairs")]`.
pub mod pairs {
    use std::collections::HashMap;
    use std::hash::Hash;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<K: Serialize, V: Serialize, S: Serializer>(
        map: &HashMap<K, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(map.iter())
    }

    pub fn deserialize<'de, K, V, D>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Eq + Hash,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(deserializer)?
            .into_iter()
            .collect())
    }
}
