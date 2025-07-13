use dioxus::prelude::*;
use tak_core::{TakGameState, TakPlayer, TakWinReason};

use crate::{
    components::tak_board_state::TakBoardState,
    server::room::{leave_room, LeaveRoomResponse},
    Route,
};

#[component]
pub fn TakWinModal(is_local: bool) -> Element {
    let state = use_context::<TakBoardState>();
    let nav = use_navigator();

    let data = use_memo(move || {
        let _ = state.on_change.read();
        state
            .with_game(|game| match &game.game().game_state {
                TakGameState::Ongoing => None,
                TakGameState::Win(player, reason) => Some(Some((*player, reason.clone()))),
                TakGameState::Draw => Some(None),
            })
            .expect("Should have game state")
    });

    let data = data.read();
    let Some(data) = data.as_ref() else {
        return rsx! {};
    };

    let message = match data {
        Some((player, reason)) => {
            let player_str = match player {
                TakPlayer::White => "White",
                TakPlayer::Black => "Black",
            };
            match reason {
                TakWinReason::Flat => format!("{} wins by flats!", player_str),
                TakWinReason::Road => format!("{} wins by road!", player_str),
                TakWinReason::Timeout => format!("{} wins by timeout!", player_str),
            }
        }
        None => "It's a draw!".to_string(),
    };

    let on_click_leave = move |_| {
        if is_local {
            nav.push(Route::Home {});
            return;
        }
        spawn(async move {
            let res = leave_room().await;
            match res {
                Ok(LeaveRoomResponse::Unauthorized) => {
                    nav.push(Route::Auth {});
                }
                Ok(_) => {
                    nav.push(Route::Home {});
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to leave room: {}", e);
                }
            }
        });
    };

    rsx! {
        div { class: "tak-win-modal",
            div { class: "tak-win-modal-content",
                p { class: "tak-win-message", "{message}" }
                button {
                    onclick: on_click_leave,
                    "Leave"
                }
            }
        }
    }
}
