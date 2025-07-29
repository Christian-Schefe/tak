use std::{collections::HashMap, sync::Arc};

use futures::{
    StreamExt,
    channel::{
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
        oneshot,
    },
    stream::{SplitSink, SplitStream},
};
use futures_intrusive::sync::Mutex;
use futures_util::SinkExt;
use tokio_tungstenite_wasm::{Message, WebSocketStream};

use crate::message::{ClientMessage, ServerMessage};

pub enum ClientAction {
    Subscribe(String, UnboundedSender<serde_json::Value>),
    Unsubscribe(String),
    Publish(String, serde_json::Value),
}

pub struct WebSocketSender {
    sender: UnboundedSender<(ClientAction, oneshot::Sender<Result<(), WebSocketError>>)>,
}

impl WebSocketSender {
    pub fn new(
        sender: UnboundedSender<(ClientAction, oneshot::Sender<Result<(), WebSocketError>>)>,
    ) -> Self {
        WebSocketSender { sender }
    }

    pub async fn send(&mut self, message: ClientAction) -> Result<(), WebSocketError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send((message, tx))
            .await
            .map_err(WebSocketError::LocalConnectionError)?;
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(WebSocketError::ConnectionClosed),
        }
    }
}

pub struct WebSocket {
    sender: Option<SplitSink<WebSocketStream, Message>>,
    receiver: Option<SplitStream<WebSocketStream>>,
    handlers: Arc<Mutex<HashMap<String, Vec<UnboundedSender<serde_json::Value>>>>>,
    send_tx: Option<UnboundedSender<Message>>,
    send_rx: Option<UnboundedReceiver<Message>>,
    channel: Option<(
        UnboundedSender<(ClientAction, oneshot::Sender<Result<(), WebSocketError>>)>,
        UnboundedReceiver<(ClientAction, oneshot::Sender<Result<(), WebSocketError>>)>,
    )>,
}

#[derive(Debug)]
pub enum WebSocketError {
    SerializationError(serde_json::Error),
    ConnectionError(tokio_tungstenite_wasm::Error),
    LocalConnectionError(futures::channel::mpsc::SendError),
    ProtocolError(String),
    ConnectionClosed,
}

impl WebSocket {
    pub async fn connect(url: &str) -> Option<WebSocket> {
        if let Ok(stream) = tokio_tungstenite_wasm::connect(url).await {
            let (sender, receiver) = stream.split();
            let (channel_tx, channel_rx) = unbounded();
            let (send_tx, send_rx) = unbounded();
            Some(WebSocket {
                sender: Some(sender),
                receiver: Some(receiver),
                handlers: Arc::new(Mutex::new(HashMap::new(), true)),
                send_tx: Some(send_tx),
                send_rx: Some(send_rx),
                channel: Some((channel_tx, channel_rx)),
            })
        } else {
            None
        }
    }

    pub async fn connect_auth(url: &str, auth: &str) -> Option<WebSocket> {
        if let Ok(stream) = tokio_tungstenite_wasm::connect(url).await {
            let (mut sender, receiver) = stream.split();
            let (channel_tx, channel_rx) = unbounded();
            let (send_tx, send_rx) = unbounded();

            // Send authentication token
            if let Err(e) = sender.send(Message::Text(auth.into())).await {
                eprintln!("Failed to send auth token: {}", e);
                return None;
            }

            Some(WebSocket {
                sender: Some(sender),
                receiver: Some(receiver),
                handlers: Arc::new(Mutex::new(HashMap::new(), true)),
                send_tx: Some(send_tx),
                send_rx: Some(send_rx),
                channel: Some((channel_tx, channel_rx)),
            })
        } else {
            None
        }
    }

    pub fn get_sender(&self) -> Option<WebSocketSender> {
        self.channel
            .as_ref()
            .map(|(tx, _)| WebSocketSender::new(tx.clone()))
    }

    async fn send(
        send_channel: &mut UnboundedSender<Message>,
        message: ClientMessage,
    ) -> Result<(), WebSocketError> {
        let payload =
            serde_json::to_string(&message).map_err(WebSocketError::SerializationError)?;
        send_channel
            .send(Message::Text(payload.into()))
            .await
            .map_err(WebSocketError::LocalConnectionError)
    }

    async fn publish(
        send_channel: &mut UnboundedSender<Message>,
        topic: &str,
        payload: serde_json::Value,
    ) -> Result<(), WebSocketError> {
        let msg = ClientMessage::Publish {
            topic: topic.to_string(),
            payload,
        };
        Self::send(send_channel, msg).await
    }

    async fn subscribe(
        send_channel: &mut UnboundedSender<Message>,
        handlers: Arc<Mutex<HashMap<String, Vec<UnboundedSender<serde_json::Value>>>>>,
        topic: &str,
        handler: UnboundedSender<serde_json::Value>,
    ) -> Result<(), WebSocketError> {
        let mut handlers = handlers.lock().await;
        if handlers.contains_key(topic) {
            handlers.get_mut(topic).unwrap().push(handler);
            Ok(())
        } else {
            handlers.insert(topic.to_string(), vec![handler]);
            let msg = ClientMessage::Subscribe {
                topic: topic.to_string(),
            };
            drop(handlers);
            Self::send(send_channel, msg).await
        }
    }

    async fn unsubscribe(
        send_channel: &mut UnboundedSender<Message>,
        handlers: Arc<Mutex<HashMap<String, Vec<UnboundedSender<serde_json::Value>>>>>,
        topic: &str,
    ) -> Result<(), WebSocketError> {
        let mut handlers = handlers.lock().await;
        if let Some(_) = handlers.remove(topic) {
            let msg = ClientMessage::Unsubscribe {
                topic: topic.to_string(),
            };
            drop(handlers);
            Self::send(send_channel, msg).await?;
        }
        Ok(())
    }

    pub async fn handle_ws_send(ws: Arc<Mutex<Self>>) -> Result<(), WebSocketError> {
        let mut lock = ws.lock().await;
        let mut sender = lock.sender.take().ok_or(WebSocketError::ConnectionClosed)?;
        let mut rx = lock
            .send_rx
            .take()
            .ok_or(WebSocketError::ConnectionClosed)?;
        drop(lock);

        while let Some(msg) = rx.next().await {
            sender
                .send(msg)
                .await
                .map_err(WebSocketError::ConnectionError)?;
        }
        Ok(())
    }

    pub async fn handle_client_actions(ws: Arc<Mutex<Self>>) -> Result<(), WebSocketError> {
        let mut lock = ws.lock().await;
        let mut sender = lock
            .send_tx
            .take()
            .ok_or(WebSocketError::ConnectionClosed)?;
        let handlers = lock.handlers.clone();
        let (_, mut rx) = lock
            .channel
            .take()
            .ok_or(WebSocketError::ConnectionClosed)?;
        drop(lock);

        while let Some((action, callback)) = rx.next().await {
            let res = match action {
                ClientAction::Subscribe(topic, handler) => {
                    Self::subscribe(&mut sender, handlers.clone(), &topic, handler).await
                }
                ClientAction::Unsubscribe(topic) => {
                    Self::unsubscribe(&mut sender, handlers.clone(), &topic).await
                }
                ClientAction::Publish(topic, payload) => {
                    Self::publish(&mut sender, &topic, payload).await
                }
            };
            let _ = callback.send(res);
        }
        Ok(())
    }

    pub async fn handle_receive(ws: Arc<Mutex<Self>>) -> Result<(), WebSocketError> {
        let mut lock = ws.lock().await;
        let mut receiver = lock
            .receiver
            .take()
            .ok_or(WebSocketError::ConnectionClosed)?;
        let handlers = lock.handlers.clone();
        drop(lock);

        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(msg) = serde_json::from_str::<ServerMessage>(&text) {
                        let mut lock = handlers.lock().await;
                        if let Some(handlers) = lock.get_mut(&msg.topic) {
                            for handler in handlers {
                                let _ = handler.send(msg.payload.clone());
                            }
                        }
                        drop(lock);
                    } else {
                        println!("Failed to parse message: {}", text);
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => return Err(WebSocketError::ConnectionError(e)),
                Ok(Message::Binary(_)) => {
                    println!("Received binary message, which is not supported.")
                }
            }
        }
        Ok(())
    }
}
