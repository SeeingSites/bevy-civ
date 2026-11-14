use bevy::prelude::*;
use hexx::Hex;
use crate::city::City;

#[derive(Resource, Default)]
pub struct TradeRoutes {
    pub routes: Vec<TradeRoute>,
}

#[derive(Clone)]
pub struct TradeRoute {
    pub from: Hex,
    pub to: Hex,
    pub gold_per_turn: u32,
    pub science_per_turn: u32,
}

impl TradeRoutes {
    pub fn add_route(&mut self, from: Hex, to: Hex) {
        if !self.routes.iter().any(|r| (r.from == from && r.to == to) || (r.from == to && r.to == from)) {
            self.routes.push(TradeRoute {
                from,
                to,
                gold_per_turn: 3,
                science_per_turn: 1,
            });
            info!("Trade route established between {:?} and {:?}", from, to);
        }
    }

    pub fn total_gold(&self) -> u32 {
        self.routes.iter().map(|r| r.gold_per_turn).sum()
    }

    pub fn total_science(&self) -> u32 {
        self.routes.iter().map(|r| r.science_per_turn).sum()
    }
}
