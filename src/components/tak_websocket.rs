use crate::tak::TakAction;
use crate::views::TakBoardState;
use dioxus::core_macro::component;
use dioxus::prelude::*;
use futures_util::{SinkExt, StreamExt};
use gloo::net::websocket::futures::WebSocket;
use gloo::net::websocket::{Message, WebSocketError};
use wasm_bindgen_futures::spawn_local;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
enum ServerGameMessage {
    Sync(String),
    StartGame(usize),
    Move(String),
}

#[component]
pub fn TakWebSocket() -> Element {
    let mut board = use_context::<TakBoardState>();

    let board_clone = board.clone();

    let handle_game_message = move |board: &mut TakBoardState, msg: ServerGameMessage| match msg {
        ServerGameMessage::Sync(state) => {
            dioxus::logger::tracing::info!("[WebSocket] Syncing game state: {state}");
        }
        ServerGameMessage::StartGame(size) => {
            dioxus::logger::tracing::info!("[WebSocket] Starting game with size: {size}");
        }
        ServerGameMessage::Move(action) => {
            dioxus::logger::tracing::info!("[WebSocket] Processing move action: {action}");
            if let None = TakAction::from_ptn(&action).and_then(|x| board.try_do_action(&x).ok()) {
                dioxus::logger::tracing::error!("[WebSocket] Invalid action received: {action}");
            }
        }
    };

    let ws = use_coroutine(move |mut rx: UnboundedReceiver<Message>| {
        let mut board_clone = board_clone.clone();
        async move {
            let url = "ws://localhost:8080/ws";
            let ws = WebSocket::open(url).unwrap();
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

    rsx! {}
}
