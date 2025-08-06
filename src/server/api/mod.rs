use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

mod auth_client;
mod matches;
mod seek;

pub use auth_client::*;
pub use matches::*;
pub use seek::*;

use crate::server::GameId;
use crate::server::GameInformation;
use crate::server::JWTToken;
use crate::server::UserId;
use crate::server::error::{ServerError, ServerResult};

#[cfg(feature = "server")]
use super::internal::*;

#[cfg(feature = "server")]
pub async fn authorize() -> ServerResult<UserId> {
    let Some(auth::Claims { sub: user_id, .. }) = extract().await.ok() else {
        return Err(ServerError::Unauthorized);
    };
    Ok(user_id)
}

#[macro_export]
macro_rules! bail_api {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Ok(Err(e)),
        }
    };
}

#[macro_export]
macro_rules! bail_api_with_user {
    ($expr:expr,$user:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                return Ok(ApiResult {
                    data: Err(e),
                    user_id: Some($user),
                });
            }
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatsData {
    pub rating: f64,
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
}

#[server(client=AuthClient)]
pub async fn get_stats() -> Result<ServerResult<StatsData>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let player = bail_api!(player::get_or_insert_player(&user_id).await);
    Ok(Ok(StatsData {
        rating: player.rating,
        wins: player.wins,
        losses: player.losses,
        draws: player.draws,
    }))
}

#[server]
pub async fn post_register(
    username: String,
    password: String,
) -> Result<ServerResult<JWTToken>, ServerFnError> {
    let token = bail_api!(auth::try_register(username, password).await);
    Ok(Ok(token))
}

#[server]
pub async fn post_login(
    username: String,
    password: String,
) -> Result<ServerResult<JWTToken>, ServerFnError> {
    let token = bail_api!(auth::try_login(username, password).await);
    Ok(Ok(token))
}

#[server(client=AuthClient)]
pub async fn post_renew_token() -> Result<ServerResult<JWTToken>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let token = bail_api!(auth::renew_token(&user_id));
    Ok(Ok(token))
}

#[server(client=AuthClient)]
pub async fn post_change_password(
    old_password: String,
    new_password: String,
) -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    bail_api!(auth::try_change_password(&user_id, old_password, new_password).await);
    Ok(Ok(()))
}

#[server(client=AuthClient)]
pub async fn get_user_id() -> Result<ServerResult<UserId>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(Ok(user_id))
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
pub async fn get_history(
    pagination: Option<(usize, usize)>,
) -> Result<ServerResult<(UserId, Vec<GameInformation>)>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let games = bail_api!(player::get_games_of_player(&user_id, pagination).await);
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
    Ok(Ok((user_id, game_info)))
}

#[server(client=AuthClient)]
pub async fn post_pubsub_subscribe(topic: String) -> Result<ServerResult<String>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(ws_pubsub::client_subscribe(&topic, &user_id)
        .await
        .ok_or(ServerError::NotFound))
}

#[server(client=AuthClient)]
pub async fn post_pubsub_unsubscribe(
    subscription_id: String,
) -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(Ok(ws_pubsub::client_unsubscribe(
        &user_id,
        &subscription_id,
    )
    .await))
}

pub struct MyServerFunctions;

#[async_trait::async_trait]
impl ws_pubsub::ServerFunctions for MyServerFunctions {
    type Error = Result<ServerError, ServerFnError>;
    async fn subscribe(topic: String) -> Result<String, Self::Error> {
        match post_pubsub_subscribe(topic).await {
            Ok(Ok(subscription_id)) => Ok(subscription_id),
            Ok(Err(e)) => Err(Ok(e)),
            Err(e) => Err(Err(e)),
        }
    }
    async fn unsubscribe(subscription_id: String) -> Result<(), Self::Error> {
        match post_pubsub_unsubscribe(subscription_id).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(Ok(e)),
            Err(e) => Err(Err(e)),
        }
    }
}
