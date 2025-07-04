use axum::extract::ConnectInfo;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    Extension,
};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::server::auth::SessionStore;
use crate::server::room::{PlayerId, Room, Rooms};
use crate::tak::{TakAction, TakGameAPI};
use crate::views::ClientGameMessage;
use axum_extra::TypedHeader;
use futures_util::stream::{SplitSink, SplitStream};

pub struct PlayerSocket {
    pub sender: SplitSink<WebSocket, Message>,
}

#[derive(Clone)]
pub struct SharedState {
    pub rooms: Arc<Mutex<Rooms>>,
}

impl SharedState {
    pub fn new() -> Self {
        SharedState {
            rooms: Arc::new(Mutex::new(Rooms::new())),
        }
    }
}

pub(crate) async fn ws_test_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    ws.on_upgrade(move |mut socket| async move {
        println!("WebSocket connection established with {addr}");
        while let Some(Ok(msg)) = socket.next().await {
            match msg {
                Message::Text(text) => {
                    println!("Received text message: {text}");
                    if socket.send(Message::Text(text)).await.is_err() {
                        println!("Failed to echo text message back to {addr}");
                    }
                }
                Message::Ping(_) => {
                    println!("Received ping, sending pong...");
                    if socket.send(Message::Pong(vec![])).await.is_err() {
                        println!("Failed to send pong");
                    }
                }
                _ => {}
            }
        }
        println!("WebSocket connection with {addr} closed");
    })
}

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(state): Extension<SharedState>,
    Extension(store): Extension<SessionStore>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    ws.on_upgrade(move |socket| handle_socket(socket, store, addr, state))
}

async fn handle_socket(
    mut socket: WebSocket,
    session_store: SessionStore,
    who: SocketAddr,
    state: SharedState,
) {
    let Some(Ok(msg)) = socket.next().await else {
        println!("Unauthorized access from {who}");
        let _ = socket.close().await;
        return;
    };
    
    let Message::Text(session_id) = msg else {
        println!("Unauthorized access from {who} with non-text message: {msg:?}");
        let _ = socket.close().await;
        return;
    };

    let session_store = session_store.lock().await;
    let Some(player_id) = session_store.get(&session_id) else {
        println!("Unauthorized access from {who} with session_id {session_id}");
        let _ = socket.close().await;
        return;
    };

    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {who}...");
    } else {
        println!("Could not send ping {who}!");
        let _ = socket.close().await;
        return;
    }
    
    let (mut sender, receiver) = socket.split();

    let rooms = state.rooms.lock().await;
    let Some(room_id) = rooms.player_mapping.get(player_id) else {
        let _ = sender.close().await;
        return;
    };
    let Some(room) = rooms.rooms.get(room_id).cloned() else {
        println!("Room {room_id} not found for player {player_id}");
        let _ = sender.close().await;
        return;
    };
    drop(rooms);

    let mut room_guard = room.lock().await;
    room_guard
        .player_sockets
        .insert(player_id.clone(), PlayerSocket { sender });
    drop(room_guard);

    let recv_room = room.clone();
    let recv_player = player_id.clone();
    let recv_task = tokio::spawn(async move {
        room_receive_task(recv_room, receiver, recv_player).await;
    });

    let res = recv_task.await;
    match res {
        Ok(_) => {}
        Err(b) => println!("Error receiving messages {b:?}"),
    }

    let mut room_guard = room.lock().await;
    room_guard.player_sockets.remove(player_id);
    drop(room_guard);

    println!("Websocket context {who} destroyed");
}

async fn on_room_receive_move(player: &PlayerId, room: &mut Room, action: &str) {
    if let Some(action) = TakAction::from_ptn(action) {
        let Some(opponent) = &room.opponent else {
            println!("No opponent to play against");
            return;
        };
        if let Err(e) = room.game.try_do_action(action.clone()) {
            println!("Error processing action: {e:?}");
        }
        let other_player = if room.owner == *player {
            opponent
        } else {
            &room.owner
        };
        let Some(socket) = room.player_sockets.get_mut(other_player) else {
            println!("No socket found for player {other_player}");
            return;
        };
        let msg = serde_json::to_string(&ClientGameMessage::Move(action.to_ptn())).unwrap();
        if socket.sender.send(Message::Text(msg)).await.is_err() {
            println!("Failed to send message to player {other_player}");
        } else {
            println!("Sent move action to player {other_player}: {action:?}");
        }
    } else {
        println!("Invalid action received: {action}");
    }
}

async fn room_receive_task(
    room: Arc<Mutex<Room>>,
    mut receiver: SplitStream<WebSocket>,
    player: PlayerId,
) {
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(msg) = &msg {
            if let Ok(msg) = serde_json::from_str::<ClientGameMessage>(msg) {
                match msg {
                    ClientGameMessage::Move(action) => {
                        let mut room_lock = room.lock().await;
                        on_room_receive_move(&player, room_lock.deref_mut(), &action).await;
                    }
                }
            }
        }
        println!(">>> {player} sent str: {msg:?}");
    }
}
