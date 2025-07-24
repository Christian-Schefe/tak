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
use tak_core::{TakAction, TakGameState, TakPlayer};
use tokio::sync::Mutex;

use crate::components::ServerGameMessage;
use crate::server::internal::auth::SessionStore;
use crate::server::internal::room::{ArcMutexDashMap, Room, ROOMS};
use crate::server::UserId;
use crate::views::ClientGameMessage;
use axum_extra::TypedHeader;
use futures_util::stream::{SplitSink, SplitStream};

pub struct PlayerSocket {
    pub connections: Vec<Option<PlayerConnection>>,
}

impl PlayerSocket {
    pub async fn send(&mut self, msg: &str) -> bool {
        let mut all_success = true;
        for conn in self.connections.iter_mut().filter_map(|x| x.as_mut()) {
            all_success &= conn
                .sender
                .send(Message::Text(msg.to_string()))
                .await
                .is_ok();
        }
        all_success
    }
}

pub struct PlayerConnection {
    pub sender: SplitSink<WebSocket, Message>,
    pub join_handle: Option<tokio::task::JoinHandle<()>>,
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
    Extension(store): Extension<SessionStore>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    ws.on_upgrade(move |socket| handle_socket(socket, store, addr))
}

async fn handle_socket(mut socket: WebSocket, session_store: SessionStore, who: SocketAddr) {
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

    let rooms = ROOMS.read().await;
    let Some(room) = rooms.try_get_room(&player_id) else {
        println!("Player {player_id} not in any room");
        let _ = sender.close().await;
        return;
    };
    let socket = PlayerConnection {
        sender,
        join_handle: None,
    };
    let id = rooms.add_connection(&player_id, socket).await;
    let recv_sockets = rooms.player_sockets.clone();
    drop(rooms);

    let recv_room = room.clone();
    let recv_player = player_id.clone();
    let recv_task = tokio::spawn(async move {
        room_receive_task(
            recv_room,
            recv_sockets.clone(),
            receiver,
            recv_player.clone(),
        )
        .await;
        let rooms = ROOMS.read().await;
        rooms.remove_connection(&recv_player, id).await;
        println!("Websocket receiver task of {who} ended.");
    });

    let rooms = ROOMS.read().await;
    rooms
        .add_handle_to_connection(&player_id, id, recv_task)
        .await;

    println!("Waiting for end for {who}");
}

async fn on_room_receive_move(
    sockets: ArcMutexDashMap<UserId, PlayerSocket>,
    player: &str,
    room: &mut Room,
    action: &str,
) {
    let Some(room_game) = &mut room.game else {
        println!("Game hasn't started yet");
        return;
    };
    let game = &mut room_game.game;
    if game.game_state != TakGameState::Ongoing {
        return;
    }
    let Some((action_player, _)) = room_game
        .player_mapping
        .iter()
        .find(|(_, p)| p.as_str() == player)
    else {
        println!("Not a player of this game");
        return;
    };
    if game.current_player != action_player {
        println!("Not your turn");
        return;
    }
    if let Some(action) = TakAction::from_ptn(action) {
        if !game.check_timeout() {
            println!("Received action: {action:?}");
            let move_index = game.ply_index;
            let res = match game.try_do_action(action) {
                Ok(()) => game
                    .get_last_action()
                    .expect("Action history should not be empty"),
                Err(e) => {
                    println!(
                        "Error processing action: {e:?}, {}",
                        game.to_tps().to_string()
                    );
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
                    let sender = &mut socket.lock().await;
                    if sender.send(&msg).await {
                        println!("Sent move action to player {other_player}: {res:?}");
                    } else {
                        println!(
                            "Failed to send message to some connections of player {other_player}"
                        );
                    }
                }
            }
        }

        room.check_end_game();
    } else {
        println!("Invalid action received: {action}");
    }
}

async fn room_receive_task(
    room: Arc<Mutex<Room>>,
    sockets: ArcMutexDashMap<UserId, PlayerSocket>,
    mut receiver: SplitStream<WebSocket>,
    player: String,
) {
    while let Some(msg) = receiver.next().await {
        if let Ok(Message::Text(msg)) = &msg {
            if let Ok(msg) = serde_json::from_str::<ClientGameMessage>(msg) {
                match msg {
                    ClientGameMessage::Move(action) => {
                        let mut room_lock = room.lock().await;
                        on_room_receive_move(
                            sockets.clone(),
                            &player,
                            room_lock.deref_mut(),
                            &action,
                        )
                        .await;
                    }
                }
            }
        }
    }
}
