use std::collections::HashMap;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use tak_core::{TakGame, TakPtn};

use crate::{
    components::{tak_board_state::TakBoardState, TakBoard},
    server::PlayerInformation,
};

#[component]
pub fn ReviewBoard(game_id: String) -> Element {
    let mut state = use_context_provider(|| {
        let mut player_info = HashMap::new();
        TakBoardState::new(player_info)
    });

    let board_clone = state.clone();

    use_effect(move || {
        let settings = LOCAL_SETTINGS.peek().clone();
        state
            .try_set_from_settings(settings.game_settings)
            .expect("Settings should be valid");
        state.has_started.set(true);
    });

    let show_board = use_memo(move || {
        let _ = board_clone.on_change.read();
        board_clone.has_game()
    });

    rsx! {
        div { id: "review-board-view",
            if *show_board.read() {
                TakBoard {}
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum GetGameResult {
    Success {
        game: TakGame,
        white_player: PlayerInformation,
        black_player: PlayerInformation,
    },
    NotFound,
}

#[server]
async fn get_game(game_id: String) -> Result<GetGameResult, ServerFnError> {
    let Ok(game) = crate::server::player::get_game(&game_id).await else {
        return Ok(GetGameResult::NotFound);
    };
    let Some(tak_game) = TakPtn::try_from_str(&game.ptn).and_then(|ptn| TakGame::try_from_ptn(ptn))
    else {
        return Ok(GetGameResult::NotFound);
    };
    Ok(GetGameResult::Success {
        game: tak_game,
        white_player: PlayerInformation {
            user_id: String::from("white_user"),
            username: String::from("White Player"),
            rating: 1500.0,
        },
        black_player: PlayerInformation {
            user_id: String::from("black_user"),
            username: String::from("Black Player"),
            rating: 1500.0,
        },
    })
}
