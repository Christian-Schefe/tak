use ws_pubsub::WebSocket;

static TOPIC2: &str = "test_topic2";

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

    ws.subscribe(&TOPIC2, |value: usize| {
        println!("Received value: {:?}", value);
    })
    .expect("Failed to subscribe");

    loop {
        if let Err(e) = ws.publish("test_topic/hello/hi", 5) {
            eprintln!("Failed to publish message: {:?}", e);
        }
        if let Err(e) = ws.publish("test_topic/ab3/hi", 6) {
            eprintln!("Failed to publish message: {:?}", e);
        }
        if let Err(e) = ws.publish("test_topic/ab3/hi/test", 7) {
            eprintln!("Failed to publish message: {:?}", e);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
