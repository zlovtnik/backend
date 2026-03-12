use actix_web::{web, HttpRequest, HttpResponse, Result};

use crate::{api::crud_engine::CrudHandler, error::ServiceError, models::nfag::Nfag};

// Type alias for the handler
type NfagHandler = CrudHandler<Nfag>;

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
    dto: web::Json<crate::models::nfag::CreateNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    NfagHandler::create(req, dto).await
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
    req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ServiceError> {
    NfagHandler::find_all(req, query).await
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
    req: HttpRequest,
    path: web::Path<i32>,
) -> Result<HttpResponse, ServiceError> {
    NfagHandler::find_by_id(req, path).await
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
    req: HttpRequest,
    path: web::Path<i32>,
    dto: web::Json<crate::models::nfag::UpdateNfagRequest>,
) -> Result<HttpResponse, ServiceError> {
    NfagHandler::update(req, path, dto).await
}

/// Delete NFAg by ID
#[utoipa::path(
    delete,
    path = "/api/nfag/{id}",
    params(
        ("id" = i32, Path, description = "NFAg id")
    ),
    responses(
        (status = 204, description = "No Content"),
        (status = 404, description = "Not found")
    ),
    tag = "nfag"
)]
pub async fn delete(req: HttpRequest, path: web::Path<i32>) -> Result<HttpResponse, ServiceError> {
    NfagHandler::delete(req, path).await
}
