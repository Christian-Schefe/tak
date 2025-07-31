use std::sync::{Arc, OnceLock};

use dioxus::prelude::*;
use ws_pubsub::WebSocket;

use crate::{
    server::api::get_auth,
    views::{AUTH_CHANGED, AUTH_TOKEN_KEY},
};

pub static WS_CLIENT: OnceLock<Arc<ws_pubsub::WebSocket>> = OnceLock::new();
static IS_CONNECTING: OnceLock<()> = OnceLock::new();

#[component]
pub fn PubSubClient() -> Element {
    use_effect(|| {
        let _ = AUTH_CHANGED.read();
        let url = option_env!("WEBSOCKET_URL").unwrap_or("ws://localhost:8080/ws2");
        let token = crate::storage::get(AUTH_TOKEN_KEY).unwrap_or(None::<String>);

        dioxus::prelude::spawn(async move {
            if !IS_CONNECTING.set(()).is_ok() {
                return;
            }
            let Some(token) = token else {
                return;
            };
            let Ok(Ok(_)) = get_auth().await else {
                return;
            };
            dioxus::logger::tracing::info!(
                "[WebSocket] Connecting to WebSocket at: {url}, {:?}",
                option_env!("WEBSOCKET_URL")
            );
            let connection_data = ws_pubsub::ConnectionData {
                url: url.to_string(),
                auth: Some(token.clone()),
            };
            let (ws, ws_runner) = WebSocket::try_connect(&connection_data, None)
                .await
                .expect("Failed to connect to WebSocket server");

            WS_CLIENT
                .set(Arc::new(ws))
                .ok()
                .expect("Failed to set WebSocket client");

            dioxus::prelude::spawn(async move {
                let err =
                    WebSocket::run_reconnecting(move || connection_data.clone(), ws_runner, None)
                        .await;
                dioxus::logger::tracing::error!("WebSocket failed irrecoverably: {:?}", err);
            });
        });
    });
    rsx! {}
}
