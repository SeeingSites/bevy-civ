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
    pub fn calculate<'a>(
        cities: impl Iterator<Item = (&'a City, &'a CityBuildings)>,
        tech: &TechState,
    ) -> Self {
        let mut population = 0;
        let mut wonders = 0;
        for (city, buildings) in cities.filter(|(c, _)| c.owner == 0) {
            population += city.population;
            wonders += buildings.buildings.iter().filter(|b| b.is_wonder()).count() as u32 * 50;
        }
        let techs = tech.researched.len() as u32 * 10;
        let total = population * 2 + techs + wonders;
        Self { population, techs, wonders, total }
    }
}
