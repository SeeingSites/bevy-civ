use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub turn: u32,
    pub gold: i32,
    pub government: String,
}

pub fn save_game(turn: u32, gold: i32, government: &str) {
    let data = SaveData {
        turn,
        gold,
        government: government.to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = fs::write("save.json", json);
        info!("Game saved to save.json");
    }
}

pub fn load_game() -> Option<SaveData> {
    if let Ok(json) = fs::read_to_string("save.json") {
        serde_json::from_str(&json).ok()
    } else {
        None
    }
}
