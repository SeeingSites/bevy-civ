use bevy::prelude::*;
use crate::tech::TechState;

#[derive(Resource, Default)]
pub struct Espionage {
    pub spies: u32,
    pub stolen_techs: Vec<String>,
    pub sabotage_count: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EspionageAction {
    StealTech,
    SabotageProduction,
    StealGold,
    InciteUnrest,
}

impl EspionageAction {
    pub fn cost(&self) -> u32 {
        match self {
            EspionageAction::StealTech => 50,
            EspionageAction::SabotageProduction => 30,
            EspionageAction::StealGold => 40,
            EspionageAction::InciteUnrest => 60,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EspionageAction::StealTech => "Steal Technology",
            EspionageAction::SabotageProduction => "Sabotage Production",
            EspionageAction::StealGold => "Steal Gold",
            EspionageAction::InciteUnrest => "Incite Unrest",
        }
    }
}

impl Espionage {
    pub fn recruit_spy(&mut self, gold: &mut i32) -> bool {
        if *gold >= 100 {
            *gold -= 100;
            self.spies += 1;
            true
        } else {
            false
        }
    }

    pub fn attempt(&mut self, action: EspionageAction, target_tech: &TechState, gold: &mut i32) -> bool {
        if self.spies == 0 || *gold < action.cost() as i32 {
            return false;
        }

        *gold -= action.cost() as i32;
        self.spies -= 1;

        let success = rand::random::<f32>() < 0.6;
        if success {
            match action {
                EspionageAction::StealTech => {
                    if let Some(tech) = target_tech.researched.iter().next() {
                        self.stolen_techs.push(format!("{:?}", tech));
                    }
                }
                EspionageAction::StealGold => {
                    *gold += 80;
                }
                EspionageAction::SabotageProduction => {
                    self.sabotage_count += 1;
                }
                EspionageAction::InciteUnrest => {}
            }
        }
        success
    }
}
