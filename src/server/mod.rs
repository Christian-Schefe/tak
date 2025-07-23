use serde::{Deserialize, Serialize};
use tak_core::{TakGameSettings, TakPlayer};

pub mod api;

pub mod error;
#[cfg(feature = "server")]
pub mod internal;
pub mod room;
#[cfg(feature = "server")]
pub mod websocket;

pub type UserId = String;
pub type GameId = String;
pub type RoomId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInformation {
    pub user_id: UserId,
    pub username: String,
    pub rating: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSettings {
    pub game_settings: TakGameSettings,
    pub first_player_mode: Option<TakPlayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInformation {
    pub room_id: RoomId,
    pub settings: RoomSettings,
    pub players: Vec<PlayerInformation>,
    pub can_join: bool,
}

pub use error::{ServerError, ServerResult};
