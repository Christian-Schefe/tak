use crate::components::tak_board_state::TakBoardState;
use crate::server::room::{get_game_state, GetGameStateResponse};
use dioxus::core_macro::component;
use dioxus::prelude::*;
use futures_util::{SinkExt, StreamExt};
use gloo::net::websocket::futures::WebSocket;
use gloo::net::websocket::{Message, WebSocketError};
use tak_core::{TakAction, TakGameState, TakPlayer};
use wasm_bindgen_futures::spawn_local;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ServerGameMessage {
    StartGame,
    Move(usize, Vec<(TakPlayer, u64)>, String),
    GameOver(TakGameState),
}

#[component]
pub fn TakWebSocket(session_id: String) -> Element {
    let mut board = use_context::<TakBoardState>();

    let board_clone = board.clone();

    let handle_game_message = move |board: &mut TakBoardState, msg: ServerGameMessage| {
        let mut board_clone = board.clone();
        match msg {
            ServerGameMessage::StartGame => {
                dioxus::logger::tracing::info!("[WebSocket] Starting game");
                let mut board_clone = board_clone.clone();
                spawn_local(async move {
                    board_clone.reset();
                    board_clone.update_player_info().await;
                    board_clone.has_started.set(true);
                });
            }
            ServerGameMessage::Move(move_index, time_remaining, action) => {
                dioxus::logger::tracing::info!("[WebSocket] Processing move action: {action}");
                let Some(action) = TakAction::from_ptn(&action) else {
                    dioxus::logger::tracing::error!(
                        "[WebSocket] Invalid action received: {action}"
                    );
                    return;
                };
                let should_resync = board
                    .maybe_try_do_remote_action(move_index, action)
                    .is_err();
                for (player, duration) in time_remaining {
                    board_clone.set_time_remaining(player, duration);
                }
                if should_resync {
                    dioxus::logger::tracing::info!(
                        "[WebSocket] Resyncing game state after message"
                    );
                    resync_game_state(board_clone);
                }
            }
            ServerGameMessage::GameOver(game_state) => {
                dioxus::logger::tracing::info!("[WebSocket] Game over: {game_state:?}");
                if board_clone
                    .with_game(|game| game.game().game_state != game_state)
                    .unwrap_or(true)
                {
                    resync_game_state(board_clone);
                }
            }
        };
    };

    let ws = use_coroutine(move |mut rx: UnboundedReceiver<Message>| {
        let mut board_clone = board_clone.clone();
        let session_id = session_id.clone();
        let url = option_env!("WEBSOCKET_URL").unwrap_or("ws://localhost:8080/ws");
        dioxus::logger::tracing::info!(
            "[WebSocket] Connecting to WebSocket at: {url}, {:?}",
            option_env!("WEBSOCKET_URL")
        );
        async move {
            let Ok(mut ws) = WebSocket::open(url) else {
                return;
            };
            let _ = ws.send(Message::Text(session_id)).await;
            let (mut write, mut read) = ws.split();

            spawn_local(async move {
                while let Some(msg) = rx.next().await {
                    match write.send(msg).await {
                        Ok(_) => {
                            dioxus::logger::tracing::info!("[WebSocket] Message sent to server")
                        }
                        Err(ws_err) => dioxus::logger::tracing::error!(
                            "[WebSocket] Error sending to server -> {ws_err}"
                        ),
                    };
                }
            });

            spawn_local(async move {
                while let Some(recv_msg) = read.next().await {
                    match recv_msg {
                        Ok(Message::Text(text)) => {
                            dioxus::logger::tracing::info!(
                                "[WebSocket] Received text message: {text}"
                            );
                            if let Ok(game_msg) = serde_json::from_str::<ServerGameMessage>(&text) {
                                dioxus::logger::tracing::info!(
                                    "[WebSocket] Game message received: {:#?}",
                                    game_msg
                                );
                                handle_game_message(&mut board_clone, game_msg);
                            } else {
                                dioxus::logger::tracing::warn!(
                                    "[WebSocket] Failed to parse game message: {text}"
                                );
                            }
                        }
                        Ok(Message::Bytes(bytes)) => dioxus::logger::tracing::info!(
                            "[WebSocket] Received bytes message: {:#?}",
                            bytes
                        ),
                        Err(WebSocketError::ConnectionClose(close_event))
                            if close_event.was_clean =>
                        {
                            dioxus::logger::tracing::info!(
                                "[WebSocket] ConnectionClose: {:#?}",
                                close_event
                            )
                        }
                        Err(ws_err) => {
                            dioxus::logger::tracing::error!("[WebSocketError]: {:#?}", ws_err)
                        }
                    }
                }
            });
        }
    });

    let mut has_new_messages = use_signal(|| false);

    use_effect(move || {
        if board.message_queue.len() > 0 {
            has_new_messages.set(true);
        }
    });

    use_effect(move || {
        if *has_new_messages.read() {
            for message in board.message_queue.write().drain(..) {
                ws.send(Message::Text(serde_json::to_string(&message).unwrap()));
                dioxus::logger::tracing::info!(
                    "[WebSocket] Sent message from queue: {:?}",
                    message
                );
            }
        }
    });

    use_effect(move || {
        let board = board.clone();
        resync_game_state(board);
    });

    rsx! {}
}

fn resync_game_state(mut board: TakBoardState) {
    spawn_local(async move {
        dioxus::logger::tracing::info!("[WebSocket] Resyncing game state");
        let res = get_game_state().await;
        dioxus::logger::tracing::info!("[WebSocket] Game state resyncing: {:?}", res);
        if let Ok(GetGameStateResponse::Success(game_state)) = res {
            if let Some((ptn, time_remaining)) = game_state {
                board.try_set_from_ptn(ptn.to_string());
                board.has_started.set(true);
                for (player, duration) in time_remaining {
                    board.set_time_remaining(player, duration);
                }
            } else {
                dioxus::logger::tracing::warn!("[WebSocket] Game hasn't started yet");
                board.reset();
            }
        }
        dioxus::logger::tracing::info!("[WebSocket] Game state resynced.");
    });
}
