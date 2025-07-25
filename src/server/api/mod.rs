use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

mod room;

pub use room::*;

use crate::server::error::{ServerError, ServerResult};
use crate::server::UserId;

#[cfg(feature = "server")]
use super::internal::*;

#[cfg(feature = "server")]
pub async fn authorize() -> ServerResult<UserId> {
    let Some(auth::AuthenticatedUser(Some(user_id))) = extract().await.ok() else {
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

#[server]
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
) -> Result<ServerResult<UserId>, ServerFnError> {
    let user_id = bail_api!(auth::try_register(username, password).await);
    bail_api!(auth::add_session(&user_id).await);
    Ok(Ok(user_id))
}

#[server]
pub async fn post_login(
    username: String,
    password: String,
) -> Result<ServerResult<UserId>, ServerFnError> {
    let user_id = bail_api!(auth::try_login(username, password).await);
    bail_api!(auth::add_session(&user_id).await);
    Ok(Ok(user_id))
}

#[server]
pub async fn post_logout() -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    bail_api!(auth::remove_session(&user_id).await);
    Ok(Ok(()))
}

#[server]
pub async fn get_auth() -> Result<ServerResult<UserId>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(Ok(user_id))
}

#[server]
pub async fn get_session_id() -> Result<ServerResult<String>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let Some(session_id) = bail_api!(auth::get_session(&user_id).await) else {
        let session_id = bail_api!(auth::add_session(&user_id).await);
        return Ok(Ok(session_id));
    };
    Ok(Ok(session_id))
}
