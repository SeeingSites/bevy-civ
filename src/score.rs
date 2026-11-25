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

/// The sides of a scenario's war at its end, best first: each side (peoples joined by alliances,
/// directly or through an ally) with its gain in objective cities since the start - those it holds
/// now less those it held then. `start` and `now` give each objective city's holder.
pub fn objective_standing(start: &[(String, u32)], now: &[(String, u32)], peoples: &[u32], allied: impl Fn(u32, u32) -> bool) -> Vec<(Vec<u32>, i32)> {
    let mut sides: Vec<Vec<u32>> = Vec::new();
    for &p in peoples {
        let joined: Vec<usize> = sides.iter().enumerate().filter(|(_, s)| s.iter().any(|&q| allied(p, q))).map(|(i, _)| i).collect();
        let mut side = vec![p];
        for i in joined.into_iter().rev() {
            side.extend(sides.remove(i));
        }
        side.sort();
        sides.push(side);
    }
    let held = |list: &[(String, u32)], side: &[u32]| list.iter().filter(|(_, o)| side.contains(o)).count() as i32;
    let mut standing: Vec<(Vec<u32>, i32)> = sides.into_iter().map(|s| { let gain = held(now, &s) - held(start, &s); (s, gain) }).collect();
    standing.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    standing
}

/// One people's own gain in objective cities.
pub fn objective_gain(start: &[(String, u32)], now: &[(String, u32)], people: u32) -> i32 {
    now.iter().filter(|(_, o)| *o == people).count() as i32 - start.iter().filter(|(_, o)| *o == people).count() as i32
}

#[cfg(test)]
mod objective_tests {
    use super::*;

    #[test]
    fn allies_share_their_gains_in_objective_cities() {
        let o = |n: &str, p: u32| (n.to_string(), p);
        let start = [o("Berlin", 1), o("Rome", 2), o("Paris", 3), o("London", 4), o("Moscow", 5)];
        let now = [o("Berlin", 1), o("Rome", 2), o("Paris", 1), o("London", 4), o("Moscow", 5)];
        // Germany (1) and Italy (2) are allied, as are France (3) and Britain (4); Russia (5) alone.
        let allied = |a: u32, b: u32| matches!((a.min(b), a.max(b)), (1, 2) | (3, 4));
        let standing = objective_standing(&start, &now, &[1, 2, 3, 4, 5], allied);
        assert_eq!(standing[0], (vec![1, 2], 1));
        assert_eq!(standing.last().unwrap(), &(vec![3, 4], -1));
        assert_eq!(objective_gain(&start, &now, 1), 1);
        // An ally of an ally is on the same side.
        let chain = |a: u32, b: u32| matches!((a.min(b), a.max(b)), (1, 2) | (2, 3));
        assert!(objective_standing(&start, &now, &[1, 2, 3, 4, 5], chain).iter().any(|(s, _)| s == &vec![1, 2, 3]));
    }
}

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
