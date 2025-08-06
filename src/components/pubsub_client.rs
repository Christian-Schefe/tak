use dioxus::prelude::*;
use ws_pubsub::use_ws_connection;

use crate::{
    server::api::get_user_id,
    views::{AUTH_CHANGED, AUTH_TOKEN_KEY},
};

#[component]
pub fn PubSubClient() -> Element {
    let mut connector = use_ws_connection();

    let auth = use_resource(move || async move {
        let _ = AUTH_CHANGED.read();
        let _ = connector.token.read();
        let _ = connector.url.read();
        dioxus::logger::tracing::info!("Fetching auth for WebSocket connection");
        get_user_id().await
    });

    use_effect(move || {
        let _ = AUTH_CHANGED.read();

        let url = option_env!("WEBSOCKET_URL").unwrap_or("ws://localhost:8080/ws");
        let mut token = crate::storage::get(AUTH_TOKEN_KEY).unwrap_or(None::<String>);
        let is_auth = match auth.read().as_ref() {
            Some(Ok(Ok(_user_id))) => true,
            _ => false,
        };
        token.take_if(|_| !is_auth);

        if !connector.url.peek().as_ref().is_some_and(|u| u == url) {
            connector.url.set(Some(url.to_string()));
        }
        if connector.token.peek().as_ref() != token.as_ref() {
            connector.token.set(token);
        }
    });

    rsx! {}
}
