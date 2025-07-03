use crate::tak::{TakGameAPI, TimeMode, TimedTakGame};
use dioxus::prelude::*;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

pub type PlayerId = String;
pub type RoomId = String;

struct Room {
    owner: PlayerId,
    opponent: Option<PlayerId>,
    game: TimedTakGame,
}

impl Room {
    fn new(owner: PlayerId) -> Self {
        Room {
            owner,
            opponent: None,
            game: TimedTakGame::new_game(
                5,
                TimeMode::new(Duration::from_secs(300), Duration::from_secs(5)),
            ),
        }
    }
}

pub struct Rooms {
    rooms: HashMap<RoomId, Room>,
    player_mapping: HashMap<PlayerId, RoomId>,
}

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
    rooms.rooms.insert(id.clone(), Room::new(user.0.clone()));
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
pub async fn join_room(room: String) -> Result<JoinRoomResponse, ServerFnError> {
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
    let Some(room) = rooms.rooms.get_mut(&room) else {
        return Ok(JoinRoomResponse::RoomNotFound);
    };
    if room.opponent.is_some() {
        return Ok(JoinRoomResponse::RoomFull);
    }
    room.opponent = Some(user.0.clone());
    Ok(JoinRoomResponse::Success)
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
