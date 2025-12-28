use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result};
use diesel::prelude::*;
use serde_json::json;
use validator::Validate;

use crate::{
    config::db::Pool,
    error::ServiceError,
    models::ret_nfag::{RetNfag, NewRetNfag},    types::TenantId,};

// Constants for pagination validation
const MAX_LIMIT: i64 = 1000;

/// Extract the database pool from the request extensions.
fn extract_pool(req: &HttpRequest) -> Result<Pool, ServiceError> {
    req.extensions().get::<Pool>().cloned().ok_or_else(|| {
        ServiceError::internal_server_error("Pool not found")
            .with_detail("Missing tenant pool in request extensions")
            .with_tag("tenant")
    })
}

/// Extract tenant ID from request extensions.
fn extract_tenant_id(req: &HttpRequest) -> Result<TenantId, ServiceError> {
    req.extensions()
        .get::<TenantId>()
        .cloned()
        .ok_or_else(|| {
            ServiceError::internal_server_error("Tenant ID not found")
                .with_detail("Missing tenant ID in request extensions")
                .with_tag("tenant")
        })
}

#[utoipa::path(
    post,
    path = "/api/ret-nfag",
    request_body = crate::models::ret_nfag::CreateRetNfagRequest,
    responses(
        (status = 201, description = "RetNfag created", body = crate::models::ret_nfag::RetNfag)
    ),
    tag = "ret-nfag"
)]
pub async fn create_ret_nfag(
    req: HttpRequest,
    req_body: web::Json<crate::models::ret_nfag::CreateRetNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    req_body.validate().map_err(|e| {
        ServiceError::bad_request("Validation failed")
            .with_detail(format!("{:?}", e))
            .with_tag("validation")
    })?;

    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = web::block(move || pool.get())
        .await
        .map_err(|e| {
            ServiceError::internal_server_error("Thread pool error")
                .with_detail(e.to_string())
                .with_tag("threading")
        })?
        .map_err(|e| {
            ServiceError::internal_server_error("Database connection failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    let new_ret: NewRetNfag = (req_body.into_inner(), tenant_id).into();

    let created_ret = RetNfag::create(new_ret, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to create Ret NFAg")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Created().json(created_ret))
}

#[utoipa::path(
    get,
    path = "/api/ret-nfag",
    params(
        ("limit" = Option<i64>, Query, description = "Max results (default 50, max 1000)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses(
        (status = 200, description = "List RetNfag", body = [crate::models::ret_nfag::RetNfag])
    ),
    tag = "ret-nfag"
)]
pub async fn find_all_ret_nfag(
    req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ServiceError> {
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;
    let limit: i64 = query.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50)
        .clamp(1, MAX_LIMIT); // Min 1 record, max 1000 records

    let offset: i64 = query.get("offset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
        .max(0); // Ensure non-negative

    let mut conn = web::block(move || pool.get())
        .await
        .map_err(|e| {
            ServiceError::internal_server_error("Thread pool error")
                .with_detail(e.to_string())
                .with_tag("threading")
        })?
        .map_err(|e| {
            ServiceError::internal_server_error("Database connection failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    let ret_list = RetNfag::find_all_by_tenant(tenant_id.as_str(), limit, offset, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to find Ret NFAg")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let total_count = RetNfag::count_by_tenant(tenant_id.as_str(), &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to count Ret NFAg")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Ok().json(json!({
        "data": ret_list,
        "pagination": {
            "limit": limit,
            "offset": offset,
            "total": total_count
        }
    })))
}

#[utoipa::path(
    get,
    path = "/api/ret-nfag/{id}",
    params(
        ("id" = i32, Path, description = "RetNfag id")
    ),
    responses(
        (status = 200, description = "RetNfag by id", body = crate::models::ret_nfag::RetNfag),
        (status = 404, description = "Not found")
    ),
    tag = "ret-nfag"
)]
pub async fn find_ret_nfag_by_id(
    path: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let ret_id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = web::block(move || pool.get())
        .await
        .map_err(|e| {
            ServiceError::internal_server_error("Thread pool error")
                .with_detail(e.to_string())
                .with_tag("threading")
        })?
        .map_err(|e| {
            ServiceError::internal_server_error("Database connection failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    let ret = RetNfag::find_by_id_and_tenant(ret_id, tenant_id.as_str(), &mut conn).map_err(|e| {
        match e {
            diesel::result::Error::NotFound => {
                ServiceError::not_found("Ret NFAg not found")
                    .with_detail(format!("No record with id {} for tenant {}", ret_id, tenant_id))
                    .with_tag("not_found")
            }
            _ => ServiceError::internal_server_error("Failed to find Ret NFAg")
                .with_detail(e.to_string())
                .with_tag("database"),
        }
    })?;

    Ok(HttpResponse::Ok().json(ret))
}

#[utoipa::path(
    put,
    path = "/api/ret-nfag/{id}",
    request_body = crate::models::ret_nfag::UpdateRetNfagRequest,
    params(
        ("id" = i32, Path, description = "RetNfag id")
    ),
    responses(
        (status = 200, description = "RetNfag updated", body = crate::models::ret_nfag::RetNfag),
        (status = 404, description = "Not found")
    ),
    tag = "ret-nfag"
)]
pub async fn update_ret_nfag(
    path: web::Path<i32>,
    req: HttpRequest,
    req_body: web::Json<crate::models::ret_nfag::UpdateRetNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    req_body.validate().map_err(|e| {
        ServiceError::bad_request("Validation failed")
            .with_detail(format!("{:?}", e))
            .with_tag("validation")
    })?;

    let ret_id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;
    let update_request = req_body.into_inner();

    let mut conn = web::block(move || pool.get())
        .await
        .map_err(|e| {
            ServiceError::internal_server_error("Thread pool error")
                .with_detail(e.to_string())
                .with_tag("threading")
        })?
        .map_err(|e| {
            ServiceError::internal_server_error("Database connection failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    let updated_ret = RetNfag::update_by_id_and_tenant(ret_id, tenant_id.as_str(), update_request.into(), &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to update Ret NFAg")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Ok().json(updated_ret))
}

#[utoipa::path(
    delete,
    path = "/api/ret-nfag/{id}",
    params(
        ("id" = i32, Path, description = "RetNfag id")
    ),
    responses(
        (status = 200, description = "RetNfag deleted"),
        (status = 404, description = "Not found")
    ),
    tag = "ret-nfag"
)]
pub async fn delete_ret_nfag(
    path: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let ret_id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = web::block(move || pool.get())
        .await
        .map_err(|e| {
            ServiceError::internal_server_error("Thread pool error")
                .with_detail(e.to_string())
                .with_tag("threading")
        })?
        .map_err(|e| {
            ServiceError::internal_server_error("Database connection failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    let deleted_count = RetNfag::delete_by_id_and_tenant(ret_id, tenant_id.as_str(), &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to delete Ret NFAg")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    if deleted_count == 0 {
        return Err(ServiceError::not_found("Ret NFAg not found")
            .with_detail(format!("No record with id {} for tenant", ret_id))
            .with_tag("not_found"));
    }

    Ok(HttpResponse::Ok().json(json!({
        "message": "Ret NFAg deleted successfully",
        "deleted_count": deleted_count
    })))
}