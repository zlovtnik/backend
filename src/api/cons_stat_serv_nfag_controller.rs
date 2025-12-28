use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result};
use serde_json::json;
use validator::Validate;

use crate::{
    config::db::Pool,
    error::ServiceError,
    models::cons_stat_serv_nfag::{ConsStatServNfag, CreateConsStatServNfagRequest, NewConsStatServNfag, UpdateConsStatServNfag, UpdateConsStatServNfagRequest},
    types::TenantId,
};

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

/// Create a new ConsStatServNfag
#[utoipa::path(
    post,
    path = "/api/cons-stat-serv-nfag",
    request_body = crate::models::cons_stat_serv_nfag::CreateConsStatServNfagRequest,
    responses(
        (status = 201, description = "ConsStatServNfag created", body = crate::models::cons_stat_serv_nfag::ConsStatServNfag)
    ),
    tag = "cons-stat-serv-nfag"
)]
pub async fn create(
    req: HttpRequest,
    dto: web::Json<CreateConsStatServNfagRequest>,
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

    let new_cons_stat_serv_nfag = NewConsStatServNfag {
        tenant_id,
        tpamb: dto.tpamb,
        xserv: dto.xserv.clone(),
        versao: dto.versao.clone(),
        xml_request: dto.xml_request.clone(),
        xml_response: None,
        cstat: None,
        xmotivo: None,
    };

    let cons_stat_serv_nfag = ConsStatServNfag::create(new_cons_stat_serv_nfag, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to create ConsStatServNfag")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Created().json(json!({
        "message": "ConsStatServNfag created successfully",
        "data": cons_stat_serv_nfag
    })))
}

/// Get all ConsStatServNfag for the tenant
#[utoipa::path(
    get,
    path = "/api/cons-stat-serv-nfag",
    params(
        ("limit" = Option<i64>, Query, description = "Max results (default 50, max 1000)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses(
        (status = 200, description = "List ConsStatServNfag", body = [crate::models::cons_stat_serv_nfag::ConsStatServNfag])
    ),
    tag = "cons-stat-serv-nfag"
)]
pub async fn find_all(
    query: web::Query<std::collections::HashMap<String, String>>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|l| *l > 0)
        .map(|l| l.min(1000))
        .unwrap_or(50);
    let offset = query
        .get("offset")
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|o| *o >= 0)
        .unwrap_or(0);

    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error("Database connection failed")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let cons_stat_serv_nfags = ConsStatServNfag::find_all_by_tenant(&tenant_id, limit, offset, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to fetch ConsStatServNfag records")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let total = ConsStatServNfag::count_by_tenant(&tenant_id, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to count ConsStatServNfag records")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "ConsStatServNfag records retrieved successfully",
        "data": cons_stat_serv_nfags,
        "pagination": {
            "limit": limit,
            "offset": offset,
            "total": total,
            "count": cons_stat_serv_nfags.len()
        }
    })))
}

/// Get ConsStatServNfag by ID
#[utoipa::path(
    get,
    path = "/api/cons-stat-serv-nfag/{id}",
    params(
        ("id" = i32, Path, description = "ConsStatServNfag id")
    ),
    responses(
        (status = 200, description = "ConsStatServNfag by id", body = crate::models::cons_stat_serv_nfag::ConsStatServNfag),
        (status = 404, description = "Not found")
    ),
    tag = "cons-stat-serv-nfag"
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

    let cons_stat_serv_nfag = ConsStatServNfag::find_by_id_and_tenant(id, &tenant_id, &mut conn).map_err(|e| match e {
        diesel::result::Error::NotFound => {
            ServiceError::not_found("ConsStatServNfag not found")
                .with_detail(e.to_string())
                .with_tag("not_found")
        }
        _ => ServiceError::internal_server_error("Database error")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "ConsStatServNfag retrieved successfully",
        "data": cons_stat_serv_nfag
    })))
}

/// Update ConsStatServNfag by ID
#[utoipa::path(
    put,
    path = "/api/cons-stat-serv-nfag/{id}",
    request_body = crate::models::cons_stat_serv_nfag::UpdateConsStatServNfagRequest,
    params(
        ("id" = i32, Path, description = "ConsStatServNfag id")
    ),
    responses(
        (status = 200, description = "ConsStatServNfag updated", body = crate::models::cons_stat_serv_nfag::ConsStatServNfag),
        (status = 404, description = "Not found")
    ),
    tag = "cons-stat-serv-nfag"
)]
pub async fn update(
    path: web::Path<i32>,
    req: HttpRequest,
    dto: web::Json<UpdateConsStatServNfagRequest>,
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

    let update_dto = UpdateConsStatServNfag {
        xml_response: dto.xml_response.clone().map(|s| s.trim().to_string()),
        cstat: dto.cstat,
        xmotivo: dto.xmotivo.clone().map(|s| s.trim().to_string()),
        updated_at: Some(chrono::Utc::now()),
    };

    let cons_stat_serv_nfag = ConsStatServNfag::update_by_id_and_tenant(id, &tenant_id, update_dto, &mut conn).map_err(|e| match e {
        diesel::result::Error::NotFound => {
            ServiceError::not_found("ConsStatServNfag not found")
                .with_detail(e.to_string())
                .with_tag("database")
        }
        _ => ServiceError::internal_server_error("Failed to update ConsStatServNfag")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "ConsStatServNfag updated successfully",
        "data": cons_stat_serv_nfag
    })))
}

/// Delete ConsStatServNfag by ID
#[utoipa::path(
    delete,
    path = "/api/cons-stat-serv-nfag/{id}",
    params(
        ("id" = i32, Path, description = "ConsStatServNfag id")
    ),
    responses(
        (status = 200, description = "ConsStatServNfag deleted"),
        (status = 404, description = "Not found")
    ),
    tag = "cons-stat-serv-nfag"
)]
pub async fn delete(
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

    let deleted_count = ConsStatServNfag::delete_by_id_and_tenant(id, &tenant_id, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to delete ConsStatServNfag")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    if deleted_count == 0 {
        return Err(ServiceError::not_found("ConsStatServNfag not found")
            .with_tag("not_found"));
    }

    Ok(HttpResponse::Ok().json(json!({
        "message": "ConsStatServNfag deleted successfully"
    })))
}