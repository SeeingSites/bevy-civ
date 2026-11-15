use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub turn: u32,
    pub gold: i32,
    pub government: String,
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

pub fn save_game(turn: u32, gold: i32, government: &str) {
    let data = SaveData {
        turn,
        gold,
        government: government.to_string(),
    };
    match serde_json::to_string_pretty(&data) {
        Ok(json) if storage::write(SAVE_KEY, &json) => info!("Game saved to {SAVE_KEY}"),
        _ => warn!("Could not save game"),
    }
}

pub fn load_game() -> Option<SaveData> {
    serde_json::from_str(&storage::read(SAVE_KEY)?).ok()
}
