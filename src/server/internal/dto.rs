use serde::{de::DeserializeOwned, Deserialize, Serialize};
use surrealdb::RecordIdKey;

use crate::server::{
    error::{ServerError, ServerResult},
    internal::db::DB,
    GameId, PlayerInformation, UserId,
};

pub trait Record {
    type K;
    fn table_name() -> &'static str;
    fn record_id_key(key: &Self::K) -> RecordIdKey;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub user_id: UserId,
    pub username: String,
    pub password_hash: String,
}

impl Record for UserRecord {
    type K = UserId;
    fn table_name() -> &'static str {
        "user"
    }
    fn record_id_key(key: &Self::K) -> RecordIdKey {
        RecordIdKey::from(key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRecord {
    pub user_id: UserId,
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
    pub rating: f64,
}

impl Record for PlayerRecord {
    type K = UserId;
    fn table_name() -> &'static str {
        "player"
    }
    fn record_id_key(key: &Self::K) -> RecordIdKey {
        RecordIdKey::from(key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub game_id: GameId,
    pub white_player: PlayerInformation,
    pub black_player: PlayerInformation,
    pub ptn: String,
    pub timestamp: surrealdb::sql::Datetime,
}

impl Record for GameRecord {
    type K = GameId;
    fn table_name() -> &'static str {
        "game"
    }
    fn record_id_key(key: &Self::K) -> RecordIdKey {
        RecordIdKey::from(key)
    }
}

pub async fn setup_db() -> ServerResult<()> {
    DB.query("DEFINE FIELD IF NOT EXISTS username ON user TYPE string ASSERT $value != NONE;")
        .query("DEFINE INDEX IF NOT EXISTS idx_unique_username ON user FIELDS username UNIQUE;")
        .await?;
    Ok(())
}

pub async fn try_get<T: Record + DeserializeOwned>(key: &T::K) -> ServerResult<T> {
    DB.select((T::table_name(), T::record_id_key(key)))
        .await?
        .ok_or(ServerError::NotFound)
}

pub async fn try_get_or_insert<T: Record + DeserializeOwned + Serialize + 'static>(
    key: &T::K,
    default_value: impl FnOnce() -> T,
) -> ServerResult<T> {
    match try_get(key).await {
        Ok(record) => Ok(record),
        Err(ServerError::NotFound) => try_create(key, default_value()).await,
        Err(e) => Err(e),
    }
}

pub async fn try_create<T: Record + DeserializeOwned + Serialize + 'static>(
    key: &T::K,
    value: T,
) -> ServerResult<T> {
    DB.create((T::table_name(), T::record_id_key(key)))
        .content(value)
        .await?
        .ok_or(ServerError::NotFound)
}

pub async fn try_update<T: Record + DeserializeOwned + Serialize + 'static>(
    key: &T::K,
    value: T,
) -> ServerResult<T> {
    DB.update((T::table_name(), T::record_id_key(key)))
        .content(value)
        .await?
        .ok_or(ServerError::NotFound)
}
