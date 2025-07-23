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

macro_rules! bail_api {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Ok(Err(e)),
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
