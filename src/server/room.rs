use crate::components::ServerGameMessage;
use crate::tak::{TakGameAPI, TakPlayer, TimeMode, TimedTakGame};
use dioxus::prelude::*;
use futures_util::SinkExt;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;

pub type PlayerId = String;
pub type RoomId = String;

#[cfg(feature = "server")]
pub struct Room {
    pub owner: PlayerId,
    pub opponent: Option<PlayerId>,
    pub game: TimedTakGame,
    pub player_sockets: HashMap<PlayerId, crate::server::websocket::PlayerSocket>,
}

#[cfg(feature = "server")]
impl Room {
    fn new(owner: PlayerId) -> Self {
        Room {
            owner,
            opponent: None,
            game: TimedTakGame::new_game(
                5,
                TimeMode::new(Duration::from_secs(300), Duration::from_secs(5)),
            ),
            player_sockets: HashMap::new(),
        }
    }
}

#[cfg(feature = "server")]
pub struct Rooms {
    pub rooms: HashMap<RoomId, Arc<tokio::sync::Mutex<Room>>>,
    pub player_mapping: HashMap<PlayerId, RoomId>,
}

#[cfg(feature = "server")]
impl Rooms {
    pub fn new() -> Self {
        Rooms {
            rooms: HashMap::new(),
            player_mapping: HashMap::new(),
        }
    }

    fn generate_room_id() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_uppercase() as char)
            .take(6)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreateRoomResponse {
    Success(RoomId),
    AlreadyInRoom,
    FailedToGenerateId,
    Unauthorized,
}

#[server]
pub async fn create_room() -> Result<CreateRoomResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Ok(user) = extract::<AuthenticatedUser, _>().await else {
        return Ok(CreateRoomResponse::Unauthorized);
    };
    let mut rooms = state.rooms.lock().await;
    if rooms.player_mapping.contains_key(&user.0) {
        return Ok(CreateRoomResponse::AlreadyInRoom);
    }
    let mut attempts = 100;
    let id = loop {
        let id = Rooms::generate_room_id();
        if !rooms.rooms.contains_key(&id) {
            break id;
        }
        attempts -= 1;
        if attempts == 0 {
            return Ok(CreateRoomResponse::FailedToGenerateId);
        }
    };
    rooms.rooms.insert(
        id.clone(),
        Arc::new(tokio::sync::Mutex::new(Room::new(user.0.clone()))),
    );
    rooms.player_mapping.insert(user.0, id.clone());
    Ok(CreateRoomResponse::Success(id))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinRoomResponse {
    Success,
    RoomNotFound,
    AlreadyInRoom,
    RoomFull,
    Unauthorized,
}

#[server]
pub async fn join_room(room_id: String) -> Result<JoinRoomResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Some(user) = extract::<AuthenticatedUser, _>().await.ok() else {
        return Ok(JoinRoomResponse::Unauthorized);
    };
    let mut rooms = state.rooms.lock().await;
    if rooms.player_mapping.contains_key(&user.0) {
        return Ok(JoinRoomResponse::AlreadyInRoom);
    }
    let Some(room) = rooms.rooms.get(&room_id) else {
        return Ok(JoinRoomResponse::RoomNotFound);
    };
    let room = room.clone();
    let mut room_lock = room.lock().await;
    if room_lock.opponent.is_some() {
        return Ok(JoinRoomResponse::RoomFull);
    }
    room_lock.opponent = Some(user.0.clone());
    rooms.player_mapping.insert(user.0, room_id.clone());
    start_game(room_lock.deref_mut()).await;
    Ok(JoinRoomResponse::Success)
}

#[cfg(feature = "server")]
async fn start_game(room: &mut Room) {
    let msg = ServerGameMessage::StartGame(5);
    let msg = axum::extract::ws::Message::Text(serde_json::to_string(&msg).unwrap());

    if let Some(owner_socket) = room.player_sockets.get_mut(&room.owner) {
        let _ = owner_socket.sender.send(msg.clone()).await;
    }

    if let Some(opponent_socket) = room.player_sockets.get_mut(room.opponent.as_ref().unwrap()) {
        let _ = opponent_socket.sender.send(msg.clone());
    }

    println!("Sent start game message");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeaveRoomResponse {
    Success,
    NotInARoom,
    Unauthorized,
}

#[server]
pub async fn leave_room() -> Result<LeaveRoomResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Some(user) = extract::<AuthenticatedUser, _>().await.ok() else {
        return Ok(LeaveRoomResponse::Unauthorized);
    };
    let mut rooms = state.rooms.lock().await;
    let Some(room_id) = rooms.player_mapping.remove(&user.0) else {
        return Ok(LeaveRoomResponse::NotInARoom);
    };
    if let Some(room) = rooms.rooms.get_mut(&room_id) {
        let mut room = room.lock().await;
        if room.owner == user.0 {
            let opponent = room.opponent.clone();
            drop(room);
            rooms.rooms.remove(&room_id);
            if let Some(opponent) = opponent {
                rooms.player_mapping.remove(&opponent);
            }
        } else {
            room.opponent = None;
        }
    }
    Ok(LeaveRoomResponse::Success)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GetRoomResponse {
    Success(RoomId),
    NotInARoom,
    Unauthorized,
}

#[server]
pub async fn get_room() -> Result<GetRoomResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Some(user) = extract::<AuthenticatedUser, _>().await.ok() else {
        return Ok(GetRoomResponse::Unauthorized);
    };
    let rooms = state.rooms.lock().await;
    if let Some(room_id) = rooms.player_mapping.get(&user.0) {
        Ok(GetRoomResponse::Success(room_id.clone()))
    } else {
        Ok(GetRoomResponse::NotInARoom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub player_id: PlayerId,
    pub username: String,
    pub is_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GetPlayersResponse {
    Success(Vec<(TakPlayer, PlayerInfo)>),
    NotInARoom,
    Unauthorized,
}

#[server]
pub async fn get_players() -> Result<GetPlayersResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Some(user) = extract::<AuthenticatedUser, _>().await.ok() else {
        return Ok(GetPlayersResponse::Unauthorized);
    };

    println!("Getting players for user: {}", user.0);

    let Extension(state): Extension<SharedState> = extract().await?;
    let rooms = state.rooms.lock().await;
    let Some(room_id) = rooms.player_mapping.get(&user.0) else {
        return Ok(GetPlayersResponse::NotInARoom);
    };
    let room = rooms.rooms.get(room_id).ok_or_else(|| {
        crate::server::auth::error::Error::InternalServerError("Room not found".to_string())
    })?;
    let room = room.lock().await;

    println!("Getting players in room: {}", room_id);

    let Some(owner): Option<crate::server::auth::User> = crate::server::auth::DB
        .select(("user", &room.owner))
        .await?
    else {
        return Err(crate::server::auth::error::Error::InternalServerError(
            format!("Player {} doesn't exist", &room.owner),
        ))?;
    };
    let opponent = {
        if let Some(opponent_id) = &room.opponent {
            let Some(opponent): Option<crate::server::auth::User> = crate::server::auth::DB
                .select(("user", opponent_id))
                .await?
            else {
                return Err(crate::server::auth::error::Error::InternalServerError(
                    format!("Player {} doesn't exist", opponent_id),
                ))?;
            };
            Some(opponent)
        } else {
            None
        }
    };

    drop(room);

    let owner_info = PlayerInfo {
        is_local: owner.user_id == user.0,
        player_id: owner.user_id,
        username: owner.username,
    };
    let mut player_info = vec![(TakPlayer::White, owner_info)];
    if let Some(opponent) = opponent {
        let opponent_info = PlayerInfo {
            is_local: opponent.user_id == user.0,
            player_id: opponent.user_id,
            username: opponent.username,
        };
        player_info.push((TakPlayer::Black, opponent_info));
    }
    Ok(GetPlayersResponse::Success(player_info))
}
