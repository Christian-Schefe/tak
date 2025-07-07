use crate::components::tak_board_state::{PlayerInfo, PlayerType, TakBoardState};
use crate::components::{TakBoard, TakWebSocket};
use crate::server::room::{get_room, GetPlayersResponse, GetRoomResponse};
use crate::tak::action::TakActionResult;
use crate::tak::{
    TakCoord, TakFeedback, TakGameState, TakPieceType, TakPlayer, TimeMode, TimedTakGame,
};
use crate::views::get_session_id;
use crate::Route;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientGameMessage {
    Move(String),
}

const CSS: Asset = asset!("/assets/styling/board.css");

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
    let mut board = use_context_provider(|| TakBoardState::new(5, player_info));

    use_effect(move || {
        board.has_started.set(true);
    });

    rsx! {
        document::Link { rel: "stylesheet", href: CSS }
        div {
            id: "play-view",
            TakBoard {}
        }
    }
}

#[component]
pub fn PlayOnline() -> Element {
    let room = use_server_future(|| get_room())?;
    let session_id = use_server_future(|| get_session_id())?;

    let nav = use_navigator();

    let player_info = HashMap::new();
    let board = use_context_provider(|| TakBoardState::new(5, player_info));

    use_effect(move || {
        let mut board = board.clone();
        spawn(async move {
            board.update_player_info().await;
        });
    });

    let room_id = use_memo(move || {
        if let Some(Ok(GetRoomResponse::Success(id))) = room.read().as_ref() {
            Some(id.clone())
        } else {
            None
        }
    });

    use_effect(move || {
        println!("room: {:?}", room.read());
        match room.read().as_ref() {
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
        document::Link { rel: "stylesheet", href: CSS }
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
                } else {
                    {format!("{:?}", session_id.read())}
                }
            } else {
                h2 { "No room found or not connected." }
            }
        }
    }
}
