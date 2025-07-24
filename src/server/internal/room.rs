use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
    time::Duration,
};

use futures_util::SinkExt;
use rand::Rng;
use tak_core::{TakGame, TakGameState, TakPlayer};

use crate::{
    components::ServerGameMessage,
    server::{
        error::{ServerError, ServerResult},
        internal::websocket::{PlayerConnection, PlayerSocket},
        PlayerInformation, RoomId, RoomInformation, RoomSettings, UserId, ROOM_ID_LEN,
    },
};

pub struct Room {
    pub settings: RoomSettings,
    pub game: Option<RoomGame>,
    pub players: Vec<UserId>,
    pub spectators: Vec<UserId>,
    pub rematch_agree: HashSet<UserId>,
}

pub struct RoomGame {
    pub game: TakGame,
    pub game_end_sender: tokio::sync::watch::Sender<TakGameState>,
    pub player_mapping: fixed_map::Map<TakPlayer, UserId>,
}

pub struct Rooms {
    rooms: HashMap<RoomId, Arc<tokio::sync::Mutex<Room>>>,
    player_mapping: HashMap<UserId, RoomId>,
    pub player_sockets: ArcMutexDashMap<UserId, PlayerSocket>,
}

pub type ArcMutexDashMap<K, V> = Arc<dashmap::DashMap<K, Arc<tokio::sync::Mutex<V>>>>;

pub static ROOMS: LazyLock<tokio::sync::RwLock<Rooms>> =
    LazyLock::new(|| tokio::sync::RwLock::new(Rooms::new()));

impl Room {
    fn new(owner: UserId, settings: RoomSettings) -> Self {
        Self {
            settings,
            players: vec![owner],
            spectators: Vec::new(),
            game: None,
            rematch_agree: HashSet::new(),
        }
    }

    fn remove_player(&mut self, player_id: &UserId) -> Option<bool> {
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

    fn abort_game(&mut self, player_id: &UserId) {
        if let Some(game) = &mut self.game {
            if !game.game.check_timeout() {
                let (winner, _) = game
                    .player_mapping
                    .iter()
                    .find(|(_, p)| **p != *player_id)
                    .expect("Game should have a winner");
                game.game.abort(winner);
            }
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

    pub fn is_rematch_ready(&self) -> bool {
        self.game
            .as_ref()
            .is_some_and(|game| game.game.game_state != TakGameState::Ongoing)
            && self.players.len() == TakPlayer::ALL.len()
            && self.players.iter().all(|p| self.rematch_agree.contains(p))
    }

    fn is_empty(&self) -> bool {
        self.players.is_empty() && self.spectators.is_empty()
    }

    pub fn get_broadcast_player_ids(&self) -> Vec<UserId> {
        self.players
            .iter()
            .cloned()
            .chain(self.spectators.iter().cloned())
            .collect()
    }
}

impl Rooms {
    pub fn new() -> Self {
        Rooms {
            rooms: HashMap::new(),
            player_mapping: HashMap::new(),
            player_sockets: Arc::new(dashmap::DashMap::new()),
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

    pub fn try_create_room(
        &mut self,
        settings: RoomSettings,
        owner: UserId,
    ) -> ServerResult<RoomId> {
        if self.player_mapping.contains_key(&owner) {
            return Err(ServerError::Conflict(
                "User is already in a room".to_string(),
            ));
        }
        let Some(id) = self.try_generate_room_id() else {
            return Err(ServerError::InternalServerError(
                "Failed to generate room ID".to_string(),
            ));
        };
        if TakGame::new(settings.game_settings.clone()).is_none() {
            return Err(ServerError::BadRequest("Invalid game settings".to_string()));
        }
        self.rooms.insert(
            id.clone(),
            Arc::new(tokio::sync::Mutex::new(Room::new(owner.clone(), settings))),
        );
        self.player_mapping.insert(owner, id.clone());
        Ok(id)
    }

    pub async fn try_join_room_as_player(
        &mut self,
        room_id: RoomId,
        player_id: UserId,
    ) -> ServerResult<()> {
        if self.player_mapping.contains_key(&player_id) {
            return Err(ServerError::Conflict(
                "User is already in a room".to_string(),
            ));
        }
        let Some(room) = self.rooms.get(&room_id) else {
            return Err(ServerError::NotFound);
        };
        let mut room_lock = room.lock().await;
        if !room_lock.can_join() {
            return Err(ServerError::Conflict("Room is full".to_string()));
        }
        room_lock.players.push(player_id.clone());
        self.player_mapping.insert(player_id, room_id.clone());

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            maybe_start_game(&room_id).await;
        });

        Ok(())
    }

    pub async fn try_join_room_as_spectator(
        &mut self,
        room_id: RoomId,
        player_id: UserId,
    ) -> ServerResult<()> {
        if self.player_mapping.contains_key(&player_id) {
            return Err(ServerError::Conflict(
                "User is already in a room".to_string(),
            ));
        }
        let Some(room) = self.rooms.get(&room_id) else {
            return Err(ServerError::NotFound);
        };
        let mut room_lock = room.lock().await;
        room_lock.spectators.push(player_id.clone());
        self.player_mapping.insert(player_id, room_id);
        Ok(())
    }

    pub async fn try_leave_room(&mut self, player_id: UserId) -> ServerResult<()> {
        let Some(room_id) = self.player_mapping.remove(&player_id) else {
            return Err(ServerError::NotFound);
        };
        let room = self.rooms.get(&room_id).unwrap().clone();

        let mut room_lock = room.lock().await;
        let Some(was_player) = room_lock.remove_player(&player_id) else {
            return Err(ServerError::NotFound);
        };

        if was_player {
            room_lock.abort_game(&player_id);
        }

        if room_lock.is_empty() {
            drop(room_lock);
            self.rooms.remove(&room_id);
            println!("Room {} was empty and removed", room_id);
        }

        self.terminate_socket(&player_id).await;
        Ok(())
    }

    pub async fn try_get_room_id(
        &self,
        player_id: &UserId,
    ) -> ServerResult<(RoomId, RoomSettings)> {
        if let Some(room_id) = self.player_mapping.get(player_id) {
            let room = self.rooms.get(room_id).unwrap();
            let room_lock = room.lock().await;
            let settings = room_lock.settings.clone();
            drop(room_lock);
            Ok((room_id.clone(), settings))
        } else {
            Err(ServerError::NotFound)
        }
    }

    pub async fn try_agree_rematch(&self, user_id: &UserId) -> ServerResult<()> {
        let Some((room_id, room)) = self.try_get_room_pair(&user_id) else {
            return Err(ServerError::NotFound);
        };
        let mut room_lock = room.lock().await;
        if !room_lock.players.contains(&user_id) {
            return Err(ServerError::NotFound);
        }
        room_lock.rematch_agree.insert(user_id.clone());

        if room_lock.is_rematch_ready() {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(250)).await;
                maybe_start_game(&room_id).await;
            });
        }
        Ok(())
    }

    pub fn try_get_room_pair(
        &self,
        player_id: &UserId,
    ) -> Option<(RoomId, Arc<tokio::sync::Mutex<Room>>)> {
        self.player_mapping
            .get(player_id)
            .map(|room_id| (room_id.clone(), self.rooms.get(room_id).unwrap().clone()))
    }

    pub fn try_get_room(&self, player_id: &UserId) -> Option<Arc<tokio::sync::Mutex<Room>>> {
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
        func(&mut lock)
    }

    pub async fn try_get_players(
        &self,
        player_id: &UserId,
    ) -> ServerResult<Vec<(PlayerInformation, TakPlayer, bool)>> {
        let Some(room_id) = self.player_mapping.get(player_id) else {
            return Err(ServerError::NotFound);
        };
        let Some(room) = self.rooms.get(room_id) else {
            return Err(ServerError::NotFound);
        };
        let room_lock = room.lock().await;
        let mut player_info = Vec::with_capacity(room_lock.players.len());
        for (player, id) in room_lock.game.as_ref().map_or_else(
            || Vec::new(),
            |game| game.player_mapping.iter().collect::<Vec<_>>(),
        ) {
            let player_information = super::cache::get_or_retrieve_player_info(id).await?;
            player_info.push((player_information, player, id == player_id));
        }
        Ok(player_info)
    }

    pub async fn get_room_list(&self) -> ServerResult<Vec<RoomInformation>> {
        let mut room_list = Vec::with_capacity(self.rooms.len());
        for (room_id, room) in &self.rooms {
            let room_lock = room.lock().await;
            let mut players = Vec::new();
            for player_id in &room_lock.players {
                let player_information =
                    super::cache::get_or_retrieve_player_info(player_id).await?;
                players.push(player_information);
            }
            room_list.push(RoomInformation {
                room_id: room_id.clone(),
                settings: room_lock.settings.clone(),
                players,
                can_join: room_lock.can_join(),
            });
        }
        Ok(room_list)
    }

    pub async fn add_connection(&self, player_id: &UserId, connection: PlayerConnection) -> usize {
        let socket = self
            .player_sockets
            .entry(player_id.to_string())
            .or_insert_with(|| {
                Arc::new(tokio::sync::Mutex::new(PlayerSocket {
                    connections: Vec::new(),
                }))
            });

        let mut lock = socket.lock().await;
        let id = lock.connections.len();
        lock.connections.push(Some(connection));
        id
    }

    pub async fn add_handle_to_connection(
        &self,
        player_id: &UserId,
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

    pub async fn terminate_socket(&self, player_id: &UserId) {
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

    pub async fn remove_connection(&self, player_id: &UserId, id: usize) {
        if let Some(socket) = self.player_sockets.get_mut(player_id) {
            let mut lock = socket.lock().await;
            lock.connections[id] = None;
            if let Some(last_some_index) = lock.connections.iter().rposition(|x| x.is_some()) {
                lock.connections.truncate(last_some_index + 1);
            } else {
                lock.connections.clear();
            }
        };
    }

    pub async fn get_broadcast_player_ids(&self, room_id: &RoomId) -> Vec<UserId> {
        let room = self.rooms.get(room_id).unwrap();
        let room_lock = room.lock().await;
        room_lock.get_broadcast_player_ids()
    }
}

async fn maybe_start_game(room_id: &RoomId) {
    let rooms = ROOMS.read().await;
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

async fn room_check_timeout_task(room: Arc<tokio::sync::Mutex<Room>>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        let mut room_lock = room.lock().await;
        if let Some(game) = room_lock.game.as_mut() {
            game.game.check_timeout();
            if game.game.game_state != TakGameState::Ongoing {
                room_lock.check_end_game();
                break;
            };
        } else {
            println!("No game in room, stopping timeout check");
            break;
        }
    }
}

async fn room_check_gameover_task(
    mut game_end_receiver: tokio::sync::watch::Receiver<TakGameState>,
    room: Arc<tokio::sync::Mutex<Room>>,
    sockets: ArcMutexDashMap<UserId, PlayerSocket>,
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
    if room_lock.game.is_none() {
        println!("No game in room, stopping gameover check");
        return;
    }

    let game_clone = room_lock.game.as_ref().unwrap().game.clone();
    let player_mapping = room_lock.game.as_ref().unwrap().player_mapping.clone();

    println!("Game over: {:?}, {:?}", game_state, game_clone.game_state);
    drop(room_lock);

    if let Err(e) = super::player::add_game(game_clone, player_mapping).await {
        eprintln!("Failed to add game: {:?}", e);
    }
}
