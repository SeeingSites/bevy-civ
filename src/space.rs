use bevy::prelude::*;
use crate::tech::{Tech, TechState};

#[derive(Resource, Default)]
pub struct SpaceRace {
    pub components: Vec<SpaceComponent>,
    pub progress: u32,
}

#[derive(Clone, Debug)]
pub enum SpaceComponent {
    Thrusters,
    LifeSupport,
    Habitation,
    SolarPanels,
}

impl SpaceComponent {
    pub fn cost(&self) -> u32 {
        match self {
            SpaceComponent::Thrusters => 100,
            SpaceComponent::LifeSupport => 80,
            SpaceComponent::Habitation => 120,
            SpaceComponent::SolarPanels => 90,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SpaceComponent::Thrusters => "Thrusters",
            SpaceComponent::LifeSupport => "Life Support",
            SpaceComponent::Habitation => "Habitation",
            SpaceComponent::SolarPanels => "Solar Panels",
        }
    }
}

impl SpaceRace {
    pub fn can_build(&self, tech: &TechState) -> bool {
        tech.researched.contains(&Tech::Engineering) && tech.researched.contains(&Tech::Mathematics)
    }

    pub fn add_component(&mut self, component: SpaceComponent) {
        if !self.components.iter().any(|c| std::mem::discriminant(c) == std::mem::discriminant(&component)) {
            self.components.push(component);
            self.progress += 1;
            info!("Space component completed. Progress: {}/4", self.progress);
        }
    }

    pub fn is_complete(&self) -> bool {
        self.progress >= 4
    }
}
