use std::sync::{Arc, Mutex};

use dioxus::prelude::*;
use futures::{
    SinkExt, StreamExt,
    channel::{mpsc, oneshot},
    stream::{SplitSink, SplitStream},
};
use tokio_tungstenite_wasm::{Message, WebSocketStream};

use crate::{AuthResponse, ServerFunctions};

pub static WS_CLIENT: GlobalSignal<Option<WsClient>> = GlobalSignal::new(|| None);
pub static WS_CONFIG: GlobalSignal<WsConfig> = GlobalSignal::new(|| WsConfig::default());

#[derive(Clone, Default)]
pub struct WsConfig {
    url: Option<String>,
    token: Option<String>,
}

pub struct Replaceable<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> Replaceable<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn replace(&self, new_inner: T) {
        let mut inner = self.inner.lock().unwrap();
        *inner = new_inner;
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.inner.lock().unwrap().clone()
    }
}

pub struct WsConnection {
    ws_stream: Replaceable<Option<SplitStream<Message>>>,
    ws_sink: Replaceable<Option<SplitSink<WebSocketStream, Message>>>,
}

pub struct WsSender {
    receiver:
        Option<UnboundedReceiver<(serde_json::Value, oneshot::Sender<Result<(), WsSendError>>)>>,
    ws_sink: Replaceable<Option<SplitSink<WebSocketStream, Message>>>,
}

impl WsSender {
    pub fn new(ws_sink: SplitSink<WebSocketStream, Message>) -> Self {
        Self {
            receiver: None,
            ws_sink: Replaceable::new(ws_sink),
        }
    }

    pub async fn send(&mut self) {
        if let Some(receiver) = self.receiver.take() {
            while let Some((payload, response_tx)) = receiver.next().await {
                if let Err(e) = self
                    .ws_sink
                    .get()
                    .send(Message::Text(payload.to_string().into()))
                    .await
                {
                    response_tx
                        .send(Err(WsSendError::ConnectionFailed(e)))
                        .unwrap_or(());
                    continue;
                }
                response_tx.send(Ok(())).unwrap_or(());
            }
        }
    }
}

#[derive(Clone)]
pub struct WsClient {
    connected: Arc<Mutex<bool>>,
    connection: Signal<Option<String>>,
    sender: Arc<UnboundedSender<(serde_json::Value, oneshot::Sender<Result<(), WsSendError>>)>>,
    send_receiver: Arc<
        Mutex<
            Option<
                UnboundedReceiver<(serde_json::Value, oneshot::Sender<Result<(), WsSendError>>)>,
            >,
        >,
    >,
}

pub fn use_ws_client() -> WsClient {
    let client = use_hook(|| WsClient::new());
    let client_clone = client.clone();
    use_effect(move || {
        let config = WS_CONFIG.read().clone();
        let url = config.url.clone();
        let token = config.token.clone();
        let client_clone = client_clone.clone();
        if let Some(token) = token
            && let Some(url) = url
        {
            spawn(async move {
                if let Err(e) = client_clone.connect(url, token).await {
                    dioxus::logger::tracing::error!("WebSocket connection failed: {:?}", e);
                }
            });
        }
    });
    let client_clone = client.clone();
    use_future(move || {
        let client = client_clone.clone();
        async move {
            client.send().await;
        }
    });
    client
}

pub fn use_ws_channel<T, ServerFut: ServerFunctions>(
    topic: &str,
    handler: impl Fn(T) + Send + 'static,
) -> Coroutine<T> {
    let client = use_ws_client();
    let topic_clone = topic.to_string();
    let mut subscription_id = use_signal(|| None);
    let tx = use_coroutine(move |mut rx| {
        let topic = topic_clone.clone();
        async move { while let Some(msg) = rx.next().await {} }
    });
    let topic_clone = topic.to_string();
    use_future(move || {
        let topic = topic_clone.clone();
        async move {
            let sub = match ServerFut::subscribe(topic).await {
                Ok(subscription_id) => subscription_id,
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to subscribe to topic: {:?}", e);
                    return;
                }
            };
            subscription_id.set(Some(sub));
        }
    });
    use_drop(move || {
        let Some(sub_id) = subscription_id.peek().clone() else {
            return;
        };
        spawn(async move {
            ServerFut::unsubscribe(sub_id).await;
        });
    });
    tx
}

#[derive(Debug)]
pub enum WsConnectError {
    ConnectionFailed(tokio_tungstenite_wasm::Error),
    AuthFailed,
}

#[derive(Debug)]
pub enum WsSendError {
    ConnectionFailed(tokio_tungstenite_wasm::Error),
    NotConnected,
}

impl WsClient {
    pub fn new() -> Self {
        let (send_channel_tx, send_channel_rx) = mpsc::unbounded();
        Self {
            connected: Arc::new(Mutex::new(false)),
            connection: Signal::new(None),
            sender: Arc::new(send_channel_tx),
            send_receiver: Arc::new(Mutex::new(Some(send_channel_rx))),
        }
    }

    pub async fn connect(&mut self, url: String, token: String) -> Result<(), WsConnectError> {
        let mut connected = self.connected.lock().unwrap();
        if *connected {
            return Ok(());
        }
        *connected = true;
        drop(connected);

        let mut stream = match tokio_tungstenite_wasm::connect(&url).await {
            Ok(stream) => stream,
            Err(e) => {
                self.set_connected(false);
                return Err(WsConnectError::ConnectionFailed(e));
            }
        };

        let connection_id = match self.auth(token, &mut stream).await {
            Ok(AuthResponse::Success(connection_id)) => connection_id,
            _ => {
                self.set_connected(false);
                return Err(WsConnectError::AuthFailed);
            }
        };
        self.connection.set(Some(connection_id));
        self.run(stream).await;
        self.set_connected(false);
        Ok(())
    }

    fn set_connected(&self, connected: bool) {
        let mut conn = self.connected.lock().unwrap();
        *conn = connected;
    }

    async fn auth(
        &self,
        token: String,
        stream: &mut WebSocketStream,
    ) -> Result<AuthResponse, tokio_tungstenite_wasm::Error> {
        stream.send(Message::Text(token.into())).await?;
        let response = stream.next().await;
        match response {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<AuthResponse>(&text) {
                Ok(resp) => Ok(resp),
                Err(_) => Ok(AuthResponse::Failure),
            },
            Some(Err(e)) => Err(e),
            _ => Ok(AuthResponse::Failure),
        }
    }

    async fn run(&self, stream: WebSocketStream) {
        let (tx, rx) = stream.split();

        let (stop_tx, stop_rx) = oneshot::channel();
        let (stopped_tx, stopped_rx) = oneshot::channel();
        let send_self = self.clone();
        spawn(async move {
            send_self.send(tx, stop_rx, stopped_tx).await;
        });
        self.receive(rx).await;
        stop_tx.send(()).unwrap();
        stopped_rx.await.unwrap();
    }

    async fn receive(&self, mut rx: SplitStream<WebSocketStream>) {
        while let Some(message) = rx.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    dioxus::logger::tracing::info!("Received message: {}", text);
                }
                Ok(Message::Binary(_)) => {
                    dioxus::logger::tracing::warn!(
                        "Received binary message, which is not handled."
                    );
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Error receiving message: {:?}", e);
                    break;
                }
                _ => {}
            }
        }
    }

    async fn send(self) {
        let mut lock = self.send_receiver.lock().unwrap();
        let mut receiver = lock.take().unwrap();
        drop(lock);

        while let Some((payload, response_tx)) = receiver.next().await {
            if let Err(e) = tx.send(Message::Text(payload.to_string().into())).await {
                dioxus::logger::tracing::error!("Failed to send message: {:?}", e);
                response_tx
                    .send(Err(WsSendError::ConnectionFailed(e)))
                    .unwrap_or(());
                continue;
            }
            response_tx.send(Ok(())).unwrap_or(());
        }

        let mut lock = self.send_receiver.lock().unwrap();
        *lock = Some(receiver);
        drop(lock);
    }

    async fn publish(
        &self,
        topic: String,
        payload: serde_json::Value,
    ) -> Result<(), WsConnectError> {
    }
}
