use bevy::prelude::*;
use crate::city::City;
use crate::building::CityBuildings;
use crate::government::Government;

#[derive(Resource)]
pub struct Economy {
    pub inflation: f32,
    pub maintenance_per_building: u32,
    pub total_maintenance: i32,
}

impl Default for Economy {
    fn default() -> Self {
        Self {
            inflation: 1.0,
            maintenance_per_building: 1,
            total_maintenance: 0,
        }
    }
}

impl Economy {
    pub fn process_turn(&mut self, cities: &Query<(&City, &CityBuildings)>, gold: &mut i32, gov: Government) {
        // Inflation slowly increases
        self.inflation += 0.01;
        if self.inflation > 2.0 {
            self.inflation = 2.0;
        }

        // Calculate maintenance
        let mut total = 0i32;
        for (city, buildings) in cities.iter() {
            if city.owner == 0 {
                let maint = buildings.buildings.len() as i32 * self.maintenance_per_building as i32;
                total += maint;
            }
        }

        // Government modifier
        let gov_mod = match gov {
            Government::Communism => 0.8,
            Government::Democracy => 1.2,
            _ => 1.0,
        };

        self.total_maintenance = (total as f32 * gov_mod * self.inflation) as i32;
        *gold -= self.total_maintenance;
        if *gold < 0 {
            *gold = 0;
        }
    }

    pub fn unit_cost_modifier(&self) -> f32 {
        self.inflation
    }
}
