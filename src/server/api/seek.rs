use dioxus::prelude::*;

use crate::{
    bail_api,
    server::{MatchId, PlayerInformation, SeekSettings, UserId, api::AuthClient},
};

use crate::server::error::ServerResult;

#[cfg(feature = "server")]
use crate::server::api::authorize;
#[cfg(feature = "server")]
use crate::server::internal::*;

pub static SEEK_TOPIC: &str = "seeks";

#[server(client=AuthClient)]
pub async fn create_seek(settings: SeekSettings) -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(seek::create_seek(&user_id, settings).await)
}

#[server(client=AuthClient)]
pub async fn cancel_seek() -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(seek::cancel_seek(&user_id).await)
}

#[server(client=AuthClient)]
pub async fn get_seek() -> Result<ServerResult<SeekSettings>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(seek::get_seek(&user_id).await)
}

#[server(client=AuthClient)]
pub async fn get_seeks()
-> Result<ServerResult<Vec<(PlayerInformation, SeekSettings)>>, ServerFnError> {
    let _ = bail_api!(authorize().await);
    Ok(seek::get_seeks().await)
}

#[server(client=AuthClient)]
pub async fn accept_seek(seek_owner: UserId) -> Result<ServerResult<MatchId>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(seek::accept_seek(&user_id, &seek_owner).await)
}
