#[cfg(not(feature = "server"))]
use dioxus::core::use_drop;
use dioxus::prelude::*;
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio_tungstenite_wasm::{Message, WebSocketStream};

use crate::{
    AuthResponse, PublishMessage, ServerFunctions, Topic,
    future::{Service, run_service},
};

pub static WS_CLIENT: Global<WsConnector> = Global::new(|| WsConnector::new());

#[derive(Clone)]
pub struct WsConnector {
    ws_connection: Signal<Option<String>>,
    ws_stream: Signal<Option<SplitStream<WebSocketStream>>>,
    ws_sink: Signal<Option<SplitSink<WebSocketStream, Message>>>,
    pub url: Signal<Option<String>>,
    pub token: Signal<Option<String>>,
    send_service: Service<serde_json::Value, Result<(), String>>,
    handlers: Arc<WsHandlers>,
}

pub struct WsHandlers {
    pub handlers: Mutex<HashMap<String, HashMap<String, UnboundedSender<serde_json::Value>>>>,
}

impl WsHandlers {
    pub fn new() -> Self {
        Self {
            handlers: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_handler(&self, topic: String, sender: UnboundedSender<serde_json::Value>) -> String {
        let key = uuid::Uuid::new_v4().to_string();
        let mut topic_handlers = self.handlers.lock().unwrap();
        topic_handlers
            .entry(topic)
            .or_insert_with(HashMap::new)
            .insert(key.clone(), sender);
        key
    }

    pub fn remove_handler(&self, topic: &String, id: &String) {
        self.handlers
            .lock()
            .unwrap()
            .get_mut(topic)
            .map(|handlers| {
                handlers.remove(id);
            });
    }

    pub fn send_to_topic(&self, topic: &String, payload: serde_json::Value) {
        let topic_handlers = self.handlers.lock().unwrap();
        if let Some(handlers) = topic_handlers.get(topic) {
            for handler in handlers.values() {
                if handler.unbounded_send(payload.clone()).is_err() {
                    dioxus::logger::tracing::error!(
                        "Failed to send message to handler for topic: {}",
                        topic
                    );
                }
            }
        } else {
            dioxus::logger::tracing::warn!("No handlers found for topic: {}", topic);
        }
    }
}

impl WsConnector {
    pub fn new() -> Self {
        Self {
            ws_connection: Signal::new(None),
            ws_stream: Signal::new(None),
            ws_sink: Signal::new(None),
            url: Signal::new(None),
            token: Signal::new(None),
            send_service: Service::new(),
            handlers: Arc::new(WsHandlers::new()),
        }
    }

    async fn close_connection(&mut self) {
        if let Some(sink) = self.ws_sink.write().as_mut() {
            let _ = sink.close().await;
        }
        self.ws_stream.write().take();
        self.ws_sink.write().take();
        self.ws_connection.write().take();
    }

    async fn try_connect(&mut self) -> Option<()> {
        let url = self.url.peek().clone()?;
        let token = self.token.peek().clone()?;
        let mut stream = match tokio_tungstenite_wasm::connect(&url).await {
            Ok(stream) => stream,
            Err(e) => {
                dioxus::logger::tracing::error!("Failed to connect: {}", e);
                return None;
            }
        };
        if let Err(e) = stream.send(Message::Text(token.into())).await {
            dioxus::logger::tracing::error!("Failed to send token: {}", e);
            return None;
        };
        let auth_response = stream.next().await;
        let connection_id = match auth_response {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<AuthResponse>(&text) {
                Ok(AuthResponse::Failure) => {
                    self.token.write().take();
                    None
                }
                Ok(AuthResponse::Success(r)) => Some(r),
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to parse auth response: {}", e);
                    None
                }
            },
            Some(Err(e)) => {
                dioxus::logger::tracing::error!("Failed to receive auth response: {}", e);
                None
            }
            _ => None,
        }?;
        self.close_connection().await;
        let (ws_sink, ws_stream) = stream.split();
        *self.ws_sink.write() = Some(ws_sink);
        *self.ws_stream.write() = Some(ws_stream);
        *self.ws_connection.write() = Some(connection_id);
        Some(())
    }
}

fn use_create_connection(connector: WsConnector) {
    let mut try_reconnect = use_signal(|| 0);
    use_effect(move || {
        let retry_count = *try_reconnect.read();
        let mut connector = connector.clone();
        if connector.url.read().is_none() {
            spawn(async move {
                connector.close_connection().await;
            });
            return;
        };
        if connector.token.read().is_none() {
            spawn(async move {
                connector.close_connection().await;
            });
            return;
        };
        if connector.ws_connection.read().is_some() {
            return;
        };
        spawn(async move {
            dioxus::logger::tracing::info!("Attempting to connect to WebSocket");
            if connector.try_connect().await.is_none() {
                dioxus::logger::tracing::error!("Failed to connect to WebSocket");
                crate::future::sleep(std::time::Duration::from_secs(5)).await;
                try_reconnect.set(retry_count + 1);
            }
        });
    });
}

fn use_receive(connector: WsConnector) {
    let _ = use_resource(move || {
        let mut connector = connector.clone();
        async move {
            let _ = connector.ws_connection.read();
            let stream = connector.ws_stream.write().take();
            let Some(mut stream) = stream else {
                return;
            };
            while let Some(msg) = stream.next().await {
                match msg {
                    Err(e) => {
                        dioxus::logger::tracing::error!("WebSocket error: {}", e);
                        connector.close_connection().await;
                        return;
                    }
                    Ok(Message::Text(text)) => {
                        dioxus::logger::tracing::info!("Received message: {}", text);
                        let parsed = serde_json::from_str::<PublishMessage>(&text);
                        match parsed {
                            Ok(PublishMessage { topic, payload }) => {
                                connector.handlers.send_to_topic(&topic, payload);
                            }
                            Err(e) => {
                                dioxus::logger::tracing::error!("Failed to parse message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Binary(_)) => {
                        dioxus::logger::tracing::warn!(
                            "Received binary message, which is not handled."
                        );
                    }
                    Ok(Message::Close(_)) => {
                        dioxus::logger::tracing::info!("WebSocket connection closed.");
                        connector.close_connection().await;
                        return;
                    }
                }
            }
        }
    });
}

fn use_send(connector: WsConnector) {
    let service = connector.send_service.clone();

    let _ = use_resource(move || {
        let mut connector = connector.clone();
        let service = service.clone();
        async move {
            let _ = connector.ws_connection.read();
            let sink = connector.ws_sink.write().take();
            let Some(sink) = sink else {
                return;
            };
            dioxus::logger::tracing::info!("WebSocket sink is ready for sending messages");
            run_service(service, sink, move |mut sink: SplitSink<WebSocketStream, Message>, msg: serde_json::Value| async move {
                if let Err(e) = sink.send(Message::Text(msg.to_string().into())).await {
                    return (Err(format!("Failed to send message: {}", e)), sink);
                }
                (Ok(()), sink)
            })
            .await;
            dioxus::logger::tracing::info!("WebSocket sink ended");
        }
    });
}

pub fn use_ws_connection() -> WsConnector {
    let connector = WS_CLIENT.resolve();
    use_create_connection(connector.clone());
    use_receive(connector.clone());
    use_send(connector.clone());
    connector
}

fn use_ws_topic_subscription<ServerFut: ServerFunctions>(
    connector: WsConnector,
    topic: ReadOnlySignal<Option<String>>,
) {
    let mut retry_subscribe = use_signal(|| 0);
    let mut cur_sub = use_signal(|| None);
    let mut old_sub = use_signal(|| None);

    let drop_subscription = move |sub_id: Option<String>| {
        if let Some(subscription_id) = sub_id {
            dioxus::logger::tracing::info!("Dropping old subscription: {}", subscription_id);
            spawn(async move {
                if let Err(e) = ServerFut::unsubscribe(subscription_id).await {
                    dioxus::logger::tracing::error!("Failed to unsubscribe: {:?}", e);
                }
            });
        }
    };

    use_effect(move || {
        let sub_id = old_sub.read().clone();
        drop_subscription(sub_id);
    });

    #[cfg(not(feature = "server"))]
    use_drop(move || {
        let sub_id = cur_sub.read().clone();
        drop_subscription(sub_id);
    });

    let _ = use_resource(move || {
        let connector = connector.clone();
        let retry_count = *retry_subscribe.read();
        async move {
            old_sub.set(cur_sub.peek().clone());
            let Some(topic) = topic.read().clone() else {
                cur_sub.set(None);
                return;
            };
            dioxus::logger::tracing::info!("Subscribing to topic: {}", topic);
            if connector.ws_connection.read().is_none() {
                return;
            }
            match ServerFut::subscribe(topic).await {
                Ok(subscription_id) => {
                    dioxus::logger::tracing::info!(
                        "Subscribed to topic with ID: {}",
                        subscription_id
                    );
                    cur_sub.set(Some(subscription_id.clone()));
                }
                Err(e) => {
                    dioxus::logger::tracing::error!("Failed to subscribe to topic: {:?}", e);
                    spawn(async move {
                        crate::future::sleep(std::time::Duration::from_secs(5)).await;
                        retry_subscribe.set(retry_count + 1);
                    });
                }
            }
        }
    });
}

fn use_ws_topic_local_subscription<T: DeserializeOwned + 'static, ServerFut: ServerFunctions, Fut>(
    connector: WsConnector,
    topic: ReadOnlySignal<Option<String>>,
    handler: impl Fn(T) -> Fut + 'static,
) where
    Fut: Future<Output = ()>,
{
    let handler = Arc::new(handler);
    let mut cur_sub = use_signal(|| None);
    let mut old_sub = use_signal(|| None);

    let connector_handlers = connector.handlers.clone();
    let drop_subscription = move |sub_id: Option<(String, String)>| {
        if let Some((subscription_topic, subscription_id)) = sub_id {
            dioxus::logger::tracing::info!(
                "Dropping old local subscription: {} {}",
                subscription_topic,
                subscription_id
            );
            connector_handlers.remove_handler(&subscription_topic, &subscription_id);
        }
    };

    let drop_subscription_clone = drop_subscription.clone();
    use_effect(move || {
        let sub_id = old_sub.read().clone();
        drop_subscription_clone(sub_id);
    });

    #[cfg(not(feature = "server"))]
    use_drop(move || {
        let sub_id = cur_sub.read().clone();
        drop_subscription(sub_id);
    });

    let _ = use_resource(move || {
        let connector = connector.clone();
        let handler = handler.clone();
        async move {
            old_sub.set(cur_sub.peek().clone());
            let Some(topic) = topic.read().clone() else {
                cur_sub.set(None);
                return;
            };
            dioxus::logger::tracing::info!("Setting up topic handler for: {}", topic);
            let (tx, mut rx) = futures::channel::mpsc::unbounded();
            let local_sub = connector.handlers.add_handler(topic.clone(), tx);
            cur_sub.set(Some((topic.clone(), local_sub.clone())));
            while let Some(message) = rx.next().await {
                dioxus::logger::tracing::info!(
                    "Received message on topic {}: {:?}",
                    topic,
                    message
                );
                let parsed = serde_json::from_value::<T>(message);
                match parsed {
                    Ok(data) => {
                        handler(data).await;
                    }
                    Err(e) => {
                        dioxus::logger::tracing::error!("Failed to parse message: {}", e);
                    }
                }
            }
        }
    });
}

pub fn use_ws_topic_receive<T: DeserializeOwned + 'static, ServerFut: ServerFunctions, Fut>(
    topic: impl Into<Topic>,
    handler: impl Fn(T) -> Fut + 'static,
) where
    Fut: Future<Output = ()>,
{
    let topic = topic.into();
    use_ws_topic_receive_dynamic::<T, ServerFut, Fut>(move || Some(topic.clone()), handler);
}

pub fn use_ws_topic_receive_dynamic<
    T: DeserializeOwned + 'static,
    ServerFut: ServerFunctions,
    Fut,
>(
    get_topic: impl Fn() -> Option<Topic> + 'static,
    handler: impl Fn(T) -> Fut + 'static,
) where
    Fut: Future<Output = ()>,
{
    let topic = use_memo(move || get_topic());
    let connector = WS_CLIENT.resolve();
    use_ws_topic_local_subscription::<T, ServerFut, Fut>(connector.clone(), topic.into(), handler);
    use_ws_topic_subscription::<ServerFut>(connector, topic.into());
}

pub fn use_ws_topic_send<T: Serialize + 'static>(
    topic: impl Into<Topic>,
) -> Service<T, Result<(), String>> {
    let connector = WS_CLIENT.resolve();
    let topic_clone = topic.into();
    let service = use_hook(|| crate::future::Service::<T, Result<(), String>>::new());
    let service_clone = service.clone();
    use_future(move || {
        let topic = topic_clone.clone();
        let service = service_clone.clone();
        let send_service = connector.send_service.clone();
        async move {
            crate::future::run_service(service, (), move |(), msg: T| {
                let value = serde_json::to_value(&PublishMessage {
                    topic: topic.clone(),
                    payload: serde_json::to_value(msg).unwrap(),
                })
                .unwrap();
                let send_service = send_service.clone();
                async move {
                    let res = send_service
                        .send(value)
                        .await
                        .unwrap_or(Err("Service not running".to_string()));
                    (res, ())
                }
            })
            .await;
        }
    });
    service
}
