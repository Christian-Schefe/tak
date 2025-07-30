use ws_pubsub::WebSocket;

#[tokio::main]
async fn main() {
    let url = "ws://localhost:3000/ws";
    let auth = "password";
    let connection_data = ws_pubsub::ConnectionData {
        url: url.to_string(),
        auth: Some(auth.to_string()),
    };
    let (ws, ws_runner) = WebSocket::connect(&connection_data)
        .await
        .expect("Failed to connect to WebSocket server");

    tokio::spawn(
        async move { WebSocket::run_reconnecting(move || connection_data.clone(), ws_runner) },
    );

    ws.subscribe::<_, usize>("test_topic2".to_string(), |value| {
        println!("Received value: {:?}", value);
    })
    .expect("Failed to subscribe");

    loop {
        ws.publish("test_topic".to_string(), 5)
            .expect("Failed to publish");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
