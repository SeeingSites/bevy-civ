//! The economy of a city, after the rules of Civilization III.
//!
//! The land a city works brings in trade (commerce). Corruption takes a share of it - more the
//! farther the city lies from a palace (the capital or a Forbidden Palace) and the more cities its
//! people already have nearer the palace than it, depending on the government. What is left is
//! split by the people's rates into science, luxury and taxes, and the city's buildings multiply
//! each part (a Library adds half to science, a Marketplace half to taxes and luxury). Waste takes
//! a share of the city's shields the same way, if less of it. Buildings cost gold to keep up.
//!
//! The government also changes what each worked tile gives: under Despotism (and Anarchy) a tile
//! that would give 3 or more of anything gives one less, a Republic or a Democracy adds a trade to
//! every tile that already brings some, and a golden age adds a shield and a trade to tiles that
//! have them.

use std::collections::HashMap;

use hexx::Hex;
use serde::{Deserialize, Serialize};

use crate::building::Building;
use crate::government::Government;
use crate::tiles::Yield;

/// Rates move in steps of this many percent.
pub const RATE_STEP: u32 = 10;

/// Science a scientist adds, and gold a tax collector adds (before the city's buildings multiply
/// them, as they do the rest).
pub const SCIENTIST: u32 = 3;
pub const TAX_COLLECTOR: u32 = 2;

/// A city this many tiles or more from its nearest palace suffers the full distance part of
/// corruption.
pub const FAR: u32 = 24;

/// The most corruption (and so waste) can take.
pub const MOST_CORRUPTION: f32 = 0.9;

/// Waste takes this share of what corruption would (shields suffer less than trade).
pub const WASTE_SHARE: f32 = 0.6;

/// How a people splits its cities' trade, in percent: science and luxury, taxes taking the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rates {
    pub science: u32,
    pub luxury: u32,
}

impl Default for Rates {
    fn default() -> Self {
        Rates { science: 60, luxury: 0 }
    }
}

impl Rates {
    pub fn tax(&self) -> u32 {
        100u32.saturating_sub(self.science + self.luxury)
    }

    /// Whether every share is within what the government allows.
    pub fn allowed(&self, gov: Government) -> bool {
        let cap = gov.rate_cap();
        gov == Government::Anarchy || (self.science + self.luxury <= 100 && self.science <= cap && self.luxury <= cap && self.tax() <= cap)
    }

    /// The nearest split the government allows: a share over the limit gives the rest to the
    /// others (what taxes cannot take goes to science, then to luxury).
    pub fn fit(self, gov: Government) -> Rates {
        let cap = gov.rate_cap();
        if cap == 0 {
            return self;
        }
        let mut science = self.science.min(cap);
        let mut luxury = self.luxury.min(cap).min(100 - science);
        let tax = 100 - science - luxury;
        if tax > cap {
            let excess = tax - cap;
            let to_science = excess.min(cap - science);
            science += to_science;
            luxury += excess - to_science;
        }
        Rates { science, luxury }
    }

    /// Splits a city's trade (after corruption) into science, luxury and taxes. Anarchy collects no
    /// taxes and does no research: all of it goes to luxury.
    pub fn split(&self, trade: u32, gov: Government) -> (u32, u32, u32) {
        if gov == Government::Anarchy {
            return (0, trade, 0);
        }
        let science = (trade * self.science + 50) / 100;
        let luxury = ((trade * self.luxury + 50) / 100).min(trade - science);
        (science, luxury, trade - science - luxury)
    }
}

/// What a worked tile gives under a government (and in a golden age).
pub fn tile_rule(tile: Yield, gov: Government, golden_age: bool) -> Yield {
    let (mut food, mut shields, mut trade) = tile;
    let had_shields = shields >= 1;
    let had_trade = trade >= 1;
    if gov.commerce_bonus() && had_trade {
        trade += 1;
    }
    if golden_age {
        shields += u32::from(had_shields);
        trade += u32::from(had_trade);
    }
    if gov.tile_penalty() {
        for value in [&mut food, &mut shields, &mut trade] {
            if *value >= 3 {
                *value -= 1;
            }
        }
    }
    (food, shields, trade)
}

/// Where a city stands towards its government: tiles to its nearest palace (the capital or a
/// Forbidden Palace), and how many of its people's cities lie nearer to a palace than it does
/// (the palace cities included).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    pub distance: u32,
    pub rank: u32,
}

/// The standing of every city in `cities` (position, owner, has a palace). A people with no palace
/// left has every city counted far away.
pub fn standings(cities: &[(Hex, u32, bool)]) -> HashMap<Hex, Standing> {
    let mut seats: HashMap<u32, Vec<Hex>> = HashMap::new();
    for (at, owner, palace) in cities {
        if *palace {
            seats.entry(*owner).or_default().push(*at);
        }
    }
    let distance: HashMap<Hex, u32> = cities
        .iter()
        .map(|(at, owner, _)| (*at, seats.get(owner).and_then(|s| s.iter().map(|p| p.distance_to(*at) as u32).min()).unwrap_or(FAR)))
        .collect();
    let mut by_owner: HashMap<u32, Vec<u32>> = HashMap::new();
    for (at, owner, _) in cities {
        by_owner.entry(*owner).or_default().push(distance[at]);
    }
    for list in by_owner.values_mut() {
        list.sort_unstable();
    }
    cities
        .iter()
        .map(|(at, owner, _)| {
            let d = distance[at];
            // Cities strictly nearer to a palace than this one.
            let rank = by_owner[owner].partition_point(|x| *x < d) as u32;
            (*at, Standing { distance: d, rank })
        })
        .collect()
}

/// The number of cities a people can rule before corruption climbs steeply: the land of the world
/// shared among its peoples (between 10 and 20).
pub fn optimal_cities(land_tiles: usize, peoples: usize) -> u32 {
    (land_tiles / (peoples.max(1) * 9)).clamp(10, 20) as u32
}

/// The share of a city's trade lost to corruption. Half of it comes from distance (to the nearest
/// palace; the same for every city under Communism) and half from rank (cities nearer a palace),
/// which climbs twice as fast once the rank passes the optimal number of cities. A Courthouse and a
/// Police Station each halve the distance part and let the city count a quarter more cities as
/// optimal; a road or harbour link to the capital takes 15% off the distance part. The government
/// sets the scale; a palace city has none.
pub fn corruption(gov: Government, standing: Standing, optimal: u32, buildings: &[Building], palace: bool, connected: bool) -> f32 {
    if palace {
        return 0.0;
    }
    let mut distance_factor = 1.0;
    let mut reach = 1.0;
    for building in [Building::Courthouse, Building::PoliceStation] {
        if buildings.contains(&building) {
            distance_factor *= 0.5;
            reach += 0.25;
        }
    }
    if connected {
        distance_factor *= 0.85;
    }
    let distance = if gov.communal() { 0.5 } else { (standing.distance as f32 / FAR as f32).min(1.0) };
    let optimal = optimal.max(1) as f32 * reach;
    let rank = standing.rank as f32;
    let rank_part = if rank <= optimal { rank / optimal } else { 1.0 + 2.0 * (rank - optimal) / optimal };
    (gov.corruption_level() * (distance * distance_factor + rank_part) / 2.0).clamp(0.0, MOST_CORRUPTION)
}

/// The share of a city's shields lost to waste, given its corruption.
pub fn waste(corruption: f32) -> f32 {
    corruption * WASTE_SHARE
}

/// Percent a city's buildings add to its science (Copernicus' Observatory and Isaac Newton's
/// College double it in their city).
pub fn science_bonus(buildings: &[Building]) -> u32 {
    buildings
        .iter()
        .map(|b| match b {
            Building::Library | Building::University | Building::ResearchLab => 50,
            Building::Observatory => 25,
            Building::Copernicus | Building::Newton => 100,
            _ => 0,
        })
        .sum()
}

/// Percent a city's Factory and Power Plant add to its shields (after waste).
pub fn production_bonus(buildings: &[Building]) -> u32 {
    buildings.iter().map(|b| if matches!(b, Building::Factory | Building::PowerPlant) { 25 } else { 0 }).sum()
}

/// Percent a city's buildings add to its taxes and to its luxury.
pub fn tax_bonus(buildings: &[Building]) -> u32 {
    buildings
        .iter()
        .map(|b| match b {
            Building::Marketplace => 50,
            Building::Bank | Building::StockExchange => 30,
            _ => 0,
        })
        .sum()
}

/// Gold a building costs to keep up every turn: most cost 1, the great works of learning, finance
/// and industry 2; walls, coastal forts and the Forbidden Palace nothing, nor do wonders and
/// spaceship parts.
pub fn upkeep(building: Building) -> u32 {
    use Building::*;
    match building {
        University | Bank | StockExchange | ResearchLab | Factory | PowerPlant => 2,
        Walls | CoastalFortress | ForbiddenPalace => 0,
        b if b.is_wonder() || b.is_spaceship_part() => 0,
        _ => 1,
    }
}

/// Upkeep of all of a city's buildings.
pub fn upkeep_of(buildings: &[Building]) -> u32 {
    buildings.iter().map(|b| upkeep(*b)).sum()
}

fn boosted(amount: u32, percent: u32) -> u32 {
    (amount * (100 + percent) + 50) / 100
}

/// What goes into a city's accounts.
#[derive(Clone, Copy, Debug)]
pub struct CityTrade<'a> {
    /// Trade from the land it works.
    pub trade: u32,
    /// Share of it lost to corruption.
    pub corruption: f32,
    pub rates: Rates,
    pub government: Government,
    pub buildings: &'a [Building],
    pub scientists: u32,
    pub tax_collectors: u32,
    /// Science and gold the city has outright (wonders, traits, trading houses): neither split nor
    /// multiplied.
    pub extra_science: u32,
    pub extra_gold: u32,
}

/// A city's (or a people's) accounts for a turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    /// Trade from the land.
    pub trade: u32,
    /// Trade lost to corruption.
    pub corruption: u32,
    pub science: u32,
    pub luxury: u32,
    /// Taxes (and other gold) collected.
    pub gold: u32,
    /// Gold owed for the upkeep of buildings.
    pub upkeep: u32,
}

impl Ledger {
    pub fn add(&mut self, other: &Ledger) {
        self.trade += other.trade;
        self.corruption += other.corruption;
        self.science += other.science;
        self.luxury += other.luxury;
        self.gold += other.gold;
        self.upkeep += other.upkeep;
    }
}

/// A city's accounts: trade less corruption, split by the rates, multiplied by its buildings.
pub fn ledger(city: &CityTrade) -> Ledger {
    let gov = city.government;
    let corruption = ((city.trade as f32 * city.corruption).round() as u32).min(city.trade);
    let (science, luxury, taxes) = city.rates.split(city.trade - corruption, gov);
    let anarchy = gov == Government::Anarchy;
    let science = if anarchy { 0 } else { science + SCIENTIST * city.scientists };
    let taxes = if anarchy { 0 } else { taxes + TAX_COLLECTOR * city.tax_collectors };
    let science = (boosted(science, science_bonus(city.buildings)) as f32 * gov.science_modifier()).round() as u32;
    let lift = tax_bonus(city.buildings);
    Ledger {
        trade: city.trade,
        corruption,
        science: science + if anarchy { 0 } else { city.extra_science },
        luxury: boosted(luxury, lift),
        gold: boosted(taxes, lift) + city.extra_gold,
        upkeep: upkeep_of(city.buildings),
    }
}

/// Rates for a computer player: the luxury it needs (see `luxury_for`), then as much science as it
/// can afford. `full_taxes` is the gold its cities would raise at a 100% tax rate; `income` the
/// rest of what it earns each turn and `costs` what it owes (upkeep and the like). It keeps a
/// small surplus while its treasury is low, and spends a large one down.
pub fn computer_rates(gov: Government, luxury: u32, full_taxes: u32, income: i32, costs: i32, treasury: i32) -> Rates {
    let cap = gov.rate_cap();
    if cap == 0 {
        // Anarchy ignores the rates: keep the usual ones for the government to come.
        return Rates::default();
    }
    let luxury = luxury.min(cap);
    let target = if treasury < 60 {
        2 + costs / 20
    } else if treasury > 300 {
        -(treasury - 300) / 10
    } else {
        0
    };
    // The least tax that meets the target, leaving the most for science.
    let lowest = (100 - luxury).saturating_sub(cap);
    let tax = (lowest..=cap.min(100 - luxury))
        .step_by(RATE_STEP as usize)
        .find(|tax| (full_taxes * tax / 100) as i32 + income - costs >= target)
        .unwrap_or(cap.min(100 - luxury));
    Rates { science: 100 - luxury - tax, luxury }.fit(gov)
}

/// The luxury rate a computer player sets next: up while many of its cities are in disorder, back
/// down once none is.
pub fn luxury_for(current: u32, in_disorder: usize, cities: usize) -> u32 {
    if in_disorder * 4 > cities.max(1) {
        (current + RATE_STEP).min(40)
    } else if in_disorder == 0 {
        current.saturating_sub(RATE_STEP)
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_share_stays_within_the_government_limit() {
        let rates = Rates { science: 90, luxury: 0 };
        assert!(!rates.allowed(Government::Despotism));
        assert_eq!(rates.fit(Government::Despotism), Rates { science: 60, luxury: 0 });
        assert!(rates.allowed(Government::Democracy));
        // Taxes are limited too: a despot cannot tax 70%.
        let low = Rates { science: 30, luxury: 0 };
        assert!(!low.allowed(Government::Despotism));
        assert_eq!(low.fit(Government::Despotism), Rates { science: 40, luxury: 0 });
        assert_eq!(Rates { science: 60, luxury: 60 }.fit(Government::Despotism), Rates { science: 60, luxury: 40 });
        for gov in Government::CHOICES {
            for science in (0..=100).step_by(10) {
                for luxury in (0..=100 - science).step_by(10) {
                    assert!(Rates { science, luxury }.fit(gov).allowed(gov), "{gov:?} {science} {luxury}");
                }
            }
        }
    }

    #[test]
    fn trade_splits_by_the_rates_and_anarchy_gives_it_all_to_luxury() {
        let rates = Rates { science: 50, luxury: 20 };
        assert_eq!(rates.split(10, Government::Monarchy), (5, 2, 3));
        assert_eq!(rates.split(0, Government::Monarchy), (0, 0, 0));
        assert_eq!(rates.split(10, Government::Anarchy), (0, 10, 0));
        let (s, l, t) = Rates { science: 70, luxury: 30 }.split(7, Government::Democracy);
        assert_eq!(s + l + t, 7);
    }

    #[test]
    fn governments_and_golden_ages_change_the_land() {
        // Despotism: 3 or more of anything gives one less.
        assert_eq!(tile_rule((3, 1, 2), Government::Despotism, false), (2, 1, 2));
        assert_eq!(tile_rule((4, 3, 3), Government::Anarchy, false), (3, 2, 2));
        // A Republic adds a trade to tiles that have some.
        assert_eq!(tile_rule((1, 0, 2), Government::Republic, false), (1, 0, 3));
        assert_eq!(tile_rule((2, 0, 0), Government::Democracy, false), (2, 0, 0));
        assert_eq!(tile_rule((3, 1, 2), Government::Monarchy, false), (3, 1, 2));
        // A golden age adds a shield and a trade where there are some; a despot's penalty eats it.
        assert_eq!(tile_rule((1, 2, 1), Government::Monarchy, true), (1, 3, 2));
        assert_eq!(tile_rule((1, 2, 0), Government::Despotism, true), (1, 2, 0));
        assert_eq!(tile_rule((1, 1, 2), Government::Republic, true), (1, 2, 4));
    }

    #[test]
    fn distance_and_rank_decide_corruption() {
        let a = Hex::new(0, 0);
        let near = Hex::new(3, 0);
        let far = Hex::new(12, 0);
        let fp = Hex::new(14, 0);
        let places = standings(&[(a, 0, true), (near, 0, false), (far, 0, false), (Hex::new(5, 5), 1, false)]);
        assert_eq!(places[&a], Standing { distance: 0, rank: 0 });
        assert_eq!(places[&near], Standing { distance: 3, rank: 1 });
        assert_eq!(places[&far], Standing { distance: 12, rank: 2 });
        assert_eq!(places[&Hex::new(5, 5)].distance, FAR, "no palace, as if far away");
        // A Forbidden Palace brings the far city close.
        let places = standings(&[(a, 0, true), (near, 0, false), (far, 0, false), (fp, 0, true)]);
        assert_eq!(places[&far], Standing { distance: 2, rank: 2 });

        let at = |distance, rank| Standing { distance, rank };
        let c = |gov, s| corruption(gov, s, 12, &[], false, false);
        assert_eq!(corruption(Government::Despotism, at(10, 5), 12, &[], true, false), 0.0, "none in a palace city");
        assert!(c(Government::Monarchy, at(3, 1)) < c(Government::Monarchy, at(12, 8)));
        assert!(c(Government::Democracy, at(12, 8)) < c(Government::Republic, at(12, 8)));
        assert!(c(Government::Republic, at(12, 8)) < c(Government::Monarchy, at(12, 8)));
        assert!(c(Government::Monarchy, at(12, 8)) < c(Government::Despotism, at(12, 8)));
        // Past the optimal number of cities it climbs twice as fast.
        let step_before = c(Government::Monarchy, at(6, 11)) - c(Government::Monarchy, at(6, 10));
        let step_after = c(Government::Monarchy, at(6, 15)) - c(Government::Monarchy, at(6, 14));
        assert!(step_after > step_before * 1.9, "{step_before} {step_after}");
        // Communism does not mind distance.
        assert_eq!(c(Government::Communism, at(2, 6)), c(Government::Communism, at(20, 6)));
        // A Courthouse cuts it, a Police Station more, a link to the capital a little.
        let plain = c(Government::Monarchy, at(15, 14));
        let court = corruption(Government::Monarchy, at(15, 14), 12, &[Building::Courthouse], false, false);
        let both = corruption(Government::Monarchy, at(15, 14), 12, &[Building::Courthouse, Building::PoliceStation], false, false);
        let linked = corruption(Government::Monarchy, at(15, 14), 12, &[], false, true);
        assert!(both < court && court < plain && linked < plain);
        assert!(c(Government::Anarchy, at(40, 60)) <= MOST_CORRUPTION);
        assert!(waste(0.5) < 0.5);
    }

    #[test]
    fn the_optimal_number_of_cities_follows_the_land() {
        assert_eq!(optimal_cities(1280, 11), 12);
        assert_eq!(optimal_cities(1305, 6), 20);
        assert_eq!(optimal_cities(300, 8), 10);
    }

    #[test]
    fn buildings_multiply_their_share_and_cost_upkeep() {
        let built = [Building::Library, Building::Marketplace, Building::Temple];
        let city = CityTrade {
            trade: 20,
            corruption: 0.1,
            rates: Rates { science: 50, luxury: 10 },
            government: Government::Monarchy,
            buildings: &built,
            scientists: 0,
            tax_collectors: 0,
            extra_science: 0,
            extra_gold: 0,
        };
        // 20 trade, 2 lost: 9 science, 2 luxury, 7 taxes; the library adds half to science, the
        // market half to luxury and taxes.
        assert_eq!(ledger(&city), Ledger { trade: 20, corruption: 2, science: 14, luxury: 3, gold: 11, upkeep: 3 });
        // Specialists add before the buildings multiply; extras after.
        let staffed = CityTrade { scientists: 1, tax_collectors: 1, extra_gold: 3, ..city };
        let books = ledger(&staffed);
        assert_eq!((books.science, books.gold), (boosted(9 + SCIENTIST, 50), boosted(7 + TAX_COLLECTOR, 50) + 3));
        // Fundamentalism halves science; anarchy has none and no taxes either.
        assert_eq!(ledger(&CityTrade { government: Government::Fundamentalism, rates: Rates { science: 50, luxury: 10 }, ..city }).science, 7);
        let lawless = ledger(&CityTrade { government: Government::Anarchy, ..staffed });
        assert_eq!((lawless.science, lawless.gold), (0, 3));
        assert!(lawless.luxury > 0);
        assert_eq!(upkeep(Building::Pyramids), 0);
        assert_eq!(upkeep(Building::Walls), 0);
        assert!(upkeep(Building::Factory) > upkeep(Building::Temple));
    }

    #[test]
    fn computer_players_tax_what_they_need_and_research_the_rest() {
        // Plenty of trade and nothing to pay: as much science as allowed.
        assert_eq!(computer_rates(Government::Republic, 0, 100, 0, 0, 200), Rates { science: 80, luxury: 0 });
        // Bills of 30 a turn on 100 gold of full taxes: 30% taxes.
        assert_eq!(computer_rates(Government::Republic, 0, 100, 0, 30, 200), Rates { science: 70, luxury: 0 });
        // Short of money it keeps a surplus.
        assert_eq!(computer_rates(Government::Republic, 0, 100, 0, 30, 20), Rates { science: 60, luxury: 0 });
        // A full treasury is spent down on science.
        assert_eq!(computer_rates(Government::Republic, 0, 100, 0, 30, 1000), Rates { science: 80, luxury: 0 });
        // Luxury first; never more than the government allows in any share.
        let rates = computer_rates(Government::Despotism, 20, 100, 0, 90, 200);
        assert!(rates.allowed(Government::Despotism) && rates.luxury == 20, "{rates:?}");
        assert_eq!(luxury_for(0, 3, 8), 10);
        assert_eq!(luxury_for(10, 0, 8), 0);
        assert_eq!(luxury_for(40, 5, 8), 40);
    }
}
