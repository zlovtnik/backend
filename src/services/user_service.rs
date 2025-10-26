//! User Service - Functional Patterns for User Operations
//!
//! Provides functional programming patterns for user management operations,
//! using QueryReader monads, validators, and composable pipelines.

use crate::{
    config::db::Pool,
    error::{ServiceError, ServiceResult},
    models::user::{operations as user_ops, UserResponseDTO, UserUpdateDTO},
    services::functional_patterns::{self as functional_patterns, validation_rules, QueryReader, Validator},
};

/// Pagination parameters with functional validation
#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub limit: i64,
    pub offset: i64,
}

impl PaginationParams {
    /// Create pagination params with functional validation and clamping
    pub fn from_query(limit_str: Option<&str>, offset_str: Option<&str>) -> Self {
        let limit = limit_str
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(50)
            .clamp(1, 500);

        let offset = offset_str
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0)
            .max(0);

        Self { limit, offset }
    }
}

/// Validator for user update operations
pub fn user_update_validator() -> Validator<UserUpdateDTO> {
    Validator::new()
        .rule(|dto: &UserUpdateDTO| validation_rules::required("username")(&dto.username))
        .rule(|dto: &UserUpdateDTO| validation_rules::email("email")(&dto.email))
        .rule(|dto: &UserUpdateDTO| validation_rules::max_length("email", 255)(&dto.email))
}

/// Build a QueryReader for listing all users with pagination
pub fn list_users_reader(limit: i64, offset: i64) -> QueryReader<Vec<UserResponseDTO>> {
    QueryReader::new(move |conn| {
        user_ops::find_all_users(limit, offset, conn)
            .map_err(|e| {
                ServiceError::internal_server_error(format!("Failed to list users: {}", e))
                    .with_tag("user")
            })
            .map(|users| users.into_iter().map(UserResponseDTO::from).collect())
    })
}

/// Build a QueryReader for finding a user by ID
pub fn find_user_by_id_reader(user_id: i32) -> QueryReader<UserResponseDTO> {
    QueryReader::new(move |conn| {
        user_ops::find_user_by_id(user_id, conn)
            .map_err(|e| {
                log::error!("Failed to find user {}: {}", user_id, e);
                ServiceError::not_found(format!("User {} not found", user_id))
                    .with_tag("user")
                    .with_metadata("db_error", e.to_string())
            })
            .map(UserResponseDTO::from)
    })
}

/// Build a QueryReader for updating a user
pub fn update_user_reader(
    user_id: i32,
    dto: UserUpdateDTO,
) -> Result<QueryReader<UserResponseDTO>, ServiceError> {
    // Validate the update DTO first
    user_update_validator().validate(&dto)?;

    Ok(QueryReader::new(move |conn| {
        // Update the user
        user_ops::update_user(user_id, dto.clone(), conn)
            .map_err(|e| {
                ServiceError::internal_server_error(format!("Failed to update user: {}", e))
                    .with_tag("user")
            })?;
        
        // Fetch and return the updated user
        user_ops::find_user_by_id(user_id, conn)
            .map_err(|e| {
                ServiceError::internal_server_error(format!("Failed to fetch updated user: {}", e))
                    .with_tag("user")
            })
            .map(UserResponseDTO::from)
    }))
}

/// Build a QueryReader for deleting a user
pub fn delete_user_reader(user_id: i32) -> QueryReader<usize> {
    QueryReader::new(move |conn| {
        user_ops::delete_user_by_id(user_id, conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to delete user: {}", e))
                .with_tag("user")
        })
    })
}

/// Execute a QueryReader with a database pool (re-exported for convenience)
pub fn run_query<T>(reader: QueryReader<T>, pool: &Pool) -> ServiceResult<T> {
    functional_patterns::run_query(reader, pool)
}
