use bevy::prelude::*;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum Government {
    /// The turns of upheaval between two governments: little gets done.
    Anarchy,
    Despotism,
    Monarchy,
    Republic,
    Democracy,
    Communism,
    Fundamentalism,
}

impl Government {
    /// Citizens made unhappy in every city by a war that has lasted `turns` turns: the people of a
    /// republic, and more so a democracy, tire of fighting. Other governments do not mind.
    pub fn war_weariness(&self, turns: u32) -> u32 {
        match self {
            Government::Democracy => (turns / 8).min(3),
            Government::Republic => (turns / 12).min(2),
            _ => 0,
        }
    }

    pub fn name(&self) -> &'static str {
        crate::theme::rename(crate::theme::Kind::Government, self, self.base_name())
    }

    /// The name in the default (Earth) theme.
    pub fn base_name(&self) -> &'static str {
        match self {
            Government::Anarchy => "Anarchy",
            Government::Despotism => "Despotism",
            Government::Monarchy => "Monarchy",
            Government::Republic => "Republic",
            Government::Democracy => "Democracy",
            Government::Communism => "Communism",
            Government::Fundamentalism => "Fundamentalism",
        }
    }

    pub fn production_modifier(&self) -> f32 {
        match self {
            Government::Anarchy => 0.75,
            Government::Despotism => 1.0,
            Government::Monarchy => 1.1,
            Government::Republic => 1.0,
            Government::Democracy => 0.9,
            Government::Communism => 1.15,
            Government::Fundamentalism => 1.05,
        }
    }

    pub fn science_modifier(&self) -> f32 {
        match self {
            Government::Anarchy => 0.0,
            Government::Despotism => 0.8,
            Government::Monarchy => 1.0,
            Government::Republic => 1.2,
            Government::Democracy => 1.5,
            Government::Communism => 1.0,
            Government::Fundamentalism => 0.7,
        }
    }

    /// Military units each city supports free (Civilization III's figures: four under Despotism,
    /// three under Monarchy and Communism, none under a Republic or Democracy).
    pub fn free_unit_support(&self) -> u32 {
        match self {
            Government::Anarchy => 2,
            Government::Despotism => 4,
            Government::Monarchy => 3,
            Government::Republic => 0,
            Government::Democracy => 0,
            Government::Communism => 3,
            Government::Fundamentalism => 4,
        }
    }

    /// Corruption factor (higher = more corruption far from capital)
    pub fn corruption_factor(&self) -> f32 {
        match self {
            Government::Anarchy => 0.25,
            Government::Despotism => 0.15,
            Government::Monarchy => 0.08,
            Government::Republic => 0.05,
            Government::Democracy => 0.02,
            Government::Communism => 0.06,
            Government::Fundamentalism => 0.04,
        }
    }

    /// Science rate cap (percentage)
    pub fn science_cap(&self) -> u32 {
        match self {
            Government::Anarchy => 0,
            Government::Despotism => 60,
            Government::Monarchy => 70,
            Government::Republic => 80,
            Government::Democracy => 100,
            Government::Communism => 70,
            Government::Fundamentalism => 50,
        }
    }

    pub fn unit_upkeep_food(&self) -> u32 {
        match self {
            Government::Anarchy | Government::Despotism | Government::Monarchy => 1,
            Government::Republic | Government::Democracy => 2,
            Government::Communism => 1,
            Government::Fundamentalism => 0,
        }
    }
}

impl Government {
    /// Every government a civilization can choose (not Anarchy).
    pub const CHOICES: [Government; 6] =
        [Government::Despotism, Government::Monarchy, Government::Republic, Government::Democracy, Government::Communism, Government::Fundamentalism];

    /// Turns of anarchy a revolution costs.
    pub const REVOLUTION_TURNS: u32 = 2;

    /// The technology that makes this government available.
    pub fn required_tech(&self) -> Option<crate::tech::Tech> {
        use crate::tech::Tech;
        match self {
            Government::Anarchy | Government::Despotism => None,
            Government::Monarchy => Some(Tech::Monarchy),
            Government::Republic => Some(Tech::TheRepublic),
            Government::Democracy => Some(Tech::Democracy),
            Government::Communism => Some(Tech::Communism),
            Government::Fundamentalism => Some(Tech::Fundamentalism),
        }
    }

    pub fn available(&self, tech: &crate::tech::TechState) -> bool {
        *self != Government::Anarchy && self.required_tech().is_none_or(|t| tech.researched.contains(&t))
    }

    /// Military units in a city that each keep one unhappy citizen in line.
    pub fn martial_law(&self) -> u32 {
        match self {
            Government::Anarchy | Government::Despotism | Government::Monarchy => 3,
            Government::Communism => 4,
            _ => 0,
        }
    }

    /// Despotism and Communism let a city rush its project by working citizens to death.
    /// Citizens can be drafted into the army (once Gunpowder is known).
    pub fn allows_draft(&self) -> bool {
        matches!(self, Government::Monarchy | Government::Communism | Government::Fundamentalism | Government::Republic)
    }

    pub fn allows_forced_labour(&self) -> bool {
        matches!(self, Government::Despotism | Government::Communism)
    }

    /// Extra content citizens in every city.
    pub fn content_bonus(&self) -> u32 {
        match self {
            Government::Fundamentalism => 3,
            _ => 0,
        }
    }

    /// In a few words, for the government advisor.
    pub fn summary(&self) -> &'static str {
        match self {
            Government::Anarchy => "No science, poor production, rampant corruption",
            Government::Despotism => "Slow science, heavy corruption; martial law",
            Government::Monarchy => "+10% shields, steady science; martial law",
            Government::Republic => "+20% science, low corruption; units cost upkeep",
            Government::Democracy => "+50% science, least corruption; units cost upkeep",
            Government::Communism => "+15% shields, even corruption; strong martial law",
            Government::Fundamentalism => "+3 content everywhere, cheap armies; poor science",
        }
    }

    /// The government a computer player prefers among those it knows.
    pub fn preferred(tech: &crate::tech::TechState, aggression: f32) -> Government {
        let order: &[Government] = if aggression > 0.6 {
            &[Government::Fundamentalism, Government::Communism, Government::Monarchy, Government::Republic, Government::Despotism]
        } else {
            &[Government::Democracy, Government::Republic, Government::Communism, Government::Monarchy, Government::Despotism]
        };
        order.iter().copied().find(|g| g.available(tech)).unwrap_or(Government::Despotism)
    }
}

impl Default for Government {
    fn default() -> Self {
        Government::Despotism
    }
}

#[cfg(test)]
mod weariness_tests {
    use super::*;

    #[test]
    fn only_republics_and_democracies_tire_of_war() {
        assert_eq!(Government::Monarchy.war_weariness(100), 0);
        assert_eq!(Government::Republic.war_weariness(11), 0);
        assert_eq!(Government::Republic.war_weariness(24), 2);
        assert_eq!(Government::Democracy.war_weariness(16), 2);
        assert_eq!(Government::Democracy.war_weariness(200), 3);
    }
}
