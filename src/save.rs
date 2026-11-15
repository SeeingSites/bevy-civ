use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use std::fs;

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

pub fn save_game(
    turn: u32,
    gold: i32,
    science: i32,
    government: &str,
    score: u32,
    player_cities: u32,
    researched_techs: u32,
    mode: &str,
) {
    let data = SaveData {
        turn,
        gold,
        science,
        government: government.to_string(),
        score,
        player_cities,
        researched_techs,
        mode: mode.to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = fs::write("save.json", json);
        info!("Game saved to save.json (turn {})", turn);
    }
}

pub fn load_game() -> Option<SaveData> {
    if let Ok(json) = fs::read_to_string("save.json") {
        serde_json::from_str(&json).ok()
    } else {
        None
    }
}

pub fn save_exists() -> bool {
    std::path::Path::new("save.json").exists()
}
