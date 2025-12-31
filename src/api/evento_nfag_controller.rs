use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result};
use serde_json::json;
use validator::Validate;

use crate::{
    config::db::Pool,
    error::ServiceError,
    models::evento_nfag::{EventoNfag, NewEventoNfag},
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

#[utoipa::path(
    post,
    path = "/api/evento-nfag",
    request_body = crate::models::evento_nfag::CreateEventoNfagRequest,
    responses(
        (status = 201, description = "EventoNfag created", body = crate::models::evento_nfag::EventoNfag)
    ),
    tag = "evento-nfag"
)]
pub async fn create_evento_nfag(
    req: HttpRequest,
    req_body: web::Json<crate::models::evento_nfag::CreateEventoNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    req_body.validate().map_err(|e| {
        ServiceError::bad_request("Validation failed")
            .with_detail(format!("{:?}", e))
            .with_tag("validation")
    })?;

    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;
    let new_evento_request = req_body.into_inner();

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

    let new_evento: NewEventoNfag = (new_evento_request, tenant_id).into();

    let created_evento = EventoNfag::create(new_evento, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to create Evento NFAg")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    Ok(HttpResponse::Created().json(created_evento))
}

#[utoipa::path(
    get,
    path = "/api/evento-nfag",
    params(
        ("limit" = Option<i64>, Query, description = "Max results (default 50)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses(
        (status = 200, description = "List EventoNfag", body = [crate::models::evento_nfag::EventoNfag])
    ),
    tag = "evento-nfag"
)]
pub async fn find_all_eventos_nfag(
    query: web::Query<std::collections::HashMap<String, String>>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let pool = extract_pool(&req)?;
    let tenant_id = extract_tenant_id(&req)?;

    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50);
    let offset = query
        .get("offset")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

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

    let eventos =
        EventoNfag::find_all_by_tenant(&tenant_id, limit, offset, &mut conn).map_err(|e| {
            ServiceError::internal_server_error("Failed to fetch Evento NFAg records")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    let total = EventoNfag::count_by_tenant(&tenant_id, &mut conn).map_err(|e| {
        ServiceError::internal_server_error("Failed to count Evento NFAg records")
            .with_detail(e.to_string())
            .with_tag("database")
    })?;

    let response = json!({
        "data": eventos,
        "total": total,
        "limit": limit,
        "offset": offset
    });

    Ok(HttpResponse::Ok().json(response))
}

#[utoipa::path(
    get,
    path = "/api/evento-nfag/{id}",
    params(
        ("id" = i32, Path, description = "EventoNfag id")
    ),
    responses(
        (status = 200, description = "EventoNfag by id", body = crate::models::evento_nfag::EventoNfag),
        (status = 404, description = "Not found")
    ),
    tag = "evento-nfag"
)]
pub async fn find_evento_nfag_by_id(
    path: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let evento_id = path.into_inner();
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

    let evento = EventoNfag::find_by_id_and_tenant(evento_id, &tenant_id, &mut conn).map_err(
        |e| match e {
            diesel::result::Error::NotFound => ServiceError::not_found("Evento NFAg not found")
                .with_detail(format!(
                    "No Evento NFAg with id {} found for tenant",
                    evento_id
                ))
                .with_tag("database"),
            _ => ServiceError::internal_server_error("Failed to find Evento NFAg")
                .with_detail(e.to_string())
                .with_tag("database"),
        },
    )?;
    Ok(HttpResponse::Ok().json(evento))
}

#[utoipa::path(
    put,
    path = "/api/evento-nfag/{id}",
    request_body = crate::models::evento_nfag::UpdateEventoNfagRequest,
    params(
        ("id" = i32, Path, description = "EventoNfag id")
    ),
    responses(
        (status = 200, description = "EventoNfag updated", body = crate::models::evento_nfag::EventoNfag),
        (status = 404, description = "Not found")
    ),
    tag = "evento-nfag"
)]
pub async fn update_evento_nfag(
    path: web::Path<i32>,
    req: HttpRequest,
    req_body: web::Json<crate::models::evento_nfag::UpdateEventoNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    req_body.validate().map_err(|e| {
        ServiceError::bad_request("Validation failed")
            .with_detail(format!("{:?}", e))
            .with_tag("validation")
    })?;

    let evento_id = path.into_inner();
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

    let updated_evento = EventoNfag::update_by_id_and_tenant(
        evento_id,
        &tenant_id,
        update_request.into(),
        &mut conn,
    )
    .map_err(|e| match e {
        diesel::result::Error::NotFound => ServiceError::not_found("Evento NFAg not found")
            .with_detail(format!(
                "No Evento NFAg with id {} found for tenant to update",
                evento_id
            ))
            .with_tag("database"),
        _ => ServiceError::internal_server_error("Failed to update Evento NFAg")
            .with_detail(e.to_string())
            .with_tag("database"),
    })?;

    Ok(HttpResponse::Ok().json(updated_evento))
}

#[utoipa::path(
    delete,
    path = "/api/evento-nfag/{id}",
    params(
        ("id" = i32, Path, description = "EventoNfag id")
    ),
    responses(
        (status = 200, description = "EventoNfag deleted"),
        (status = 404, description = "Not found")
    ),
    tag = "evento-nfag"
)]
pub async fn delete_evento_nfag(
    path: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let evento_id = path.into_inner();
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

    let deleted_count = EventoNfag::delete_by_id_and_tenant(evento_id, &tenant_id, &mut conn)
        .map_err(|e| {
            ServiceError::internal_server_error("Failed to delete Evento NFAg")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

    if deleted_count == 0 {
        return Err(ServiceError::not_found("Evento NFAg not found").with_tag("not_found"));
    }

    Ok(HttpResponse::Ok().json(json!({
        "message": "Evento NFAg deleted successfully",
        "deleted_count": deleted_count
    })))
}
