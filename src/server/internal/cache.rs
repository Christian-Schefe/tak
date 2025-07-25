use std::{sync::LazyLock, time::Duration};

use moka::future::Cache;

use crate::server::{error::ServerResult, internal::dto::UserRecord, PlayerInformation, UserId};

pub static PLAYER_INFO_CACHE: LazyLock<Cache<UserId, PlayerInformation>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(5 * 60))
        .build()
});

pub async fn get_player_info(user_id: &UserId) -> Option<PlayerInformation> {
    PLAYER_INFO_CACHE.get(user_id).await
}

pub async fn set_player_info(user_id: &UserId, info: PlayerInformation) {
    PLAYER_INFO_CACHE.insert(user_id.clone(), info).await;
}

pub async fn get_or_retrieve_player_info(user_id: &UserId) -> ServerResult<PlayerInformation> {
    if let Some(info) = get_player_info(user_id).await {
        Ok(info)
    } else {
        retrieve_player_info(user_id).await
    }
}

pub async fn retrieve_player_info(user_id: &UserId) -> ServerResult<PlayerInformation> {
    let user = super::dto::try_get::<UserRecord>(user_id).await?;
    let player = super::player::get_or_insert_player(user_id).await?;

    let info = PlayerInformation {
        user_id: user_id.clone(),
        username: user.username.clone(),
        rating: player.rating,
    };
    set_player_info(user_id, info.clone()).await;
    Ok(info)
}
