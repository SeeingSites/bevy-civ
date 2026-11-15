use bevy::prelude::*;
use crate::city::City;
use crate::culture::Culture;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VictoryType {
    Domination,
    Democracy,
    SpaceRace,
    Culture,
}

impl VictoryType {
    pub fn name(&self) -> &'static str {
        match self {
            VictoryType::Domination => "Domination",
            VictoryType::Democracy => "Democracy",
            VictoryType::SpaceRace => "Space Race",
            VictoryType::Culture => "Culture",
        }
    }
}

pub fn check_culture_victory(
    cities: &Query<(&City, &Culture)>,
    threshold: u32,
) -> bool {
    let total_culture: u32 = cities
        .iter()
        .filter(|(c, _)| c.owner == 0)
        .map(|(_, culture)| culture.points)
        .sum();
    total_culture >= threshold
}

pub fn check_domination_victory(
    cities: &Query<&City>,
) -> bool {
    let player_cities = cities.iter().filter(|c| c.owner == 0).count();
    let total = cities.iter().count();
    total > 0 && player_cities as f32 / total as f32 >= 0.6
}
