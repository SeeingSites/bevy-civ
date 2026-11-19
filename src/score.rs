use bevy::prelude::*;
use crate::city::City;
use crate::tech::TechState;
use crate::building::CityBuildings;

#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Score {
    pub population: u32,
    pub techs: u32,
    pub wonders: u32,
    /// Points for the scenario's objective cities held (see [`OBJECTIVE_POINTS`]).
    #[serde(default)]
    pub objectives: u32,
    pub total: u32,
}

/// What each of a scenario's objective cities is worth to the people holding it.
pub const OBJECTIVE_POINTS: u32 = 200;

/// Points for the objective cities among `held` (the names of a people's cities).
pub fn objective_points<'a>(held: impl Iterator<Item = &'a str>, objectives: &[String]) -> u32 {
    held.filter(|name| objectives.iter().any(|o| o == name)).count() as u32 * OBJECTIVE_POINTS
}

impl Score {
    pub fn calculate<'a>(
        cities: impl Iterator<Item = (&'a City, &'a CityBuildings)>,
        tech: &TechState,
        objectives: &[String],
    ) -> Self {
        let mut population = 0;
        let mut wonders = 0;
        let mut names: Vec<&str> = Vec::new();
        for (city, buildings) in cities.filter(|(c, _)| c.owner == 0) {
            population += city.population;
            wonders += buildings.buildings.iter().filter(|b| b.is_wonder()).count() as u32 * 50;
            names.push(&city.name);
        }
        let techs = tech.researched.len() as u32 * 10;
        let objectives = objective_points(names.into_iter(), objectives);
        let total = population * 2 + techs + wonders + objectives;
        Self { population, techs, wonders, objectives, total }
    }
}
