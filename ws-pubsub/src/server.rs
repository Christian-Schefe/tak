use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};

use axum::extract::ws::{Message, WebSocket};
use dashmap::{DashMap, mapref::one::RefMut};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};

use crate::{AuthResponse, PublishMessage, Topic, TopicMatcher};
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender},
};

pub type SubscriptionId = String;
pub type UserId = String;
pub type ConnectionId = String;

pub type ServerHandler = UnboundedSender<(UserId, Topic, serde_json::Value)>;

pub struct ClientInfo {
    subscriptions: HashMap<SubscriptionId, Topic>,
    topics: HashMap<Topic, HashSet<SubscriptionId>>,
}

impl Default for ClientInfo {
    fn default() -> Self {
        ClientInfo {
            subscriptions: HashMap::new(),
            topics: HashMap::new(),
        }
    }
}

impl ClientInfo {
    fn add_subscription(
        &mut self,
        subscription_id: &SubscriptionId,
        topic: &Topic,
    ) -> Option<Topic> {
        if self.subscriptions.len() >= 500 {
            return None;
        }
        self.subscriptions
            .insert(subscription_id.clone(), topic.clone());
        let topics = self.topics.entry(topic.clone()).or_default();
        let was_empty = topics.is_empty();
        topics.insert(subscription_id.clone());
        if was_empty { Some(topic.clone()) } else { None }
    }

    fn remove_subscription(&mut self, subscription_id: &SubscriptionId) -> Option<Topic> {
        if let Some(topic) = self.subscriptions.remove(subscription_id) {
            if let Some(topics) = self.topics.get_mut(&topic) {
                topics.remove(subscription_id);
                if topics.is_empty() {
                    self.topics.remove(&topic);
                    return Some(topic);
                }
            }
        }
        None
    }

    fn all_topics(&self) -> impl Iterator<Item = &Topic> {
        self.topics.keys()
    }

    fn is_empty(&self) -> bool {
        self.subscriptions.is_empty() && self.topics.is_empty()
    }
}

struct PubSub {
    topic_to_subscribers: DashMap<Topic, HashSet<UserId>>,
    client_info: DashMap<UserId, ClientInfo>,

    connections: DashMap<UserId, HashMap<ConnectionId, SplitSink<WebSocket, Message>>>,
    handlers: Arc<Mutex<TopicMatcher<Vec<ServerHandler>>>>,
}

impl PubSub {
    fn new() -> Self {
        PubSub {
            topic_to_subscribers: DashMap::new(),
            client_info: DashMap::new(),

            connections: DashMap::new(),
            handlers: Arc::new(Mutex::new(TopicMatcher::new())),
        }
    }

    fn subscribe(&self, user_id: &UserId, subscription_id: &SubscriptionId, topic: &Topic) -> bool {
        if self.connections.get(user_id).is_none() {
            return false;
        }
        let mut client_info = self.client_info.entry(user_id.clone()).or_default();
        if let Some(topic) = client_info.add_subscription(subscription_id, topic) {
            let mut subscribers = self.topic_to_subscribers.entry(topic.clone()).or_default();
            subscribers.insert(user_id.clone());
        }
        true
    }

    fn unsubscribe(&self, user_id: &UserId, subscription_id: &SubscriptionId) {
        if let Some(mut client_info) = self.client_info.get_mut(user_id) {
            if let Some(topic) = client_info.remove_subscription(subscription_id) {
                if let Some(mut subscribers) = self.topic_to_subscribers.get_mut(&topic) {
                    subscribers.remove(user_id);
                    if subscribers.is_empty() {
                        drop(subscribers);
                        self.topic_to_subscribers.remove(&topic);
                    }
                }
            }
            if client_info.is_empty() {
                drop(client_info);
                self.client_info.remove(user_id);
            }
        }
    }

    fn get_subscribers(&self, topic: impl AsRef<str>) -> HashSet<UserId> {
        self.topic_to_subscribers
            .get(topic.as_ref())
            .map_or_else(HashSet::new, |s| s.value().clone())
    }

    fn remove_all_subscriptions(&self, user_id: &UserId) {
        if let Some((_, client_info)) = self.client_info.remove(user_id) {
            for topic in client_info.all_topics() {
                if let Some(mut subscribers) = self.topic_to_subscribers.get_mut(topic) {
                    subscribers.remove(user_id);
                    if subscribers.is_empty() {
                        drop(subscribers);
                        self.topic_to_subscribers.remove(topic);
                    }
                }
            }
        }
    }

    fn add_connection(
        &self,
        user_id: &UserId,
        connection_id: &ConnectionId,
        socket: SplitSink<WebSocket, Message>,
    ) {
        let mut connections = self.connections.entry(user_id.clone()).or_default();
        connections.insert(connection_id.clone(), socket);
    }

    fn get_connections(
        &self,
        user_id: &UserId,
    ) -> Option<RefMut<'_, UserId, HashMap<ConnectionId, SplitSink<WebSocket, Message>>>> {
        self.connections.get_mut(user_id)
    }

    fn remove_connection(
        &self,
        user_id: &UserId,
        connection_id: &ConnectionId,
    ) -> Option<SplitSink<WebSocket, Message>> {
        let mut connections = self.connections.get_mut(user_id)?;
        if let Some(socket) = connections.remove(connection_id) {
            if connections.is_empty() {
                drop(connections);
                self.connections.remove(user_id);
                self.remove_all_subscriptions(user_id);
            }
            return Some(socket);
        }
        None
    }

    async fn add_handler(&self, topic: impl AsRef<str>, handler: ServerHandler) {
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
        callback: impl FnOnce(Vec<&ServerHandler>),
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
        let _ = tx
            .send(Message::Text(
                serde_json::to_string(&AuthResponse::Failure).unwrap(),
            ))
            .await;
        let _ = tx.close().await;
        return;
    };
    println!("User {user_id} connected with token: {token}");
    let connection_id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = tx
        .send(Message::Text(
            serde_json::to_string(&AuthResponse::Success(connection_id.clone())).unwrap(),
        ))
        .await
    {
        println!("Failed to send auth acknowledgment: {e}");
        let _ = tx.close().await;
        return;
    }

    println!("New connection established: {}", connection_id);

    SERVER.add_connection(&user_id, &connection_id, tx);

    process_socket(rx, &user_id).await;
    println!("Processor ended for client: {}", connection_id);

    if let Some(mut tx) = SERVER.remove_connection(&user_id, &connection_id) {
        let _ = tx.close().await;
        println!("Connection closed for client: {}", connection_id);
    }
    println!("Handler ended for client: {}", connection_id);
}

async fn process_socket(mut rx: futures::stream::SplitStream<WebSocket>, user_id: &UserId) {
    while let Some(msg) = rx.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error receiving message: {}", e);
                break;
            }
        };
        if let Message::Text(text) = msg {
            let Ok(PublishMessage { topic, payload }) =
                serde_json::from_str::<PublishMessage>(&text)
            else {
                println!("Failed to parse message: {text}");
                continue;
            };
            SERVER
                .with_handlers(&topic, |handlers| {
                    for handler in handlers {
                        if let Err(e) =
                            handler.send((user_id.clone(), topic.clone(), payload.clone()))
                        {
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
                user_id, topic, payload
            );
        }
    }
}

pub async fn subscribe_to_topic(
    topic: impl Into<Topic>,
) -> UnboundedReceiver<(UserId, Topic, serde_json::Value)> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    SERVER.add_handler(topic.into(), tx).await;
    rx
}

pub fn handle_subscribe_to_topic<T, Fut>(
    topic: impl Into<Topic>,
    handler: impl (Fn(UserId, Topic, T) -> Fut) + Send + 'static,
) where
    T: serde::de::DeserializeOwned + Send + 'static,
    Fut: Future<Output = ()> + Send,
{
    let topic = topic.into();
    tokio::spawn(async move {
        let mut rx = subscribe_to_topic(&topic).await;
        while let Some((player_id, actual_topic, value)) = rx.recv().await {
            let value = match serde_json::from_value(value) {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("Failed to deserialize value: {:?}", e);
                    continue;
                }
            };
            handler(player_id, actual_topic.clone(), value).await;
        }
    });
}

pub async fn publish_to_topic<T>(topic: impl Into<Topic>, payload: T)
where
    T: serde::Serialize + Send + 'static,
{
    let topic = topic.into();
    let msg = PublishMessage {
        topic: topic.clone(),
        payload: serde_json::to_value(payload).unwrap(),
    };
    for user_id in SERVER.get_subscribers(&topic) {
        if let Some(mut connections) = SERVER.get_connections(&user_id) {
            for tx in connections.values_mut() {
                if let Err(e) = tx
                    .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                    .await
                {
                    println!("Failed to send message to subscriber {}: {}", user_id, e);
                }
            }
        }
    }
}

pub async fn client_subscribe(topic: &String, user_id: &UserId) -> Option<String> {
    let subscription_id = uuid::Uuid::new_v4().to_string();
    if SERVER.subscribe(user_id, &subscription_id, topic) {
        Some(subscription_id)
    } else {
        None
    }
}

pub async fn client_unsubscribe(user_id: &UserId, subscription_id: &SubscriptionId) {
    SERVER.unsubscribe(user_id, subscription_id);
}
