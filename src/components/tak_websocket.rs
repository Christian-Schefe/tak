use crate::server::room::{get_game_state, GetGameStateResponse};
use crate::tak::TakAction;
use crate::views::TakBoardState;
use dioxus::core_macro::component;
use dioxus::prelude::*;
use futures_util::{SinkExt, StreamExt};
use gloo::net::websocket::futures::WebSocket;
use gloo::net::websocket::{Message, WebSocketError};
use wasm_bindgen_futures::spawn_local;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ServerGameMessage {
    StartGame(usize),
    Move(String),
}

#[component]
pub fn TakWebSocket(session_id: String) -> Element {
    let mut board = use_context::<TakBoardState>();

    let board_clone = board.clone();

    let handle_game_message = move |board: &mut TakBoardState, msg: ServerGameMessage| {
        let mut board_clone = board.clone();
        match msg {
            ServerGameMessage::StartGame(size) => {
                dioxus::logger::tracing::info!("[WebSocket] Starting game with size: {size}");
                spawn_local(async move {
                    board_clone.update_player_info().await;
                    board_clone.has_started.set(true);
                })
            }
            ServerGameMessage::Move(action) => {
                dioxus::logger::tracing::info!("[WebSocket] Processing move action: {action}");
                if let None =
                    TakAction::from_ptn(&action).and_then(|x| board.try_do_remote_action(&x).ok())
                {
                    dioxus::logger::tracing::error!(
                        "[WebSocket] Invalid action received: {action}"
                    );
                }
            }
        }
    };

    let ws = use_coroutine(move |mut rx: UnboundedReceiver<Message>| {
        let mut board_clone = board_clone.clone();
        let session_id = session_id.clone();
        async move {
            let url = "ws://localhost:8080/ws";
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
        let mut board = board.clone();
        spawn_local(async move {
            dioxus::logger::tracing::info!("[WebSocket] Resyncing game state on first render");
            let res = get_game_state().await;
            if let Ok(GetGameStateResponse::Success(game_state)) = &res {
                board.has_started.set(game_state.is_some());
                if let Some(ptn) = game_state {
                    board.set_game_from_ptn(ptn.to_string());
                } else {
                    board.reset_game();
                }
            }
            dioxus::logger::tracing::info!("[WebSocket] Game state resynced: {:?}", res);
        });
    });

    rsx! {}
}
