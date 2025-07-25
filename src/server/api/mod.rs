use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

mod auth_client;
mod room;

pub use auth_client::*;
pub use room::*;

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

pub fn accept<T>(result: T, user_id: UserId) -> Result<AuthServerResult<T>, ServerFnError> {
    Ok(Ok((result, user_id)))
}

pub type AuthServerResult<T> = ServerResult<(T, UserId)>;

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
pub async fn get_auth() -> Result<ServerResult<JWTToken>, ServerFnError> {
    let token = bail_api!(authorize().await);
    Ok(Ok(token))
}
