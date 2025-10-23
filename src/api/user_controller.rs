use actix_web::http::StatusCode;
use actix_web::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use log::info;
use serde_json::json;

use crate::{
    config::db::Pool,
    constants,
    error::ServiceError,
    functional::response_transformers::{ResponseTransformError, ResponseTransformer},
    models::user::UserUpdateDTO,
    services::{account_service, functional_service_base::FunctionalErrorHandling},
};

fn response_composition_error(err: ResponseTransformError) -> ServiceError {
    ServiceError::internal_server_error(constants::MESSAGE_INTERNAL_SERVER_ERROR)
        .with_tag("response")
        .with_detail(err.to_string())
}

fn respond_empty(req: &HttpRequest, status: StatusCode, message: &str) -> HttpResponse {
    ResponseTransformer::new(constants::EMPTY)
        .with_message(message.to_string())
        .with_status(status)
        .respond_to(req)
}

/// Extracts the tenant Pool stored in the request's extensions.
///
/// # Returns
///
/// `Ok(pool)` with a clone of the tenant `Pool` if present in the request extensions, `Err(ServiceError::BadRequest)` with message `"Tenant not found"` otherwise.
///
/// # Examples
///
/// ```no_run
/// use actix_web::HttpRequest;
/// // assume `Pool` and `ServiceError` are in scope
/// // let req: HttpRequest = /* request with Pool inserted into extensions */ ;
/// // let pool = extract_tenant_pool(&req)?;
/// ```
fn extract_tenant_pool(req: &HttpRequest) -> Result<Pool, ServiceError> {
    match req.extensions().get::<Pool>() {
        Some(pool) => Ok(pool.clone()),
        None => Err(ServiceError::unauthorized("Tenant not found")
            .with_tag("tenant")
            .with_detail("Missing tenant pool in request extensions")),
    }
}

/// Get all users with pagination support.
///
/// Returns a paginated list of users for the current tenant.
/// Supports query parameters: `limit` (default 50, max 500) and `offset` (default 0).
///
/// # Examples
///
/// ```no_run
/// use actix_web::{test, web, HttpRequest};
///
/// // GET /api/users
/// // GET /api/users?limit=10&offset=20
/// ```
pub async fn find_all(
    query: web::Query<std::collections::HashMap<String, String>>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    info!("Processing find_all users request");

    let mut limit = query
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(50);

    if limit < 1 {
        limit = 1;
    } else if limit > 500 {
        limit = 500;
    }

    let mut offset = query
        .get("offset")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);

    if offset < 0 {
        offset = 0;
    }

    let pool = extract_tenant_pool(&req)?;

    account_service::find_all_users(limit, offset, &pool)
        .log_error("user_controller::find_all")
        .and_then(|users| {
            ResponseTransformer::new(json!(users))
                .with_message(constants::MESSAGE_OK.to_string())
                .try_with_metadata(json!({
                    "pagination": {
                        "limit": limit,
                        "offset": offset,
                        "count": users.len()
                    }
                }))
                .map_err(response_composition_error)
                .map(|transformer| transformer.respond_to(&req))
        })
}

/// Get a user by ID.
///
/// Returns the user with the specified ID if it exists and belongs to the current tenant.
///
/// # Examples
///
/// ```no_run
/// use actix_web::{test, web, HttpRequest};
///
/// // GET /api/users/123
/// ```
pub async fn find_by_id(
    user_id: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    info!("Processing find_by_id user request for id: {}", user_id);

    let pool = extract_tenant_pool(&req)?;

    account_service::find_user_by_id(user_id.into_inner(), &pool)
        .log_error("user_controller::find_by_id")
        .and_then(|user| {
            Ok(ResponseTransformer::new(json!(user))
                .with_message(constants::MESSAGE_OK.to_string())
                .respond_to(&req))
        })
}

/// Update an existing user.
///
/// Updates the user with the specified ID using the provided data.
/// Only username, email, and active status can be updated through this endpoint.
///
/// # Examples
///
/// ```no_run
/// use actix_web::{test, web, HttpRequest};
/// use serde_json::json;
///
/// // PUT /api/users/123
/// // Body: {"username": "newname", "email": "new@example.com", "active": true}
/// ```
pub async fn update(
    user_id: web::Path<i32>,
    user_update: web::Json<UserUpdateDTO>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    info!("Processing update user request for id: {}", user_id);

    let pool = extract_tenant_pool(&req)?;

    account_service::update_user(user_id.into_inner(), user_update.into_inner(), &pool)
        .log_error("user_controller::update")
        .map(|_| respond_empty(&req, StatusCode::OK, constants::MESSAGE_OK))
}

/// Delete a user by ID.
///
/// Permanently removes the user with the specified ID.
/// This action cannot be undone.
///
/// # Examples
///
/// ```no_run
/// use actix_web::{test, web, HttpRequest};
///
/// // DELETE /api/users/123
/// ```
pub async fn delete(
    user_id: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    info!("Processing delete user request for id: {}", user_id);

    let pool = extract_tenant_pool(&req)?;

    account_service::delete_user(user_id.into_inner(), &pool)
        .log_error("user_controller::delete")
        .map(|_| respond_empty(&req, StatusCode::OK, constants::MESSAGE_OK))
}
