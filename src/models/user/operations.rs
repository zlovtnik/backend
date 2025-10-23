//! Pure functional operations for User model
//!
//! This module contains all database and business logic operations for Users,
//! implemented as pure functions with functional composition patterns.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use diesel::{prelude::*, result::DatabaseErrorKind, result::QueryResult};
use uuid::Uuid;

use crate::{
    config::db::Connection,
    constants,
    error::ServiceError,
    models::{
        login_history::LoginHistory,
        user::{LoginDTO, LoginInfoDTO, User, UserDTO},
        user_token::UserToken,
    },
    schema::users::dsl::*,
};

/// Hash a plain password using Argon2 with a randomly generated salt.
///
/// Returns the encoded Argon2 password hash on success or an error message on failure.
///
/// # Examples
///
/// ```
/// let res = hash_password_argon2("s3cr3t");
/// assert!(res.is_ok());
/// let hash = res.unwrap();
/// assert!(hash.starts_with("$argon2"));
/// ```
pub fn hash_password_argon2(plain_password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(plain_password.as_bytes(), &salt)
        .map_err(|e| format!("Password hashing failed: {}", e))
        .map(|hash| hash.to_string())
}

/// Verify a user-supplied password against a stored password hash that may be Argon2 or bcrypt.
///
/// Attempts bcrypt verification when the stored hash has a bcrypt prefix (`$2`); otherwise attempts Argon2 verification. Returns `true` on successful verification, `false` on any mismatch or parsing/verification error.
///
/// # Examples
///
/// ```
/// use rand::thread_rng;
/// use argon2::Argon2;
/// // bcrypt example
/// let password = "s3cr3t";
/// let bcrypt_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
/// assert!(verify_password_hybrid(&bcrypt_hash, password));
///
/// // argon2 example
/// use argon2::password_hash::SaltString;
/// let salt = SaltString::generate(&mut thread_rng());
/// let argon2 = Argon2::default();
/// let argon2_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();
/// assert!(verify_password_hybrid(&argon2_hash, password));
/// ```
pub fn verify_password_hybrid(stored_hash: &str, provided_password: &str) -> bool {
    let is_bcrypt = stored_hash.starts_with("$2");

    if is_bcrypt {
        // Hash looks like bcrypt, use bcrypt verification
        bcrypt::verify(provided_password, stored_hash).unwrap_or(false)
    } else {
        // Default to Argon2 verification
        PasswordHash::new(stored_hash)
            .map(|parsed_hash| {
                let argon2 = Argon2::default();
                argon2
                    .verify_password(provided_password.as_bytes(), &parsed_hash)
                    .is_ok()
            })
            .unwrap_or(false)
    }
}

/// Generates a new login session identifier as a UUID v4 string.
///
/// # Examples
///
/// ```
/// use uuid::Uuid;
/// let s = crate::models::user::operations::generate_login_session();
/// assert!(Uuid::parse_str(&s).is_ok());
/// ```
pub fn generate_login_session() -> String {
    Uuid::new_v4().to_string()
}

/// Creates and persistently records a new login session for a user.
///
/// Attempts to save a login history entry for `user_name`, generates a new session identifier,
/// updates the user's `login_session` in the database, and returns the corresponding `LoginInfoDTO`
/// on success; returns `None` if any operation fails.
///
/// # Parameters
///
/// - `user_name`: username to create the session for.
/// - `tenant_id`: tenant identifier to include in the returned `LoginInfoDTO`.
/// - `conn`: mutable database connection used for persistence operations.
///
/// # Returns
///
/// `Some(LoginInfoDTO)` containing `username`, `login_session`, and `tenant_id` if the login history
/// was saved and the user's `login_session` was updated successfully; `None` otherwise.
///
/// # Examples
///
/// ```
/// // assumes a helper `establish_connection()` exists in test context
/// let mut conn = establish_connection();
/// let result = create_login_session("alice", "tenant-1".to_string(), &mut conn);
/// assert!(result.is_some());
/// ```
pub fn create_login_session(
    user_name: &str,
    tenant_id: String,
    conn: &mut Connection,
) -> Option<LoginInfoDTO> {
    // Create and save login history
    let login_history = LoginHistory::create(user_name, conn)?;
    if let Err(e) = LoginHistory::save_login_history(login_history, conn) {
        log::error!(
            "Failed to save login history for user '{}': {}",
            user_name,
            e
        );
        return None;
    }

    // Generate session and update database
    let login_session_str = generate_login_session();
    match update_login_session_to_db(user_name, &login_session_str, conn) {
        Ok(()) => Some(LoginInfoDTO {
            username: user_name.to_string(),
            login_session: login_session_str,
            tenant_id,
        }),
        Err(e) => {
            log::error!(
                "Failed to update login session for user '{}': {}",
                user_name,
                e
            );
            None
        }
    }
}

/// Updates a user's login session string in the database by username.
///
/// Attempts to find a user with `username_str` and set their `login_session` to `login_session_str`.
///
/// # Parameters
///
/// - `username_str`: username of the user whose session should be updated.
/// - `login_session_str`: new login session string to store for the user.
///
/// # Returns
///
/// `true` if the update succeeded and affected the user's record, `false` otherwise.
///
/// # Examples
///
/// ```no_run
/// // Assuming `conn` is a valid diesel::PgConnection
/// let updated = update_login_session_to_db("alice", "new-session-uuid", &mut conn);
/// if updated {
///     println!("session updated");
/// }
/// ```
pub fn update_login_session_to_db(
    username_str: &str,
    login_session_str: &str,
    conn: &mut Connection,
) -> Result<(), diesel::result::Error> {
    let user = find_user_by_username(username_str, conn)?;

    let rows_updated = diesel::update(users.filter(id.eq(user.id)))
        .set(login_session.eq(login_session_str.to_string()))
        .execute(conn)?;

    if rows_updated > 0 {
        Ok(())
    } else {
        Err(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::Unknown,
            Box::new("No rows updated while updating login session".to_string()),
        ))
    }
}

/// Registers a new user by hashing their password and inserting the user record into the database.
///
/// Hashes the provided plaintext password with Argon2, constructs a new UserDTO containing the hash,
/// and attempts to insert it into the users table. If the username (or other unique constraint) already
/// exists, returns a `bad_request` ServiceError identifying the duplicate; on hashing failures or other
/// database errors returns an `internal_server_error`.
///
/// # Returns
///
/// `Ok(String)` with a success message on successful registration.
/// `Err(ServiceError)` with `bad_request` when the user is already registered, or `internal_server_error` for hashing or other database failures.
///
/// # Examples
///
/// ```no_run
/// # use crate::models::user::{signup_user, UserDTO};
/// # use crate::db::establish_connection;
/// let user = UserDTO {
///     username: "alice".to_string(),
///     email: "alice@example.com".to_string(),
///     password: "s3cr3t".to_string(),
///     active: true,
///     ..Default::default()
/// };
/// let mut conn = establish_connection();
/// let result = signup_user(user, &mut conn);
/// ```
pub fn signup_user(user: UserDTO, conn: &mut Connection) -> Result<String, ServiceError> {
    // Hash password using functional composition
    let password_hash = hash_password_argon2(&user.password)
        .map_err(|_| ServiceError::internal_server_error("Failed to hash password".to_string()))?;

    let user_name = user.username.clone();
    let new_user = UserDTO {
        password: password_hash,
        ..user
    };

    // Insert with functional error handling
    diesel::insert_into(users)
        .values(new_user)
        .execute(conn)
        .map_err(|err| match err {
            diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                ServiceError::bad_request(format!("User '{}' is already registered", user_name))
            }
            _ => {
                log::error!("Signup failed: {}", err);
                ServiceError::internal_server_error("Internal server error".to_string())
            }
        })?;

    Ok(constants::MESSAGE_SIGNUP_SUCCESS.to_string())
}

/// Authenticate credentials and create a login session for a user.
///
/// Attempts to find an active user matching the provided username or email, verifies the provided password
/// (supports bcrypt and Argon2 formats), and creates a new login session on success.
///
/// # Parameters
///
/// - `login`: credentials containing `username_or_email`, `password`, and `tenant_id`.
///
/// # Returns
///
/// `Some(LoginInfoDTO)` with the created login session and username on successful authentication and session creation, `None` if authentication fails or a required database operation does not succeed.
///
/// # Examples
///
/// ```
/// let login = LoginDTO {
///     username_or_email: "alice".into(),
///     password: "s3cret".into(),
///     tenant_id: "tenant_1".into(),
/// };
/// let result = login_user(login, &mut conn);
/// if let Some(info) = result {
///     assert_eq!(info.username, "alice");
/// }
/// ```
pub fn login_user(login: LoginDTO, conn: &mut Connection) -> Option<LoginInfoDTO> {
    // Functional composition: lookup -> validate -> verify -> create session
    find_user_by_credentials(&login.username_or_email, conn)
        .filter(|user| user.active && !user.password.is_empty())
        .filter(|user| verify_password_hybrid(&user.password, &login.password))
        .and_then(|user| create_login_session(&user.username, login.tenant_id, conn))
}

/// Retrieves a user whose username or email matches the given identifier.
///
/// Searches the users table for a row where `username` or `email` equals `identifier` and returns that user if found.
///
/// # Examples
///
/// ```
/// let maybe_user = find_user_by_credentials("alice@example.com", &mut conn);
/// if let Some(user) = maybe_user {
///     assert!(user.username == "alice" || user.email == "alice@example.com");
/// }
/// ```
///
/// # Returns
///
/// `Some(User)` when a matching user is found, `None` if no match is found or the query fails.
pub fn find_user_by_credentials(identifier: &str, conn: &mut Connection) -> Option<User> {
    users
        .filter(username.eq(identifier))
        .or_filter(email.eq(identifier))
        .get_result::<User>(conn)
        .ok()
}

pub fn logout_user(user_id: i32, conn: &mut Connection) -> Result<(), diesel::result::Error> {
    let _ = diesel::update(users.filter(id.eq(user_id)))
        .set(login_session.eq(""))
        .execute(conn)?;
    Ok(())
}

/// Validates that a UserToken matches an existing user's login session in the database.
///
/// Returns `true` if a user with matching `username` and `login_session` exists, `false` otherwise.
///
/// # Examples
///
/// ```no_run
/// let token = UserToken { user: "alice".to_string(), login_session: "sess-123".to_string(), tenant_id: "t1".to_string() };
/// // `conn` is a live database connection (diesel::PgConnection or similar)
/// let valid = is_valid_login_session(&token, &mut conn);
/// println!("session valid: {}", valid);
/// ```
pub fn is_valid_login_session(user_token: &UserToken, conn: &mut Connection) -> bool {
    let username_trimmed = user_token.user.trim();
    let session_trimmed = user_token.login_session.trim();

    if username_trimmed.is_empty() || session_trimmed.is_empty() {
        return false;
    }

    users
        .filter(username.eq(username_trimmed))
        .filter(login_session.eq(session_trimmed))
        .filter(login_session.ne(""))
        .get_result::<User>(conn)
        .is_ok()
}

/// Retrieve login information that corresponds to a user token.
///
/// Looks up a user whose `username` and `login_session` match the supplied `UserToken` and returns a `LoginInfoDTO`
/// containing the username, stored login session, and the token's tenant id. If no matching user is found this
/// returns a `ServiceError::not_found`; unexpected database errors are mapped to `ServiceError::internal_server_error`.
///
/// # Examples
///
/// ```
/// // Assumes `conn` is a valid &mut Connection and a user with the matching session exists.
/// let token = UserToken {
///     user: "alice".into(),
///     login_session: "session-uuid".into(),
///     tenant_id: "tenant-1".into(),
/// };
/// let info = find_login_info_by_token(&token, &mut conn).unwrap();
/// assert_eq!(info.username, "alice");
/// assert_eq!(info.tenant_id, "tenant-1");
/// ```
pub fn find_login_info_by_token(
    user_token: &UserToken,
    conn: &mut Connection,
) -> Result<LoginInfoDTO, ServiceError> {
    let username_trimmed = user_token.user.trim();
    let session_trimmed = user_token.login_session.trim();

    if session_trimmed.is_empty() {
        return Err(ServiceError::bad_request(
            "Login session token cannot be empty",
        ));
    }

    if username_trimmed.is_empty() {
        return Err(ServiceError::bad_request("Username cannot be empty"));
    }

    let user_result = users
        .filter(username.eq(username_trimmed))
        .filter(login_session.eq(session_trimmed))
        .filter(login_session.ne(""))
        .get_result::<User>(conn);

    match user_result {
        Ok(user) => Ok(LoginInfoDTO {
            username: user.username,
            login_session: user.login_session,
            tenant_id: user_token.tenant_id.clone(),
        }),
        Err(diesel::result::Error::NotFound) => Err(ServiceError::not_found("User not found")),
        Err(e) => {
            log::error!("Failed to query user: {}", e);
            Err(ServiceError::internal_server_error(
                "Internal server error".to_string(),
            ))
        }
    }
}

/// Retrieves the user record that exactly matches the provided username.
///
/// # Examples
///
/// ```
/// let user = find_user_by_username("alice", &mut conn).expect("user exists");
/// assert_eq!(user.username, "alice");
/// ```
pub fn find_user_by_username(username_str: &str, conn: &mut Connection) -> QueryResult<User> {
    users
        .filter(username.eq(username_str))
        .get_result::<User>(conn)
}

/// Retrieve a user record that matches the given email.
///
/// # Parameters
///
/// - `email_str`: Email address to search for.
/// - `conn`: Mutable database connection.
///
/// # Returns
///
/// `Ok(User)` with the matching user when a row exists; `Err` with a Diesel error (for example `NotFound` if no match or other database errors) otherwise.
///
/// # Examples
///
/// ```no_run
/// # use crate::models::user::operations::find_user_by_email;
/// # fn example(conn: &mut crate::db::Connection) {
/// let res = find_user_by_email("alice@example.com", conn);
/// if let Ok(user) = res {
///     assert_eq!(user.email, "alice@example.com");
/// }
/// # }
/// ```
pub fn find_user_by_email(email_str: &str, conn: &mut Connection) -> QueryResult<User> {
    users.filter(email.eq(email_str)).get_result::<User>(conn)
}

/// Retrieves the user with the specified ID from the database.
///
/// # Examples
///
/// ```
/// let user = find_user_by_id(1, &mut conn).unwrap();
/// assert_eq!(user.id, 1);
/// ```
pub fn find_user_by_id(user_id: i32, conn: &mut Connection) -> QueryResult<User> {
    users.filter(id.eq(user_id)).first::<User>(conn)
}

/// Retrieve the total number of users in the database.
///
/// # Returns
///
/// `Ok(i64)` with the total user count on success, or a `diesel::result::Error` wrapped in `Err` on failure.
///
/// # Examples
///
/// ```no_run
/// # use diesel::prelude::*;
/// # use crate::models::user::operations::count_all_users;
/// # fn example(conn: &mut diesel::PgConnection) {
/// let total = count_all_users(conn).expect("query failed");
/// println!("Total users: {}", total);
/// # }
/// ```
pub fn count_all_users(conn: &mut Connection) -> QueryResult<i64> {
    users.count().get_result(conn)
}

/// Retrieves a paginated list of users ordered by `id`.
///
/// Uses `limit` and `offset` to control pagination; results are ordered ascending by `id`.
///
/// # Examples
///
/// ```no_run
/// # use crate::models::user::operations::find_all_users;
/// # use diesel::r2d2::ConnectionManager;
/// # use diesel::PgConnection;
/// // `conn` must be a valid `&mut PgConnection`.
/// let users = find_all_users(10, 0, conn).expect("query failed");
/// assert!(users.len() <= 10);
/// ```
pub fn find_all_users(limit: i64, offset: i64, conn: &mut Connection) -> QueryResult<Vec<User>> {
    users
        .order(id)
        .limit(limit)
        .offset(offset)
        .load::<User>(conn)
}

/// Update a user's username, email, and active status by their ID.
///
/// Returns the number of rows updated on success, or a `diesel::QueryResult` error on failure.
///
/// # Examples
///
/// ```
/// # use diesel::connection::Connection;
/// # fn example(conn: &mut diesel::pg::PgConnection) {
/// let dto = crate::models::user::UserDTO {
///     username: "new_name".into(),
///     email: "new@example.com".into(),
///     active: true,
///     ..Default::default()
/// };
/// let res = crate::models::user::operations::update_user(42, dto, conn);
/// assert!(res.is_ok());
/// if let Ok(rows) = res {
///     assert_eq!(rows, 1); // expected one row updated
/// }
/// # }
/// ```
pub fn update_user(
    user_id: i32,
    updated_user: crate::models::user::UserUpdateDTO,
    conn: &mut Connection,
) -> QueryResult<usize> {
    diesel::update(users.filter(id.eq(user_id)))
        .set((
            username.eq(updated_user.username),
            email.eq(updated_user.email),
            active.eq(updated_user.active),
        ))
        .execute(conn)
}

/// Deletes the user record with the specified ID from the database.
///
/// # Examples
///
/// ```no_run
/// # use diesel::prelude::*;
/// # use crate::models::user::operations::delete_user_by_id;
/// # fn establish_connection() -> diesel::SqliteConnection { unimplemented!() }
/// # let mut conn = establish_connection();
/// let deleted = delete_user_by_id(42, &mut conn).expect("query failed");
/// assert!(deleted == 0 || deleted == 1);
/// ```
///
/// # Returns
///
/// `Ok(n)` with the number of rows deleted (typically 0 or 1), or a `QueryResult` error.
pub fn delete_user_by_id(user_id: i32, conn: &mut Connection) -> QueryResult<usize> {
    diesel::delete(users.filter(id.eq(user_id))).execute(conn)
}

/// Counts users that currently have a non-empty login session.
///
/// Returns the number of users whose `login_session` is not null and not an empty string.
///
/// # Examples
///
/// ```no_run
/// # use diesel::pg::PgConnection;
/// # use crate::models::user::operations::count_logged_in_users;
/// # fn get_conn() -> PgConnection { unimplemented!() }
/// let mut conn = get_conn();
/// let total = count_logged_in_users(&mut conn).expect("query failed");
/// println!("Logged in users: {}", total);
/// ```
pub fn count_logged_in_users(conn: &mut Connection) -> QueryResult<i64> {
    users.filter(login_session.ne("")).count().get_result(conn)
}

pub fn update_user_in_db(
    user_id: i32,
    user_dto: crate::models::user::UserDTO,
    conn: &mut Connection,
) -> diesel::QueryResult<usize> {
    update_user(
        user_id,
        crate::models::user::UserUpdateDTO {
            username: user_dto.username,
            email: user_dto.email,
            active: user_dto.active,
        },
        conn,
    )
}

pub fn delete_user_from_db(user_id: i32, conn: &mut Connection) -> diesel::QueryResult<usize> {
    delete_user_by_id(user_id, conn)
}

/// Convert a `User` model into a `UserResponseDTO`.
///
/// # Examples
///
/// ```
/// use crate::models::user::{User, UserResponseDTO};
///
/// let user = User {
///     id: 1,
///     username: "alice".into(),
///     email: "alice@example.com".into(),
///     active: true,
///     password: "".into(),
///     login_session: None,
/// };
///
/// let dto: UserResponseDTO = crate::models::user::operations::user_to_response_dto(&user);
/// assert_eq!(dto.id, 1);
/// assert_eq!(dto.username, "alice");
/// assert_eq!(dto.email, "alice@example.com");
/// assert!(dto.active);
/// ```
pub fn user_to_response_dto(user: &User) -> crate::models::user::UserResponseDTO {
    crate::models::user::UserResponseDTO {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        active: user.active,
    }
}
