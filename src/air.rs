use bevy::prelude::*;
use hexx::Hex;

#[derive(Component)]
pub struct AirUnit {
    pub unit_type: AirType,
    pub movement_left: u32,
    pub owner: u32,
    pub position: Hex,
    pub fuel: u32,
    pub max_fuel: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AirType {
    Fighter,
    Bomber,
    Helicopter,
    JetFighter,
    StealthBomber,
}

impl AirType {
    pub fn stats(&self) -> (u32, u32, u32, u32) {
        // attack, defense, movement, fuel
        match self {
            AirType::Fighter => (4, 2, 6, 4),
            AirType::Bomber => (8, 1, 4, 3),
            AirType::Helicopter => (3, 2, 4, 3),
            AirType::JetFighter => (8, 4, 8, 3),
            AirType::StealthBomber => (12, 2, 6, 2),
        }
    }

    pub fn cost(&self) -> u32 {
        match self {
            AirType::Fighter => 60,
            AirType::Bomber => 80,
            AirType::Helicopter => 70,
            AirType::JetFighter => 120,
            AirType::StealthBomber => 150,
        }
    }

    pub fn required_tech(&self) -> Option<&'static str> {
        match self {
            AirType::Fighter => Some("Flight"),
            AirType::Bomber => Some("Flight"),
            AirType::Helicopter => Some("Advanced Flight"),
            AirType::JetFighter => Some("Rocketry"),
            AirType::StealthBomber => Some("Stealth"),
        }
    }

    pub fn is_bomber(&self) -> bool {
        matches!(self, AirType::Bomber | AirType::StealthBomber)
    }
}

impl AirUnit {
    pub fn can_reach(&self, target: Hex) -> bool {
        let dist = self.position.distance_to(target) as u32;
        dist <= self.movement_left && dist <= self.fuel
    }

    pub fn consume_fuel(&mut self, amount: u32) {
        self.fuel = self.fuel.saturating_sub(amount);
    }

    pub fn refuel(&mut self) {
        self.fuel = self.max_fuel;
    }

    pub fn resolve_air_combat(self_type: AirType, enemy_type: AirType) -> bool {
        let (atk, _, _, _) = self_type.stats();
        let (_, def, _, _) = enemy_type.stats();
        let atk_roll = atk as f32 * (0.8 + rand::random::<f32>() * 0.4);
        let def_roll = def as f32 * (0.8 + rand::random::<f32>() * 0.4);
        atk_roll > def_roll
    }

    /// Bombing damage against a city or ground unit (returns damage amount).
    pub fn bomb_damage(self_type: AirType) -> u32 {
        if self_type.is_bomber() {
            self_type.stats().0 / 2
        } else {
            0
        }
    }
}
