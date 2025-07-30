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

use crate::{
    future::spawn_local,
    message::{ClientMessage, ServerMessage},
};

#[derive(Debug)]
pub enum WebSocketError {
    SerializationError(serde_json::Error),
    ConnectionError(tokio_tungstenite_wasm::Error),
    LocalConnectionError(String),
    ProtocolError(String),
    ConnectionClosed,
}

pub enum ClientAction {
    Subscribe(String, UnboundedSender<serde_json::Value>),
    Unsubscribe(String),
    Publish(String, serde_json::Value),
}

pub struct WebSocket {
    action_sender: UnboundedSender<ClientAction>,
}

pub struct WebSocketRunner {
    stream: WebSocketStream,
    action_receiver: UnboundedReceiver<ClientAction>,
}

pub struct WebSocketReconnector {
    action_receiver: UnboundedReceiver<ClientAction>,
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
            if let Err(e) = stream.send(Message::Text(auth.into())).await {
                eprintln!("Failed to send auth token: {}", e);
                return Err(WebSocketError::ConnectionError(e));
            }
        }
        let (tx, rx) = unbounded();

        let run_data = WebSocketRunner {
            stream,
            action_receiver: rx,
        };
        Ok((WebSocket { action_sender: tx }, run_data))
    }

    pub async fn reconnect(
        connection_data: &ConnectionData,
        reconnector: WebSocketReconnector,
    ) -> Result<WebSocketRunner, (WebSocketError, WebSocketReconnector)> {
        let mut stream = match tokio_tungstenite_wasm::connect(&connection_data.url).await {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to reconnect: {}", e);
                return Err((WebSocketError::ConnectionError(e), reconnector));
            }
        };

        if let Some(auth) = &connection_data.auth {
            if let Err(e) = stream.send(Message::Text(auth.into())).await {
                eprintln!("Failed to send auth token: {}", e);
                return Err((WebSocketError::ConnectionError(e), reconnector));
            }
        }

        let run_data = WebSocketRunner {
            stream,
            action_receiver: reconnector.action_receiver,
        };
        Ok(run_data)
    }

    pub fn run_reconnecting(
        connection_data: impl Fn() -> ConnectionData,
        run_data: WebSocketRunner,
    ) -> impl Future<Output = WebSocketError> {
        async move {
            let mut runner = run_data;
            loop {
                let mut reconnector = WebSocket::run(runner).await;
                dioxus::logger::tracing::info!("Disconnected, attempting to reconnect...");
                let data = connection_data();
                let mut retry_count = 0;
                runner = loop {
                    reconnector = match WebSocket::reconnect(&data, reconnector).await {
                        Ok(new_runner) => {
                            dioxus::logger::tracing::info!("Reconnected successfully");
                            break new_runner;
                        }
                        Err((e, reconnector)) => {
                            dioxus::logger::tracing::error!("Reconnection failed: {:?}", e);
                            retry_count += 1;
                            if retry_count == 5 {
                                dioxus::logger::tracing::error!(
                                    "Failed to reconnect after 5 attempts"
                                );
                                return WebSocketError::ConnectionClosed;
                            }
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            reconnector
                        }
                    };
                };
                dioxus::logger::tracing::info!("Reconnected successfully");
            }
        }
    }

    pub fn run(run_data: WebSocketRunner) -> impl Future<Output = WebSocketReconnector> {
        dioxus::logger::tracing::info!("Run started");
        async move {
            let stream = run_data.stream;
            let mut rx = run_data.action_receiver;
            let (mut sender, mut receiver) = stream.split();

            let handlers_arc = Arc::new(Mutex::new(
                HashMap::<String, Vec<UnboundedSender<serde_json::Value>>>::new(),
                true,
            ));

            let handlers = handlers_arc.clone();
            let (stop_send, mut stop_recv) = oneshot::channel();

            let receiver_fut = async move {
                while let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            let msg = match serde_json::from_str::<ServerMessage>(&text) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    eprintln!("Failed to deserialize message: {}", e);
                                    continue;
                                }
                            };
                            let mut handlers = handlers.lock().await;
                            let Some(topic_handlers) = handlers.get_mut(&msg.topic) else {
                                println!("No handlers for topic {}", msg.topic);
                                continue;
                            };
                            for handler in topic_handlers {
                                if let Err(e) = handler.send(msg.payload.clone()).await {
                                    eprintln!("Failed to send message to handler: {}", e);
                                }
                            }
                            drop(handlers);
                        }
                        Ok(Message::Close(_)) => {
                            println!("Connection closed");
                            break;
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                dioxus::logger::tracing::info!("Receiver task finished");
                stop_send.send(()).unwrap_or_else(|_| {
                    eprintln!("Failed to send stop signal");
                });
            };

            let handlers = handlers_arc.clone();
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
                            let msg = Message::Text(
                                serde_json::to_string(&ClientMessage::Subscribe { topic })
                                    .unwrap_or_else(|e| {
                                        eprintln!("Serialization error: {}", e);
                                        String::new()
                                    })
                                    .into(),
                            );
                            sender.send(msg).await.unwrap_or_else(|e| {
                                eprintln!("Failed to send message: {}", e);
                            });
                        }
                        ClientAction::Unsubscribe(topic) => {
                            let mut handlers = handlers.lock().await;
                            handlers.remove(&topic);
                            drop(handlers);
                            let msg = Message::Text(
                                serde_json::to_string(&ClientMessage::Unsubscribe { topic })
                                    .unwrap_or_else(|e| {
                                        eprintln!("Serialization error: {}", e);
                                        String::new()
                                    })
                                    .into(),
                            );
                            sender.send(msg).await.unwrap_or_else(|e| {
                                eprintln!("Failed to send message: {}", e);
                            });
                        }
                        ClientAction::Publish(topic, message) => {
                            let msg = Message::Text(
                                serde_json::to_string(&ClientMessage::Publish {
                                    topic,
                                    payload: message,
                                })
                                .unwrap_or_else(|e| {
                                    eprintln!("Serialization error: {}", e);
                                    String::new()
                                })
                                .into(),
                            );
                            sender.send(msg).await.unwrap_or_else(|e| {
                                eprintln!("Failed to send message: {}", e);
                            });
                        }
                    }
                }
                dioxus::logger::tracing::info!("Send task finished");
                rx
            };

            let res = join!(receiver_fut, sender_fut);
            dioxus::logger::tracing::info!("Run task finished");
            WebSocketReconnector {
                action_receiver: res.1,
            }
        }
    }

    pub fn publish<T>(&self, topic: String, message: T) -> Result<(), WebSocketError>
    where
        T: Serialize + 'static,
    {
        let payload = serde_json::to_value(message).map_err(WebSocketError::SerializationError)?;

        self.action_sender
            .unbounded_send(ClientAction::Publish(topic, payload))
            .map_err(|e| WebSocketError::LocalConnectionError(e.to_string()))
    }

    pub fn subscribe<F, T>(&self, topic: String, callback: F) -> Result<(), SendError>
    where
        F: Fn(Result<T, serde_json::Error>) + Send + 'static,
        T: DeserializeOwned + 'static,
    {
        let (tx, mut rx) = unbounded();

        self.action_sender
            .unbounded_send(ClientAction::Subscribe(topic, tx))
            .map_err(|e| e.into_send_error())?;

        spawn_local(async move {
            while let Some(message) = rx.next().await {
                callback(serde_json::from_value(message));
            }
        });

        Ok(())
    }

    pub fn unsubscribe(&self, topic: String) -> Result<(), SendError> {
        self.action_sender
            .unbounded_send(ClientAction::Unsubscribe(topic))
            .map_err(|e| e.into_send_error())
    }
}
