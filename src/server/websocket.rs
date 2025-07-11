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
use tak_core::{TakAction, TakPlayer};
use tokio::sync::Mutex;

use crate::components::ServerGameMessage;
use crate::server::auth::SessionStore;
use crate::server::room::{PlayerSocketMap, Room, Rooms};
use crate::views::ClientGameMessage;
use axum_extra::TypedHeader;
use futures_util::stream::{SplitSink, SplitStream};

pub struct PlayerSocket {
    pub sender: SplitSink<WebSocket, Message>,
    pub abort_handle: Option<(
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
    )>,
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
    let Some(player_id) = session_store.get(&session_id).cloned() else {
        println!("Unauthorized access from {who} with session_id {session_id}");
        let _ = socket.close().await;
        return;
    };

    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {who}");
    } else {
        println!("Could not send ping {who}!");
        let _ = socket.close().await;
        return;
    }

    let (mut sender, receiver) = socket.split();

    let rooms = state.rooms.lock().await;
    let Some(room) = rooms.try_get_room(&player_id) else {
        println!("Player {player_id} not in any room");
        let _ = sender.close().await;
        return;
    };

    let recv_sockets = rooms.player_sockets.clone();

    if Rooms::try_remove_socket(rooms, &player_id).await {
        println!("Old socket for {who} removed.");
    }

    let (sx, rx) = tokio::sync::oneshot::channel();

    let recv_room = room.clone();
    let recv_task = tokio::spawn(async move {
        room_receive_task(recv_room, recv_sockets, receiver).await;
        println!("Websocket receiver task of {who} ended.");
    });

    let wait_player = player_id.clone();
    let wait_rooms = state.rooms.clone();
    let wait_task = tokio::spawn(async move {
        tokio::select! {
            biased;
            _ = rx => {
                println!("Websocket handler for {who} was aborted.");
            }
            _ = recv_task => {
                println!("Websocket handler for {who} finished.");
            }
        }
        let mut rooms = wait_rooms.lock().await;
        rooms.try_remove_socket_no_cancel(&wait_player).await;
        println!("Websocket handler for {who} finished fully.");
    });

    let socket = PlayerSocket {
        sender,
        abort_handle: Some((sx, wait_task)),
    };

    let mut rooms = state.rooms.lock().await;
    rooms.add_socket(&player_id, socket);
    drop(rooms);

    println!("Waiting for end for {who}");
}

async fn on_room_receive_move(sockets: Arc<PlayerSocketMap>, room: &mut Room, action: &str) {
    let Some(game) = &mut room.game else {
        println!("Game hasn't started yet");
        return;
    };
    if let Some(action) = TakAction::from_ptn(game.board.size as i32, action) {
        let move_index = game.turn_index;
        let res = match game.try_do_action(action) {
            Ok(()) => game
                .get_last_action()
                .expect("Action history should not be empty"),
            Err(e) => {
                println!("Error processing action: {e:?}");
                return;
            }
        }
        .clone();
        let time_remaining = TakPlayer::ALL
            .into_iter()
            .map(|x| (x, game.get_time_remaining(x, true).unwrap()))
            .collect::<Vec<_>>();
        let msg = serde_json::to_string(&ServerGameMessage::Move(
            move_index,
            time_remaining,
            res.to_ptn(),
        ))
        .unwrap();
        for other_player in room.get_broadcast_player_ids() {
            if let Some(socket) = sockets.get(&other_player) {
                let socket = socket.clone();
                let sender = &mut socket.lock().await.sender;
                if sender.send(Message::Text(msg.clone())).await.is_err() {
                    println!("Failed to send message to player {other_player}");
                } else {
                    println!("Sent move action to player {other_player}: {res:?}");
                }
            }
        }
    } else {
        println!("Invalid action received: {action}");
    }
}

async fn room_receive_task(
    room: Arc<Mutex<Room>>,
    sockets: Arc<PlayerSocketMap>,
    mut receiver: SplitStream<WebSocket>,
) {
    while let Some(msg) = receiver.next().await {
        if let Ok(Message::Text(msg)) = &msg {
            if let Ok(msg) = serde_json::from_str::<ClientGameMessage>(msg) {
                match msg {
                    ClientGameMessage::Move(action) => {
                        let mut room_lock = room.lock().await;
                        on_room_receive_move(sockets.clone(), room_lock.deref_mut(), &action).await;
                    }
                }
            }
        }
    }
}
