use std::{
    sync::{Arc, mpsc},
    time::Duration,
};

use crate::components::tak_board_state::TakBoardState;
use crate::server::api::get_current_game;
use crate::views::AUTH_TOKEN_KEY;
use async_trait::async_trait;
use dioxus::core_macro::component;
use dioxus::prelude::*;
use ezsockets::{ClientConfig, RawMessage, SocketConfig};
use futures_util::{SinkExt, StreamExt};
use tak_core::{TakAction, TakGameState, TakPlayer};
use wasm_bindgen_futures::spawn_local;

struct Client {
    on_text_sender: mpsc::Sender<String>,
}

#[async_trait]
impl ezsockets::ClientExt for Client {
    type Call = ();

    async fn on_text(&mut self, text: ezsockets::Utf8Bytes) -> Result<(), ezsockets::Error> {
        if let Err(e) = self.on_text_sender.send(text.to_string()) {
            dioxus::logger::tracing::error!("[WebSocket] Error sending text message: {e}");
        }
        Ok(())
    }

    async fn on_binary(&mut self, bytes: ezsockets::Bytes) -> Result<(), ezsockets::Error> {
        dioxus::logger::tracing::info!("[WebSocket] Received binary message: {:?}", bytes);
        Ok(())
    }

    async fn on_call(&mut self, call: Self::Call) -> Result<(), ezsockets::Error> {
        let () = call;
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ServerGameMessage {
    StartGame,
    Move(usize, Vec<(TakPlayer, u64)>, String),
    GameOver(TakGameState),
}

#[component]
pub fn TakWebSocket() -> Element {
    let mut board = use_context::<TakBoardState>();

    let board_clone = board.clone();

    let handle_game_message = move |board: &mut TakBoardState, msg: ServerGameMessage| {
        match msg {
            ServerGameMessage::StartGame => {
                dioxus::logger::tracing::info!("[WebSocket] Starting game");
                let mut board_clone = board.clone();
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
                    board.set_time_remaining(player, duration);
                }
                if should_resync {
                    dioxus::logger::tracing::info!(
                        "[WebSocket] Resyncing game state after message"
                    );
                    resync_game_state(board);
                }
            }
            ServerGameMessage::GameOver(game_state) => {
                dioxus::logger::tracing::info!("[WebSocket] Game over: {game_state:?}");
                if board
                    .with_game(|game| game.game().game_state != game_state)
                    .unwrap_or(true)
                {
                    resync_game_state(board);
                }
            }
        };
    };

    let ws = use_coroutine(move |mut rx: UnboundedReceiver<ezsockets::Message>| {
        let mut board_clone = board_clone.clone();
        let token = crate::storage::get(AUTH_TOKEN_KEY).unwrap_or(None::<String>);
        let url = option_env!("WEBSOCKET_URL").unwrap_or("ws://localhost:8080/ws");
        dioxus::logger::tracing::info!(
            "[WebSocket] Connecting to WebSocket at: {url}, {:?}",
            option_env!("WEBSOCKET_URL")
        );
        async move {
            let Some(token) = token else {
                dioxus::logger::tracing::error!("[WebSocket] No auth token found, cannot send");
                return;
            };

            let (on_message, on_message_rx) = mpsc::channel::<String>();

            let config = ClientConfig::new(url).socket_config(SocketConfig {
                heartbeat_ping_msg_fn: Arc::new(|_t: Duration| RawMessage::Binary("ping".into())),
                ..Default::default()
            });
            let (ws, mut future) = ezsockets::connect_with(
                |_client| Client {
                    on_text_sender: on_message,
                },
                config,
                ezsockets::ClientConnectorWasm::default(),
            );

            let _ = ws.text(token);
            let a = future.extract().await;

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
                while let Some(text) = on_message_rx.recv().await {
                    dioxus::logger::tracing::info!("[WebSocket] Received text message: {text}");
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
                ws.send(ezsockets::Message::Text(
                    serde_json::to_string(&message).unwrap(),
                ));
                dioxus::logger::tracing::info!(
                    "[WebSocket] Sent message from queue: {:?}",
                    message
                );
            }
        }
    });

    use_effect(move || {
        resync_game_state(&board);
    });

    rsx! {}
}

fn resync_game_state(board: &TakBoardState) {
    let mut board = board.clone();
    spawn_local(async move {
        dioxus::logger::tracing::info!("[WebSocket] Resyncing game state");
        let res = get_current_game().await;
        dioxus::logger::tracing::info!("[WebSocket] Game state resyncing: {:?}", res);
        match res {
            Ok(Ok(game_state)) => {
                if let Some(game) = game_state {
                    board.set_from_game(game.clone());
                    board.has_started.set(true);
                } else {
                    dioxus::logger::tracing::warn!("[WebSocket] Game hasn't started yet");
                    board.reset();
                }
            }
            Ok(Err(e)) => {
                dioxus::logger::tracing::error!("[WebSocket] Error resyncing game state: {e}");
                board.reset();
            }
            Err(e) => {
                dioxus::logger::tracing::error!("[WebSocket] Error resyncing game state: {e}");
                board.reset();
            }
        }
        dioxus::logger::tracing::info!("[WebSocket] Game state resynced.");
    });
}
