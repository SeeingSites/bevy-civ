use bevy::prelude::*;
use crate::city::City;
use crate::building::CityBuildings;
use crate::government::Government;
use crate::trade::TradeRoutes;

#[derive(Resource)]
pub struct Economy {
    pub inflation: f32,
    pub maintenance_per_building: u32,
    pub total_maintenance: i32,
    pub total_trade_gold: i32,
}

impl Default for Economy {
    fn default() -> Self {
        Self {
            inflation: 1.0,
            maintenance_per_building: 1,
            total_maintenance: 0,
            total_trade_gold: 0,
        }
    }
}

impl Economy {
    pub fn process_turn(
        &mut self,
        cities: &Query<(&City, &CityBuildings)>,
        trade: &TradeRoutes,
        gold: &mut i32,
        gov: Government,
    ) {
        // Inflation slowly increases
        self.inflation += 0.005;
        if self.inflation > 1.5 {
            self.inflation = 1.5;
        }

        // Building maintenance
        let mut total = 0i32;
        for (city, buildings) in cities.iter() {
            if city.owner == 0 {
                let maint = buildings.buildings.len() as i32 * self.maintenance_per_building as i32;
                total += maint;
            }
        }

        let gov_mod = match gov {
            Government::Communism => 0.75,
            Government::Democracy => 1.25,
            Government::Republic => 1.1,
            _ => 1.0,
        };

        self.total_maintenance = (total as f32 * gov_mod * self.inflation) as i32;
        self.total_trade_gold = trade.total_gold() as i32;

        *gold += self.total_trade_gold;
        *gold -= self.total_maintenance;
        if *gold < 0 {
            *gold = 0;
        }
    }

    pub fn unit_cost_modifier(&self) -> f32 {
        self.inflation
    }
}
