use async_std::sync::RwLock;
use dioxus::core_macro::component;
use dioxus::prelude::*;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use gloo::net::websocket::futures::WebSocket;
use gloo::net::websocket::{Message, WebSocketError};
use serde::Serialize;
use serde_json;
use std::sync::{Arc, Mutex};
use wasm_bindgen_futures::spawn_local;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct MyMessage {
    msg_type: String,
    payload: String,
}

#[component]
pub fn TakWebSocket() -> Element {
    let ws = use_coroutine(|mut rx: UnboundedReceiver<Message>| async move {
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
                        dioxus::logger::tracing::info!("[WebSocket] Received text message: {text}")
                    }
                    Ok(Message::Bytes(bytes)) => dioxus::logger::tracing::info!(
                        "[WebSocket] Received bytes message: {:#?}",
                        bytes
                    ),
                    Err(WebSocketError::ConnectionClose(close_event)) if close_event.was_clean => {
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
    });

    let send_message = move |_| {
        let message = MyMessage {
            msg_type: "greeting".to_string(),
            payload: "Hello, WebSocket!".to_string(),
        };
        ws.send(Message::Text(serde_json::to_string(&message).unwrap()));
    };

    rsx!(
        div {
            "WebSocket client running"
            button {
                onclick: send_message,
                "Send Message"
            }
        }
    )
}
