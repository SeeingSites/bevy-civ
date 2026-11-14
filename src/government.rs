use bevy::prelude::*;

#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub enum Government {
    Despotism,
    Monarchy,
    Republic,
    Democracy,
    Communism,
    Fundamentalism,
}

impl Government {
    pub fn name(&self) -> &'static str {
        match self {
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
            Government::Despotism => 0.8,
            Government::Monarchy => 1.0,
            Government::Republic => 1.2,
            Government::Democracy => 1.5,
            Government::Communism => 1.0,
            Government::Fundamentalism => 0.7,
        }
    }

    /// Free unit support per city
    pub fn free_unit_support(&self) -> u32 {
        match self {
            Government::Despotism => 1,
            Government::Monarchy => 2,
            Government::Republic => 0,
            Government::Democracy => 0,
            Government::Communism => 3,
            Government::Fundamentalism => 4,
        }
    }

    /// Corruption factor (higher = more corruption far from capital)
    pub fn corruption_factor(&self) -> f32 {
        match self {
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
            Government::Despotism | Government::Monarchy => 1,
            Government::Republic | Government::Democracy => 2,
            Government::Communism => 1,
            Government::Fundamentalism => 0,
        }
    }
}

impl Default for Government {
    fn default() -> Self {
        Government::Despotism
    }
}
