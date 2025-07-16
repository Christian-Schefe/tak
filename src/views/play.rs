use crate::components::tak_board_state::{PlayerInfo, PlayerType, TakBoardState};
use crate::components::{TakBoard, TakWebSocket, TakWinModal};
use crate::server::room::{get_room, GetRoomResponse};
use crate::views::{get_session_id, LOCAL_SETTINGS};
use crate::Route;
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
    let mut player_info = HashMap::new();
    player_info.insert(
        TakPlayer::White,
        PlayerInfo::new("You".to_string(), PlayerType::Local),
    );
    player_info.insert(
        TakPlayer::Black,
        PlayerInfo::new("Computer".to_string(), PlayerType::Local),
    );

    let mut state = use_context_provider(|| TakBoardState::new(player_info));
    let board_clone = state.clone();

    use_effect(move || {
        let settings = LOCAL_SETTINGS.peek().clone();
        state
            .try_set_from_settings(settings)
            .expect("Settings should be valid");
        state.has_started.set(true);
    });

    let show_board = use_memo(move || {
        let _ = board_clone.on_change.read();
        dioxus::logger::tracing::info!("Checking if board has game, {}", board_clone.has_game());
        board_clone.has_game()
    });

    rsx! {
        div { id: "play-view",
            if *show_board.read() {
                TakBoard {}
                TakWinModal { is_local: true }
            }
        }
    }
}

#[component]
pub fn PlayOnline() -> Element {
    let room = use_resource(|| get_room());
    let session_id = use_resource(|| get_session_id());

    let nav = use_navigator();

    let player_info = HashMap::new();
    let board = use_context_provider(|| TakBoardState::new(player_info));
    let board_clone = board.clone();

    use_effect(move || {
        let mut board = board_clone.clone();
        spawn(async move {
            board.update_player_info().await;
        });
    });

    let room_id = use_memo(move || {
        if let Some(Ok(GetRoomResponse::Success(id, _))) = room.read().as_ref() {
            Some(id.clone())
        } else {
            None
        }
    });

    let mut board_clone = board.clone();
    use_effect(move || {
        dioxus::logger::tracing::info!("room: {:?}", room.read());
        match room.read().as_ref() {
            Some(Ok(GetRoomResponse::Success(_, settings))) => {
                board_clone.try_set_from_settings(settings.game_settings.clone());
            }
            Some(Ok(GetRoomResponse::Unauthorized)) => {
                nav.replace(Route::Auth {});
            }
            Some(Ok(GetRoomResponse::NotInARoom)) => {
                nav.replace(Route::Home {});
            }
            _ => {}
        }
    });

    let board_clone = board.clone();
    let show_board = use_memo(move || {
        let _ = board_clone.on_change.read();
        dioxus::logger::tracing::info!("Checking if board has game, {}", board_clone.has_game());
        board_clone.has_game()
    });

    rsx! {
        div { id: "play-view",
            if let Some(room) = room_id.read().as_ref() {
                h2 { "Room ID: {room}" }
                if *show_board.read() {
                    TakBoard {
                    }
                    TakWinModal { is_local: false }
                    if let Some(Ok(Some(session_id))) = session_id.read().as_ref() {
                        TakWebSocket { session_id }
                    }
                }
            } else {
                h2 { "No room found or not connected." }
            }
        }
    }
}
