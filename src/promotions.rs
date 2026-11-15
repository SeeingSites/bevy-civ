use bevy::prelude::*;
use crate::unit::Unit;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Promotion {
    Veteran,
    Elite,
    CombatI,
    CombatII,
    Mobility,
    Siege,
}

impl Promotion {
    pub fn name(&self) -> &'static str {
        match self {
            Promotion::Veteran => "Veteran",
            Promotion::Elite => "Elite",
            Promotion::CombatI => "Combat I",
            Promotion::CombatII => "Combat II",
            Promotion::Mobility => "Mobility",
            Promotion::Siege => "Siege",
        }
    }

    pub fn attack_bonus(&self) -> f32 {
        match self {
            Promotion::Veteran => 0.15,
            Promotion::Elite => 0.25,
            Promotion::CombatI => 0.2,
            Promotion::CombatII => 0.3,
            Promotion::Siege => 0.4,
            _ => 0.0,
        }
    }

    pub fn defense_bonus(&self) -> f32 {
        match self {
            Promotion::Veteran => 0.1,
            Promotion::Elite => 0.2,
            Promotion::CombatI => 0.15,
            Promotion::CombatII => 0.25,
            _ => 0.0,
        }
    }

    pub fn movement_bonus(&self) -> u32 {
        match self {
            Promotion::Mobility => 1,
            _ => 0,
        }
    }
}

#[derive(Component, Default)]
pub struct UnitPromotions {
    pub promotions: Vec<Promotion>,
    pub xp: u32,
}

impl UnitPromotions {
    pub fn add_xp(&mut self, amount: u32) {
        self.xp += amount;
        // Auto-promote at thresholds
        if self.xp >= 3 && !self.has(Promotion::Veteran) {
            self.promotions.push(Promotion::Veteran);
        }
        if self.xp >= 10 && !self.has(Promotion::Elite) {
            self.promotions.push(Promotion::Elite);
        }
    }

    pub fn has(&self, promo: Promotion) -> bool {
        self.promotions.contains(&promo)
    }

    pub fn total_attack_bonus(&self) -> f32 {
        self.promotions.iter().map(|p| p.attack_bonus()).sum()
    }

    pub fn total_defense_bonus(&self) -> f32 {
        self.promotions.iter().map(|p| p.defense_bonus()).sum()
    }

    pub fn total_movement_bonus(&self) -> u32 {
        self.promotions.iter().map(|p| p.movement_bonus()).sum()
    }
}
