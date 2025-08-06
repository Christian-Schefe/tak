use axum::{extract::WebSocketUpgrade, response::IntoResponse};

use crate::{
    server::internal::{auth::validate_token, matches::handle_player_publish},
    views::ClientGameMessage,
};

pub fn setup_handlers() {
    ws_pubsub::handle_subscribe_to_topic(
        "matches/+",
        |player_id, topic, msg: ClientGameMessage| async move {
            handle_player_publish(&player_id, topic, msg).await;
        },
    );
}

pub(crate) async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        ws_pubsub::handle_socket(socket, |token| {
            validate_token(&token.to_string())
                .ok()
                .map(|claims| claims.sub)
        })
    })
}
