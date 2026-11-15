use bevy::prelude::*;
use hexx::Hex;
use crate::city::City;
use crate::building::CityBuildings;

#[derive(Component)]
pub struct Culture {
    pub points: u32,
    pub border_radius: u32,
    pub per_turn: u32,
}

impl Culture {
    pub fn new() -> Self {
        Self {
            points: 0,
            border_radius: 1,
            per_turn: 1,
        }
    }

    pub fn add(&mut self, amount: u32) {
        self.points += amount;
        self.border_radius = 1 + self.points / 50;
        if self.border_radius > 4 {
            self.border_radius = 4;
        }
    }

    pub fn contains(&self, city_pos: Hex, hex: Hex) -> bool {
        city_pos.distance_to(hex) <= self.border_radius as i32
    }

    /// Culture generated this turn from population + buildings.
    pub fn calculate_yield(city: &City, buildings: &CityBuildings) -> u32 {
        let base = 1 + city.population / 2;
        let building_bonus = buildings.buildings.iter()
            .filter(|b| b.is_wonder() || matches!(b, crate::building::Building::Library | crate::building::Building::University))
            .count() as u32 * 2;
        base + building_bonus
    }
}

#[derive(Resource, Default)]
pub struct CultureMap {
    pub owner_at: std::collections::HashMap<Hex, u32>,
}

pub fn update_culture_borders(
    cities: Query<(&City, &Culture)>,
    mut culture_map: ResMut<CultureMap>,
) {
    culture_map.owner_at.clear();
    for (city, culture) in &cities {
        for dx in -(culture.border_radius as i32)..=(culture.border_radius as i32) {
            for dy in -(culture.border_radius as i32)..=(culture.border_radius as i32) {
                let hex = Hex::new(city.position.x + dx, city.position.y + dy);
                if culture.contains(city.position, hex) {
                    culture_map.owner_at.insert(hex, city.owner);
                }
            }
        }
    }
}

pub fn process_culture_turn(
    mut cities: Query<(&City, &CityBuildings, &mut Culture)>,
) {
    for (city, buildings, mut culture) in &mut cities {
        let yield_amount = Culture::calculate_yield(city, buildings);
        culture.per_turn = yield_amount;
        culture.add(yield_amount);
    }
}
