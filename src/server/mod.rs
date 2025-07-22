pub mod api;

pub mod error;
#[cfg(feature = "server")]
pub mod internal;
pub mod room;
#[cfg(feature = "server")]
pub mod websocket;

pub type UserId = String;
pub type GameId = String;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerInformation {
    pub user_id: UserId,
    pub username: String,
    pub rating: f64,
}
