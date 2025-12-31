use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result};
use serde_json::json;
use validator::Validate;

use crate::{
    config::db::Pool,
    error::ServiceError,
    models::cons_sit_nfag::{
        ConsSitNfag, CreateConsSitNfagRequest, NewConsSitNfag, UpdateConsSitNfag,
        UpdateConsSitNfagRequest,
    },
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
fn extract_tenant_id(req: &HttpRequest) -> Result<TenantId, ServiceError> {
    req.extensions().get::<TenantId>().cloned().ok_or_else(|| {
        ServiceError::unauthorized("Tenant not found")
            .with_detail("Missing tenant ID in request extensions")
            .with_tag("tenant")
    })
}

/// Create a new ConsSitNfag
#[utoipa::path(
    post,
    path = "/api/cons-sit-nfag",
    request_body = crate::models::cons_sit_nfag::CreateConsSitNfagRequest,
    responses(
        (status = 201, description = "ConsSitNfag created", body = crate::models::cons_sit_nfag::ConsSitNfag)
    ),
    tag = "cons-sit-nfag"
)]
pub async fn create(
    req: HttpRequest,
    dto: web::Json<CreateConsSitNfagRequest>,
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

    let new_cons_sit_nfag = NewConsSitNfag {
        tenant_id: tenant_id.into_inner(),
        tpamb: dto.tpamb,
        xserv: dto.xserv.clone(),
        chnfag: dto.chnfag.clone(),
        versao: dto.versao.clone(),
        xml_request: dto.xml_request.clone(),
        xml_response: None,
        cstat: None,
        xmotivo: None,
    };

    let cons_sit_nfag = ConsSitNfag::create(new_cons_sit_nfag, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to create ConsSitNfag")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Created().json(json!({
        "message": "ConsSitNfag created successfully",
        "data": cons_sit_nfag
    })))
}

/// Get all ConsSitNfag for the tenant
#[utoipa::path(
    get,
    path = "/api/cons-sit-nfag",
    params(
        ("limit" = Option<i64>, Query, description = "Max results (default 50, max 1000)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses(
        (status = 200, description = "List ConsSitNfag", body = [crate::models::cons_sit_nfag::ConsSitNfag])
    ),
    tag = "cons-sit-nfag"
)]
pub async fn find_all(
    query: web::Query<std::collections::HashMap<String, String>>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
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

    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let cons_sit_nfags =
        ConsSitNfag::find_all_by_tenant(tenant_id.as_str(), limit, offset, &mut conn).map_err(
            |e| {
                ServiceError::internal_server_error("Failed to fetch ConsSitNfag records")
                    .with_detail(e.to_string())
                    .with_tag("database")
            },
        )?;

    let total = ConsSitNfag::count_by_tenant(tenant_id.as_str(), &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to count ConsSitNfag records")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "ConsSitNfag records retrieved successfully",
        "data": cons_sit_nfags,
        "pagination": {
            "limit": limit,
            "offset": offset,
            "total": total,
            "count": cons_sit_nfags.len()
        }
    })))
}

/// Get ConsSitNfag by ID
#[utoipa::path(
    get,
    path = "/api/cons-sit-nfag/{id}",
    params(
        ("id" = i32, Path, description = "ConsSitNfag id")
    ),
    responses(
        (status = 200, description = "ConsSitNfag by id", body = crate::models::cons_sit_nfag::ConsSitNfag),
        (status = 404, description = "Not found")
    ),
    tag = "cons-sit-nfag"
)]
pub async fn find_by_id(
    path: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let cons_sit_nfag = ConsSitNfag::find_by_id_and_tenant(id, tenant_id.as_str(), &mut conn)
        .map_err(|e| {
            ServiceError::not_found("ConsSitNfag not found")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "ConsSitNfag retrieved successfully",
        "data": cons_sit_nfag
    })))
}

/// Update ConsSitNfag by ID
#[utoipa::path(
    put,
    path = "/api/cons-sit-nfag/{id}",
    request_body = crate::models::cons_sit_nfag::UpdateConsSitNfagRequest,
    params(
        ("id" = i32, Path, description = "ConsSitNfag id")
    ),
    responses(
        (status = 200, description = "ConsSitNfag updated", body = crate::models::cons_sit_nfag::ConsSitNfag),
        (status = 404, description = "Not found")
    ),
    tag = "cons-sit-nfag"
)]
pub async fn update(
    path: web::Path<i32>,
    req: HttpRequest,
    dto: web::Json<UpdateConsSitNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    dto.validate().map_err(|e| {
        ServiceError::bad_request("Validation failed")
            .with_detail(format!("{:?}", e))
            .with_tag("validation")
    })?;

    let id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let update_dto: UpdateConsSitNfag = dto.into_inner().into();

    let cons_sit_nfag =
        ConsSitNfag::update_by_id_and_tenant(id, tenant_id.as_str(), update_dto, &mut conn)
            .map_err(|e| {
                ServiceError::not_found("ConsSitNfag not found or update failed")
                    .with_detail(e.to_string())
                    .with_tag("database")
            })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "ConsSitNfag updated successfully",
        "data": cons_sit_nfag
    })))
}

/// Delete ConsSitNfag by ID
#[utoipa::path(
    delete,
    path = "/api/cons-sit-nfag/{id}",
    params(
        ("id" = i32, Path, description = "ConsSitNfag id")
    ),
    responses(
        (status = 200, description = "ConsSitNfag deleted"),
        (status = 404, description = "Not found")
    ),
    tag = "cons-sit-nfag"
)]
pub async fn delete(path: web::Path<i32>, req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let deleted_count = ConsSitNfag::delete_by_id_and_tenant(id, tenant_id.as_str(), &mut conn)
        .map_err(|e| {
            ServiceError::internal_server_error("Failed to delete ConsSitNfag")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    if deleted_count == 0 {
        return Err(ServiceError::not_found("ConsSitNfag not found").with_tag("not_found"));
    }

    Ok(HttpResponse::Ok().json(json!({
        "message": "ConsSitNfag deleted successfully"
    })))
}
