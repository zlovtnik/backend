use std::{env, fs};

use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use log::debug;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::models::user::LoginInfoDTO;

/// Lazily loads the JWT secret from `JWT_SECRET` env var or `src/secret.key` fallback.
pub static SECRET_KEY: Lazy<Vec<u8>> = Lazy::new(|| {
    // dotenv is idempotent; allows local development without explicit env loading elsewhere.
    let _ = dotenv::dotenv();

    if let Ok(secret) = env::var("JWT_SECRET") {
        return secret.into_bytes();
    }

    fs::read("src/secret.key")
        .or_else(|_| fs::read("secret.key"))
        .expect("JWT secret not configured. Provide JWT_SECRET or src/secret.key")
});
static ONE_WEEK: i64 = 60 * 60 * 24 * 7; // in seconds

#[derive(Serialize, Deserialize)]
pub struct UserToken {
    // issued at
    pub iat: i64,
    // expiration
    pub exp: i64,
    // data
    pub user: String,
    pub login_session: String,
    pub tenant_id: String,
}

impl UserToken {
    /// Generates a signed JWT containing issued-at, expiration, and the provided login information.
    ///
    /// Token lifetime is taken from the `MAX_AGE` environment variable (seconds); if `MAX_AGE` is missing or cannot be parsed as an integer, `ONE_WEEK` is used.
    ///
    /// # Returns
    ///
    /// The encoded JWT as a `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::models::user::LoginInfoDTO;
    ///
    /// let login = LoginInfoDTO {
    ///     username: "alice".into(),
    ///     login_session: "session-123".into(),
    ///     tenant_id: "tenant-a".into(),
    /// };
    ///
    /// let token = generate_token(&login);
    /// assert!(!token.is_empty());
    /// ```
    pub fn generate_token(login: &LoginInfoDTO) -> String {
        let _ = dotenv::dotenv();
        let max_age: i64 = match env::var("MAX_AGE") {
            Ok(val) => val.parse::<i64>().unwrap_or(ONE_WEEK),
            Err(_) => ONE_WEEK,
        };

        debug!("Token Max Age: {}", max_age);

        let now = Utc::now().timestamp(); // in seconds
        let payload = UserToken {
            iat: now,
            exp: now + max_age,
            user: login.username.clone(),
            login_session: login.login_session.clone(),
            tenant_id: login.tenant_id.clone(),
        };

        jsonwebtoken::encode(
            &Header::default(),
            &payload,
            &EncodingKey::from_secret(SECRET_KEY.as_slice()),
        )
        .unwrap()
    }
}
