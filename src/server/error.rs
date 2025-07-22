use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum ServerError {
    #[error("Internal server error: {0}")]
    InternalServerError(String),
    #[error("Unauthorized access")]
    Unauthorized,
    #[error("Not found")]
    NotFound,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
}

pub type ServerResult<T> = Result<T, ServerError>;

#[cfg(feature = "server")]
impl From<surrealdb::Error> for ServerError {
    fn from(error: surrealdb::Error) -> Self {
        Self::InternalServerError(error.to_string())
    }
}
