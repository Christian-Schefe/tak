use crate::Route;
use crate::components::tak_board_state::{PlayerInfo, PlayerType, TakBoardState};
use crate::components::{TakBoard, TakEngine, TakWebSocket, TakWinModal, TakWinModalLocal};
use crate::server::api::get_match;
use crate::server::{MatchId, ServerError};
use crate::views::LOCAL_SETTINGS;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tak_core::TakPlayer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientGameMessage {
    Move(String),
}

#[component]
pub fn PlayComputer() -> Element {
    let mut state = use_context_provider(|| {
        let mut player_info = HashMap::new();
        let mut player_order = vec![TakPlayer::White, TakPlayer::Black];
        let should_swap = match LOCAL_SETTINGS.peek().first_player_mode {
            Some(tak_player) => tak_player != player_order[0],
            None => rand::random(),
        };
        dioxus::logger::tracing::info!(
            "Should swap players: {}, settings: {:?}",
            should_swap,
            LOCAL_SETTINGS.peek()
        );
        if should_swap {
            player_order.reverse();
        }
        player_info.insert(
            player_order[0],
            PlayerInfo::new("You".to_string(), PlayerType::Local, None),
        );
        player_info.insert(
            player_order[1],
            PlayerInfo::new("Computer".to_string(), PlayerType::Computer, None),
        );
        TakBoardState::new(player_info)
    });

    let board_clone = state.clone();

    use_effect(move || {
        let settings = LOCAL_SETTINGS.peek().clone();
        state
            .try_set_from_settings(settings.game_settings)
            .expect("Settings should be valid");
    });

    let show_board = use_memo(move || {
        let _ = board_clone.on_change.read();
        board_clone.has_game()
    });

    rsx! {
        div { id: "play-view",
            if *show_board.read() {
                TakBoard {}
                TakWinModalLocal {}
                TakEngine {}
            }
        }
    }
}

#[component]
pub fn PlayLocal() -> Element {
    let mut state = use_context_provider(|| {
        let mut player_info = HashMap::new();
        let mut player_order = vec![TakPlayer::White, TakPlayer::Black];
        let should_swap = match LOCAL_SETTINGS.peek().first_player_mode {
            Some(tak_player) => tak_player != player_order[0],
            None => rand::random(),
        };
        dioxus::logger::tracing::info!(
            "Should swap players: {}, settings: {:?}",
            should_swap,
            LOCAL_SETTINGS.peek()
        );
        if should_swap {
            player_order.reverse();
        }
        player_info.insert(
            player_order[0],
            PlayerInfo::new("Player 1".to_string(), PlayerType::Local, None),
        );
        player_info.insert(
            player_order[1],
            PlayerInfo::new("Player 2".to_string(), PlayerType::Local, None),
        );
        TakBoardState::new(player_info)
    });

    let board_clone = state.clone();

    use_effect(move || {
        let settings = LOCAL_SETTINGS.peek().clone();
        state
            .try_set_from_settings(settings.game_settings)
            .expect("Settings should be valid");
        state.trigger_change();
    });

    let show_board = use_memo(move || {
        let _ = board_clone.on_change.read();
        board_clone.has_game()
    });

    rsx! {
        div { id: "play-view",
            if *show_board.read() {
                TakBoard {}
                TakWinModalLocal {}
                TakEngine {}
            }
        }
    }
}

#[component]
pub fn PlayOnline(match_id: MatchId) -> Element {
    let match_id_clone = match_id.clone();
    let player_match = use_resource(move || get_match(match_id_clone.clone()));

    let nav = use_navigator();

    let player_info = HashMap::new();
    let board = use_context_provider(|| TakBoardState::new(player_info));
    let board_clone = board.clone();

    use_effect(move || {
        let mut board = board_clone.clone();
        spawn(async move {
            board.update_from_remote().await;
        });
    });

    let mut board_clone = board.clone();
    use_effect(move || {
        dioxus::logger::tracing::info!("room: {:?}", player_match.read());
        match player_match.read().as_ref() {
            Some(Ok(Ok(instance))) => {
                board_clone.try_set_from_settings(instance.game_settings.clone());
            }
            Some(Ok(Err(ServerError::Unauthorized))) => {
                nav.replace(Route::Auth {});
            }
            Some(Ok(Err(ServerError::NotFound))) => {
                nav.replace(Route::Home {});
            }
            _ => {}
        }
    });

    let board_clone = board.clone();
    let show_board = use_memo(move || {
        let _ = board_clone.on_change.read();
        board_clone.has_game()
    });

    rsx! {
        div { id: "play-view",
            if let Some(_) = player_match.read().as_ref() {
                if *show_board.read() {
                    TakBoard {
                    }
                    TakWinModal { match_id: match_id.clone() }
                    TakWebSocket { match_id: match_id.clone() }
                }
            } else {
                h2 { "No room found or not connected." }
            }
        }
    }
}
