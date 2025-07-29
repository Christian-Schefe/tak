use std::sync::Arc;

use futures::{StreamExt, channel::mpsc::unbounded};
use futures_intrusive::sync::Mutex;
use ws_pubsub::{ClientAction, WebSocket, WebSocketError, WebSocketSender};

#[tokio::main]
async fn main() {
    let ws = WebSocket::connect_auth("ws://localhost:3000/ws", "password")
        .await
        .expect("Failed to connect to WebSocket server");

    let mut sender = ws.get_sender().unwrap();

    let receive_ws = Arc::new(Mutex::new(ws, true));
    let send_ws = Arc::clone(&receive_ws);
    let client_ws = Arc::clone(&receive_ws);

    tokio::spawn(async move {
        WebSocket::handle_ws_send(send_ws)
            .await
            .expect("Failed to handle send");
    });

    tokio::spawn(async move {
        WebSocket::handle_client_actions(client_ws)
            .await
            .expect("Failed to handle send");
    });

    tokio::spawn(async move {
        WebSocket::handle_receive(receive_ws)
            .await
            .expect("Failed to handle receive");
    });

    handle_subscribe(&mut sender, "test_topic2".to_string(), |value| {
        println!("Received value: {:?}", value);
    })
    .await
    .expect("Failed to subscribe");

    loop {
        let action = ClientAction::Publish(
            "test_topic".to_string(),
            serde_json::json!({"key": "value"}),
        );
        sender.send(action).await.expect("Failed to send action");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

pub async fn handle_subscribe<F>(
    sender: &mut WebSocketSender,
    topic: String,
    handler: F,
) -> Result<(), WebSocketError>
where
    F: Fn(serde_json::Value) + Send + 'static,
{
    let (tx, mut rx) = unbounded();
    sender.send(ClientAction::Subscribe(topic, tx)).await?;
    tokio::spawn(async move {
        while let Some(value) = rx.next().await {
            handler(value);
        }
    });
    Ok(())
}
