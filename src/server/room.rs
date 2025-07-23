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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreateRoomResponse {
    Success(RoomId),
    AlreadyInRoom,
    FailedToGenerateId,
    InvalidSettings,
    Unauthorized,
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

#[cfg(feature = "server")]
async fn room_check_gameover_task(
    mut game_end_receiver: tokio::sync::watch::Receiver<TakGameState>,
    room: Arc<tokio::sync::Mutex<Room>>,
    sockets: Arc<PlayerSocketMap>,
) {
    use crate::server::player;

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

    let game_clone = room_lock.game.as_ref().unwrap().game.clone();
    let player_mapping = room_lock.game.as_ref().unwrap().player_mapping.clone();
    let player_ids = room_lock.players.clone();
    drop(room_lock);

    if let Err(e) = player::add_game(game_clone, player_mapping).await {
        eprintln!("Failed to add game: {:?}", e);
    }

    if let Err(e) =
        crate::server::player::add_game_result(&player_ids[0], &player_ids[1], result).await
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
    if !room_lock.players.contains(&user.0) {
        return Ok(AgreeRematchResponse::NotInARoom);
    }
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
