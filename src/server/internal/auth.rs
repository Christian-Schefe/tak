use crate::server::error::{ServerError, ServerResult};
use crate::server::internal::db::DB;
use crate::server::internal::dto::{Record, UserRecord};
use crate::server::UserId;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{async_trait, Extension};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

pub type SessionStore = Arc<Mutex<HashMap<String, String>>>;

pub struct AuthenticatedUser(pub Option<UserId>);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(Extension(store)): Option<Extension<SessionStore>> =
            Extension::from_request_parts(parts, state).await.ok()
        else {
            return Ok(AuthenticatedUser(None));
        };

        let Some(cookies): Option<Cookies> = Cookies::from_request_parts(parts, state).await.ok()
        else {
            return Ok(AuthenticatedUser(None));
        };

        if let Some(cookie) = cookies.get("session_id") {
            let session_id = cookie.value();
            if let Some(user_id) = store.lock().await.get(session_id) {
                return Ok(AuthenticatedUser(Some(user_id.clone())));
            }
        }

        Ok(AuthenticatedUser(None))
    }
}

pub async fn get_session(user_id: &UserId) -> ServerResult<Option<String>> {
    use dioxus::prelude::extract;

    let Extension(session_store): Extension<SessionStore> = extract().await.map_err(|e| {
        ServerError::InternalServerError(format!("Failed to extract session store: {}", e))
    })?;

    let lock = session_store.lock().await;
    Ok(lock.get(user_id).cloned())
}

pub async fn add_session(user_id: &UserId) -> ServerResult<String> {
    use dioxus::prelude::extract;

    let Extension(session_store): Extension<SessionStore> = extract().await.map_err(|e| {
        ServerError::InternalServerError(format!("Failed to extract session store: {}", e))
    })?;
    let cookies: tower_cookies::Cookies = extract().await.map_err(|(_, e)| {
        ServerError::InternalServerError(format!("Failed to extract cookies: {}", e))
    })?;

    let session_id = Uuid::new_v4().to_string();
    session_store
        .lock()
        .await
        .insert(session_id.clone(), user_id.to_string());
    let mut cookie = Cookie::new("session_id", session_id.clone());
    cookie.set_http_only(Some(true));
    cookies.add(cookie);
    Ok(session_id)
}

pub async fn remove_session(user_id: &UserId) -> ServerResult<()> {
    use dioxus::prelude::extract;

    let Extension(session_store): Extension<SessionStore> = extract().await.map_err(|e| {
        ServerError::InternalServerError(format!("Failed to extract session store: {}", e))
    })?;

    session_store.lock().await.remove(user_id);
    Ok(())
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

pub async fn try_register(username: String, password: String) -> ServerResult<UserId> {
    if !validate_username(&username) {
        return Err(ServerError::BadRequest("Invalid username".to_string()));
    }
    if !validate_password(&password) {
        return Err(ServerError::BadRequest("Invalid password".to_string()));
    }

    let mut result = DB
        .query("SELECT * FROM type::table($table) WHERE username = type::string($username)")
        .bind(("table", "user"))
        .bind(("username", username.clone()))
        .await?;
    let res: Option<UserRecord> = result.take(0)?;
    if let Some(_) = res {
        return Err(ServerError::Conflict("Username already exists".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| ServerError::BadRequest("Failed to create hash".to_string()))?
        .to_string();

    let user_id = Uuid::new_v4().to_string();
    let user = UserRecord {
        user_id: user_id.clone(),
        username,
        password_hash,
    };

    let res = super::dto::try_create(&user_id, user).await?;
    Ok(res.user_id)
}

pub async fn try_login(username: String, password: String) -> ServerResult<UserId> {
    let mut result = DB
        .query("SELECT * FROM type::table($table) WHERE username = type::string($username)")
        .bind(("table", UserRecord::table_name()))
        .bind(("username", username))
        .await?;

    let Some(user): Option<UserRecord> = result.take(0)? else {
        return Err(ServerError::Unauthorized);
    };

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| ServerError::BadRequest("Failed to create hash".to_string()))?;

    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        Err(ServerError::Unauthorized)
    } else {
        Ok(user.user_id)
    }
}
