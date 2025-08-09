use serde::{Deserialize, Serialize};
use tak_core::{TakGame, TakGameSettings, TakPlayer};

pub mod api;

pub mod error;
#[cfg(feature = "server")]
pub mod internal;

pub type UserId = String;
pub type GameId = String;
pub type RoomId = String;
pub type MatchId = String;

pub type JWTToken = String;

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
        player_info: PlayerInformation,
        settings: SeekSettings,
    },
    Removed {
        player_id: UserId,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MatchUpdate {
    Created {
        player_info: PlayerInformation,
        opponent_info: PlayerInformation,
        match_id: MatchId,
        settings: MatchInstance,
    },
    Removed {
        match_id: MatchId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RematchColor {
    Keep,
    Alternate,
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchInstance {
    pub player_id: UserId,
    pub opponent_id: UserId,
    pub game_settings: TakGameSettings,
    pub rated: bool,
    pub creator_color: TakPlayer,
    pub rematch_color: RematchColor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MatchData {
    pub game: TakGame,
    pub player_mapping: fixed_map::Map<TakPlayer, UserId>,
    pub rematch_agree: Vec<UserId>,
    pub draw_agree: Vec<UserId>,
    pub has_ended: bool,
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

pub const NOTIFICATION_TOPIC: &str = "notifications";
pub const SEEK_ACCEPTED_SUBTOPIC: &str = "seek_accepted";
