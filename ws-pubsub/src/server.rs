use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};

use axum::extract::ws::{Message, WebSocket};
use dashmap::{DashMap, mapref::one::RefMut};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};

use crate::{AUTH_ACK, ServerMessage, TopicMatcher, message::ClientMessage};
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender},
};

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ClientId {
    pub connection_id: String,
    pub user_id: String,
}

struct PubSub {
    subscribers: DashMap<String, HashSet<ClientId>>,
    subscriptions: DashMap<ClientId, HashSet<String>>,
    connections: DashMap<ClientId, SplitSink<WebSocket, Message>>,
    handlers: Arc<Mutex<TopicMatcher<Vec<UnboundedSender<serde_json::Value>>>>>,
}

impl PubSub {
    fn new() -> Self {
        PubSub {
            subscribers: DashMap::new(),
            subscriptions: DashMap::new(),
            connections: DashMap::new(),
            handlers: Arc::new(Mutex::new(TopicMatcher::new())),
        }
    }

    fn subscribe(&self, client_id: &ClientId, topic: impl AsRef<str>) {
        let mut topic_set = self
            .subscribers
            .entry(topic.as_ref().to_string())
            .or_default();
        topic_set.insert(client_id.clone());

        let mut subscriptions = self.subscriptions.entry(client_id.clone()).or_default();
        subscriptions.insert(topic.as_ref().to_string());
    }

    fn unsubscribe(&self, client_id: &ClientId, topic: impl AsRef<str>) {
        if let Some(mut subscribers) = self.subscribers.get_mut(topic.as_ref()) {
            subscribers.remove(client_id);
            if subscribers.is_empty() {
                drop(subscribers);
                self.subscribers.remove(topic.as_ref());
            }
        }
        if let Some(mut subscriptions) = self.subscriptions.get_mut(client_id) {
            subscriptions.remove(topic.as_ref());
            if subscriptions.is_empty() {
                drop(subscriptions);
                self.subscriptions.remove(client_id);
            }
        }
    }

    fn get_subscribers(&self, topic: impl AsRef<str>) -> HashSet<ClientId> {
        self.subscribers
            .get(topic.as_ref())
            .map_or_else(HashSet::new, |s| s.value().clone())
    }

    fn remove_all_subscriptions(&self, client_id: &ClientId) {
        if let Some((_, subscriptions)) = self.subscriptions.remove(client_id) {
            for topic in subscriptions {
                if let Some(mut subscribers) = self.subscribers.get_mut(&topic) {
                    subscribers.remove(client_id);
                    if subscribers.is_empty() {
                        drop(subscribers);
                        self.subscribers.remove(&topic);
                    }
                }
            }
        }
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
        self.remove_all_subscriptions(client_id);
        self.connections.remove(client_id).map(|(_, v)| v)
    }

    async fn add_handler(
        &self,
        topic: impl AsRef<str>,
        handler: UnboundedSender<serde_json::Value>,
    ) {
        let mut lock = self.handlers.lock().await;
        if let Some(existing) = lock.get_mut(topic.as_ref()) {
            existing.push(handler);
            return;
        } else {
            lock.insert(topic.as_ref().to_string(), vec![handler]);
            return;
        }
    }

    async fn with_handlers(
        &self,
        topic: impl AsRef<str>,
        callback: impl FnOnce(Vec<&UnboundedSender<serde_json::Value>>),
    ) {
        let lock = self.handlers.lock().await;
        let mut handlers = Vec::new();
        for (_, vec) in lock.matches(topic.as_ref()) {
            handlers.extend(vec.iter());
        }
        callback(handlers);
    }
}

static SERVER: LazyLock<PubSub> = LazyLock::new(|| PubSub::new());

pub async fn handle_socket<F: FnOnce(&str) -> Option<String> + Send + Sync>(
    stream: WebSocket,
    auth: F,
) {
    let (mut tx, mut rx) = stream.split();

    let Some(Ok(Message::Text(token))) = rx.next().await else {
        println!("Invalid initial message, expected auth token");
        let _ = tx.close().await;
        return;
    };
    let Some(user_id) = auth(&token) else {
        println!("Unauthorized access with token: {token}");
        let _ = tx.close().await;
        return;
    };
    println!("User {user_id} connected with token: {token}");
    if let Err(e) = tx.send(Message::Text(AUTH_ACK.to_string())).await {
        println!("Failed to send auth acknowledgment: {e}");
        let _ = tx.close().await;
        return;
    }

    let client_id = ClientId {
        connection_id: uuid::Uuid::new_v4().to_string(),
        user_id,
    };
    println!("New connection established: {}", client_id.connection_id);

    SERVER.add_connection(client_id.clone(), tx);

    process_socket(rx, &client_id).await;
    println!("Processor ended for client: {}", client_id.connection_id);

    if let Some(mut tx) = SERVER.remove_connection(&client_id) {
        let _ = tx.close().await;
        println!("Connection closed for client: {}", client_id.connection_id);
    }
}

async fn process_socket(mut rx: futures::stream::SplitStream<WebSocket>, client_id: &ClientId) {
    while let Some(msg) = rx.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error receiving message: {}", e);
                break;
            }
        };
        if let Message::Text(text) = msg {
            let Ok(ws_msg) = serde_json::from_str::<ClientMessage>(&text) else {
                println!("Failed to parse message: {text}");
                continue;
            };
            match ws_msg {
                ClientMessage::Subscribe { topic } => {
                    SERVER.subscribe(client_id, &topic);
                    println!(
                        "Client {} subscribed to topic: {}",
                        client_id.connection_id, topic
                    );
                }
                ClientMessage::Unsubscribe { topic } => {
                    SERVER.unsubscribe(client_id, &topic);
                    println!(
                        "Client {} unsubscribed from topic: {}",
                        client_id.connection_id, topic
                    );
                }
                ClientMessage::Publish { topic, payload } => {
                    SERVER
                        .with_handlers(&topic, |handlers| {
                            for handler in handlers {
                                if let Err(e) = handler.send(payload.clone()) {
                                    eprintln!(
                                        "Failed to send message to handler for topic {}: {}",
                                        topic, e
                                    );
                                }
                            }
                        })
                        .await;
                    println!(
                        "Client {} published message to topic: {}, payload: {:?}",
                        client_id.connection_id, topic, payload
                    );
                }
            }
        }
    }
}

pub async fn subscribe_to_topic(topic: impl AsRef<str>) -> UnboundedReceiver<serde_json::Value> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    SERVER.add_handler(topic.as_ref().to_string(), tx).await;
    rx
}

pub fn handle_subscribe_to_topic<T, Fut>(
    topic: impl AsRef<str>,
    handler: impl (Fn(T) -> Fut) + Send + 'static,
) where
    T: serde::de::DeserializeOwned + Send + 'static,
    Fut: Future<Output = ()> + Send,
{
    let topic = topic.as_ref().to_string();
    tokio::spawn(async move {
        let mut rx = subscribe_to_topic(topic).await;
        while let Some(value) = rx.recv().await {
            let value = match serde_json::from_value(value) {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("Failed to deserialize value: {:?}", e);
                    continue;
                }
            };
            handler(value).await;
        }
    });
}

pub async fn publish_to_topic<T>(topic: impl AsRef<str>, payload: T)
where
    T: serde::Serialize + Send + 'static,
{
    let msg = ServerMessage {
        topic: topic.as_ref().to_string(),
        payload: serde_json::to_value(payload).unwrap(),
    };
    for client_id in SERVER.get_subscribers(topic.as_ref()) {
        if let Some(mut tx) = SERVER.get_connection(&client_id) {
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
}

pub async fn publish_to_topic_and_server<T>(topic: impl AsRef<str>, payload: T)
where
    T: serde::Serialize + Send + 'static + Clone,
{
    publish_to_topic(topic.as_ref(), payload.clone()).await;
    let payload = serde_json::to_value(payload).unwrap();
    SERVER
        .with_handlers(topic.as_ref(), |handlers| {
            for handler in handlers {
                if let Err(e) = handler.send(payload.clone()) {
                    eprintln!(
                        "Failed to send message to handler for topic {}: {}",
                        topic.as_ref(),
                        e
                    );
                }
            }
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubsub() {
        let pubsub = PubSub::new();

        let client_id = ClientId {
            connection_id: "test_connection".to_string(),
            user_id: "test_user".to_string(),
        };

        pubsub.subscribe(&client_id, "test_topic");
        pubsub.subscribe(&client_id, "test_topic2");
        assert!(pubsub.get_subscribers("test_topic").contains(&client_id));
        assert!(pubsub.get_subscribers("test_topic2").contains(&client_id));
        assert!(
            pubsub
                .subscriptions
                .get(&client_id)
                .is_some_and(|s| s.contains("test_topic") && s.contains("test_topic2"))
        );

        pubsub.unsubscribe(&client_id, "test_topic");
        assert!(!pubsub.get_subscribers("test_topic").contains(&client_id));
        assert!(pubsub.get_subscribers("test_topic2").contains(&client_id));

        pubsub.unsubscribe(&client_id, "test_topic2");
        assert!(!pubsub.get_subscribers("test_topic2").contains(&client_id));

        assert!(pubsub.subscriptions.get(&client_id).is_none());
    }

    #[test]
    fn test_remove_client() {
        let pubsub = PubSub::new();

        let client_id = ClientId {
            connection_id: "test_connection".to_string(),
            user_id: "test_user".to_string(),
        };

        pubsub.subscribe(&client_id, "test_topic");
        pubsub.subscribe(&client_id, "test_topic2");
        pubsub.subscribe(&client_id, "test_topic/test");

        pubsub.remove_all_subscriptions(&client_id);

        assert!(!pubsub.get_subscribers("test_topic").contains(&client_id));
        assert!(!pubsub.get_subscribers("test_topic2").contains(&client_id));
        assert!(
            !pubsub
                .get_subscribers("test_topic/test")
                .contains(&client_id)
        );
        assert!(pubsub.subscriptions.get(&client_id).is_none());
    }
}
