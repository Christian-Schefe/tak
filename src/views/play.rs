use crate::components::tak_board_state::{PlayerInfo, PlayerType, TakBoardState};
use crate::components::{TakBoard, TakWebSocket};
use crate::server::room::{get_room, GetRoomResponse};
use crate::tak::{TakPlayer, TakSettings};
use crate::views::get_session_id;
use crate::Route;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    let settings = TakSettings::default();
    let mut board = use_context_provider(|| TakBoardState::new(settings, player_info));

    use_effect(move || {
        board.has_started.set(true);
    });

    rsx! {
        div {
            id: "play-view",
            TakBoard {}
        }
    }
}

#[component]
pub fn PlayOnline() -> Element {
    let room = use_resource(|| get_room());
    let session_id = use_resource(|| get_session_id());

    let nav = use_navigator();

    let player_info = HashMap::new();
    let board = use_context_provider(|| TakBoardState::new(TakSettings::default(), player_info));
    let mut board_clone = board.clone();

    use_effect(move || {
        let mut board = board.clone();
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

    use_effect(move || {
        dioxus::logger::tracing::info!("room: {:?}", room.read());
        match room.read().as_ref() {
            Some(Ok(GetRoomResponse::Success(_, settings))) => {
                board_clone.replace_settings_if_not_started(&settings.game_settings);
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

    rsx! {
        div {
            id: "play-view",
            if let Some(room) = room_id.read().as_ref() {
                h2 {
                    "Room ID: {room}"
                }
                TakBoard {
                }
                if let Some(Ok(Some(session_id))) = session_id.read().as_ref() {
                    TakWebSocket {session_id}
                }
            } else {
                h2 { "No room found or not connected." }
            }
        }
    }
}
