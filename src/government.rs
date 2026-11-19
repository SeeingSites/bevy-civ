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

    /// What science is worth under this government: Fundamentalism halves it, and Anarchy does no
    /// research at all.
    pub fn science_modifier(&self) -> f32 {
        match self {
            Government::Anarchy => 0.0,
            Government::Fundamentalism => 0.5,
            _ => 1.0,
        }
    }

    /// Free unit support per city
    pub fn free_unit_support(&self) -> u32 {
        match self {
            Government::Anarchy => 2,
            Government::Despotism => 1,
            Government::Monarchy => 2,
            Government::Republic => 0,
            Government::Democracy => 0,
            Government::Communism => 3,
            Government::Fundamentalism => 4,
        }
    }

    /// How heavy corruption and waste are: the share of trade a city loses at the full distance
    /// from its palace and at the optimal number of cities (see `economy::corruption`).
    pub fn corruption_level(&self) -> f32 {
        match self {
            Government::Anarchy => 0.8,
            Government::Despotism => 0.6,
            Government::Monarchy => 0.45,
            Government::Communism => 0.4,
            Government::Republic | Government::Fundamentalism => 0.35,
            Government::Democracy => 0.2,
        }
    }

    /// Communism spreads corruption evenly: distance from the palace does not matter.
    pub fn communal(&self) -> bool {
        *self == Government::Communism
    }

    /// The most of a people's trade that may go to any one of science, luxury and taxes (percent).
    /// Anarchy collects no taxes and does no research.
    pub fn rate_cap(&self) -> u32 {
        match self {
            Government::Anarchy => 0,
            Government::Despotism => 60,
            Government::Monarchy => 70,
            Government::Communism | Government::Republic | Government::Fundamentalism => 80,
            Government::Democracy => 90,
        }
    }

    /// The limit on any one rate (see `rate_cap`).
    pub fn science_cap(&self) -> u32 {
        self.rate_cap()
    }

    /// Despotism and Anarchy take one from any tile that would give 3 or more food, shields or trade.
    pub fn tile_penalty(&self) -> bool {
        matches!(self, Government::Anarchy | Government::Despotism)
    }

    /// A Republic or Democracy adds a trade to every worked tile that already brings some.
    pub fn commerce_bonus(&self) -> bool {
        matches!(self, Government::Republic | Government::Democracy)
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
            Government::Anarchy => "No taxes or science, rampant corruption, poor land",
            Government::Despotism => "Rich tiles give one less; heavy corruption; rates up to 60%; martial law; forced labour",
            Government::Monarchy => "Moderate corruption; rates up to 70%; martial law",
            Government::Republic => "+1 trade on tiles with trade; low corruption; rates up to 80%; war weariness",
            Government::Democracy => "+1 trade on tiles with trade; least corruption; rates up to 90%; war weariness",
            Government::Communism => "Corruption the same near and far; rates up to 80%; strong martial law; forced labour",
            Government::Fundamentalism => "+3 content everywhere, cheap armies; science halved; rates up to 80%",
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
