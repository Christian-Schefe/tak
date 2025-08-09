use dioxus::prelude::*;
use tak_core::{TakGameState, TakPlayer, TakWinReason};
use web_sys::window;
use ws_pubsub::use_ws_topic_receive;

use crate::{
    Route,
    components::tak_board_state::{GameType, TakBoardState},
    server::{
        ServerError,
        api::{
            MATCHES_TOPIC, MyServerFunctions, REMATCH_SUBTOPIC, agree_rematch, get_match_info,
            leave_match, retract_rematch,
        },
    },
};

#[component]
pub fn TakWinModal(match_id: String) -> Element {
    let state = use_context::<TakBoardState>();
    let nav = use_navigator();

    let mut match_data = use_resource(|| get_match_info());

    let state_clone = state.clone();

    let data = use_memo(move || {
        let _ = state_clone.on_change.read();
        state_clone
            .with_game(|game| game.game().game_state.clone())
            .expect("Should have game state")
    });

    let mut on_rematch_update = move || {
        match_data.restart();
    };

    use_ws_topic_receive::<_, MyServerFunctions, _>(
        format!("{}/{}/{}", MATCHES_TOPIC, match_id, REMATCH_SUBTOPIC),
        move |_: ()| async move {
            on_rematch_update();
        },
    );

    let rematch_data = use_memo(move || match match_data.read().as_ref() {
        Some(Ok(Ok((player_id, _, _, _, data)))) => {
            if data.rematch_agree.contains(player_id) {
                ("Cancel", false)
            } else {
                ("Rematch", false)
            }
        }
        Some(Ok(Err(ServerError::NotFound))) => ("Opponent left", true),
        Some(_) => ("Loading...", true),
        None => ("Rematch", false),
    });

    let (rematch_button_text, rematch_button_disabled) = rematch_data.read().clone();

    let data = data.read();
    if let TakGameState::Ongoing = &*data {
        return rsx! {};
    };

    let message = match &*data {
        TakGameState::Win(player, reason) => {
            let player_str = match player {
                TakPlayer::White => "White",
                TakPlayer::Black => "Black",
            };
            match reason {
                TakWinReason::Flat => format!("{} wins by flats!", player_str),
                TakWinReason::Road => format!("{} wins by road!", player_str),
                TakWinReason::Timeout => format!("{} wins by timeout!", player_str),
                TakWinReason::Resignation => {
                    format!("{} wins by resignation!", player_str)
                }
            }
        }
        TakGameState::Draw(_) => "It's a draw!".to_string(),
        TakGameState::Canceled => "The game was canceled.".to_string(),
        TakGameState::Ongoing => unreachable!(),
    };

    let on_click_leave = move |_| {
        if let Some(Ok(Err(ServerError::NotFound))) = match_data.read().as_ref() {
            nav.push(Route::Home {});
            return;
        }
        spawn(async move {
            let res = leave_match().await;
            match res {
                Ok(Err(ServerError::Unauthorized)) => {
                    nav.push(Route::Auth {});
                }
                Ok(Ok(())) => {
                    nav.push(Route::Home {});
                }
                Ok(Err(e)) => {
                    dioxus::logger::tracing::error!("Failed to leave room: {}", e);
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to leave room: {}", e);
                }
            }
        });
    };

    let state_clone = state.clone();
    let on_click_copy_ptn = move |_| {
        state_clone
            .with_game(|game| {
                let ptn = game.game().to_ptn().to_str();
                copy_to_clipboard(&ptn);
                dioxus::logger::tracing::info!("PTN copied to clipboard: {}", ptn);
            })
            .expect("Should be able to copy PTN");
    };

    let state_clone = state.clone();
    let show_rematch_button = use_memo(move || state_clone.get_game_type() != GameType::Spectated);

    let on_click_rematch = move |_| {
        spawn(async move {
            let res = if let Some(Ok(Ok((player_id, _, _, _, data)))) = match_data.read().as_ref()
                && data.rematch_agree.contains(player_id)
            {
                retract_rematch().await
            } else {
                agree_rematch().await
            };
            match res {
                Ok(Err(ServerError::Unauthorized)) => {
                    nav.push(Route::Auth {});
                }
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    dioxus::logger::tracing::error!("Failed to agree to rematch: {}", e);
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to agree to rematch: {}", e);
                }
            }
        });
    };

    rsx! {
        div { class: "tak-win-modal",
            div { class: "tak-win-modal-content",
                p { class: "tak-win-message", "{message}" }
                button { onclick: on_click_leave, "Leave" }
                button { onclick: on_click_copy_ptn, "Copy PTN" }
                if *show_rematch_button.read() {
                    button {
                        onclick: on_click_rematch,
                        disabled: rematch_button_disabled,
                        {rematch_button_text}
                    }
                }
            }
        }
    }
}

#[component]
pub fn TakWinModalLocal() -> Element {
    let state = use_context::<TakBoardState>();
    let nav = use_navigator();

    let state_clone = state.clone();

    let data = use_memo(move || {
        let _ = state_clone.on_change.read();
        state_clone
            .with_game(|game| game.game().game_state.clone())
            .expect("Should have game state")
    });

    let data = data.read();
    if let TakGameState::Ongoing = &*data {
        return rsx! {};
    };

    let message = match &*data {
        TakGameState::Win(player, reason) => {
            let player_str = match player {
                TakPlayer::White => "White",
                TakPlayer::Black => "Black",
            };
            match reason {
                TakWinReason::Flat => format!("{} wins by flats!", player_str),
                TakWinReason::Road => format!("{} wins by road!", player_str),
                TakWinReason::Timeout => format!("{} wins by timeout!", player_str),
                TakWinReason::Resignation => format!("{} wins by resignation!", player_str),
            }
        }
        TakGameState::Draw(_) => "It's a draw!".to_string(),
        TakGameState::Canceled => "The game was canceled.".to_string(),
        TakGameState::Ongoing => unreachable!(),
    };

    let on_click_leave = move |_| {
        nav.push(Route::Home {});
    };

    let state_clone = state.clone();
    let on_click_copy_ptn = move |_| {
        state_clone
            .with_game(|game| {
                let ptn = game.game().to_ptn().to_str();
                copy_to_clipboard(&ptn);
                dioxus::logger::tracing::info!("PTN copied to clipboard: {}", ptn);
            })
            .expect("Should be able to copy PTN");
    };

    let state_clone = state.clone();
    let show_rematch_button = use_memo(move || state_clone.get_game_type() != GameType::Spectated);

    let mut state_clone = state.clone();
    let on_click_rematch = move |_| {
        state_clone.reset();
    };

    rsx! {
        div { class: "tak-win-modal",
            div { class: "tak-win-modal-content",
                p { class: "tak-win-message", "{message}" }
                button { onclick: on_click_leave, "Leave" }
                button { onclick: on_click_copy_ptn, "Copy PTN" }
                if *show_rematch_button.read() {
                    button { onclick: on_click_rematch, "Rematch" }
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
