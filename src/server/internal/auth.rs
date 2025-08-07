use crate::server::error::{ServerError, ServerResult};
use crate::server::internal::db::DB;
use crate::server::internal::dto::{Record, UserPasswordMerge, UserRecord};
use crate::server::{JWTToken, UserId};
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::RequestPartsExt;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::TypedHeader;
use headers::Authorization;
use headers::authorization::Bearer;
use jsonwebtoken::{DecodingKey, EncodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use uuid::Uuid;

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let secret = read_or_generate_secret();
    Keys::new(&secret)
});

fn read_or_generate_secret() -> Vec<u8> {
    if let Ok(secret) = std::env::var("JWT_SECRET") {
        secret.as_bytes().to_vec()
    } else {
        println!("JWT secret not found, generating a random one...");
        Uuid::new_v4().as_bytes().to_vec()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: UserId,
    exp: usize,
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = ();

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> {
        async move {
            let TypedHeader(Authorization(bearer)) = parts
                .extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .map_err(|_| ())?;

            let token_data =
                decode::<Claims>(bearer.token(), &KEYS.decoding, &Validation::default())
                    .map_err(|_| ())?;

            Ok(token_data.claims)
        }
    }
}

fn validate_username(username: &str) -> bool {
    let is_valid_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    username.len() >= 3 && username.len() <= 128 && username.chars().all(is_valid_char)
}

fn validate_password(password: &str) -> bool {
    password.len() >= 8 && password.len() <= 128
}

fn create_token(user_id: &UserId, expires_in_hours: usize) -> ServerResult<JWTToken> {
    let claims = Claims {
        sub: user_id.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(expires_in_hours as i64)).timestamp()
            as usize,
    };
    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &KEYS.encoding)
        .map_err(|_| ServerError::InternalServerError("Failed to create token".to_string()))
}

pub fn renew_token(user_id: &UserId) -> ServerResult<JWTToken> {
    create_token(user_id, 24)
}

pub fn validate_token(token: &JWTToken) -> ServerResult<Claims> {
    decode::<Claims>(token, &KEYS.decoding, &Validation::default())
        .map(|data| data.claims)
        .map_err(|_| ServerError::Unauthorized)
}

pub async fn try_register(username: String, password: String) -> ServerResult<JWTToken> {
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

    super::dto::try_create(&user_id, user).await?;
    let token = create_token(&user_id, 24)?;
    Ok(token)
}

pub async fn try_login(username: String, password: String) -> ServerResult<JWTToken> {
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
        let token = create_token(&user.user_id, 24)?;
        Ok(token)
    }
}

pub async fn try_change_password(
    user_id: &UserId,
    old_password: String,
    new_password: String,
) -> ServerResult<()> {
    if !validate_password(&new_password) {
        return Err(ServerError::BadRequest("Invalid new password".to_string()));
    }

    let user = super::dto::try_get::<UserRecord>(user_id).await?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| ServerError::BadRequest("Failed to create hash".to_string()))?;

    if Argon2::default()
        .verify_password(old_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(ServerError::Unauthorized);
    }

    let salt = SaltString::generate(&mut OsRng);
    let new_password_hash = Argon2::default()
        .hash_password(new_password.as_bytes(), &salt)
        .map_err(|_| ServerError::BadRequest("Failed to create hash".to_string()))?
        .to_string();

    let merge = UserPasswordMerge {
        password_hash: new_password_hash,
    };
    super::dto::try_merge(user_id, merge).await?;

    Ok(())
}
