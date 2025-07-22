use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::server::error::ServerResult;
use crate::server::{error::ServerError, UserId};

#[cfg(feature = "server")]
use super::internal::*;

#[cfg(feature = "server")]
async fn authorize() -> ServerResult<UserId> {
    let Some(auth::AuthenticatedUser(Some(user_id))) = extract().await.ok() else {
        return Err(ServerError::Unauthorized);
    };
    Ok(user_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiResponse<T> {
    Success(T),
    Error(ServerError),
}

macro_rules! bail_api {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Ok(ApiResponse::Error(e)),
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

#[server]
pub async fn get_stats() -> Result<ApiResponse<StatsData>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let player = bail_api!(player::get_or_insert_player(&user_id).await);
    Ok(ApiResponse::Success(StatsData {
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
) -> Result<ApiResponse<UserId>, ServerFnError> {
    let user_id = bail_api!(auth::try_register(username, password).await);
    bail_api!(auth::add_session(&user_id).await);
    Ok(ApiResponse::Success(user_id))
}

#[server]
pub async fn post_login(
    username: String,
    password: String,
) -> Result<ApiResponse<UserId>, ServerFnError> {
    let user_id = bail_api!(auth::try_login(username, password).await);
    bail_api!(auth::add_session(&user_id).await);
    Ok(ApiResponse::Success(user_id))
}

#[server]
pub async fn post_logout() -> Result<ApiResponse<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    bail_api!(auth::remove_session(&user_id).await);
    Ok(ApiResponse::Success(()))
}

#[server]
pub async fn get_auth() -> Result<ApiResponse<UserId>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(ApiResponse::Success(user_id))
}

#[server]
pub async fn get_session_id() -> Result<ApiResponse<String>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    let Some(session_id) = bail_api!(auth::get_session(&user_id).await) else {
        let session_id = bail_api!(auth::add_session(&user_id).await);
        return Ok(ApiResponse::Success(session_id));
    };
    Ok(ApiResponse::Success(session_id))
}
