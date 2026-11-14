use bevy::prelude::*;

#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub enum Government {
    Despotism,   // default early
    Monarchy,
    Republic,
    Democracy,
    Communism,   // placeholder
}

impl Government {
    pub fn name(&self) -> &'static str {
        match self {
            Government::Despotism => "Despotism",
            Government::Monarchy => "Monarchy",
            Government::Republic => "Republic",
            Government::Democracy => "Democracy",
            Government::Communism => "Communism",
        }
    }

    /// Production multiplier (1.0 = normal)
    pub fn production_modifier(&self) -> f32 {
        match self {
            Government::Despotism => 1.0,
            Government::Monarchy => 1.1,
            Government::Republic => 1.0,
            Government::Democracy => 0.9,
            Government::Communism => 1.15,
        }
    }

    /// Free unit support per city (Civ-style)
    pub fn free_unit_support(&self) -> u32 {
        match self {
            Government::Despotism => 1,
            Government::Monarchy => 2,
            Government::Republic => 0,
            Government::Democracy => 0,
            Government::Communism => 3,
        }
    }

    /// Science modifier
    pub fn science_modifier(&self) -> f32 {
        match self {
            Government::Despotism => 0.8,
            Government::Monarchy => 1.0,
            Government::Republic => 1.2,
            Government::Democracy => 1.5,
            Government::Communism => 1.0,
        }
    }
}

impl Default for Government {
    fn default() -> Self {
        Government::Despotism
    }
}
