use actix_web::http::header::HeaderValue;
use jsonwebtoken::{DecodingKey, TokenData, Validation};

use crate::{
    config::db::Pool,
    models::{
        user::operations as user_ops,
        user_token::{UserToken, SECRET_KEY},
    },
};

/// Decode a JWT string into `TokenData<UserToken>`.
///
/// The token is validated using the crate-level secret `KEY` and `jsonwebtoken`'s default validation settings.
/// Any decoding or validation error from `jsonwebtoken` is propagated to the caller.
///
/// # Examples
///
/// ```
/// let res = decode_token("invalid-token".to_string());
/// assert!(res.is_err());
/// ```
pub fn decode_token(token: String) -> jsonwebtoken::errors::Result<TokenData<UserToken>> {
    jsonwebtoken::decode::<UserToken>(
        &token,
        &DecodingKey::from_secret(SECRET_KEY.as_slice()),
        &Validation::default(),
    )
}

/// Verify that the JWT claims represent a valid login session and return the associated user identifier.
///
/// # Returns
/// `Ok(String)` containing the user identifier when the session is valid, `Err(String)` with `"Invalid token"` otherwise.
///
/// # Examples
///
/// ```
/// // Given a decoded `token_data: jsonwebtoken::TokenData<UserToken>` and a `pool: Pool`
/// // let user_id = verify_token(&token_data, &pool)?;
/// ```
pub fn verify_token(token_data: &TokenData<UserToken>, pool: &Pool) -> Result<String, String> {
    let mut conn = pool
        .get()
        .map_err(|e| format!("Failed to get db connection: {}", e))?;

    if user_ops::is_valid_login_session(&token_data.claims, &mut conn) {
        Ok(token_data.claims.user.to_string())
    } else {
        Err("Invalid token".to_string())
    }
}

/// Checks whether an HTTP Authorization header contains a Bearer token prefix.
///
/// Returns `true` if the header value starts with "bearer" or "Bearer", `false` otherwise.
///
/// # Examples
///
/// ```
/// use actix_web::http::header::HeaderValue;
/// let h = HeaderValue::from_static("Bearer abc.def.ghi");
/// assert!(is_auth_header_valid(&h));
///
/// let h2 = HeaderValue::from_static("Basic dXNlcjpwYXNz");
/// assert!(!is_auth_header_valid(&h2));
/// ```
pub fn is_auth_header_valid(authen_header: &HeaderValue) -> bool {
    if let Ok(authen_str) = authen_header.to_str() {
        return authen_str.starts_with("bearer") || authen_str.starts_with("Bearer");
    }

    false
}
