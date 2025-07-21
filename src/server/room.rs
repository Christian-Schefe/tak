use crate::components::ServerGameMessage;
use dioxus::prelude::*;
use futures_util::SinkExt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
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
    pub game: Option<RoomGame>,
    pub players: Vec<PlayerId>,
    pub spectators: Vec<PlayerId>,
    pub rematch_agree: HashSet<PlayerId>,
}

#[cfg(feature = "server")]
pub struct RoomGame {
    pub game: TakGame,
    pub game_end_sender: tokio::sync::watch::Sender<TakGameState>,
    pub player_mapping: fixed_map::Map<TakPlayer, PlayerId>,
}

#[cfg(feature = "server")]
impl Room {
    fn new(owner: PlayerId, settings: RoomSettings) -> Self {
        Self {
            settings,
            players: vec![owner],
            spectators: Vec::new(),
            game: None,
            rematch_agree: HashSet::new(),
        }
    }

    fn remove_player(&mut self, player_id: &PlayerId) -> Option<bool> {
        if let Some(pos) = self.players.iter().position(|id| id == player_id) {
            self.players.swap_remove(pos);
            Some(true)
        } else if let Some(pos) = self.spectators.iter().position(|id| id == player_id) {
            self.spectators.swap_remove(pos);
            Some(false)
        } else {
            None
        }
    }

    fn can_join(&self) -> bool {
        !self.is_full() && self.game.is_none()
    }

    fn is_full(&self) -> bool {
        self.players.len() >= TakPlayer::ALL.len()
    }

    fn try_start_game(&mut self) -> bool {
        if !self.is_ready() && !self.is_rematch_ready() {
            return false;
        }
        self.rematch_agree.clear();
        let rev = match self.settings.first_player_mode {
            Some(first_player) => first_player != TakPlayer::ALL[0],
            None => rand::random(),
        };
        let player_iter = if rev {
            TakPlayer::ALL.into_iter().rev().collect::<Vec<_>>()
        } else {
            TakPlayer::ALL.into_iter().collect::<Vec<_>>()
        };
        let player_mapping =
            fixed_map::Map::from_iter(player_iter.into_iter().zip(self.players.iter().cloned()));
        let (game_end_sender, _) = tokio::sync::watch::channel(TakGameState::Ongoing);
        self.game = Some(RoomGame {
            game: TakGame::new(self.settings.game_settings.clone())
                .expect("Settings should be valid"),
            player_mapping,
            game_end_sender,
        });
        true
    }

    fn abort_game(&mut self, player_id: &PlayerId) {
        if let Some(game) = &mut self.game {
            let (winner, _) = game
                .player_mapping
                .iter()
                .find(|(_, p)| **p != *player_id)
                .expect("Game should have a winner");
            game.game.abort(winner);
            self.check_end_game();
        }
    }

    pub fn check_end_game(&mut self) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        if game.game.game_state == TakGameState::Ongoing {
            return;
        }
        let _ = game.game_end_sender.send(game.game.game_state.clone());
    }

    fn is_ready(&self) -> bool {
        self.game.is_none() && self.players.len() == TakPlayer::ALL.len()
    }

    fn is_rematch_ready(&self) -> bool {
        self.game
            .as_ref()
            .is_some_and(|game| game.game.game_state != TakGameState::Ongoing)
            && self.players.len() == TakPlayer::ALL.len()
            && self.players.iter().all(|p| self.rematch_agree.contains(p))
    }

    fn is_empty(&self) -> bool {
        self.players.is_empty() && self.spectators.is_empty()
    }

    pub fn get_broadcast_player_ids(&self) -> Vec<PlayerId> {
        self.players
            .iter()
            .cloned()
            .chain(self.spectators.iter().cloned())
            .collect()
    }
}

#[cfg(feature = "server")]
pub struct Rooms {
    rooms: HashMap<RoomId, Arc<tokio::sync::Mutex<Room>>>,
    player_mapping: HashMap<PlayerId, RoomId>,
    pub player_sockets: Arc<PlayerSocketMap>,
    player_data_cache: moka::future::Cache<PlayerId, (String, f64)>,
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
            player_data_cache: moka::future::Cache::builder()
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
        if !room_lock.can_join() {
            return JoinRoomResponse::RoomFull;
        }
        room_lock.players.push(player_id.clone());
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
        let Some(was_player) = room_lock.remove_player(&player_id) else {
            return LeaveRoomResponse::NotInARoom;
        };

        if was_player {
            room_lock.abort_game(&player_id);
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

    pub fn try_get_room_pair(
        &self,
        player_id: &PlayerId,
    ) -> Option<(RoomId, Arc<tokio::sync::Mutex<Room>>)> {
        self.player_mapping
            .get(player_id)
            .map(|room_id| (room_id.clone(), self.rooms.get(room_id).unwrap().clone()))
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
        for (player, id) in room_lock.game.as_ref().map_or_else(
            || Vec::new(),
            |game| game.player_mapping.iter().collect::<Vec<_>>(),
        ) {
            let Some((username, rating)) = self.get_user_data(id).await else {
                return Err(crate::server::auth::error::Error::InternalServerError(
                    "Failed to fetch user information".to_string(),
                ))?;
            };
            player_info.push((
                player,
                PlayerInfo {
                    player_id: id.to_string(),
                    username,
                    rating,
                    is_local: *id == *player_id,
                },
            ));
        }
        Ok(GetPlayersResponse::Success(player_info))
    }

    async fn get_user_data(&self, player_id: &PlayerId) -> Option<(String, f64)> {
        let cached_data = self.player_data_cache.get(player_id).await;
        if let Some(data) = cached_data {
            return Some(data);
        }
        let user: crate::server::auth::User = crate::server::auth::handle_try_get_user(player_id)
            .await
            .ok()??;
        let player = crate::server::player::get_or_insert_player(player_id)
            .await
            .ok()?;

        self.player_data_cache
            .insert(player_id.clone(), (user.username.clone(), player.rating))
            .await;

        Some((user.username, player.rating))
    }

    async fn get_room_list(&self) -> GetRoomListResponse {
        let mut room_list = Vec::with_capacity(self.rooms.len());
        for (id, room) in &self.rooms {
            let room_lock = room.lock().await;
            let mut usernames = Vec::new();
            for player_id in &room_lock.players {
                let Some((username, _)) = self.get_user_data(player_id).await else {
                    continue;
                };
                usernames.push(username);
            }
            room_list.push(RoomListItem {
                room_id: id.clone(),
                settings: room_lock.settings.clone(),
                usernames,
                can_join: room_lock.can_join(),
            });
        }
        GetRoomListResponse::Success(room_list)
    }

    pub async fn add_connection(
        &mut self,
        player_id: &PlayerId,
        connection: crate::server::websocket::PlayerConnection,
    ) -> usize {
        let socket = self
            .player_sockets
            .entry(player_id.to_string())
            .or_insert_with(|| {
                Arc::new(tokio::sync::Mutex::new(
                    crate::server::websocket::PlayerSocket {
                        connections: Vec::new(),
                    },
                ))
            });

        let mut lock = socket.lock().await;
        let id = lock.connections.len();
        lock.connections.push(Some(connection));
        id
    }

    pub async fn add_handle_to_connection(
        &mut self,
        player_id: &PlayerId,
        id: usize,
        handle: tokio::task::JoinHandle<()>,
    ) -> Option<()> {
        let socket = self.player_sockets.get_mut(player_id)?;
        let mut lock = socket.lock().await;
        let connection = lock.connections.get_mut(id)?;
        let join_handle = &mut connection.as_mut()?.join_handle;
        if join_handle.is_some() {
            return None;
        }
        *join_handle = Some(handle);
        Some(())
    }

    pub async fn terminate_socket(&mut self, player_id: &PlayerId) {
        if let Some((_, socket)) = self.player_sockets.remove(player_id) {
            let mut lock = socket.lock().await;
            for connection in lock.connections.iter_mut().filter_map(|x| x.as_mut()) {
                let _ = connection.sender.close().await;
                if let Some(handle) = connection.join_handle.take() {
                    handle.abort();
                }
            }
        }
    }

    pub async fn remove_connection(
        player_sockets: Arc<PlayerSocketMap>,
        player_id: &PlayerId,
        id: usize,
    ) {
        if let Some(socket) = player_sockets.get_mut(player_id) {
            let mut lock = socket.lock().await;
            lock.connections[id] = None;
            if let Some(last_some_index) = lock.connections.iter().rposition(|x| x.is_some()) {
                lock.connections.truncate(last_some_index + 1);
            } else {
                lock.connections.clear();
            }
        };
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
    pub first_player_mode: Option<TakPlayer>,
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
            tokio::time::sleep(Duration::from_millis(500)).await;
            maybe_start_game(state, &room_id).await;
        });
    }
    Ok(res)
}

#[cfg(feature = "server")]
async fn maybe_start_game(state: crate::server::websocket::SharedState, room_id: &RoomId) {
    let mut rooms = state.rooms.lock().await;
    if !rooms
        .with_room_mut(room_id, |room| room.try_start_game())
        .await
    {
        return;
    }

    let msg = ServerGameMessage::StartGame;
    let msg = serde_json::to_string(&msg).unwrap();

    for player in rooms.get_broadcast_player_ids(room_id).await {
        if let Some(socket_ref) = rooms.player_sockets.get(&player) {
            let socket_ref = socket_ref.clone();
            let mut socket = socket_ref.lock().await;
            socket.send(&msg).await;
        }
    }

    println!("Sent start game message");

    let Some(room) = rooms.rooms.get(room_id).cloned() else {
        println!("Room {} not found", room_id);
        return;
    };
    let sockets = rooms.player_sockets.clone();
    let game_end_receiver = rooms
        .with_room_mut(room_id, |room| {
            room.game
                .as_ref()
                .expect("Room should have game")
                .game_end_sender
                .subscribe()
        })
        .await;
    drop(rooms);
    let room_clone = room.clone();
    tokio::spawn(async move {
        room_check_timeout_task(room_clone).await;
    });
    room_check_gameover_task(game_end_receiver, room, sockets).await;
    println!("Sent end game message");
}

#[cfg(feature = "server")]
async fn room_check_timeout_task(room: Arc<tokio::sync::Mutex<Room>>) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut room_lock = room.lock().await;
        let game_state = room_lock.game.as_mut().map(|game| {
            game.game.check_timeout();
            game.game.game_state.clone()
        });

        if !matches!(game_state, Some(TakGameState::Ongoing) | None) {
            room_lock.check_end_game();
            break;
        };
    }
}

#[cfg(feature = "server")]
async fn room_check_gameover_task(
    mut game_end_receiver: tokio::sync::watch::Receiver<TakGameState>,
    room: Arc<tokio::sync::Mutex<Room>>,
    sockets: Arc<PlayerSocketMap>,
) {
    if game_end_receiver.changed().await.is_err() {
        return;
    };
    let game_state = game_end_receiver.borrow_and_update().clone();
    let msg = serde_json::to_string(&ServerGameMessage::GameOver(game_state.clone())).unwrap();
    let room_lock = room.lock().await;
    for player in room_lock.get_broadcast_player_ids() {
        if let Some(socket) = sockets.get(&player) {
            let socket = socket.clone();
            let sender = &mut socket.lock().await;
            if sender.send(&msg).await {
                println!("Sent game over {:?} to player {player}", game_state.clone());
            } else {
                println!(
                    "Failed to send message {:?} to some connections of player {player}",
                    game_state.clone()
                );
            }
        }
    }
    let result = match game_state {
        TakGameState::Draw => crate::server::player::GameResult::Draw,
        TakGameState::Win(player, _) => {
            let first_player_won = room_lock
                .game
                .as_ref()
                .unwrap()
                .player_mapping
                .get(player)
                .is_some_and(|id| id == &room_lock.players[0]);
            if first_player_won {
                crate::server::player::GameResult::Win
            } else {
                crate::server::player::GameResult::Loss
            }
        }
        TakGameState::Ongoing => unreachable!(),
    };

    if let Err(e) =
        crate::server::player::add_game_result(&room_lock.players[0], &room_lock.players[1], result)
            .await
    {
        eprintln!("Failed to add game result: {:?}", e);
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
        rooms.terminate_socket(&user.0).await;
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
    pub rating: f64,
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
        let game_state = game.game.to_ptn();
        let time_remaining = TakPlayer::ALL
            .into_iter()
            .map(|x| (x, game.game.get_time_remaining(x, true).unwrap()))
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
pub struct RoomListItem {
    pub room_id: RoomId,
    pub settings: RoomSettings,
    pub usernames: Vec<String>,
    pub can_join: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GetRoomListResponse {
    Success(Vec<RoomListItem>),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgreeRematchResponse {
    Success,
    NotInARoom,
    Unauthorized,
}

#[server]
pub async fn agree_rematch() -> Result<AgreeRematchResponse, ServerFnError> {
    use crate::server::auth::AuthenticatedUser;
    use crate::server::websocket::SharedState;
    use axum::Extension;

    let Extension(state): Extension<SharedState> = extract().await?;
    let Some(user) = extract::<AuthenticatedUser, _>().await.ok() else {
        return Ok(AgreeRematchResponse::Unauthorized);
    };
    let rooms = state.rooms.lock().await;
    let Some((room_id, room)) = rooms.try_get_room_pair(&user.0) else {
        return Ok(AgreeRematchResponse::NotInARoom);
    };
    let mut room_lock = room.lock().await;
    room_lock.rematch_agree.insert(user.0.clone());

    if room_lock.is_rematch_ready() {
        drop(rooms);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(250)).await;
            maybe_start_game(state, &room_id).await;
        });
    }

    Ok(AgreeRematchResponse::Success)
}
