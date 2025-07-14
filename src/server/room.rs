use crate::components::ServerGameMessage;
#[cfg(feature = "server")]
use crate::server::websocket::SharedState;
use dioxus::prelude::*;
use futures_util::SinkExt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;
use tak_core::{TakGame, TakPlayer};
use tak_core::{TakGameSettings, TakGameState};

pub type PlayerId = String;
pub type RoomId = String;

pub const ROOM_ID_LEN: usize = 4;

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
        self.game = Some(
            TakGame::new(self.settings.game_settings.clone()).expect("Settings should be valid"),
        );
    }

    fn is_ready(&self) -> bool {
        self.game.is_none()
            && TakPlayer::ALL
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
    username_cache: moka::future::Cache<PlayerId, String>,
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
            username_cache: moka::future::Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(5 * 60))
                .build(),
        }
    }

    fn generate_room_id() -> String {
        let mut rng = rand::thread_rng();
        (0..ROOM_ID_LEN)
            .map(|_| rng.gen_range(b'A'..=b'Z') as char)
            .collect::<String>()
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
        if TakGame::new(settings.game_settings.clone()).is_none() {
            return CreateRoomResponse::InvalidSettings;
        }
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
        let Some(player_type) = TakPlayer::ALL
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
            let Some(username) = self.get_user_username(id).await else {
                return Err(crate::server::auth::error::Error::InternalServerError(
                    "Failed to fetch user information".to_string(),
                ))?;
            };
            player_info.push((
                *player,
                PlayerInfo {
                    player_id: id.clone(),
                    username,
                    is_local: id == player_id,
                },
            ));
        }
        Ok(GetPlayersResponse::Success(player_info))
    }

    async fn get_user_username(&self, player_id: &PlayerId) -> Option<String> {
        let cached_username = self.username_cache.get(player_id).await;
        if let Some(username) = cached_username {
            return Some(username);
        }
        let user: Option<crate::server::auth::User> =
            crate::server::auth::handle_try_get_user(player_id)
                .await
                .ok()?;
        if user.is_some() {
            self.username_cache
                .insert(player_id.clone(), user.as_ref().unwrap().username.clone())
                .await;
        }
        user.map(|u| u.username)
    }

    async fn get_room_list(&self) -> GetRoomListResponse {
        let mut room_list = Vec::with_capacity(self.rooms.len());
        for (id, room) in &self.rooms {
            let room_lock = room.lock().await;
            let mut usernames = Vec::new();
            for (_, player_id) in &room_lock.players {
                let Some(username) = self.get_user_username(player_id).await else {
                    continue;
                };
                usernames.push(username);
            }
            room_list.push((id.clone(), room_lock.settings.clone(), usernames));
        }
        GetRoomListResponse::Success(room_list)
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
    InvalidSettings,
    Unauthorized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSettings {
    pub game_settings: TakGameSettings,
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
            maybe_start_game(state, &room_id).await;
        });
    }
    Ok(res)
}

#[cfg(feature = "server")]
async fn maybe_start_game(state: SharedState, room_id: &RoomId) {
    let mut rooms = state.rooms.lock().await;
    if !rooms
        .with_room_mut(room_id, |room| {
            if room.is_ready() {
                room.start_game();
                true
            } else {
                false
            }
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

    let Some(room) = rooms.rooms.get(room_id).cloned() else {
        println!("Room {} not found", room_id);
        return;
    };
    let sockets = rooms.player_sockets.clone();
    drop(rooms);
    room_check_timeout_task(room, sockets).await;
}

#[cfg(feature = "server")]
async fn room_check_timeout_task(
    room: Arc<tokio::sync::Mutex<Room>>,
    sockets: Arc<PlayerSocketMap>,
) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut room_lock = room.lock().await;
        let game_state = room_lock.game.as_mut().map(|game| {
            game.check_timeout();
            game.game_state.clone()
        });

        if !matches!(game_state, Some(TakGameState::Ongoing) | None) {
            let msg =
                serde_json::to_string(&ServerGameMessage::GameOver(game_state.unwrap())).unwrap();
            for other_player in room_lock.get_broadcast_player_ids() {
                if let Some(socket) = sockets.get(&other_player) {
                    let socket = socket.clone();
                    let sender = &mut socket.lock().await.sender;
                    if sender
                        .send(axum::extract::ws::Message::Text(msg.clone()))
                        .await
                        .is_err()
                    {
                        println!("Failed to send message to player {other_player}");
                    } else {
                        println!("Sent game over to player {other_player}");
                    }
                }
            }
            break;
        };
    }
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
    let res = rooms.try_leave_room(user.0.clone()).await;
    if let LeaveRoomResponse::Success = res {
        println!("Player {} left the room", user.0);
        Rooms::try_remove_socket(rooms, &user.0).await;
    }
    Ok(res)
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
    Success(Option<(String, Vec<(TakPlayer, u64)>)>),
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
        let time_remaining = TakPlayer::ALL
            .into_iter()
            .map(|x| (x, game.get_time_remaining(x, true).unwrap()))
            .collect::<Vec<_>>();
        Ok(GetGameStateResponse::Success(Some((
            game_state.to_str(),
            time_remaining,
        ))))
    } else {
        Ok(GetGameStateResponse::Success(None))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GetRoomListResponse {
    Success(Vec<(RoomId, RoomSettings, Vec<String>)>),
    Unauthorized,
}

#[server]
pub async fn get_room_list() -> Result<GetRoomListResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Some(_) = extract::<AuthenticatedUser, _>().await.ok() else {
        return Ok(GetRoomListResponse::Unauthorized);
    };
    let rooms = state.rooms.lock().await;
    Ok(rooms.get_room_list().await)
}
