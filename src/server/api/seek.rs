use dioxus::prelude::*;

use crate::{
    bail_api,
    server::{PlayerInformation, SeekSettings, UserId, api::AuthClient},
};

use crate::server::error::ServerResult;

#[cfg(feature = "server")]
use crate::server::api::authorize;
#[cfg(feature = "server")]
use crate::server::internal::*;

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
pub async fn get_seeks()
-> Result<ServerResult<Vec<(PlayerInformation, SeekSettings, bool)>>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(seek::get_seeks(&user_id).await)
}

#[server(client=AuthClient)]
pub async fn accept_seek(seek_owner: UserId) -> Result<ServerResult<()>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(seek::accept_seek(&user_id, &seek_owner).await)
}
