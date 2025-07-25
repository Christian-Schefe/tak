use dioxus::prelude::*;
use tak_core::{TakGame, TakPlayer};

use crate::{
    bail_api,
    server::{
        GameId, GameInformation, PlayerInformation, RoomId, RoomInformation, RoomSettings,
        ServerError,
        api::{AuthClient, AuthServerResult},
    },
};

use crate::server::error::ServerResult;

#[cfg(feature = "server")]
use crate::server::api::authorize;
#[cfg(feature = "server")]
use crate::server::internal::*;

#[server(client=AuthClient)]
pub async fn create_room(settings: RoomSettings) -> Result<ServerResult<RoomId>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let mut rooms = room::ROOMS.write().await;
    Ok(rooms.try_create_room(settings, user_id))
}

#[server(client=AuthClient)]
pub async fn join_room(
    room_id: String,
    is_spectator: bool,
) -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let mut rooms = room::ROOMS.write().await;
    Ok(if is_spectator {
        rooms
            .try_join_room_as_spectator(room_id.clone(), user_id.clone())
            .await
    } else {
        rooms
            .try_join_room_as_player(room_id.clone(), user_id.clone())
            .await
    })
}

#[server(client=AuthClient)]
pub async fn leave_room() -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let mut rooms = room::ROOMS.write().await;
    Ok(rooms.try_leave_room(user_id).await)
}

#[server(client=AuthClient)]
pub async fn get_room() -> Result<ServerResult<(RoomId, RoomSettings)>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let rooms = room::ROOMS.read().await;
    Ok(rooms.try_get_room_id(&user_id).await)
}

#[server(client=AuthClient)]
pub async fn get_players()
-> Result<ServerResult<Vec<(PlayerInformation, TakPlayer, bool)>>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let rooms = room::ROOMS.read().await;
    Ok(rooms.try_get_players(&user_id).await)
}

#[server(client=AuthClient)]
pub async fn get_current_game() -> Result<ServerResult<Option<TakGame>>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let rooms = room::ROOMS.read().await;
    let Some(room) = rooms.try_get_room(&user_id) else {
        return Ok(Err(ServerError::NotFound));
    };
    let room_lock = room.lock().await;
    if let Some(game) = &room_lock.game {
        let game_state = game.game.clone();
        Ok(Ok(Some(game_state)))
    } else {
        Ok(Ok(None))
    }
}

#[server(client=AuthClient)]
pub async fn get_room_list() -> Result<ServerResult<Vec<RoomInformation>>, ServerFnError> {
    let _ = bail_api!(authorize().await);
    let rooms = room::ROOMS.read().await;
    Ok(rooms.get_room_list().await)
}

#[server(client=AuthClient)]
pub async fn agree_rematch() -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let rooms = room::ROOMS.read().await;
    Ok(rooms.try_agree_rematch(&user_id).await)
}

#[server(client=AuthClient)]
pub async fn get_game(game_id: GameId) -> Result<ServerResult<GameInformation>, ServerFnError> {
    let game_record = bail_api!(player::get_game(&game_id).await);
    let game = GameInformation {
        game_id: game_record.game_id,
        white_player: game_record.white_player,
        black_player: game_record.black_player,
        ptn: game_record.ptn,
        timestamp: game_record.timestamp.into(),
    };
    Ok(Ok(game))
}

#[server(client=AuthClient)]
pub async fn get_history() -> Result<AuthServerResult<Vec<GameInformation>>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let games = bail_api!(player::get_games_of_player(&user_id).await);
    let game_info: Vec<GameInformation> = games
        .into_iter()
        .map(|game_record| GameInformation {
            game_id: game_record.game_id,
            white_player: game_record.white_player,
            black_player: game_record.black_player,
            ptn: game_record.ptn,
            timestamp: game_record.timestamp.into(),
        })
        .collect();
    super::accept(game_info, user_id)
}
