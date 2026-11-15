use bevy::prelude::*;

#[derive(Resource)]
pub struct MultiplayerState {
    pub mode: GameMode,
    pub current_player: u32,
    pub player_count: u32,
    pub turn_phase: TurnPhase,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameMode {
    SinglePlayer,
    Hotseat,
    Network, // placeholder for future
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TurnPhase {
    PlayerTurn,
    AITurn,
    EndTurn,
}

impl Default for MultiplayerState {
    fn default() -> Self {
        Self {
            mode: GameMode::SinglePlayer,
            current_player: 0,
            player_count: 1,
            turn_phase: TurnPhase::PlayerTurn,
        }
    }
}

impl MultiplayerState {
    pub fn set_hotseat(&mut self, players: u32) {
        self.mode = GameMode::Hotseat;
        self.player_count = players.max(2);
        self.current_player = 0;
        self.turn_phase = TurnPhase::PlayerTurn;
    }

    pub fn next_player(&mut self) -> u32 {
        self.current_player = (self.current_player + 1) % self.player_count;
        if self.current_player == 0 {
            self.turn_phase = TurnPhase::EndTurn;
        }
        self.current_player
    }

    pub fn is_human_turn(&self) -> bool {
        matches!(self.mode, GameMode::Hotseat) && self.current_player == 0
            || matches!(self.mode, GameMode::SinglePlayer)
    }

    pub fn status(&self) -> String {
        match self.mode {
            GameMode::SinglePlayer => "Single Player".into(),
            GameMode::Hotseat => format!("Hotseat: Player {}", self.current_player + 1),
            GameMode::Network => "Network (not implemented)".into(),
        }
    }
}
