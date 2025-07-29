use crate::views::AUTH_DATA;
use dioxus::core_macro::component;
use dioxus::prelude::*;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite_wasm::Message;

#[component]
pub fn PlaytakClient() -> Element {
    let handle_game_message = async move |msg: String| {
        let parts: Vec<&str> = msg.split_whitespace().collect();
        if parts.is_empty() {
            dioxus::logger::tracing::warn!("[PlaytakClient] Received empty message");
            return;
        }
        match &parts[0] {
            _ => {
                dioxus::logger::tracing::info!("[PlaytakClient] Received message: {msg}");
            }
        }
    };

    let is_connected = use_signal(|| false);

    let ws = use_coroutine(move |mut rx: UnboundedReceiver<Message>| {
        let url = option_env!("PLAYTAK_URL").unwrap_or("wss://playtak.com/ws");
        dioxus::logger::tracing::info!(
            "[WebSocket] Connecting to PlayTak at: {url}, {:?}",
            option_env!("PLAYTAK_URL")
        );
        let mut is_connected = is_connected.clone();
        async move {
            dioxus::logger::tracing::info!("[PlaytakClient] Connecting to PlayTak");
            let Some(Message::Text(auth_info)) = rx.next().await else {
                dioxus::logger::tracing::error!("[PlaytakClient] Expected text message");
                return;
            };
            let parts: Vec<&str> = auth_info.split_whitespace().collect();
            if parts.len() != 2 {
                dioxus::logger::tracing::error!(
                    "[PlaytakClient] Expected 2 parts in auth message, got: {}",
                    parts.len()
                );
                return;
            }
            let username = parts[0].to_string();
            let password = parts[1].to_string();

            let ws = match tokio_tungstenite_wasm::connect_with_protocols(url, &["binary"]).await {
                Ok(ws) => ws,
                Err(e) => {
                    dioxus::logger::tracing::error!(
                        "[PlaytakClient] Error connecting to PlayTak: {e}"
                    );
                    return;
                }
            };
            dioxus::logger::tracing::info!("[PlaytakClient] Connected to PlayTak");

            let (mut sender, mut receiver) = ws.split();

            dioxus::prelude::spawn(async move {
                while let Some(message) = receiver.next().await {
                    match message {
                        Ok(Message::Text(text)) => {
                            dioxus::logger::tracing::info!(
                                "[PlaytakClient] Received text message: {text}"
                            );
                            handle_game_message(text.to_string()).await;
                        }
                        Ok(Message::Binary(data)) => {
                            let data_str = String::from_utf8_lossy(&data).to_string();
                            dioxus::logger::tracing::info!(
                                "[PlaytakClient] Binary message received: {data_str}"
                            );
                            handle_game_message(data_str).await;
                        }
                        Ok(Message::Close(_)) => {
                            dioxus::logger::tracing::info!(
                                "[PlaytakClient] Connection closed by server"
                            );
                            break;
                        }
                        Err(e) => {
                            dioxus::logger::tracing::error!(
                                "[PlaytakClient] Error receiving message: {e}"
                            );
                            break;
                        }
                    }
                }
            });

            let _ = sender
                .send(Message::text(format!("Login {} {}", username, password)))
                .await;

            dioxus::prelude::spawn(async move {
                while let Some(msg) = rx.next().await {
                    match sender.send(msg).await {
                        Ok(_) => {
                            dioxus::logger::tracing::info!("[PlaytakClient] Message sent to server")
                        }
                        Err(ws_err) => dioxus::logger::tracing::error!(
                            "[PlaytakClient] Error sending to server -> {ws_err}"
                        ),
                    };
                }
            });

            is_connected.set(true);
        }
    });

    use_effect(move || {
        let Some((username, password)) = AUTH_DATA.read().clone() else {
            return;
        };
        ws.send(Message::text(format!("{} {}", username, password)));
    });

    use_future(move || {
        let ws = ws.clone();
        async move {
            loop {
                if *is_connected.read() {
                    ws.send(Message::text("PING"));
                }
                crate::future::sleep(std::time::Duration::from_secs(30)).await;
            }
        }
    });

    rsx! {}
}
