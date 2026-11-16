use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Resource, Clone)]
pub struct TechState {
    pub researched: HashSet<Tech>,
    pub current: Option<Tech>,
    pub progress: u32,
    pub science_per_turn: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tech {
    // Ancient
    Pottery,
    AnimalHusbandry,
    Mining,
    BronzeWorking,
    Masonry,
    Alphabet,
    TheWheel,
    Writing,
    CodeOfLaws,
    Mysticism,
    Seafaring,
    // Classical / Medieval
    IronWorking,
    Mathematics,
    HorsebackRiding,
    Construction,
    Philosophy,
    Monarchy,
    Currency,
    Engineering,
    // Later placeholders
    Gunpowder,
    PrintingPress,
    Democracy,
    // Industrial
    SteamPower,
    Industrialization,
    Refining,
    ReplaceableParts,
    MotorizedTransport,
}

impl Tech {
    pub const ALL: [Tech; 27] = [
        Tech::Pottery, Tech::AnimalHusbandry, Tech::Mining, Tech::BronzeWorking, Tech::Masonry,
        Tech::Alphabet, Tech::TheWheel, Tech::Writing, Tech::CodeOfLaws, Tech::Mysticism, Tech::Seafaring,
        Tech::IronWorking, Tech::Mathematics, Tech::HorsebackRiding, Tech::Construction,
        Tech::Philosophy, Tech::Monarchy, Tech::Currency, Tech::Engineering, Tech::Gunpowder,
        Tech::PrintingPress, Tech::Democracy, Tech::SteamPower, Tech::Industrialization, Tech::Refining,
        Tech::ReplaceableParts, Tech::MotorizedTransport,
    ];

    pub fn cost(&self) -> u32 {
        match self {
            Tech::Pottery | Tech::AnimalHusbandry | Tech::Mining => 10,
            Tech::BronzeWorking | Tech::Masonry | Tech::Alphabet | Tech::TheWheel => 15,
            Tech::Writing | Tech::CodeOfLaws | Tech::Mysticism | Tech::Seafaring => 20,
            Tech::IronWorking | Tech::Mathematics | Tech::HorsebackRiding => 25,
            Tech::Construction | Tech::Philosophy | Tech::Monarchy | Tech::Currency => 30,
            Tech::Engineering => 40,
            Tech::Gunpowder | Tech::PrintingPress => 50,
            Tech::Democracy => 60,
            Tech::SteamPower => 60,
            Tech::Industrialization => 70,
            Tech::Refining | Tech::ReplaceableParts => 80,
            Tech::MotorizedTransport => 90,
        }
    }

    pub fn prerequisites(&self) -> Vec<Tech> {
        match self {
            Tech::Writing => vec![Tech::Alphabet],
            Tech::CodeOfLaws => vec![Tech::Writing],
            Tech::Mysticism => vec![Tech::Pottery],
            Tech::Seafaring => vec![Tech::Pottery],
            Tech::IronWorking => vec![Tech::BronzeWorking],
            Tech::Mathematics => vec![Tech::Writing, Tech::Masonry],
            Tech::HorsebackRiding => vec![Tech::AnimalHusbandry],
            Tech::Construction => vec![Tech::Masonry],
            Tech::Philosophy => vec![Tech::Mysticism, Tech::Writing],
            Tech::Monarchy => vec![Tech::CodeOfLaws],
            Tech::Currency => vec![Tech::CodeOfLaws],
            Tech::Engineering => vec![Tech::Mathematics, Tech::Construction],
            Tech::Gunpowder => vec![Tech::IronWorking, Tech::Engineering],
            Tech::PrintingPress => vec![Tech::Writing],
            Tech::Democracy => vec![Tech::Philosophy, Tech::PrintingPress],
            Tech::SteamPower => vec![Tech::Gunpowder],
            Tech::Industrialization => vec![Tech::SteamPower, Tech::Currency],
            Tech::Refining => vec![Tech::Industrialization],
            Tech::ReplaceableParts => vec![Tech::Industrialization],
            Tech::MotorizedTransport => vec![Tech::Refining, Tech::ReplaceableParts],
            _ => vec![],
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Tech::Pottery => "Pottery",
            Tech::AnimalHusbandry => "Animal Husbandry",
            Tech::Mining => "Mining",
            Tech::BronzeWorking => "Bronze Working",
            Tech::Masonry => "Masonry",
            Tech::Alphabet => "Alphabet",
            Tech::TheWheel => "The Wheel",
            Tech::Writing => "Writing",
            Tech::CodeOfLaws => "Code of Laws",
            Tech::Mysticism => "Mysticism",
            Tech::Seafaring => "Seafaring",
            Tech::IronWorking => "Iron Working",
            Tech::Mathematics => "Mathematics",
            Tech::HorsebackRiding => "Horseback Riding",
            Tech::Construction => "Construction",
            Tech::Philosophy => "Philosophy",
            Tech::Monarchy => "Monarchy",
            Tech::Currency => "Currency",
            Tech::Engineering => "Engineering",
            Tech::Gunpowder => "Gunpowder",
            Tech::PrintingPress => "Printing Press",
            Tech::Democracy => "Democracy",
            Tech::SteamPower => "Steam Power",
            Tech::Industrialization => "Industrialization",
            Tech::Refining => "Refining",
            Tech::ReplaceableParts => "Replaceable Parts",
            Tech::MotorizedTransport => "Motorized Transport",
        }
    }
}

impl Default for TechState {
    fn default() -> Self {
        Self {
            researched: HashSet::new(),
            current: Some(Tech::Pottery),
            progress: 0,
            science_per_turn: 2,
        }
    }
}

impl TechState {
    /// A fresh state that already knows the named technologies.
    pub fn knowing(names: &[String]) -> Self {
        let mut state = Self { researched: HashSet::new(), current: None, progress: 0, science_per_turn: 2 };
        state.researched.extend(Tech::ALL.into_iter().filter(|t| names.iter().any(|n| n == t.name())));
        state.current = Tech::ALL.into_iter().find(|t| state.can_research(*t));
        state
    }

    /// Adds a technology gained by trade and moves research on if it was the current project.
    pub fn learn(&mut self, tech: Tech) {
        self.researched.insert(tech);
        if self.current == Some(tech) {
            self.progress = 0;
            self.current = Tech::ALL.into_iter().find(|t| self.can_research(*t));
        }
    }

    /// Research cost of a technology right now: each discovery makes the next ones dearer.
    pub fn cost_of(&self, tech: Tech) -> u32 {
        tech.cost() * (2 + self.researched.len() as u32) / 2
    }

    pub fn can_research(&self, tech: Tech) -> bool {
        !self.researched.contains(&tech)
            && tech.prerequisites().iter().all(|p| self.researched.contains(p))
    }

    pub fn process_turn(&mut self) -> Option<Tech> {
        let Some(current) = self.current else { return None };
        self.progress += self.science_per_turn;
        if self.progress >= self.cost_of(current) {
            self.researched.insert(current);
            self.progress = 0;
            // Auto-pick the next available technology; the player can change it.
            let next = Tech::ALL.into_iter().find(|t| self.can_research(*t));
            self.current = next;
            Some(current)
        } else {
            None
        }
    }
}
