use bevy::prelude::*;
use crate::unit::Unit;

#[derive(Resource, Default)]
pub struct GreatLeaders {
    pub available: u32,
}

impl GreatLeaders {
    pub fn on_elite_win(&mut self) {
        // 10% chance to generate a great leader on elite unit victory
        if rand::random::<f32>() < 0.1 {
            self.available += 1;
            info!("Great Leader generated! Total: {}", self.available);
        }
    }

    pub fn form_army(&mut self) -> bool {
        if self.available > 0 {
            self.available -= 1;
            true
        } else {
            false
        }
    }
}

#[derive(Component)]
pub struct Army {
    pub units: Vec<Entity>,
    pub leader_bonus: f32,
}

impl Army {
    pub fn new(leader_bonus: f32) -> Self {
        Self {
            units: vec![],
            leader_bonus,
        }
    }
}
