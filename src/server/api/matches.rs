use dioxus::prelude::*;

use crate::{
    bail_api,
    server::{MatchData, MatchId, MatchInstance, PlayerInformation, UserId, api::AuthClient},
};

use crate::server::error::ServerResult;

pub const MATCHES_TOPIC: &str = "matches";
pub const REMATCH_SUBTOPIC: &str = "rematch";
pub const DRAW_SUBTOPIC: &str = "draw";

#[cfg(feature = "server")]
use crate::server::api::authorize;
#[cfg(feature = "server")]
use crate::server::internal::*;

#[server(client=AuthClient)]
pub async fn get_matches() -> Result<
    ServerResult<Vec<(MatchId, PlayerInformation, PlayerInformation, MatchInstance)>>,
    ServerFnError,
> {
    let _ = bail_api!(authorize().await);
    Ok(matches::get_matches().await)
}

#[server(client=AuthClient)]
pub async fn get_match_id() -> Result<ServerResult<MatchId>, ServerFnError> {
    let user_id = bail_api!(authorize().await);
    Ok(matches::get_match_id(&user_id).await)
}

#[server(client=AuthClient)]
pub async fn get_match(match_id: String) -> Result<ServerResult<MatchInstance>, ServerFnError> {
    let _ = bail_api!(authorize().await);
    Ok(matches::get_match(&match_id).await)
}

#[server(client=AuthClient)]
pub async fn get_match_info() -> Result<
    ServerResult<(
        UserId,
        PlayerInformation,
        PlayerInformation,
        MatchInstance,
        MatchData,
    )>,
    ServerFnError,
> {
    let user_id = bail_api!(authorize().await);
    let match_id = bail_api!(matches::get_match_id(&user_id).await);
    let instance = bail_api!(matches::get_match(&match_id).await);
    let player_info = bail_api!(cache::get_or_retrieve_player_info(&instance.player_id).await);
    let opponent_info = bail_api!(cache::get_or_retrieve_player_info(&instance.opponent_id).await);
    let match_data = bail_api!(matches::get_match_data(&match_id));
    Ok(Ok((
        user_id,
        player_info,
        opponent_info,
        instance,
        match_data,
    )))
}

#[server(client=AuthClient)]
pub async fn agree_rematch() -> Result<ServerResult<()>, ServerFnError> {
    let player_id = bail_api!(authorize().await);
    Ok(matches::agree_rematch(&player_id).await)
}

#[server(client=AuthClient)]
pub async fn retract_rematch() -> Result<ServerResult<()>, ServerFnError> {
    let player_id = bail_api!(authorize().await);
    Ok(matches::retract_rematch(&player_id).await)
}

#[server(client=AuthClient)]
pub async fn leave_match() -> Result<ServerResult<()>, ServerFnError> {
    let player_id = bail_api!(authorize().await);
    bail_api!(matches::leave_match(&player_id).await);
    Ok(Ok(()))
}
