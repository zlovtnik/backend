use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result};
use diesel::result::Error as DieselError;
use serde_json::json;
use validator::Validate;

use crate::{
    config::db::Pool,
    error::ServiceError,
    models::nfag::{CreateNfagRequest, NewNfag, Nfag, UpdateNfag, UpdateNfagRequest},
    types::TenantId,
};

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
fn extract_tenant_id(req: &HttpRequest) -> Result<String, ServiceError> {
    req.extensions()
        .get::<TenantId>()
        .map(|t| t.0.clone())
        .ok_or_else(|| {
            ServiceError::unauthorized("Tenant not found")
                .with_detail("Missing tenant ID in request extensions")
                .with_tag("tenant")
        })
}

/// Create a new NFAg
#[utoipa::path(
    post,
    path = "/api/nfag",
    request_body = crate::models::nfag::CreateNfagRequest,
    responses(
        (status = 201, description = "NFAg created", body = crate::models::nfag::Nfag)
    ),
    tag = "nfag"
)]
pub async fn create(
    req: HttpRequest,
    dto: web::Json<CreateNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    dto.validate().map_err(|e| {
        ServiceError::bad_request("Validation failed")
            .with_detail(format!("{:?}", e))
            .with_tag("validation")
    })?;

    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let new_nfag = NewNfag {
        chave: dto.chave.clone(),
        tenant_id,
        versao: dto.versao.clone(),
        xml_content: dto.xml_content.clone(),
        status: dto.status.clone(),
    };

    let nfag = Nfag::create(new_nfag, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to create NFAg")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Created().json(json!({
        "message": "NFAg created successfully",
        "data": nfag
    })))
}

/// Get all NFAg for the tenant
#[utoipa::path(
    get,
    path = "/api/nfag",
    params(
        ("limit" = Option<i64>, Query, description = "Max results (default 50, max 1000)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses(
        (status = 200, description = "List NFAg", body = [crate::models::nfag::Nfag])
    ),
    tag = "nfag"
)]
pub async fn find_all(
    query: web::Query<std::collections::HashMap<String, String>>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50)
        .clamp(1, MAX_LIMIT);
    let offset = query
        .get("offset")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let nfags = Nfag::find_all_by_tenant(&tenant_id, limit, offset, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to fetch NFAg records")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let total = Nfag::count_by_tenant(&tenant_id, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to count NFAg records")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "NFAg records retrieved successfully",
        "data": nfags,
        "pagination": {
            "limit": limit,
            "offset": offset,
            "total": total,
            "count": nfags.len()
        }
    })))
}

/// Get NFAg by ID
#[utoipa::path(
    get,
    path = "/api/nfag/{id}",
    params(
        ("id" = i32, Path, description = "NFAg id")
    ),
    responses(
        (status = 200, description = "NFAg by id", body = crate::models::nfag::Nfag),
        (status = 404, description = "Not found")
    ),
    tag = "nfag"
)]
pub async fn find_by_id(
    path: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let nfag_id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let nfag =
        Nfag::find_by_id_and_tenant(nfag_id, &tenant_id, &mut conn).map_err(|e| match e {
            DieselError::NotFound => ServiceError::not_found("NFAg not found"),
            other => ServiceError::internal_server_error("NFAg fetch failed")
                .with_detail(other.to_string())
                .with_tag("database"),
        })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "NFAg retrieved successfully",
        "data": nfag
    })))
}

/// Update NFAg by ID
#[utoipa::path(
    put,
    path = "/api/nfag/{id}",
    request_body = crate::models::nfag::UpdateNfagRequest,
    params(
        ("id" = i32, Path, description = "NFAg id")
    ),
    responses(
        (status = 200, description = "NFAg updated", body = crate::models::nfag::Nfag),
        (status = 404, description = "Not found")
    ),
    tag = "nfag"
)]
pub async fn update(
    path: web::Path<i32>,
    req: HttpRequest,
    dto: web::Json<UpdateNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    dto.validate().map_err(|e| {
        ServiceError::bad_request("Validation failed")
            .with_detail(format!("{:?}", e))
            .with_tag("validation")
    })?;

    let nfag_id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let update_dto = UpdateNfag {
        versao: dto.versao.clone(),
        xml_content: dto.xml_content.clone(),
        status: dto.status.clone(),
        updated_at: Some(chrono::Utc::now()),
    };

    let nfag =
        Nfag::update_by_id_and_tenant(nfag_id, &tenant_id, update_dto, &mut conn).map_err(|e| {
            match e {
                DieselError::NotFound => ServiceError::not_found("NFAg not found"),
                other => ServiceError::internal_server_error("NFAg update failed")
                    .with_detail(other.to_string())
                    .with_tag("database"),
            }
        })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "NFAg updated successfully",
        "data": nfag
    })))
}

/// Delete NFAg by ID
#[utoipa::path(
    delete,
    path = "/api/nfag/{id}",
    params(
        ("id" = i32, Path, description = "NFAg id")
    ),
    responses(
        (status = 200, description = "NFAg deleted"),
        (status = 404, description = "Not found")
    ),
    tag = "nfag"
)]
pub async fn delete(path: web::Path<i32>, req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    let nfag_id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let deleted_count =
        Nfag::delete_by_id_and_tenant(nfag_id, &tenant_id, &mut conn).map_err(|e| {
            ServiceError::internal_server_error("Failed to delete NFAg")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    if deleted_count == 0 {
        return Err(ServiceError::not_found("NFAg not found").with_tag("not_found"));
    }

    Ok(HttpResponse::Ok().json(json!({
        "message": "NFAg deleted successfully"
    })))
}
