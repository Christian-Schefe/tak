use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum RegisterResult {
    Success(String),
    ValidationError,
    UserExists,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoginResult {
    Success(String),
    InvalidCredentials,
}

#[cfg(feature = "server")]
pub use server::*;

#[cfg(feature = "server")]
mod server {
    use crate::server::auth::{LoginResult, RegisterResult};
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
    use axum::extract::FromRequestParts;
    use axum::http::request::Parts;
    use axum::{async_trait, Extension};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::sync::{Arc, LazyLock};
    use surrealdb::engine::remote::ws::{Client, Ws};
    use surrealdb::opt::auth::Root;
    use surrealdb::Surreal;
    use tokio::sync::Mutex;
    use tower_cookies::Cookies;
    use uuid::Uuid;

    pub type SessionStore = Arc<Mutex<HashMap<String, String>>>;

    pub mod error {
        use axum::http::StatusCode;
        use axum::response::IntoResponse;
        use axum::response::Response;
        use axum::Json;
        use thiserror::Error;

        #[derive(Error, Debug)]
        pub enum Error {
            #[error(transparent)]
            Db(surrealdb::Error),
            #[error("internal server error: {0}")]
            InternalServerError(String),
            #[error("unauthorized")]
            Unauthorized,
            #[error("invalid request: {0}")]
            InvalidRequest(String),
        }

        impl IntoResponse for Error {
            fn into_response(self) -> Response {
                let status_code = match &self {
                    Self::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    Self::Unauthorized => StatusCode::UNAUTHORIZED,
                    Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
                };
                (status_code, Json(self.to_string())).into_response()
            }
        }

        impl From<surrealdb::Error> for Error {
            fn from(error: surrealdb::Error) -> Self {
                Self::Db(error)
            }
        }
    }

    pub static DB: LazyLock<Surreal<Client>> = LazyLock::new(Surreal::init);

    #[derive(Debug, Serialize, Deserialize)]
    pub struct User {
        pub user_id: String,
        pub username: String,
        password_hash: String,
    }

    async fn retry_connect_db(url: &str, max_attempts: usize) -> Result<(), error::Error> {
        let mut attempts = 0;
        loop {
            match DB.connect::<Ws>(url).await {
                Ok(_) => return Ok(()),
                Err(e) if attempts < max_attempts => {
                    attempts += 1;
                    eprintln!("Failed to connect to database, retrying... ({})", e);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => return Err(error::Error::InternalServerError(e.to_string())),
            }
        }
    }

    pub async fn connect_db(url: &str) -> Result<(), error::Error> {
        println!("Connecting to database at {}...", url);
        retry_connect_db(url, 5).await?;

        println!("Connected to database");
        DB.signin(Root {
            username: "root",
            password: "secret",
        })
        .await?;

        DB.use_ns("app").use_db("auth").await?;

        DB.query("DEFINE FIELD IF NOT EXISTS username ON user TYPE string ASSERT $value != NONE;")
            .query("DEFINE INDEX IF NOT EXISTS idx_unique_username ON user FIELDS username UNIQUE;")
            .await?;

        Ok(())
    }

    pub struct AuthenticatedUser(pub String); // user_id

    #[async_trait]
    impl<S> FromRequestParts<S> for AuthenticatedUser
    where
        S: Send + Sync,
    {
        type Rejection = error::Error;

        async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
            let Extension(store): Extension<SessionStore> =
                Extension::from_request_parts(parts, state)
                    .await
                    .map_err(|_| {
                        error::Error::InternalServerError("no session store found".to_string())
                    })?;

            let cookies: Cookies = Cookies::from_request_parts(parts, state)
                .await
                .map_err(|_| error::Error::InternalServerError("no cookies found".to_string()))?;

            if let Some(cookie) = cookies.get("session_id") {
                let session_id = cookie.value();
                if let Some(user_id) = store.lock().await.get(session_id) {
                    return Ok(AuthenticatedUser(user_id.clone()));
                }
            }

            Err(error::Error::Unauthorized)
        }
    }

    pub fn create_session_store() -> SessionStore {
        Arc::new(Mutex::new(HashMap::new()))
    }

    fn validate_username(username: &str) -> bool {
        let is_valid_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
        username.len() >= 3 && username.len() <= 128 && username.chars().all(is_valid_char)
    }

    fn validate_password(password: &str) -> bool {
        password.len() >= 8 && password.len() <= 128
    }

    pub async fn handle_register(
        username: String,
        password: String,
    ) -> Result<RegisterResult, error::Error> {
        if !validate_username(&username) {
            return Ok(RegisterResult::ValidationError);
        }
        if !validate_password(&password) {
            return Ok(RegisterResult::ValidationError);
        }

        let mut result = DB
            .query("SELECT * FROM type::table($table) WHERE username = type::string($username)")
            .bind(("table", "user"))
            .bind(("username", username.clone()))
            .await?;
        let res: Option<User> = result.take(0)?;
        if let Some(_) = res {
            return Ok(RegisterResult::UserExists);
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| error::Error::InvalidRequest("Failed to create hash".to_string()))?
            .to_string();

        let user_id = Uuid::new_v4().to_string();
        let user = User {
            user_id: user_id.clone(),
            username,
            password_hash,
        };

        let res: Option<User> = DB.create(("user", user_id)).content(user).await?;
        res.ok_or_else(|| error::Error::InternalServerError("Failed to create user".to_string()))
            .map(|x| RegisterResult::Success(x.user_id))
    }

    pub async fn handle_try_login(
        username: String,
        password: String,
    ) -> Result<LoginResult, error::Error> {
        let mut result = DB
            .query("SELECT * FROM type::table($table) WHERE username = type::string($username)")
            .bind(("table", "user"))
            .bind(("username", username))
            .await?;

        let Some(user): Option<User> = result.take(0)? else {
            return Ok(LoginResult::InvalidCredentials);
        };

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| error::Error::InvalidRequest("Failed to create hash".to_string()))?;

        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_err()
        {
            Ok(LoginResult::InvalidCredentials)
        } else {
            Ok(LoginResult::Success(user.user_id))
        }
    }

    pub async fn handle_try_get_user(user_id: &String) -> Result<Option<User>, error::Error> {
        Ok(DB.select(("user", user_id)).await?)
    }
}
