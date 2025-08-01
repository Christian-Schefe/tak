use std::net::SocketAddr;

use axum::{Router, extract::ws::WebSocketUpgrade, response::IntoResponse, routing::get};
use ws_pubsub::handle_socket;

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| {
        handle_socket(socket, |x| {
            if x.trim() == "password" {
                Some("user".to_string())
            } else {
                None
            }
        })
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/ws", get(ws_handler));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Server running at {}", addr);

    ws_pubsub::handle_subscribe_to_topic("test_topic/+/hi", |message: usize| async move {
        println!("Received message on test_topic: {:?}", message);
        ws_pubsub::publish_to_topic("test_topic2", message * 2).await;
    });

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
