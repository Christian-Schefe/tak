use ws_pubsub::{StaticTopic, WebSocket};

static TOPIC1: StaticTopic<usize> = StaticTopic::new("test_topic");
static TOPIC2: StaticTopic<usize> = StaticTopic::new("test_topic2");

#[tokio::main]
async fn main() {
    let url = "ws://localhost:3000/ws";
    let auth = "password";
    let connection_data = ws_pubsub::ConnectionData {
        url: url.to_string(),
        auth: Some(auth.to_string()),
    };
    let (ws, ws_runner) = WebSocket::try_connect(&connection_data, Some(5))
        .await
        .expect("Failed to connect to WebSocket server");

    tokio::spawn(async move {
        let err =
            WebSocket::run_reconnecting(move || connection_data.clone(), ws_runner, None).await;
        eprintln!("WebSocket client error: {:?}", err);
    });

    ws.subscribe(&TOPIC2, |value| {
        println!("Received value: {:?}", value);
    })
    .expect("Failed to subscribe");

    loop {
        if let Err(e) = ws.publish(&TOPIC1, 5) {
            eprintln!("Failed to publish message: {:?}", e);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
