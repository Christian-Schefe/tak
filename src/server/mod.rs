use serde::{Deserialize, Serialize};
use tak_core::{TakGameSettings, TakPlayer};

pub mod api;

pub mod error;
#[cfg(feature = "server")]
pub mod internal;

pub type UserId = String;
pub type GameId = String;
pub type RoomId = String;

pub type JWTToken = String;

pub const ROOM_ID_LEN: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInformation {
    pub user_id: UserId,
    pub username: String,
    pub rating: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeekSettings {
    pub game_settings: TakGameSettings,
    pub rated: bool,
    pub creator_color: Option<TakPlayer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SeekUpdate {
    Created {
        player_id: UserId,
        settings: SeekSettings,
    },
    Removed {
        player_id: UserId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomSettings {
    pub game_settings: TakGameSettings,
    pub first_player_mode: Option<TakPlayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomInformation {
    pub room_id: RoomId,
    pub settings: RoomSettings,
    pub players: Vec<PlayerInformation>,
    pub can_join: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameInformation {
    pub game_id: GameId,
    pub white_player: PlayerInformation,
    pub black_player: PlayerInformation,
    pub ptn: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub use error::{ServerError, ServerResult};
