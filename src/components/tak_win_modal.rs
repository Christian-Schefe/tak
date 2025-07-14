use dioxus::prelude::*;
use tak_core::{TakGameState, TakPlayer, TakWinReason};
use web_sys::window;

use crate::{
    components::tak_board_state::TakBoardState,
    server::room::{leave_room, LeaveRoomResponse},
    Route,
};

#[component]
pub fn TakWinModal(is_local: bool) -> Element {
    let state = use_context::<TakBoardState>();
    let nav = use_navigator();

    let state_clone = state.clone();

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

    let on_click_copy_ptn = move |_| {
        state_clone
            .with_game(|game| {
                let ptn = game.game().to_ptn().to_str();
                copy_to_clipboard(&ptn);
                dioxus::logger::tracing::info!("PTN copied to clipboard: {}", ptn);
            })
            .expect("Should be able to copy PTN");
    };

    rsx! {
        div { class: "tak-win-modal",
            div { class: "tak-win-modal-content",
                p { class: "tak-win-message", "{message}" }
                button {
                    onclick: on_click_leave,
                    "Leave"
                }
                button {
                    onclick: on_click_copy_ptn,
                    "Copy PTN"
                }
            }
        }
    }
}

fn copy_to_clipboard(text: &str) {
    let Some(window) = window() else {
        dioxus::logger::tracing::error!("Window not available for clipboard access");
        return;
    };
    let navigator = window.navigator();

    let clipboard = navigator.clipboard();
    let promise = clipboard.write_text(text);
    wasm_bindgen_futures::spawn_local(async move {
        match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(_) => dioxus::logger::tracing::info!("Copied to clipboard"),
            Err(err) => dioxus::logger::tracing::error!("Failed to copy: {:?}", err),
        }
    });
}
