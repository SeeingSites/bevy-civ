use bevy::prelude::*;
use crate::city::City;
use crate::tech::TechState;
use crate::building::CityBuildings;

#[derive(Resource, Default)]
pub struct Score {
    pub population: u32,
    pub techs: u32,
    pub wonders: u32,
    pub total: u32,
}

impl Score {
    pub fn calculate(
        cities: &Query<(&City, &CityBuildings)>,
        tech: &TechState,
    ) -> Self {
        let population: u32 = cities.iter().filter(|(c, _)| c.owner == 0).map(|(c, _)| c.population).sum();
        let techs = tech.researched.len() as u32 * 10;
        let wonders: u32 = cities.iter()
            .filter(|(c, _)| c.owner == 0)
            .map(|(_, b)| b.buildings.iter().filter(|b| b.is_wonder()).count() as u32 * 50)
            .sum();
        let total = population * 2 + techs + wonders;
        Self { population, techs, wonders, total }
    }
}
