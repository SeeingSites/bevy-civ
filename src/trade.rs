//! Trade routes, opened by caravans.
//!
//! A caravan that reaches another city opens a route from its home city: between two of the owner's
//! own cities (if they are far enough apart) or to a foreign city at peace. A route pays its owner
//! gold and science every turn, more the longer it is and the bigger the two cities; foreign routes
//! pay half again as much, and the foreign partner earns half the gold too. War closes foreign
//! routes, and a route ends when either city changes hands.

use bevy::prelude::*;
use hexx::Hex;

/// Own cities must be at least this far apart to trade with each other.
pub const MIN_DOMESTIC_DISTANCE: i32 = 6;
/// Routes a city can have, counting both ends.
pub const MAX_ROUTES_PER_CITY: usize = 3;

#[derive(Resource, Default, Clone, Debug)]
pub struct TradeRoutes {
    pub routes: Vec<TradeRoute>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TradeRoute {
    pub from: Hex,
    pub to: Hex,
    /// Whose caravan opened it (and who gets the full income).
    pub owner: u32,
    /// Who owns the far city (the owner again for a domestic route).
    pub partner: u32,
    pub gold_per_turn: u32,
    pub science_per_turn: u32,
}

impl TradeRoute {
    pub fn is_foreign(&self) -> bool {
        self.owner != self.partner
    }
}

/// What a route pays its owner each turn: (gold, science).
pub fn route_yield(distance: i32, population_from: u32, population_to: u32, foreign: bool) -> (u32, u32) {
    let base = 1 + distance.max(0) as u32 / 4 + (population_from + population_to) / 6;
    let gold = if foreign { base * 3 / 2 } else { base };
    (gold.max(1), (gold / 2).max(1))
}

impl TradeRoutes {
    /// Routes touching a city.
    pub fn at(&self, city: Hex) -> usize {
        self.routes.iter().filter(|r| r.from == city || r.to == city).count()
    }

    /// Why a route from `from` (owned by `owner`) to `to` (owned by `partner`) cannot be opened, if it cannot.
    pub fn refusal(&self, from: Hex, to: Hex, owner: u32, partner: u32) -> Option<&'static str> {
        if from == to {
            return Some("A caravan must reach a different city from its home to open a trade route.");
        }
        if owner == partner && from.distance_to(to) < MIN_DOMESTIC_DISTANCE {
            return Some("Your own cities must be at least 6 tiles apart to trade with each other.");
        }
        if self.routes.iter().any(|r| (r.from == from && r.to == to) || (r.from == to && r.to == from)) {
            return Some("These two cities already trade with each other.");
        }
        if self.at(from) >= MAX_ROUTES_PER_CITY || self.at(to) >= MAX_ROUTES_PER_CITY {
            return Some("One of these cities already has as many trade routes as it can handle (3).");
        }
        None
    }

    /// Opens a route. Returns it, or why it cannot be opened.
    pub fn establish(&mut self, from: (Hex, u32, u32), to: (Hex, u32, u32)) -> Result<TradeRoute, &'static str> {
        let ((from_hex, owner, from_pop), (to_hex, partner, to_pop)) = (from, to);
        if let Some(reason) = self.refusal(from_hex, to_hex, owner, partner) {
            return Err(reason);
        }
        let (gold, science) = route_yield(from_hex.distance_to(to_hex), from_pop, to_pop, owner != partner);
        let route = TradeRoute { from: from_hex, to: to_hex, owner, partner, gold_per_turn: gold, science_per_turn: science };
        self.routes.push(route.clone());
        Ok(route)
    }

    /// What `civ` earns from trade each turn: (gold, science). Partners in foreign routes get half the gold.
    pub fn income(&self, civ: u32) -> (u32, u32) {
        self.routes.iter().fold((0, 0), |(gold, science), r| {
            if r.owner == civ {
                (gold + r.gold_per_turn, science + r.science_per_turn)
            } else if r.partner == civ {
                (gold + r.gold_per_turn / 2, science)
            } else {
                (gold, science)
            }
        })
    }

    /// Gold the player earns from trade (kept for older callers).
    pub fn total_gold(&self) -> u32 {
        self.income(0).0
    }

    /// Science the player earns from trade (kept for older callers).
    pub fn total_science(&self) -> u32 {
        self.income(0).1
    }

    /// Closes routes whose cities changed hands or whose two owners went to war. Returns them.
    pub fn prune(&mut self, owner_of: impl Fn(Hex) -> Option<u32>, at_war: impl Fn(u32, u32) -> bool) -> Vec<TradeRoute> {
        let (keep, closed): (Vec<_>, Vec<_>) = std::mem::take(&mut self.routes)
            .into_iter()
            .partition(|r| owner_of(r.from) == Some(r.owner) && owner_of(r.to) == Some(r.partner) && !(r.is_foreign() && at_war(r.owner, r.partner)));
        self.routes = keep;
        closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longer_bigger_and_foreign_routes_pay_more() {
        let near = route_yield(6, 2, 2, false);
        assert_eq!(near, (2, 1));
        assert!(route_yield(16, 2, 2, false).0 > near.0);
        assert!(route_yield(6, 8, 8, false).0 > near.0);
        assert!(route_yield(6, 2, 2, true).0 > near.0);
    }

    #[test]
    fn routes_need_distance_and_room_and_are_not_repeated() {
        let mut trade = TradeRoutes::default();
        let home = Hex::ZERO;
        assert!(trade.establish((home, 0, 3), (Hex::new(3, 0), 0, 3)).is_err(), "own cities too close");
        assert!(trade.establish((home, 0, 3), (Hex::new(3, 0), 1, 3)).is_ok(), "foreign cities may be close");
        assert!(trade.establish((Hex::new(3, 0), 0, 3), (home, 1, 3)).is_err(), "same pair again");
        assert!(trade.establish((home, 0, 3), (Hex::new(8, 0), 0, 3)).is_ok());
        assert!(trade.establish((home, 0, 3), (Hex::new(0, 8), 0, 3)).is_ok());
        assert!(trade.establish((home, 0, 3), (Hex::new(-8, 0), 0, 3)).is_err(), "home already has three");
    }

    #[test]
    fn partners_share_the_gold_and_war_closes_foreign_routes() {
        let mut trade = TradeRoutes::default();
        let route = trade.establish((Hex::ZERO, 0, 6), (Hex::new(9, 0), 1, 6)).unwrap();
        assert_eq!(trade.income(0), (route.gold_per_turn, route.science_per_turn));
        assert_eq!(trade.income(1), (route.gold_per_turn / 2, 0));
        let owners = |h: Hex| Some(if h == Hex::ZERO { 0 } else { 1 });
        assert!(trade.prune(owners, |_, _| false).is_empty());
        assert_eq!(trade.prune(owners, |_, _| true).len(), 1);
        assert!(trade.routes.is_empty());
    }
}
