use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Resource)]
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
}

impl Tech {
    pub fn cost(&self) -> u32 {
        match self {
            Tech::Pottery | Tech::AnimalHusbandry | Tech::Mining => 10,
            Tech::BronzeWorking | Tech::Masonry | Tech::Alphabet | Tech::TheWheel => 15,
            Tech::Writing | Tech::CodeOfLaws | Tech::Mysticism => 20,
            Tech::IronWorking | Tech::Mathematics | Tech::HorsebackRiding => 25,
            Tech::Construction | Tech::Philosophy | Tech::Monarchy | Tech::Currency => 30,
            Tech::Engineering => 40,
            Tech::Gunpowder | Tech::PrintingPress => 50,
            Tech::Democracy => 60,
        }
    }

    pub fn prerequisites(&self) -> Vec<Tech> {
        match self {
            Tech::Writing => vec![Tech::Alphabet],
            Tech::CodeOfLaws => vec![Tech::Writing],
            Tech::Mysticism => vec![Tech::Pottery],
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
    pub fn can_research(&self, tech: Tech) -> bool {
        !self.researched.contains(&tech)
            && tech.prerequisites().iter().all(|p| self.researched.contains(p))
    }

    pub fn process_turn(&mut self) -> Option<Tech> {
        let Some(current) = self.current else { return None };
        self.progress += self.science_per_turn;
        if self.progress >= current.cost() {
            self.researched.insert(current);
            self.progress = 0;
            // Auto-pick next available (simple)
            let next = [
                Tech::AnimalHusbandry,
                Tech::Mining,
                Tech::BronzeWorking,
                Tech::Alphabet,
                Tech::Masonry,
                Tech::Writing,
                Tech::TheWheel,
            ]
            .into_iter()
            .find(|t| self.can_research(*t));
            self.current = next;
            Some(current)
        } else {
            None
        }
    }
}
