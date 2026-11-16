//! Resources and the trade network: which civilization owns which sources, and which cities can use them.
//!
//! A source counts for its owner when it lies inside their borders, their technology can see it, and
//! it is connected: the tile needs a road (or a city on it), and roads or harbours must link it to
//! the city that wants to use it. Roads count inside the owner's borders and on unclaimed land.
//! The same rules apply to the player and to computer players.

use bevy::prelude::*;
use hexx::Hex;
use std::collections::{HashMap, HashSet};

use bevy_civ::tech::Tech;
use bevy_civ::terrain::{HexTile, Improvement, Resource as Good, ResourceKind};

use crate::{CITY_RADIUS, PLAYER};

/// A resource one civilization supplies to another: (seller, buyer, good).
pub type Transfer = (u32, u32, Good);

#[derive(Default, Clone)]
pub struct CivSupply {
    /// Visible sources inside the civilization's borders.
    pub sources: HashMap<Good, u32>,
    /// Sources linked to the capital's trade network.
    pub connected: HashMap<Good, u32>,
    pub imports: HashSet<Good>,
    pub exports: HashMap<Good, u32>,
}

impl CivSupply {
    /// Copies that could be traded away without losing access.
    pub fn spare(&self, good: Good) -> u32 {
        let have = self.connected.get(&good).copied().unwrap_or(0);
        have.saturating_sub(1 + self.exports.get(&good).copied().unwrap_or(0))
    }

    pub fn has(&self, good: Good) -> bool {
        self.connected.get(&good).copied().unwrap_or(0) > self.exports.get(&good).copied().unwrap_or(0) || self.imports.contains(&good)
    }
}

/// A city as the supply network sees it.
pub struct CityNode {
    pub position: Hex,
    pub owner: u32,
    pub capital: bool,
    pub harbor: bool,
}

#[derive(Resource, Default)]
pub struct Supplies {
    pub civs: HashMap<u32, CivSupply>,
    /// What each city can draw on, keyed by the city's tile.
    pub city: HashMap<Hex, HashSet<Good>>,
    /// Extra (food, shields, gold) each city gets from farms, mines and bonus resources on its land.
    pub yields: HashMap<Hex, (u32, u32, u32)>,
    /// Owner and governing city of every claimed tile.
    pub territory: HashMap<Hex, (u32, Hex)>,
    /// Sources inside someone's borders that no road links to their capital yet, with the owner.
    pub unconnected: HashMap<Hex, u32>,
    /// Cities joined to their capital's trade network.
    pub linked: HashSet<Hex>,
}

impl Supplies {
    pub fn missing(&self, city: Hex, needs: &[Good]) -> Vec<Good> {
        let have = self.city.get(&city);
        needs.iter().copied().filter(|g| !have.is_some_and(|set| set.contains(g))).collect()
    }

    pub fn has(&self, city: Hex, needs: &[Good]) -> bool {
        self.missing(city, needs).is_empty()
    }

    pub fn luxuries(&self, city: Hex) -> u32 {
        self.city.get(&city).map(|set| set.iter().filter(|g| g.is_luxury()).count() as u32).unwrap_or(0)
    }

    pub fn civ(&self, id: u32) -> CivSupply {
        self.civs.get(&id).cloned().unwrap_or_default()
    }
}

/// Content faces from luxuries; a marketplace makes each further luxury worth more (1, 1, 2, 2, 3, 3...).
pub fn luxury_happiness(count: u32, marketplace: bool) -> u32 {
    if marketplace { (1..=count).map(|i| (i + 1) / 2).sum() } else { count }
}

pub fn recompute<'a>(
    supplies: &mut Supplies,
    tiles: impl Iterator<Item = &'a HexTile>,
    cities: &[CityNode],
    techs: &HashMap<u32, HashSet<Tech>>,
    transfers: &[Transfer],
) {
    *supplies = Supplies::default();
    let tiles: Vec<&HexTile> = tiles.collect();
    let known: HashSet<Hex> = tiles.iter().map(|t| t.coord).collect();

    // Borders: every tile within reach of a city belongs to the nearest one.
    let mut claim: HashMap<Hex, (u32, Hex, i32)> = HashMap::new();
    for city in cities {
        for hex in hexx::shapes::hexagon(city.position, CITY_RADIUS as u32) {
            let dist = hex.distance_to(city.position);
            if known.contains(&hex) && claim.get(&hex).is_none_or(|(_, _, d)| dist < *d) {
                claim.insert(hex, (city.owner, city.position, dist));
            }
        }
    }
    supplies.territory = claim.iter().map(|(hex, (owner, city, _))| (*hex, (*owner, *city))).collect();
    let sees = |owner: u32, good: Good| good.reveal_tech().is_none_or(|t| techs.get(&owner).is_some_and(|known| known.contains(&t)));

    // Yields from the land, and every usable source with its owner.
    let mut usable: Vec<(Hex, Good, u32)> = Vec::new();
    for tile in &tiles {
        let Some((owner, city)) = supplies.territory.get(&tile.coord).copied() else { continue };
        let entry = supplies.yields.entry(city).or_default();
        if tile.improvements.contains(&Improvement::Farm) {
            entry.0 += 1;
        }
        if tile.improvements.contains(&Improvement::Mine) {
            entry.1 += 1;
        }
        let Some(good) = tile.resource else { continue };
        if !sees(owner, good) {
            continue;
        }
        if good.kind() == ResourceKind::Bonus {
            let (food, shields, gold) = good.bonus_yield();
            entry.0 += food;
            entry.1 += shields;
            entry.2 += gold;
        } else {
            *supplies.civs.entry(owner).or_default().sources.entry(good).or_default() += 1;
            usable.push((tile.coord, good, owner));
        }
    }

    // Each civilization's road network: its cities plus roads on its own or unclaimed land,
    // grouped into connected pieces. Harbours join their cities into one piece across the sea.
    struct Network {
        piece: HashMap<Hex, usize>,
        goods: HashMap<usize, HashSet<Good>>,
        capital: Option<usize>,
    }
    let roads: Vec<Hex> = tiles.iter().filter(|t| t.has_road()).map(|t| t.coord).collect();
    let owners: HashSet<u32> = cities.iter().map(|c| c.owner).collect();
    let mut networks: HashMap<u32, Network> = HashMap::new();
    for owner in owners {
        let own: Vec<&CityNode> = cities.iter().filter(|c| c.owner == owner).collect();
        let nodes: HashSet<Hex> = roads
            .iter()
            .copied()
            .filter(|h| supplies.territory.get(h).is_none_or(|(o, _)| *o == owner))
            .chain(own.iter().map(|c| c.position))
            .collect();
        let mut piece: HashMap<Hex, usize> = HashMap::new();
        let mut pieces = 0;
        for start in &nodes {
            if piece.contains_key(start) {
                continue;
            }
            let mut stack = vec![*start];
            piece.insert(*start, pieces);
            while let Some(hex) = stack.pop() {
                for next in hex.all_neighbors() {
                    if nodes.contains(&next) && !piece.contains_key(&next) {
                        piece.insert(next, pieces);
                        stack.push(next);
                    }
                }
            }
            pieces += 1;
        }
        let harbour_pieces: HashSet<usize> = own.iter().filter(|c| c.harbor).filter_map(|c| piece.get(&c.position).copied()).collect();
        if let Some(first) = harbour_pieces.iter().copied().min() {
            for p in piece.values_mut() {
                if harbour_pieces.contains(p) {
                    *p = first;
                }
            }
        }
        let capital = own.iter().max_by_key(|c| c.capital).and_then(|c| piece.get(&c.position).copied());

        let mut goods: HashMap<usize, HashSet<Good>> = HashMap::new();
        for (hex, good, _) in usable.iter().filter(|(_, _, o)| *o == owner) {
            match piece.get(hex).copied() {
                Some(p) => {
                    goods.entry(p).or_default().insert(*good);
                    if Some(p) == capital {
                        *supplies.civs.entry(owner).or_default().connected.entry(*good).or_default() += 1;
                    } else {
                        supplies.unconnected.insert(*hex, owner);
                    }
                }
                None => {
                    supplies.unconnected.insert(*hex, owner);
                }
            }
        }
        for city in &own {
            if piece.get(&city.position).copied() == capital {
                supplies.linked.insert(city.position);
            }
        }
        networks.insert(owner, Network { piece, goods, capital });
    }

    for (seller, buyer, good) in transfers {
        *supplies.civs.entry(*seller).or_default().exports.entry(*good).or_default() += 1;
        supplies.civs.entry(*buyer).or_default().imports.insert(*good);
    }

    for city in cities {
        let supply = supplies.civ(city.owner);
        let Some(network) = networks.get(&city.owner) else { continue };
        let p = network.piece.get(&city.position).copied();
        let mut access = p.and_then(|p| network.goods.get(&p)).cloned().unwrap_or_default();
        if p == network.capital {
            // Goods sold abroad leave the capital's network; goods bought abroad arrive there.
            access.retain(|g| supply.has(*g));
            access.extend(supply.imports.iter().copied());
        }
        supplies.city.insert(city.position, access);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_civ::terrain::Terrain;

    fn tile(q: i32, r: i32, road: bool, resource: Option<Good>) -> HexTile {
        HexTile { coord: Hex::new(q, r), terrain: Terrain::Plains, improvements: if road { vec![Improvement::Road] } else { vec![] }, resource }
    }

    fn city(q: i32, r: i32, owner: u32, capital: bool, harbor: bool) -> CityNode {
        CityNode { position: Hex::new(q, r), owner, capital, harbor }
    }

    fn all_techs(owners: &[u32]) -> HashMap<u32, HashSet<Tech>> {
        owners.iter().map(|o| (*o, Tech::ALL.into_iter().collect())).collect()
    }

    /// A strip of tiles from x = 0 to `len`, with iron at x = 2.
    fn strip(len: i32, road_on_iron: bool, road_between: bool) -> Vec<HexTile> {
        (0..=len).map(|q| tile(q, 0, if q == 2 { road_on_iron } else { q == 1 && road_between }, (q == 2).then_some(Good::Iron))).collect()
    }

    #[test]
    fn source_needs_a_road_to_the_city() {
        let cities = [city(0, 0, PLAYER, true, false)];
        let techs = all_techs(&[PLAYER]);
        let mut supplies = Supplies::default();

        let tiles = strip(3, false, false);
        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[]);
        assert_eq!(supplies.civ(PLAYER).sources.get(&Good::Iron), Some(&1));
        assert!(!supplies.has(Hex::ZERO, &[Good::Iron]), "no road, no iron");
        assert_eq!(supplies.unconnected.get(&Hex::new(2, 0)), Some(&PLAYER));

        let tiles = strip(3, true, false);
        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[]);
        assert!(!supplies.has(Hex::ZERO, &[Good::Iron]), "a road on the source alone does not reach the city");

        let tiles = strip(3, true, true);
        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[]);
        assert!(supplies.has(Hex::ZERO, &[Good::Iron]));
        assert!(supplies.unconnected.is_empty());
    }

    #[test]
    fn hidden_until_the_technology_is_known() {
        let cities = [city(0, 0, PLAYER, true, false)];
        let tiles = strip(3, true, true);
        let mut supplies = Supplies::default();
        recompute(&mut supplies, tiles.iter(), &cities, &HashMap::new(), &[]);
        assert!(!supplies.has(Hex::ZERO, &[Good::Iron]));
    }

    #[test]
    fn harbours_link_distant_cities() {
        // Two player cities ten tiles apart; iron is roaded to the first only.
        let mut tiles = strip(12, true, true);
        tiles.retain(|t| t.coord.x <= 3 || t.coord.x >= 9);
        let techs = all_techs(&[PLAYER]);
        let mut supplies = Supplies::default();

        let cities = [city(0, 0, PLAYER, true, false), city(11, 0, PLAYER, false, false)];
        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[]);
        assert!(supplies.has(Hex::ZERO, &[Good::Iron]));
        assert!(!supplies.has(Hex::new(11, 0), &[Good::Iron]));

        let cities = [city(0, 0, PLAYER, true, true), city(11, 0, PLAYER, false, true)];
        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[]);
        assert!(supplies.has(Hex::new(11, 0), &[Good::Iron]));
    }

    #[test]
    fn deals_move_goods_between_civilizations() {
        let rival = 1;
        // The rival has two iron sources (so one to spare); the player has none.
        let mut tiles = strip(3, false, false);
        tiles[2].resource = None;
        tiles.extend([tile(20, 0, false, None), tile(21, 0, true, Some(Good::Iron)), tile(22, 0, true, Some(Good::Iron))]);
        let cities = [city(0, 0, PLAYER, true, false), city(20, 0, rival, true, false)];
        let techs = all_techs(&[PLAYER, rival]);
        let mut supplies = Supplies::default();

        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[]);
        assert_eq!(supplies.civ(rival).spare(Good::Iron), 1);
        assert!(!supplies.has(Hex::ZERO, &[Good::Iron]));

        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[(rival, PLAYER, Good::Iron)]);
        assert!(supplies.has(Hex::ZERO, &[Good::Iron]), "imports arrive in the capital");
        assert_eq!(supplies.civ(rival).spare(Good::Iron), 0);
        assert!(supplies.has(Hex::new(20, 0), &[Good::Iron]), "the seller keeps one for itself");
    }

    #[test]
    fn computer_players_need_roads_too() {
        let rival = 1;
        let cities = [city(0, 0, rival, true, false)];
        let techs = all_techs(&[rival]);
        let mut supplies = Supplies::default();

        let tiles = strip(3, false, false);
        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[]);
        assert!(!supplies.has(Hex::ZERO, &[Good::Iron]));
        assert_eq!(supplies.unconnected.get(&Hex::new(2, 0)), Some(&rival));

        let tiles = strip(3, true, true);
        recompute(&mut supplies, tiles.iter(), &cities, &techs, &[]);
        assert!(supplies.has(Hex::ZERO, &[Good::Iron]));
    }

    #[test]
    fn roads_across_unclaimed_land_link_cities() {
        // Cities eight tiles apart leave unclaimed land between their borders.
        let roaded: Vec<HexTile> = (0..=8).map(|q| tile(q, 0, q != 0 && q != 8, (q == 1).then_some(Good::Wine))).collect();
        let cities = [city(0, 0, PLAYER, true, false), city(8, 0, PLAYER, false, false)];
        let mut supplies = Supplies::default();
        recompute(&mut supplies, roaded.iter(), &cities, &all_techs(&[PLAYER]), &[]);
        assert!(supplies.linked.contains(&Hex::new(8, 0)));
        assert!(supplies.has(Hex::new(8, 0), &[Good::Wine]));

        // A rival's border in the gap breaks the route.
        let cities = [city(0, 0, PLAYER, true, false), city(8, 0, PLAYER, false, false), city(4, 0, 1, true, false)];
        recompute(&mut supplies, roaded.iter(), &cities, &all_techs(&[PLAYER, 1]), &[]);
        assert!(!supplies.linked.contains(&Hex::new(8, 0)));
    }

    #[test]
    fn marketplace_multiplies_luxuries() {
        assert_eq!(luxury_happiness(4, false), 4);
        assert_eq!(luxury_happiness(4, true), 6);
        assert_eq!(luxury_happiness(8, true), 20);
    }
}
