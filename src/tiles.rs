//! Working the land: every tile yields food, shields and gold, and each working citizen of a city
//! works one tile of the city's own area (the city's tile is always worked). A governor chooses the
//! tiles by the city's focus - growth, production, trade or a balance, always feeding the city
//! first - and the player can move citizens. On Auto the governor picks the focus itself from the
//! city's situation, and says why.

use hexx::Hex;
use serde::{Deserialize, Serialize};

use crate::terrain::{Improvement, Resource, ResourceKind, Terrain};

/// Food, shields and gold.
pub type Yield = (u32, u32, u32);

/// Food each citizen eats a turn.
pub const FOOD_PER_CITIZEN: u32 = 2;

/// What a tile gives a city working it. `resource` is the resource the owner can see (if any).
pub fn tile_yield(terrain: Terrain, improvements: &[Improvement], resource: Option<Resource>, river: bool) -> Yield {
    if improvements.contains(&Improvement::Pollution) {
        return (0, 0, 0);
    }
    let (mut food, mut shields, mut gold) = match terrain {
        Terrain::Water => (1, 0, 2),
        Terrain::Grassland => (2, 0, 0),
        Terrain::Plains => (1, 1, 0),
        Terrain::Desert => (0, 1, 0),
        Terrain::Hills => (1, 0, 0),
        Terrain::Mountain => (0, 1, 0),
        Terrain::Forest => (1, 2, 0),
    };
    let farmland = matches!(terrain, Terrain::Grassland | Terrain::Plains | Terrain::Desert | Terrain::Hills);
    if improvements.contains(&Improvement::Farm) && farmland {
        food += 1;
    }
    if improvements.contains(&Improvement::Mine) {
        shields += match terrain {
            Terrain::Hills | Terrain::Mountain => 2,
            _ => 1,
        };
    }
    let has_road = improvements.contains(&Improvement::Road) || improvements.contains(&Improvement::Railroad);
    if has_road && matches!(terrain, Terrain::Grassland | Terrain::Plains | Terrain::Desert) {
        gold += 1;
    }
    if improvements.contains(&Improvement::Railroad) {
        shields += 1;
    }
    if let Some(resource) = resource {
        match resource.kind() {
            ResourceKind::Bonus => {
                let (f, s, g) = resource.bonus_yield();
                food += f;
                shields += s;
                gold += g;
            }
            ResourceKind::Luxury => gold += 1,
            ResourceKind::Strategic => shields += 1,
        }
    }
    if river {
        gold += 1;
    }
    (food, shields, gold)
}

/// The city's own tile: worked for free, with a little extra (it is farmed, and the market is there).
pub fn centre_yield(tile: Yield) -> Yield {
    (tile.0 + 1, tile.1.max(1) + 1, tile.2 + 1)
}

/// What a city's governor works the land for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Focus {
    /// The governor chooses one of the others from the city's situation.
    #[default]
    Auto,
    /// Food, shields and trade alike, with a little food to spare for growth.
    Balanced,
    /// As much food as the land gives: the city grows fast.
    Growth,
    /// Shields first, once everyone is fed: projects finish sooner.
    Production,
    /// Gold-rich tiles first (sea, rivers, roads), once everyone is fed: more science and taxes.
    Trade,
}

impl Focus {
    pub const ALL: [Focus; 5] = [Focus::Auto, Focus::Balanced, Focus::Growth, Focus::Production, Focus::Trade];

    pub fn name(&self) -> &'static str {
        match self {
            Focus::Auto => "Auto",
            Focus::Balanced => "Balanced",
            Focus::Growth => "Growth",
            Focus::Production => "Production",
            Focus::Trade => "Trade",
        }
    }

    /// Food the city aims to have once everyone has eaten, and how it weighs (food, shields, gold)
    /// beyond that.
    fn plan(&self, population: u32) -> (u32, (u32, u32, u32)) {
        let fed = population * FOOD_PER_CITIZEN;
        match self {
            Focus::Growth => (u32::MAX, (4, 1, 1)),
            Focus::Production => (fed, (1, 4, 1)),
            Focus::Trade => (fed, (1, 1, 4)),
            Focus::Balanced | Focus::Auto => (fed + 2, (2, 2, 1)),
        }
    }
}

/// What the governor weighs in choosing a focus for a city.
#[derive(Clone, Copy, Debug, Default)]
pub struct Situation {
    pub population: u32,
    /// The city cannot grow now: at its water supply's limit, or more citizens would be unhappy.
    pub growth_blocked: bool,
    /// Turns its current project still needs at the present pace (0 when there is none).
    pub project_turns: u32,
    /// The project is a wonder.
    pub building_wonder: bool,
    /// It trains troops while its people are at war.
    pub arming_for_war: bool,
    /// It has a Library, Marketplace or University to multiply its trade.
    pub trade_buildings: bool,
}

/// The focus the governor recommends for a city, and why.
pub fn suggest(s: &Situation) -> (Focus, &'static str) {
    if s.building_wonder {
        return (Focus::Production, "a wonder is going up: every shield counts");
    }
    if s.arming_for_war {
        return (Focus::Production, "we are at war and it is training troops");
    }
    if s.growth_blocked {
        return if s.trade_buildings {
            (Focus::Trade, "it cannot grow for now, and its library or market multiplies trade")
        } else {
            (Focus::Production, "it cannot grow for now, so it builds instead")
        };
    }
    if s.population < 4 {
        return (Focus::Growth, "a small city: more citizens work more land");
    }
    if s.project_turns >= 12 {
        return (Focus::Production, "its project would take a long time");
    }
    if s.trade_buildings && s.population >= 6 {
        return (Focus::Trade, "a large city whose library or market multiplies trade");
    }
    (Focus::Balanced, "nothing pressing: a little of everything")
}

/// Chooses the tiles a city's `workers` citizens work among `candidates` (tile and yield).
/// Tiles in `keep` (the player's choices) stay worked if they are still candidates; the rest are
/// filled food first - until the city feeds `population` (with something to spare, by `focus`) -
/// then by what the focus favours.
pub fn assign(candidates: &[(Hex, Yield)], keep: &[Hex], centre: Yield, workers: u32, population: u32, focus: Focus) -> Vec<Hex> {
    let mut chosen: Vec<Hex> = keep.iter().copied().filter(|h| candidates.iter().any(|(c, _)| c == h)).collect();
    chosen.dedup();
    let (target, (wf, ws, wg)) = focus.plan(population);
    let value = |y: &Yield| y.0 * wf + y.1 * ws + y.2 * wg;
    let yield_of = |h: &Hex| candidates.iter().find(|(c, _)| c == h).map(|(_, y)| *y).unwrap_or((0, 0, 0));
    // Too many kept (the city shrank): let go of the least useful.
    while chosen.len() > workers as usize {
        let worst = chosen.iter().enumerate().min_by_key(|(_, h)| value(&yield_of(h))).map(|(i, _)| i).unwrap_or(0);
        chosen.remove(worst);
    }
    while chosen.len() < workers as usize {
        let food: u32 = centre.0 + chosen.iter().map(|h| yield_of(h).0).sum::<u32>();
        let free = candidates.iter().filter(|(h, _)| !chosen.contains(h));
        let next = if food < target {
            free.max_by_key(|(h, y)| (y.0, y.1, y.2, std::cmp::Reverse((h.x, h.y))))
        } else {
            free.max_by_key(|(h, y)| (value(y), y.0, y.1, std::cmp::Reverse((h.x, h.y))))
        };
        match next {
            Some((h, _)) => chosen.push(*h),
            None => break,
        }
    }
    chosen
}

/// The most citizens (up to `population`) the best of these tiles feed, besides the city's own
/// tile (which gives `centre` food): a scenario's starting city is no bigger than its land feeds.
pub fn sustainable(population: u32, centre: u32, mut foods: Vec<u32>) -> u32 {
    foods.sort_unstable_by(|a, b| b.cmp(a));
    (1..=population).rev().find(|n| centre + foods.iter().take(*n as usize).sum::<u32>() >= n * FOOD_PER_CITIZEN).unwrap_or(1)
}

/// Everything the city's land gives: its own tile plus the worked ones.
pub fn total(candidates: &[(Hex, Yield)], worked: &[Hex], centre: Yield) -> Yield {
    worked
        .iter()
        .filter_map(|h| candidates.iter().find(|(c, _)| c == h).map(|(_, y)| *y))
        .fold(centre, |acc, y| (acc.0 + y.0, acc.1 + y.1, acc.2 + y.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn improvements_resources_and_rivers_add_to_the_land() {
        assert_eq!(tile_yield(Terrain::Grassland, &[], None, false), (2, 0, 0));
        assert_eq!(tile_yield(Terrain::Grassland, &[Improvement::Farm, Improvement::Road], None, true), (3, 0, 2));
        assert_eq!(tile_yield(Terrain::Hills, &[Improvement::Mine], None, false), (1, 2, 0));
        assert_eq!(tile_yield(Terrain::Plains, &[Improvement::Farm, Improvement::Pollution], None, false), (0, 0, 0));
        assert_eq!(tile_yield(Terrain::Water, &[], Some(Resource::Fish), false), (3, 0, 2));
        assert_eq!(centre_yield((2, 0, 0)), (3, 2, 1));
    }

    #[test]
    fn a_city_is_no_bigger_than_its_land_feeds() {
        // Three hills and the sea around a plains town: 2 from the centre, 1 a tile.
        assert_eq!(sustainable(5, 2, vec![1, 1, 1, 1, 1, 1]), 2);
        // With a harbour the sea gives 2: all five are fed.
        assert_eq!(sustainable(5, 2, vec![2, 2, 2, 2, 2, 1]), 5);
        assert_eq!(sustainable(3, 3, vec![]), 1);
    }

    #[test]
    fn the_governor_feeds_the_city_first_and_keeps_the_players_choices() {
        let grass = (2, 0, 0);
        let hills = (1, 2, 0);
        let forest = (1, 2, 0);
        let candidates = [(Hex::new(1, 0), grass), (Hex::new(0, 1), hills), (Hex::new(-1, 0), forest), (Hex::new(0, -1), grass)];
        // Two workers in a size-2 city: needs 6 food; centre gives 3, so both grass tiles.
        let picked = assign(&candidates, &[], (3, 2, 1), 2, 2, Focus::Balanced);
        assert!(picked.contains(&Hex::new(1, 0)) && picked.contains(&Hex::new(0, -1)));
        // The player's hills stay worked.
        let picked = assign(&candidates, &[Hex::new(0, 1)], (3, 2, 1), 2, 2, Focus::Balanced);
        assert!(picked.contains(&Hex::new(0, 1)));
        assert_eq!(picked.len(), 2);
        // A shrunken city lets go of its least useful tile.
        let picked = assign(&candidates, &[Hex::new(0, 1), Hex::new(1, 0)], (3, 2, 1), 1, 1, Focus::Balanced);
        assert_eq!(picked, vec![Hex::new(0, 1)]);
        assert_eq!(total(&candidates, &[Hex::new(1, 0)], (3, 2, 1)), (5, 2, 1));
    }

    #[test]
    fn the_focus_steers_the_land_once_the_city_is_fed() {
        let grass = (2, 0, 0);
        let mine = (0, 3, 0);
        let sea = (1, 0, 3);
        let candidates = [(Hex::new(1, 0), grass), (Hex::new(0, 1), mine), (Hex::new(-1, 0), sea), (Hex::new(0, -1), grass)];
        // A size-2 city eats 4; its centre gives 3, so one grassland feeds it.
        let pick = |focus| assign(&candidates, &[], (3, 1, 1), 2, 2, focus);
        assert!(pick(Focus::Production).contains(&Hex::new(0, 1)), "the mine");
        assert!(pick(Focus::Trade).contains(&Hex::new(-1, 0)), "the sea");
        let growth = pick(Focus::Growth);
        assert!(growth.contains(&Hex::new(1, 0)) && growth.contains(&Hex::new(0, -1)), "both grasslands");
        // Every focus feeds the city first.
        for focus in Focus::ALL {
            assert!(total(&candidates, &pick(focus), (3, 1, 1)).0 >= 4, "{focus:?}");
        }
    }

    #[test]
    fn the_governor_suggests_a_focus_and_says_why() {
        let base = Situation { population: 6, ..Default::default() };
        assert_eq!(suggest(&Situation { population: 2, ..base }).0, Focus::Growth);
        assert_eq!(suggest(&Situation { building_wonder: true, ..base }).0, Focus::Production);
        assert_eq!(suggest(&Situation { growth_blocked: true, trade_buildings: true, ..base }).0, Focus::Trade);
        assert_eq!(suggest(&Situation { project_turns: 20, ..base }).0, Focus::Production);
        assert_eq!(suggest(&Situation { trade_buildings: true, ..base }).0, Focus::Trade);
        assert_eq!(suggest(&base).0, Focus::Balanced);
        assert!(!suggest(&base).1.is_empty());
    }
}
