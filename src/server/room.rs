use crate::components::ServerGameMessage;
use crate::tak::{TakGame, TakPlayer, TakSettings};
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
    pub settings: RoomSettings,
    pub game: Option<TakGame>,
    pub players: Vec<(TakPlayer, PlayerId)>,
    pub spectators: Vec<PlayerId>,
}

#[cfg(feature = "server")]
impl Room {
    fn new(owner: PlayerId, settings: RoomSettings) -> Self {
        let mut room = Room {
            settings,
            players: Vec::new(),
            spectators: Vec::new(),
            game: None,
        };
        room.players.push((TakPlayer::White, owner));
        room
    }

    fn start_game(&mut self) {
        if self.game.is_some() {
            return;
        }
        self.game = Some(TakGame::new(self.settings.game_settings.clone()))
    }

    fn is_ready(&self) -> bool {
        let all_player_types = TakPlayer::all();
        all_player_types
            .iter()
            .all(|pt| self.players.iter().any(|(p, _)| p == pt))
    }

    fn is_empty(&self) -> bool {
        self.players.is_empty() && self.spectators.is_empty()
    }

    pub fn get_broadcast_player_ids(&self) -> Vec<PlayerId> {
        self.players
            .iter()
            .map(|(_, id)| id.clone())
            .chain(self.spectators.iter().cloned())
            .collect()
    }
}

#[cfg(feature = "server")]
pub struct Rooms {
    rooms: HashMap<RoomId, Arc<tokio::sync::Mutex<Room>>>,
    player_mapping: HashMap<PlayerId, RoomId>,
    pub player_sockets: Arc<PlayerSocketMap>,
}

#[cfg(feature = "server")]
pub type PlayerSocketMap =
    dashmap::DashMap<PlayerId, Arc<tokio::sync::Mutex<crate::server::websocket::PlayerSocket>>>;

#[cfg(feature = "server")]
impl Rooms {
    pub fn new() -> Self {
        Rooms {
            rooms: HashMap::new(),
            player_mapping: HashMap::new(),
            player_sockets: Arc::new(dashmap::DashMap::new()),
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

    fn try_generate_room_id(&self) -> Option<RoomId> {
        let mut attempts = 100;
        loop {
            let id = Self::generate_room_id();
            if !self.rooms.contains_key(&id) {
                return Some(id);
            }
            attempts -= 1;
            if attempts == 0 {
                return None;
            }
        }
    }

    fn try_create_room(&mut self, settings: RoomSettings, owner: PlayerId) -> CreateRoomResponse {
        if self.player_mapping.contains_key(&owner) {
            return CreateRoomResponse::AlreadyInRoom;
        }
        let Some(id) = self.try_generate_room_id() else {
            return CreateRoomResponse::FailedToGenerateId;
        };
        self.rooms.insert(
            id.clone(),
            Arc::new(tokio::sync::Mutex::new(Room::new(owner.clone(), settings))),
        );
        self.player_mapping.insert(owner, id.clone());
        CreateRoomResponse::Success(id)
    }

    async fn try_join_room_as_player(
        &mut self,
        room_id: RoomId,
        player_id: PlayerId,
    ) -> JoinRoomResponse {
        if self.player_mapping.contains_key(&player_id) {
            return JoinRoomResponse::AlreadyInRoom;
        }
        let Some(room) = self.rooms.get(&room_id) else {
            return JoinRoomResponse::RoomNotFound;
        };
        let mut room_lock = room.lock().await;
        let all_player_types = TakPlayer::all();
        let Some(player_type) = all_player_types
            .into_iter()
            .find(|x| !room_lock.players.iter().any(|(p, _)| *p == *x))
        else {
            return JoinRoomResponse::RoomFull;
        };
        room_lock.players.push((player_type, player_id.clone()));
        self.player_mapping.insert(player_id, room_id);
        JoinRoomResponse::Success
    }

    async fn try_join_room_as_spectator(
        &mut self,
        room_id: RoomId,
        player_id: PlayerId,
    ) -> JoinRoomResponse {
        if self.player_mapping.contains_key(&player_id) {
            return JoinRoomResponse::AlreadyInRoom;
        }
        let Some(room) = self.rooms.get(&room_id) else {
            return JoinRoomResponse::RoomNotFound;
        };
        let mut room_lock = room.lock().await;
        room_lock.spectators.push(player_id.clone());
        self.player_mapping.insert(player_id, room_id);
        JoinRoomResponse::Success
    }

    async fn try_leave_room(&mut self, player_id: PlayerId) -> LeaveRoomResponse {
        let Some(room_id) = self.player_mapping.remove(&player_id) else {
            return LeaveRoomResponse::NotInARoom;
        };
        let room = self.rooms.get(&room_id).unwrap();

        let mut room_lock = room.lock().await;
        if let Some(player_pos) = room_lock
            .players
            .iter()
            .position(|(_, id)| *id == player_id)
        {
            room_lock.players.swap_remove(player_pos);
        } else if let Some(spec_pos) = room_lock.spectators.iter().position(|id| *id == player_id) {
            room_lock.spectators.swap_remove(spec_pos);
        } else {
            return LeaveRoomResponse::NotInARoom;
        }

        if room_lock.is_empty() {
            drop(room_lock);
            self.rooms.remove(&room_id);
            println!("Room {} was empty and removed", room_id);
        }
        LeaveRoomResponse::Success
    }

    pub async fn try_get_room_id(&self, player_id: &PlayerId) -> GetRoomResponse {
        if let Some(room_id) = self.player_mapping.get(player_id) {
            let room = self.rooms.get(room_id).unwrap();
            let room_lock = room.lock().await;
            let settings = room_lock.settings.clone();
            drop(room_lock);
            GetRoomResponse::Success(room_id.clone(), settings)
        } else {
            GetRoomResponse::NotInARoom
        }
    }

    pub fn try_get_room(&self, player_id: &PlayerId) -> Option<Arc<tokio::sync::Mutex<Room>>> {
        self.player_mapping
            .get(player_id)
            .map(|room_id| self.rooms.get(room_id).unwrap().clone())
    }

    pub async fn with_room_mut<F, R>(&self, room_id: &RoomId, func: F) -> R
    where
        F: FnOnce(&mut Room) -> R,
    {
        let room = self.rooms.get(room_id).unwrap();
        let mut lock = room.lock().await;
        func(lock.deref_mut())
    }

    async fn try_get_players(
        &self,
        player_id: &PlayerId,
    ) -> Result<GetPlayersResponse, ServerFnError> {
        let Some(room_id) = self.player_mapping.get(player_id) else {
            return Ok(GetPlayersResponse::NotInARoom);
        };
        let Some(room) = self.rooms.get(room_id) else {
            return Ok(GetPlayersResponse::NotInARoom);
        };
        let room_lock = room.lock().await;
        let mut player_info = Vec::with_capacity(room_lock.players.len());
        for (player, id) in &room_lock.players {
            let Ok(Some(user)) = crate::server::auth::handle_try_get_user(id).await else {
                return Err(crate::server::auth::error::Error::InternalServerError(
                    "Failed to fetch user information".to_string(),
                ))?;
            };
            player_info.push((
                *player,
                PlayerInfo {
                    player_id: id.clone(),
                    username: user.username,
                    is_local: id == player_id,
                },
            ));
        }
        Ok(GetPlayersResponse::Success(player_info))
    }

    pub fn add_socket(
        &mut self,
        player_id: &PlayerId,
        socket: crate::server::websocket::PlayerSocket,
    ) {
        self.player_sockets
            .insert(player_id.clone(), Arc::new(tokio::sync::Mutex::new(socket)));
    }

    pub async fn try_remove_socket_no_cancel(&mut self, player_id: &PlayerId) -> bool {
        if let Some((_, socket)) = self.player_sockets.remove(player_id) {
            let mut socket = socket.lock().await;
            let _ = socket.sender.close().await;
            true
        } else {
            false
        }
    }

    pub async fn try_remove_socket(
        rooms: tokio::sync::MutexGuard<'_, Self>,
        player_id: &PlayerId,
    ) -> bool {
        if let Some((_, socket)) = rooms.player_sockets.remove(player_id) {
            drop(rooms);
            let mut socket = socket.lock().await;
            let _ = socket.sender.close().await;
            if let Some((sender, wait_task)) = socket.abort_handle.take() {
                drop(socket);
                let _ = sender.send(());
                let _ = wait_task.await;
            }
            true
        } else {
            false
        }
    }

    pub async fn get_broadcast_player_ids(&mut self, room_id: &RoomId) -> Vec<PlayerId> {
        let room = self.rooms.get(room_id).unwrap();
        let room_lock = room.lock().await;
        room_lock.get_broadcast_player_ids()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreateRoomResponse {
    Success(RoomId),
    AlreadyInRoom,
    FailedToGenerateId,
    Unauthorized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSettings {
    pub game_settings: TakSettings,
}

#[server]
pub async fn create_room(settings: RoomSettings) -> Result<CreateRoomResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Ok(user) = extract::<AuthenticatedUser, _>().await else {
        return Ok(CreateRoomResponse::Unauthorized);
    };
    let mut rooms = state.rooms.lock().await;
    Ok(rooms.try_create_room(settings, user.0))
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
pub async fn join_room(
    room_id: String,
    is_spectator: bool,
) -> Result<JoinRoomResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Some(user) = extract::<AuthenticatedUser, _>().await.ok() else {
        return Ok(JoinRoomResponse::Unauthorized);
    };
    let mut rooms = state.rooms.lock().await;
    let res = if is_spectator {
        rooms
            .try_join_room_as_spectator(room_id.clone(), user.0.clone())
            .await
    } else {
        rooms
            .try_join_room_as_player(room_id.clone(), user.0.clone())
            .await
    };
    drop(rooms);

    if let JoinRoomResponse::Success = res {
        println!(
            "Player {} joined room {} as {}",
            user.0,
            room_id,
            if is_spectator { "spectator" } else { "player" }
        );
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let mut rooms = state.rooms.lock().await;
            maybe_start_game(&mut rooms, &room_id).await;
        });
    }
    Ok(res)
}

#[cfg(feature = "server")]
async fn maybe_start_game(rooms: &mut Rooms, room_id: &RoomId) {
    if !rooms
        .with_room_mut(room_id, |room| {
            let is_ready = room.is_ready();
            if is_ready {
                room.start_game();
            }
            is_ready
        })
        .await
    {
        return;
    }

    let msg = ServerGameMessage::StartGame;
    let msg = axum::extract::ws::Message::Text(serde_json::to_string(&msg).unwrap());

    for player in rooms.get_broadcast_player_ids(room_id).await {
        if let Some(socket_ref) = rooms.player_sockets.get(&player) {
            let socket_ref = socket_ref.clone();
            let mut socket = socket_ref.lock().await;
            let _ = socket.sender.send(msg.clone()).await;
        }
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
    Ok(rooms.try_leave_room(user.0.clone()).await)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GetRoomResponse {
    Success(RoomId, RoomSettings),
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
    Ok(rooms.try_get_room_id(&user.0).await)
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

    let Extension(state): Extension<SharedState> = extract().await?;
    let rooms = state.rooms.lock().await;
    rooms.try_get_players(&user.0).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GetGameStateResponse {
    Success(Option<(String, Vec<(TakPlayer, Duration)>)>),
    NotInARoom,
    Unauthorized,
}

#[server]
pub async fn get_game_state() -> Result<GetGameStateResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Some(user) = extract::<AuthenticatedUser, _>().await.ok() else {
        return Ok(GetGameStateResponse::Unauthorized);
    };
    let rooms = state.rooms.lock().await;
    let Some(room) = rooms.try_get_room(&user.0) else {
        return Ok(GetGameStateResponse::NotInARoom);
    };
    let room_lock = room.lock().await;
    if let Some(game) = &room_lock.game {
        let game_state = game.to_ptn();
        let time_remaining = TakPlayer::all()
            .into_iter()
            .map(|x| (x, game.get_time_remaining(x)))
            .collect::<Vec<_>>();
        Ok(GetGameStateResponse::Success(Some((
            game_state.to_str(),
            time_remaining,
        ))))
    } else {
        Ok(GetGameStateResponse::Success(None))
    }
}
