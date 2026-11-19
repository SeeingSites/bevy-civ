//! Tribal villages scattered over the land at the start of a game. The first unit to walk into one
//! finds what its people have to give: gold, a piece of knowledge, a band of warriors who join you,
//! maps of the lands around, or - now and then - angry tribesmen.

use std::collections::HashSet;

use bevy::prelude::*;
use hexx::Hex;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// The villages still standing.
#[derive(Resource, Default, Clone, Debug, Serialize, Deserialize)]
pub struct Villages {
    pub sites: HashSet<Hex>,
}

/// What a village gives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Find {
    Gold(i32),
    Technology,
    Warriors,
    Maps,
    Hostile,
}

/// One village for about every 30 land tiles, each at least 4 tiles from any start and 3 from
/// another village.
pub fn place(land: &[Hex], starts: &[Hex], rng: &mut impl Rng) -> HashSet<Hex> {
    let wanted = land.len() / 30;
    let mut sites: HashSet<Hex> = HashSet::new();
    let mut tries = 0;
    while sites.len() < wanted && tries < wanted * 40 && !land.is_empty() {
        tries += 1;
        let at = land[rng.gen_range(0..land.len())];
        if starts.iter().all(|s| s.distance_to(at) >= 4) && sites.iter().all(|v| v.distance_to(at) >= 3) {
            sites.insert(at);
        }
    }
    sites
}

/// What the village at hand gives (hostile tribesmen are rarer for a scout, who is careful).
pub fn roll(careful: bool, rng: &mut impl Rng) -> Find {
    let hostile = if careful { 5 } else { 15 };
    match rng.gen_range(0..100) {
        r if r < 35 => Find::Gold(rng.gen_range(25..=75)),
        r if r < 55 => Find::Technology,
        r if r < 72 => Find::Warriors,
        r if r < 100 - hostile => Find::Maps,
        _ => Find::Hostile,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn villages_keep_clear_of_starts_and_each_other() {
        let land: Vec<Hex> = (0..30).flat_map(|q| (0..10).map(move |r| Hex::new(q, r))).collect();
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        let sites = place(&land, &[Hex::new(2, 2)], &mut rng);
        assert!(!sites.is_empty() && sites.len() <= land.len() / 30);
        assert!(sites.iter().all(|s| s.distance_to(Hex::new(2, 2)) >= 4));
        for a in &sites {
            assert!(sites.iter().filter(|b| *b != a).all(|b| b.distance_to(*a) >= 3));
        }
        let finds: Vec<Find> = (0..500).map(|_| roll(false, &mut rng)).collect();
        assert!(finds.iter().any(|f| matches!(f, Find::Gold(_))) && finds.contains(&Find::Hostile) && finds.contains(&Find::Technology));
    }
}
