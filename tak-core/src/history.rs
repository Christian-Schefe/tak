use crate::{TakActionRecord, TakDrawReason, TakGame, TakGameSettings, TakGameState, TakWinReason};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameRecord {
    settings: TakGameSettings,
    actions: Vec<TakActionRecord>,
    result: TakGameState,
}

impl GameRecord {
    pub fn from_game(game: TakGame) -> Self {
        let settings = game.settings.clone();
        let actions = game.action_history.clone();
        let result = game.game_state.clone();
        Self {
            settings,
            actions,
            result,
        }
    }
    pub fn to_game(&self) -> TakGame {
        let mut game =
            TakGame::new(self.settings.clone()).expect("Failed to create game from settings");
        for action in &self.actions {
            game.try_do_action_record(action)
                .expect("Failed to apply action to game");
        }
        if game.game_state != self.result {
            let is_mismatch_allowed = game.game_state == TakGameState::Ongoing
                && match self.result {
                    TakGameState::Win(_, TakWinReason::Resignation) => true,
                    TakGameState::Draw(TakDrawReason::Agreement) => true,
                    TakGameState::Canceled => true,
                    _ => false,
                };
            if !is_mismatch_allowed {
                panic!(
                    "Game state mismatch: {:?} != {:?}",
                    game.game_state, self.result
                );
            }
            game.game_state = self.result.clone();
        }
        game
    }
}
