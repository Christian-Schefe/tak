use std::{collections::HashSet, sync::LazyLock};

use axum::extract::ws::{Message, WebSocket};
use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};

use crate::message::{ClientMessage, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ClientId {
    pub connection_id: String,
    pub user_id: String,
}

struct PubSub {
    subscribers: DashMap<String, HashSet<ClientId>>,
    connections: DashMap<ClientId, SplitSink<WebSocket, Message>>,
    handlers: DashMap<String, Vec<UnboundedSender<serde_json::Value>>>,
}

impl PubSub {
    fn new() -> Self {
        PubSub {
            subscribers: DashMap::new(),
            connections: DashMap::new(),
            handlers: DashMap::new(),
        }
    }

    fn subscribe(&self, client_id: &ClientId, topic: impl AsRef<str>) {
        let mut topic_set = self
            .subscribers
            .entry(topic.as_ref().to_string())
            .or_default();
        topic_set.insert(client_id.clone());
    }

    fn unsubscribe(&self, client_id: &ClientId, topic: impl AsRef<str>) {
        if let Some(mut subscribers) = self.subscribers.get_mut(topic.as_ref()) {
            subscribers.remove(client_id);
        }
    }

    fn get_subscribers(&self, topic: impl AsRef<str>) -> HashSet<ClientId> {
        self.subscribers
            .get(topic.as_ref())
            .map_or_else(HashSet::new, |s| s.value().clone())
    }

    fn add_connection(&self, client_id: ClientId, socket: SplitSink<WebSocket, Message>) {
        self.connections.insert(client_id, socket);
    }

    fn get_connection(
        &self,
        client_id: &ClientId,
    ) -> Option<RefMut<'_, ClientId, SplitSink<WebSocket, Message>>> {
        self.connections.get_mut(client_id)
    }

    fn remove_connection(&self, client_id: &ClientId) -> Option<SplitSink<WebSocket, Message>> {
        self.connections.remove(client_id).map(|(_, v)| v)
    }

    fn add_handler(&self, topic: impl AsRef<str>, handler: UnboundedSender<serde_json::Value>) {
        let mut handlers = self.handlers.entry(topic.as_ref().to_string()).or_default();
        handlers.push(handler);
    }

    fn get_handlers(
        &self,
        topic: impl AsRef<str>,
    ) -> Option<Ref<'_, String, Vec<UnboundedSender<serde_json::Value>>>> {
        self.handlers.get(topic.as_ref())
    }
}

static SERVER: LazyLock<PubSub> = LazyLock::new(|| PubSub::new());

pub async fn handle_socket<F: FnOnce(&str) -> Option<String> + Send + Sync>(
    stream: WebSocket,
    auth: F,
) {
    let (mut tx, mut rx) = stream.split();

    let Some(Ok(Message::Text(token))) = rx.next().await else {
        println!("Connection closed before auth");
        return;
    };
    let Some(user_id) = auth(&token) else {
        println!("Unauthorized access with token: {token}");
        let _ = tx.close().await;
        return;
    };
    println!("User {user_id} connected with token: {token}");

    let client_id = ClientId {
        connection_id: uuid::Uuid::new_v4().to_string(),
        user_id,
    };

    SERVER.add_connection(client_id.clone(), tx);

    while let Some(Ok(msg)) = rx.next().await {
        if let Message::Text(text) = msg {
            let Ok(ws_msg) = serde_json::from_str::<ClientMessage>(&text) else {
                println!("Failed to parse message: {text}");
                continue;
            };
            match ws_msg {
                ClientMessage::Subscribe { topic } => {
                    SERVER.subscribe(&client_id, &topic);
                    println!(
                        "Client {} subscribed to topic: {}",
                        client_id.connection_id, topic
                    );
                }
                ClientMessage::Unsubscribe { topic } => {
                    SERVER.unsubscribe(&client_id, &topic);
                    println!(
                        "Client {} unsubscribed from topic: {}",
                        client_id.connection_id, topic
                    );
                }
                ClientMessage::Publish { topic, payload } => {
                    for sub_id in SERVER.get_subscribers(&topic) {
                        if let Some(mut tx) = SERVER.get_connection(&sub_id) {
                            let msg = ServerMessage {
                                topic: topic.clone(),
                                payload: payload.clone(),
                            };
                            let _ = tx.send(Message::Text(serde_json::to_string(&msg).unwrap()));
                        }
                    }
                    if let Some(handlers) = SERVER.get_handlers(&topic) {
                        for handler in handlers.value() {
                            let _ = handler.send(payload.clone());
                        }
                    }
                    println!(
                        "Client {} published message to topic: {}, payload: {:?}",
                        client_id.connection_id, topic, payload
                    );
                }
            }
        }
    }

    if let Some(mut tx) = SERVER.remove_connection(&client_id) {
        let _ = tx.close().await;
        println!("Connection closed for client: {}", client_id.connection_id);
    }
}

pub async fn subscribe_to_topic(topic: impl AsRef<str>) -> UnboundedReceiver<serde_json::Value> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    SERVER.add_handler(topic.as_ref().to_string(), tx);
    rx
}

pub async fn handle_subscribe_to_topic<F, Fut>(topic: impl AsRef<str>, handler: F)
where
    F: (Fn(serde_json::Value) -> Fut) + Send + 'static,
    Fut: Future<Output = ()> + Send,
{
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    SERVER.add_handler(topic.as_ref().to_string(), tx);

    tokio::spawn(async move {
        while let Some(value) = rx.recv().await {
            handler(value).await;
        }
    });
}

pub async fn publish_to_topic<T>(topic: impl AsRef<str>, payload: T)
where
    T: serde::Serialize + Send + 'static,
{
    let payload = serde_json::to_value(payload).unwrap();
    for client_id in SERVER.get_subscribers(topic.as_ref()) {
        if let Some(mut tx) = SERVER.get_connection(&client_id) {
            let msg = ServerMessage {
                topic: topic.as_ref().to_string(),
                payload: payload.clone(),
            };
            if let Err(e) = tx
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await
            {
                println!(
                    "Failed to send message to subscriber {}: {}",
                    client_id.connection_id, e
                );
            }
        }
    }
    if let Some(handlers) = SERVER.get_handlers(&topic) {
        for handler in handlers.value() {
            let _ = handler.send(payload.clone());
        }
    }
}
