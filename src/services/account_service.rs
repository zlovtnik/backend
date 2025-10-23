//! Account Service - User Authentication and Management
//!
//! Provides comprehensive user account operations with advanced functional programming patterns.
//! All operations use iterator-based validation, functional composition, and pure function patterns
//! for enhanced testability, maintainability, and performance.
//!
//! ## Functional Programming Features
//!
//! - **Iterator-based validation**: All input validation uses composable validation chains
//! - **Monadic error handling**: Comprehensive Result/Option chaining for error propagation
//! - **Pure functional composition**: Business logic composed from pure, testable functions
//! - **Immutable data flow**: Token and session operations preserve immutability
//! - **Lazy evaluation**: Database queries defer execution until results are needed

use actix_web::http::header::HeaderValue;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    config::db::Pool,
    constants,
    error::ServiceError,
    models::user::operations as user_ops,
    models::{
        refresh_token::RefreshToken,
        user::{LoginDTO, LoginInfoDTO, UserDTO, UserResponseDTO, UserUpdateDTO},
        user_token::UserToken,
    },
    services::functional_patterns::{Either, Validator, validation_rules},
    services::functional_service_base::{FunctionalErrorHandling, FunctionalQueryService},
    utils::token_utils,
};
use diesel::result::{DatabaseErrorKind, Error as DieselError};

// Email validation regex - pragmatic pattern for production use
static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").expect("Invalid email regex"));

/// Iterator-based validation using functional combinator pattern for UserDTO
fn create_user_validator() -> Validator<UserDTO> {
    Validator::new()
        .rule(|dto: &UserDTO| validation_rules::required("username")(&dto.username))
        .rule(|dto: &UserDTO| validation_rules::min_length("username", 3)(&dto.username))
        .rule(|dto: &UserDTO| validation_rules::max_length("username", 50)(&dto.username))
        .rule(|dto: &UserDTO| {
            let char_count = dto.password.chars().count();
            if char_count < 8 {
                Err(ServiceError::bad_request(
                    "Password too short (min 8 characters)",
                ))
            } else if char_count > 64 {
                Err(ServiceError::bad_request(
                    "Password too long (max 64 characters)",
                ))
            } else if !dto.password.chars().any(|c| c.is_uppercase()) {
                Err(ServiceError::bad_request(
                    "Password must contain at least one uppercase letter",
                ))
            } else if !dto.password.chars().any(|c| c.is_lowercase()) {
                Err(ServiceError::bad_request(
                    "Password must contain at least one lowercase letter",
                ))
            } else if !dto.password.chars().any(|c| c.is_numeric()) {
                Err(ServiceError::bad_request(
                    "Password must contain at least one number",
                ))
            } else {
                Ok(())
            }
        })
        .rule(|dto: &UserDTO| validation_rules::required("email")(&dto.email))
        .rule(|dto: &UserDTO| validation_rules::email("email")(&dto.email))
        .rule(|dto: &UserDTO| validation_rules::max_length("email", 255)(&dto.email))
}

/// Iterator-based validation using functional combinator pattern for LoginDTO
fn create_login_validator() -> Validator<LoginDTO> {
    Validator::new()
        .rule(|dto: &LoginDTO| validation_rules::required("username_or_email")(&dto.username_or_email))
        .rule(|dto: &LoginDTO| validation_rules::max_length("username_or_email", 255)(&dto.username_or_email))
        .rule(|dto: &LoginDTO| validation_rules::required("password")(&dto.password))
        .rule(|dto: &LoginDTO| validation_rules::max_length("password", 128)(&dto.password))
}

/// Legacy validation for backward compatibility - uses new functional validator
fn validate_user_dto(dto: &UserDTO) -> Result<(), ServiceError> {
    create_user_validator().validate(dto)
}

/// Legacy validation for backward compatibility - uses new functional validator
fn validate_login_dto(dto: &LoginDTO) -> Result<(), ServiceError> {
    create_login_validator().validate(dto)
}

#[derive(Serialize, Deserialize)]
pub struct TokenBodyResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
    pub tenant_id: String,
}

/// Creates a new user account after validating the provided `UserDTO`.
///
/// Validation is performed using the module's iterator-based validators; on success the function
/// executes a functional pipeline that persists the user via the database and returns a signup message.
///
/// # Returns
///
/// `Ok(String)` with a success message on success, `Err(ServiceError)` on failure.
///
/// # Examples
///
/// ```
/// # use crate::models::user::UserDTO;
/// # use crate::db::Pool;
/// # use crate::services::account::signup;
/// // Construct a valid UserDTO and obtain a `Pool` from your application context.
/// let user = UserDTO { username: "alice".into(), password: "Password1".into(), email: "alice@example.com".into() };
/// let pool: Pool = /* obtain pool from app context */;
/// let result = signup(user, &pool);
/// // `result` will be Ok(...) on success or Err(...) on failure.
/// ```
pub fn signup(user: UserDTO, pool: &Pool) -> Result<String, ServiceError> {
    // Use iterator-based validation pipeline
    validate_user_dto(&user)?;

    // Use functional pipeline with validated data
    crate::services::functional_service_base::ServicePipeline::new(pool.clone())
        .with_data(user)
        .execute(|user, conn| user_ops::signup_user(user, conn))
        .log_error("signup operation")
}

/// Creates a new user account using Either types for functional composition
pub fn signup_either(user: UserDTO, pool: &Pool) -> Either<ServiceError, String> {
    // Convert validation result to Either
    match validate_user_dto(&user) {
        Ok(()) => {
            // Use functional pipeline with validated data
            match crate::services::functional_service_base::ServicePipeline::new(pool.clone())
                .with_data(user)
                .execute(|user, conn| user_ops::signup_user(user, conn))
            {
                Ok(message) => Either::Right(message),
                Err(e) => Either::Left(e),
            }
        }
        Err(e) => Either::Left(e),
    }
}

/// Authenticate login credentials and return access and refresh tokens.
///
/// Validates the provided credentials, verifies the login session, generates an access token,
/// creates a new refresh token, and returns both tokens in a `TokenBodyResponse`.
///
/// # Returns
///
/// `Ok(TokenBodyResponse)` on successful authentication, `Err(ServiceError)` on failure.
///
/// # Examples
///
/// ```
/// // `get_test_pool` is a test helper that returns a configured `Pool`.
/// let login = LoginDTO {
///     username_or_email: "alice".to_string(),
///     password: "s3cr3tPass".to_string(),
/// };
/// let pool = get_test_pool();
/// let token_body = login(login, &pool).unwrap();
/// assert_eq!(token_body.token_type, "bearer");
/// ```
pub fn login(login: LoginDTO, pool: &Pool) -> Result<TokenBodyResponse, ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    query_service
        .query(|conn| {
            user_ops::login_user(login, conn).ok_or_else(|| {
                ServiceError::unauthorized(constants::MESSAGE_LOGIN_FAILED.to_string())
            })
        })
        .and_then(|logged_user| {
            // Since login_user now returns None for all authentication failures,
            // we no longer need to check for empty login_session
            // Get user by username and create refresh token
            query_service
                .query(|conn| {
                    user_ops::find_user_by_username(&logged_user.username, conn)
                        .map_err(|_| {
                            ServiceError::internal_server_error("Failed to find user".to_string())
                        })
                        .and_then(|user| {
                            let access_token = UserToken::generate_token(&logged_user);
                            RefreshToken::create(user.id, conn)
                                .map_err(|e| {
                                    ServiceError::internal_server_error(format!(
                                        "Failed to create refresh token: {}",
                                        e
                                    ))
                                })
                                .map(|refresh_token| (access_token, refresh_token))
                        })
                })
                .map(|(access_token, refresh_token)| TokenBodyResponse {
                    access_token,
                    refresh_token,
                    token_type: "bearer".to_string(),
                })
        })
        .log_error("login operation")
}

/// Authenticate login credentials using Either types for functional composition
pub fn login_either(login: LoginDTO, pool: &Pool) -> Either<ServiceError, TokenBodyResponse> {
    let query_service = FunctionalQueryService::new(pool.clone());

    query_service
        .query(|conn| {
            user_ops::login_user(login, conn).ok_or_else(|| {
                ServiceError::unauthorized(constants::MESSAGE_LOGIN_FAILED.to_string())
            })
        })
        .map(|logged_user| {
            // Since login_user now returns None for all authentication failures,
            // we no longer need to check for empty login_session
            // Get user by username and create refresh token
            query_service
                .query(|conn| {
                    user_ops::find_user_by_username(&logged_user.username, conn)
                        .map_err(|_| {
                            ServiceError::internal_server_error("Failed to find user".to_string())
                        })
                        .and_then(|user| {
                            let access_token = UserToken::generate_token(&logged_user);
                            RefreshToken::create(user.id, conn)
                                .map_err(|e| {
                                    ServiceError::internal_server_error(format!(
                                        "Failed to create refresh token: {}",
                                        e
                                    ))
                                })
                                .map(|refresh_token| (access_token, refresh_token))
                        })
                })
                .map(|(access_token, refresh_token)| TokenBodyResponse {
                    access_token,
                    refresh_token,
                    token_type: "bearer".to_string(),
                })
        })
        .map(|result| match result {
            Ok(response) => Either::Right(response),
            Err(e) => Either::Left(e),
        })
        .unwrap_or_else(|e| Either::Left(e))
}

/// Invalidate the user's session represented by a bearer Authorization header.
///
/// Attempts to extract and validate a bearer token from `authen_header`, verifies the token,
/// looks up the corresponding user, and invalidates that user's session in the database.
///
/// # Returns
///
/// `Ok(())` on successful logout, `Err(ServiceError)` on token validation or database errors.
///
/// # Examples
///
/// ```no_run
/// use http::HeaderValue;
/// // `pool` should be a valid database connection pool from your application's setup.
/// let auth = HeaderValue::from_static("Bearer some-valid-token");
/// let pool = /* obtain your Pool instance */ unimplemented!();
///
/// let result = logout(&auth, &pool);
/// match result {
///     Ok(()) => println!("Logged out"),
///     Err(e) => eprintln!("Logout failed: {:?}", e),
/// }
/// ```
pub fn logout(authen_header: &HeaderValue, pool: &Pool) -> Result<(), ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    authen_header
        .to_str()
        .map_err(|_| ServiceError::unauthorized(constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string()))
        .and_then(|authen_str| {
            if !token_utils::is_auth_header_valid(authen_header) {
                Err(ServiceError::unauthorized(
                    constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string(),
                ))
            } else {
                let token = authen_str[6..authen_str.len()].trim().to_string();
                Ok(token)
            }
        })
        .and_then(|token| {
            token_utils::decode_token(token).map_err(|_| {
                ServiceError::unauthorized(constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string())
            })
        })
        .and_then(|token_data| {
            token_utils::verify_token(&token_data, pool).map_err(|_| {
                ServiceError::unauthorized(constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string())
            })
        })
        .and_then(|username| {
            query_service
                .query(|conn| {
                    user_ops::find_user_by_username(&username, conn).map_err(|_| {
                        ServiceError::internal_server_error("Database error".to_string())
                    })
                })
                .map(|user| (user, username))
        })
        .and_then(|(user, _)| {
            query_service.query(|conn| {
                user_ops::logout_user(user.id, conn).map_err(|e| {
                    log::error!(
                        "Failed to clear login session for user {}: {}",
                        user.username,
                        e
                    );
                    ServiceError::internal_server_error("Failed to clear login session".to_string())
                })
            })
        })
        .log_error("logout operation")
}

/// Invalidate the user's session using Either types for functional composition
pub fn logout_either(authen_header: &HeaderValue, pool: &Pool) -> Either<ServiceError, ()> {
    let query_service = FunctionalQueryService::new(pool.clone());

    let result = authen_header
        .to_str()
        .map_err(|_| ServiceError::unauthorized(constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string()))
        .and_then(|authen_str| {
            if !token_utils::is_auth_header_valid(authen_header) {
                Err(ServiceError::unauthorized(
                    constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string(),
                ))
            } else {
                let token = authen_str[6..authen_str.len()].trim().to_string();
                Ok(token)
            }
        })
        .and_then(|token| {
            token_utils::decode_token(token).map_err(|_| {
                ServiceError::unauthorized(constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string())
            })
        })
        .and_then(|token_data| {
            token_utils::verify_token(&token_data, pool).map_err(|_| {
                ServiceError::unauthorized(constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string())
            })
        })
        .and_then(|username| {
            query_service
                .query(|conn| {
                    user_ops::find_user_by_username(&username, conn).map_err(|_| {
                        ServiceError::internal_server_error("Database error".to_string())
                    })
                })
                .map(|user| (user, username))
        })
        .and_then(|(user, _)| {
            query_service.query(|conn| {
                user_ops::logout_user(user.id, conn).map_err(|e| {
                    log::error!(
                        "Failed to clear login session for user {}: {}",
                        user.username,
                        e
                    );
                    ServiceError::internal_server_error("Failed to clear login session".to_string())
                })
            })
        });

    match result {
        Ok(()) => Either::Right(()),
        Err(e) => Either::Left(e),
    }
}

/// Refreshes an access token using the bearer token from an Authorization header.
///
/// Validates the Authorization header and token, verifies the login session, and returns a new access token. The returned `TokenBodyResponse` contains a freshly generated access token, an empty `refresh_token` (access-token refresh does not issue a new refresh token), and `token_type` set to `"bearer"`.
///
/// # Examples
///
/// ```no_run
/// use http::HeaderValue;
/// // `pool` should be an initialized database pool in real usage.
/// let header = HeaderValue::from_static("Bearer <token>");
/// let _ = refresh(&header, &pool);
/// ```
pub fn refresh(
    authen_header: &HeaderValue,
    pool: &Pool,
) -> Result<TokenBodyResponse, ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    authen_header
        .to_str()
        .map_err(|_| ServiceError::unauthorized(constants::MESSAGE_TOKEN_MISSING.to_string()))
        .and_then(|authen_str| {
            if !token_utils::is_auth_header_valid(authen_header) {
                Err(ServiceError::unauthorized(
                    constants::MESSAGE_TOKEN_MISSING.to_string(),
                ))
            } else {
                let token = authen_str[6..authen_str.len()].trim().to_string();
                Ok(token)
            }
        })
        .and_then(|token| {
            token_utils::decode_token(token).map_err(|_| {
                ServiceError::unauthorized(constants::MESSAGE_TOKEN_MISSING.to_string())
            })
        })
        .and_then(|token_data| {
            query_service.query(|conn| {
                if user_ops::is_valid_login_session(&token_data.claims, conn) {
                    user_ops::find_login_info_by_token(&token_data.claims, conn).map_err(|_| {
                        ServiceError::unauthorized(constants::MESSAGE_TOKEN_MISSING.to_string())
                    })
                } else {
                    Err(ServiceError::unauthorized(
                        constants::MESSAGE_TOKEN_MISSING.to_string(),
                    ))
                }
            })
        })
        .and_then(|login_info| {
            let access_token = UserToken::generate_token(&login_info);
            Ok(TokenBodyResponse {
                access_token,
                refresh_token: "".to_string(), // Access token refresh doesn't provide new refresh token
                token_type: "bearer".to_string(),
            })
        })
        .log_error("refresh operation")
}

/// Refreshes the access and refresh tokens for a valid refresh token and tenant.
///
/// Validates the provided refresh token, retrieves the associated user, revokes the old refresh token, creates a new refresh token, and generates a new access token.
///
/// # Arguments
/// - `refresh_token`: the refresh token string to validate and rotate.
/// - `tenant_id`: tenant identifier used when generating the access token.
///
/// # Returns
/// `TokenBodyResponse` containing the new `access_token`, the new `refresh_token`, and `token_type` set to `"bearer"`.
///
/// # Examples
///
/// ```
/// // Given a valid `pool`, `refresh_token`, and `tenant_id`
/// let resp = refresh_with_token(refresh_token, tenant_id, &pool).expect("refresh succeeds");
/// assert_eq!(resp.token_type, "bearer");
/// assert!(!resp.access_token.is_empty());
/// assert!(!resp.refresh_token.is_empty());
/// ```
pub fn refresh_with_token(
    refresh_token: &str,
    tenant_id: &str,
    pool: &Pool,
) -> Result<TokenBodyResponse, ServiceError> {
    log::debug!("refresh_with_token called for tenant: {}", tenant_id);
    let query_service = FunctionalQueryService::new(pool.clone());

    // Find and validate refresh token
    query_service
        .query(|conn| {
            RefreshToken::find_by_token(refresh_token, conn)
                .map_err(|_| ServiceError::unauthorized("Invalid refresh token".to_string()))
        })
        .and_then(|refresh_token_record| {
            log::debug!(
                "Found refresh token for user_id: {}, expires_at: {}",
                refresh_token_record.user_id,
                refresh_token_record.expires_at
            );
            // Get user info for new token generation
            query_service
                .query(|conn| {
                    user_ops::find_user_by_id(refresh_token_record.user_id, conn).map_err(|_| {
                        ServiceError::internal_server_error("Failed to find user".to_string())
                    })
                })
                .and_then(|user| {
                    // Generate new tokens
                    let access_token = UserToken::generate_token(&LoginInfoDTO {
                        username: user.username.clone(),
                        login_session: user.login_session.clone(),
                        tenant_id: tenant_id.to_string(),
                    });

                    // Revoke old refresh token and create new one
                    query_service
                        .query(|conn| {
                            // Revoke old token
                            RefreshToken::revoke(refresh_token, conn).map_err(|e| {
                                ServiceError::internal_server_error(format!(
                                    "Failed to revoke old token: {}",
                                    e
                                ))
                            })?;

                            // Create new refresh token
                            RefreshToken::create(user.id, conn).map_err(|e| {
                                ServiceError::internal_server_error(format!(
                                    "Failed to create refresh token: {}",
                                    e
                                ))
                            })
                        })
                        .map(|new_refresh_token| TokenBodyResponse {
                            access_token,
                            refresh_token: new_refresh_token,
                            token_type: "bearer".to_string(),
                        })
                })
        })
        .log_error("refresh_with_token operation")
}

/// Retrieve login information associated with a bearer token.
///
/// Validates and decodes the `Authorization` header, verifies the token, and queries the database for the corresponding login information.
///
/// # Returns
///
/// `Ok(LoginInfoDTO)` with the login information when the token is valid and the database query succeeds, `Err(ServiceError)` on token validation/decoding failure or database errors.
///
/// # Examples
///
/// ```
/// use http::header::HeaderValue;
/// # use crate::services::account::me;
/// # use crate::db::Pool;
/// let auth = HeaderValue::from_str("Bearer <token>").unwrap();
/// let pool: Pool = unimplemented!();
/// let _ = me(&auth, &pool);
/// ```
pub fn me(authen_header: &HeaderValue, pool: &Pool) -> Result<LoginInfoDTO, ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    authen_header
        .to_str()
        .map_err(|_| ServiceError::unauthorized(constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string()))
        .and_then(|authen_str| {
            if !token_utils::is_auth_header_valid(authen_header) {
                Err(ServiceError::unauthorized(
                    constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string(),
                ))
            } else {
                let token = authen_str[6..authen_str.len()].trim().to_string();
                Ok(token)
            }
        })
        .and_then(|token| {
            token_utils::decode_token(token).map_err(|_| {
                ServiceError::unauthorized(constants::MESSAGE_PROCESS_TOKEN_ERROR.to_string())
            })
        })
        .and_then(|token_data| {
            query_service.query(|conn| {
                user_ops::find_login_info_by_token(&token_data.claims, conn)
                    .map_err(|_| ServiceError::internal_server_error("Database error".to_string()))
            })
        })
        .log_error("me operation")
}

/// Retrieve users with pagination and return them as response DTOs.
///
/// Maps the paginated database user records into `UserResponseDTO` values and converts
/// database errors into `ServiceError`.
///
/// # Parameters
///
/// - `limit`: Maximum number of users to return for this page.
/// - `offset`: Number of users to skip before collecting the page.
///
/// # Returns
///
/// `Ok(Vec<UserResponseDTO>)` with the users for the requested page, `Err(ServiceError)` on database errors.
///
/// # Examples
///
/// ```
/// // Assume `pool` is a valid database connection pool available in scope.
/// let users = find_all_users(25, 0, &pool).expect("query failed");
/// assert!(users.len() <= 25);
/// ```
pub fn find_all_users(
    limit: i64,
    offset: i64,
    pool: &Pool,
) -> Result<Vec<UserResponseDTO>, ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    query_service
        .query(|conn| {
            user_ops::find_all_users(limit, offset, conn)
                .map_err(|e| ServiceError::internal_server_error(format!("Database error: {}", e)))
        })
        .map(|users| {
            users
                .into_iter()
                .map(|user| UserResponseDTO {
                    id: user.id,
                    username: user.username,
                    email: user.email,
                    active: user.active,
                })
                .collect()
        })
        .log_error("find_all_users operation")
}

/// Finds a user by their numeric ID.
///
/// Returns the user's public response DTO when the user exists; maps a missing user to a not-found service error and maps other database failures to an internal-server-error.
///
/// # Returns
///
/// `Ok(UserResponseDTO)` if the user exists, `Err(ServiceError)` when the user is not found or a database error occurs.
///
/// # Examples
///
/// ```
/// // assume `pool` is a configured `Pool`
/// let res = find_user_by_id(42, &pool);
/// if let Ok(user_dto) = res {
///     println!("username: {}", user_dto.username);
/// }
/// ```
pub fn find_user_by_id(user_id: i32, pool: &Pool) -> Result<UserResponseDTO, ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    query_service
        .query(|conn| {
            user_ops::find_user_by_id(user_id, conn).map_err(|e| match e {
                diesel::result::Error::NotFound => ServiceError::not_found("User not found"),
                _ => ServiceError::internal_server_error(format!("Database error: {}", e)),
            })
        })
        .map(|user| user_ops::user_to_response_dto(&user))
        .log_error("find_user_by_id operation")
}

/// Update an existing user's username, email, and active status.
///
/// Validates the provided `UserUpdateDTO` and applies the changes to the user record
/// identified by `user_id`. The `password` field of the DTO is ignored by this operation.
///
/// # Parameters
///
/// - `updated_user`: DTO containing the new username, email, and active flag; password is not updated.
///
/// # Returns
///
/// `Ok(())` on success, `Err(ServiceError)` on validation failure or database error.
///
/// # Examples
///
/// ```no_run
/// let dto = UserUpdateDTO {
///     username: "newname".to_string(),
///     email: "new@example.com".to_string(),
///     active: true,
/// };
/// let pool = create_pool(); // obtain database pool from application context
/// let res = update_user(42, dto, &pool);
/// assert!(res.is_ok());
/// ```
pub fn update_user(
    user_id: i32,
    updated_user: UserUpdateDTO,
    pool: &Pool,
) -> Result<(), ServiceError> {
    // Validate update DTO
    validate_user_update_dto(&updated_user)?;

    let query_service = FunctionalQueryService::new(pool.clone());

    // Check if user exists first
    query_service.query(|conn| {
        user_ops::find_user_by_id(user_id, conn).map_err(|e| match e {
            diesel::result::Error::NotFound => ServiceError::not_found("User not found"),
            _ => ServiceError::internal_server_error(format!("Database error: {}", e)),
        })
    })?;

    // Perform update
    query_service
        .query(|conn| {
            let user_dto = UserDTO {
                username: updated_user.username,
                email: updated_user.email,
                password: String::new(), // Password not updated through this endpoint
                active: updated_user.active,
            };
            user_ops::update_user_in_db(user_id, user_dto, conn).map_err(|e| match e {
                DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, info) => {
                    ServiceError::bad_request(info.message().to_string())
                }
                _ => ServiceError::internal_server_error(format!("Database error: {}", e)),
            })
        })
        .map(|_| ())
        .log_error("update_user operation")
}

/// Delete a user by ID.
///
/// Removes the user record from the database. Errors if the user does not exist or if a database
/// operation fails.
///
/// # Returns
///
/// `Ok(())` on success, `Err(ServiceError)` when the user does not exist or a database error occurs.
///
/// # Examples
///
/// ```no_run
/// let pool = /* obtain Pool from your application context */ ;
/// delete_user(42, &pool)?;
/// ```
pub fn delete_user(user_id: i32, pool: &Pool) -> Result<(), ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    // Check if user exists first
    query_service.query(|conn| {
        user_ops::find_user_by_id(user_id, conn).map_err(|e| match e {
            diesel::result::Error::NotFound => ServiceError::not_found("User not found"),
            _ => ServiceError::internal_server_error(format!("Database error: {}", e)),
        })
    })?;

    // Perform deletion
    query_service
        .query(|conn| {
            user_ops::delete_user_from_db(user_id, conn)
                .map_err(|e| ServiceError::internal_server_error(format!("Database error: {}", e)))
        })
        .map(|_| ())
        .log_error("delete_user operation")
}

/// Iterator-based validation for UserUpdateDTO
fn validate_user_update_dto(user_update: &UserUpdateDTO) -> Result<(), ServiceError> {
    Validator::new()
        .rule(|dto: &UserUpdateDTO| validation_rules::required("username")(&dto.username))
        .rule(|dto: &UserUpdateDTO| validation_rules::min_length("username", 3)(&dto.username))
        .rule(|dto: &UserUpdateDTO| validation_rules::max_length("username", 50)(&dto.username))
        .rule(|dto: &UserUpdateDTO| validation_rules::required("email")(&dto.email))
        .rule(|dto: &UserUpdateDTO| validation_rules::email("email")(&dto.email))
        .validate(user_update)
}
