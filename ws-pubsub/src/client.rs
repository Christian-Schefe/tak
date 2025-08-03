use std::{collections::HashMap, sync::Arc};

use futures::{
    StreamExt,
    channel::{
        mpsc::{SendError, UnboundedReceiver, UnboundedSender, unbounded},
        oneshot,
    },
    join, select,
};
use futures_intrusive::sync::Mutex;
use futures_util::SinkExt;
use serde::{Serialize, de::DeserializeOwned};
use tokio_tungstenite_wasm::{Message, WebSocketStream};

use crate::{error, future::spawn_maybe_local, info, message::PublishMessage};

#[derive(Debug)]
pub enum WebSocketActionError {
    SerializationError(serde_json::Error),
    LocalConnectionError(SendError),
    Disconnected,
}

#[derive(Debug)]
pub enum WebSocketError {
    ConnectionError(tokio_tungstenite_wasm::Error),
    ProtocolError(String),
    MaxRetriesExceeded,
    AuthError,
}

pub enum ClientAction {
    Subscribe(String, UnboundedSender<serde_json::Value>),
    Publish(String, serde_json::Value),
}

pub struct WebSocketState {
    connected: bool,
}

#[derive(Clone)]
pub struct WebSocket {
    action_sender: Arc<UnboundedSender<ClientAction>>,
    state: Arc<Mutex<WebSocketState>>,
}

pub struct WebSocketRunner {
    stream: WebSocketStream,
    action_receiver: UnboundedReceiver<ClientAction>,
    handlers: Arc<Mutex<HashMap<String, Vec<UnboundedSender<serde_json::Value>>>>>,
    state: Arc<Mutex<WebSocketState>>,
}

pub struct WebSocketReconnector {
    action_receiver: UnboundedReceiver<ClientAction>,
    handlers: Arc<Mutex<HashMap<String, Vec<UnboundedSender<serde_json::Value>>>>>,
    state: Arc<Mutex<WebSocketState>>,
}

#[derive(Debug, Clone)]
pub struct ConnectionData {
    pub url: String,
    pub auth: Option<String>,
}

impl WebSocket {
    pub async fn connect(
        connection_data: &ConnectionData,
    ) -> Result<(WebSocket, WebSocketRunner), WebSocketError> {
        let mut stream = tokio_tungstenite_wasm::connect(&connection_data.url)
            .await
            .map_err(WebSocketError::ConnectionError)?;

        if let Some(auth) = &connection_data.auth {
            if let Err(e) = do_auth(&mut stream, auth).await {
                let _ = stream.close().await;
                return Err(e);
            }
        }
        let (tx, rx) = unbounded();

        let run_data = WebSocketRunner {
            stream,
            action_receiver: rx,
            handlers: Arc::new(Mutex::new(HashMap::new(), true)),
            state: Arc::new(Mutex::new(WebSocketState { connected: true }, true)),
        };
        info!(
            "[WebSocket] Connected to {} with auth: {:?}",
            connection_data.url, connection_data.auth
        );
        let state = run_data.state.clone();
        Ok((
            WebSocket {
                action_sender: Arc::new(tx),
                state,
            },
            run_data,
        ))
    }

    pub async fn try_connect(
        connection_data: &ConnectionData,
        max_retries: Option<usize>,
    ) -> Result<(WebSocket, WebSocketRunner), WebSocketError> {
        let mut i = 0;
        loop {
            match Self::connect(connection_data).await {
                Ok(run_data) => return Ok(run_data),
                Err(e) => {
                    error!("Reconnection failed: {:?}", e);
                }
            }
            if max_retries.is_some_and(|max| i >= max) {
                error!("Max retries exceeded");
                return Err(WebSocketError::MaxRetriesExceeded);
            }
            crate::future::sleep(std::time::Duration::from_millis(exponential_backoff(i))).await;
            i += 1;
        }
    }

    pub async fn reconnect(
        connection_data: &ConnectionData,
        reconnector: WebSocketReconnector,
    ) -> Result<WebSocketRunner, (WebSocketError, WebSocketReconnector)> {
        let mut stream = match tokio_tungstenite_wasm::connect(&connection_data.url).await {
            Ok(stream) => stream,
            Err(e) => {
                error!("Failed to reconnect: {}", e);
                return Err((WebSocketError::ConnectionError(e), reconnector));
            }
        };

        if let Some(auth) = &connection_data.auth {
            if let Err(e) = do_auth(&mut stream, auth).await {
                let _ = stream.close().await;
                return Err((e, reconnector));
            }
        }

        /*if let Err(e) = do_resubscribe(&mut stream, reconnector.handlers.clone()).await {
            let _ = stream.close().await;
            return Err((e, reconnector));
        }*/

        let run_data = WebSocketRunner {
            stream,
            action_receiver: reconnector.action_receiver,
            handlers: reconnector.handlers,
            state: reconnector.state,
        };
        Ok(run_data)
    }

    pub async fn try_reconnect(
        connection_data: &ConnectionData,
        mut reconnector: WebSocketReconnector,
        max_retries: Option<usize>,
    ) -> Result<WebSocketRunner, WebSocketError> {
        let mut i = 0;
        loop {
            match Self::reconnect(connection_data, reconnector).await {
                Ok(run_data) => return Ok(run_data),
                Err((e, new_reconnector)) => {
                    error!("Reconnection failed: {:?}", e);
                    reconnector = new_reconnector;
                }
            }
            if max_retries.is_some_and(|max| i >= max) {
                error!("Max retries exceeded");
                return Err(WebSocketError::MaxRetriesExceeded);
            }
            crate::future::sleep(std::time::Duration::from_millis(exponential_backoff(i))).await;
            i += 1;
        }
    }

    pub fn run_reconnecting(
        connection_data: impl Fn() -> ConnectionData,
        run_data: WebSocketRunner,
        max_retries: Option<usize>,
    ) -> impl Future<Output = WebSocketError> {
        async move {
            let mut runner = run_data;
            loop {
                let reconnector = WebSocket::run(runner).await;
                info!("Disconnected, attempting to reconnect...");
                let data = connection_data();
                runner = match Self::try_reconnect(&data, reconnector, max_retries).await {
                    Ok(new_runner) => new_runner,
                    Err(e) => {
                        error!("Reconnection failed: {:?}", e);
                        return e;
                    }
                };
                info!("Reconnected successfully");
            }
        }
    }

    pub fn run(run_data: WebSocketRunner) -> impl Future<Output = WebSocketReconnector> {
        info!("Run started");
        async move {
            let stream = run_data.stream;
            let mut rx = run_data.action_receiver;
            let (mut sender, mut receiver) = stream.split();

            let handlers = run_data.handlers.clone();
            let (stop_send, mut stop_recv) = oneshot::channel();

            let receiver_fut = async move {
                while let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            let msg = match serde_json::from_str::<PublishMessage>(&text) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    error!("Failed to deserialize message: {}", e);
                                    continue;
                                }
                            };
                            let mut handlers = handlers.lock().await;
                            let Some(topic_handlers) = handlers.get_mut(&msg.topic) else {
                                error!("No handlers for topic {}", msg.topic);
                                continue;
                            };
                            let mut handlers_to_remove = vec![];
                            for (i, handler) in topic_handlers.iter().enumerate() {
                                if let Err(e) = handler.unbounded_send(msg.payload.clone()) {
                                    error!("Failed to send message to handler: {}", e);
                                    handlers_to_remove.push(i);
                                }
                            }
                            for i in handlers_to_remove.iter().rev() {
                                topic_handlers.swap_remove(*i);
                            }
                            if topic_handlers.is_empty() {
                                handlers.remove(&msg.topic);
                            }
                            drop(handlers);
                        }
                        Ok(Message::Close(_)) => {
                            info!("Connection closed");
                            break;
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                info!("Receiver task finished");
                stop_send.send(()).unwrap_or_else(|_| {
                    error!("Failed to send stop signal");
                });
            };

            let handlers = run_data.handlers.clone();
            let sender_fut = async move {
                loop {
                    let Some(action) = (select! {
                        action = rx.next() => action,
                        _ = stop_recv => break,
                    }) else {
                        break;
                    };
                    match action {
                        ClientAction::Subscribe(topic, callback) => {
                            let mut handlers = handlers.lock().await;
                            handlers.entry(topic.clone()).or_default().push(callback);
                            drop(handlers);
                        }
                        ClientAction::Publish(topic, message) => {
                            let msg = Message::Text(
                                serde_json::to_string(&PublishMessage {
                                    topic,
                                    payload: message,
                                })
                                .unwrap_or_else(|e| {
                                    error!("Serialization error: {}", e);
                                    String::new()
                                })
                                .into(),
                            );
                            sender.send(msg).await.unwrap_or_else(|e| {
                                error!("Failed to send message: {}", e);
                            });
                        }
                    }
                }
                info!("Send task finished");
                rx
            };

            let mut state = run_data.state.lock().await;
            state.connected = true;
            drop(state);

            let res = join!(receiver_fut, sender_fut);

            let mut state = run_data.state.lock().await;
            state.connected = false;
            drop(state);

            info!("Run task finished");
            WebSocketReconnector {
                action_receiver: res.1,
                handlers: run_data.handlers,
                state: run_data.state,
            }
        }
    }

    pub fn publish<T>(&self, topic: impl AsRef<str>, message: T) -> Result<(), WebSocketActionError>
    where
        T: Serialize + 'static,
    {
        if !self.is_connected() {
            return Err(WebSocketActionError::Disconnected);
        }
        let payload =
            serde_json::to_value(message).map_err(WebSocketActionError::SerializationError)?;

        self.action_sender
            .unbounded_send(ClientAction::Publish(topic.as_ref().to_string(), payload))
            .map_err(|e| WebSocketActionError::LocalConnectionError(e.into_send_error()))
    }

    pub fn subscribe_callback<T>(
        &self,
        topic: impl AsRef<str>,
    ) -> Result<CallbackReceiver<T>, WebSocketActionError>
    where
        T: DeserializeOwned + 'static,
    {
        if !self.is_connected() {
            return Err(WebSocketActionError::Disconnected);
        }
        let (tx, rx) = unbounded();

        self.action_sender
            .unbounded_send(ClientAction::Subscribe(topic.as_ref().to_string(), tx))
            .map_err(|e| WebSocketActionError::LocalConnectionError(e.into_send_error()))?;

        Ok(CallbackReceiver::new(rx))
    }

    pub fn subscribe<F, T>(
        &self,
        topic: impl AsRef<str>,
        callback: F,
    ) -> Result<(), WebSocketActionError>
    where
        F: Fn(T) + Send + 'static,
        T: DeserializeOwned + 'static,
    {
        let CallbackReceiver {
            receiver: mut rx, ..
        } = self.subscribe_callback::<T>(topic)?;

        spawn_maybe_local(async move {
            while let Some(message) = rx.next().await {
                match serde_json::from_value::<T>(message) {
                    Ok(value) => callback(value),
                    Err(e) => error!("Failed to deserialize message: {}", e),
                }
            }
        });

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.state.try_lock().map(|s| s.connected).unwrap_or(false)
    }
}

fn exponential_backoff(retry_index: usize) -> u64 {
    let base = 200;
    let max_delay = 30000;
    let delay = (retry_index + 1) * base * (2_usize.pow(retry_index as u32));
    delay.min(max_delay) as u64
}

async fn do_auth(stream: &mut WebSocketStream, auth: &String) -> Result<(), WebSocketError> {
    if let Err(e) = stream.send(Message::Text(auth.into())).await {
        error!("Failed to send auth token: {}", e);
        return Err(WebSocketError::ConnectionError(e));
    }
    match stream.next().await {
        Some(Ok(Message::Text(response))) => {
            if response != "AUTH_ACK" {
                error!("Authentication failed: {}", response);
                return Err(WebSocketError::AuthError);
            }
        }
        Some(Ok(msg)) => {
            error!("Unexpected message during authentication: {:?}", msg);
            return Err(WebSocketError::ProtocolError("Unexpected message".into()));
        }
        Some(Err(e)) => {
            error!("Failed to receive auth response: {}", e);
            return Err(WebSocketError::ConnectionError(e));
        }
        None => {
            error!("Connection closed before receiving auth response");
            return Err(WebSocketError::ProtocolError("Connection closed".into()));
        }
    }
    Ok(())
}

pub struct CallbackReceiver<T> {
    receiver: UnboundedReceiver<serde_json::Value>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> CallbackReceiver<T>
where
    T: DeserializeOwned + 'static,
{
    pub fn new(receiver: UnboundedReceiver<serde_json::Value>) -> Self {
        Self {
            receiver,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn next(&mut self) -> Option<Result<T, serde_json::Error>> {
        match self.receiver.next().await {
            Some(value) => Some(serde_json::from_value(value)),
            None => None,
        }
    }
}
