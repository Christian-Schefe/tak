use std::net::SocketAddr;

use axum::{Router, extract::ws::WebSocketUpgrade, response::IntoResponse, routing::get};
use ws_pubsub::{StaticTopic, Topic, handle_socket, static_topic};

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

static TOPIC1: StaticTopic<usize> = StaticTopic::new("test_topic");
static TOPIC2: StaticTopic<usize> = StaticTopic::new("test_topic2");

static_topic!(TEST_TOPIC3, usize, "+/foo/+/bar/+/+/++/+");

#[tokio::main]
async fn main() {
    let app = Router::new().route("/ws", get(ws_handler));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Server running at {}", addr);

    ws_pubsub::handle_subscribe_to_topic(&TOPIC1, |message| async move {
        println!("Received message on test_topic: {:?}", message);
        ws_pubsub::publish_to_topic(&TOPIC2, message * 2).await;
    });

    println!(
        "{:?}",
        TEST_TOPIC3
            .try_extract("__/foo/123/bar/36_46264/dhhah.ah,/++/ad3164§%")
            .map(|parts| {
                println!("Extracted parts: {:?}", parts);
            })
    );

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
