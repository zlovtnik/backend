use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result};
use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;

use crate::{config::db::Pool, error::ServiceError, types::TenantId};

/// Generic CRUD context extracted from request
pub struct CrudContext {
    pub pool: Pool,
    pub tenant_id: String,
}

impl CrudContext {
    /// Extract context from request in a single operation
    pub fn from_request(req: &HttpRequest) -> Result<Self, ServiceError> {
        let pool = req.extensions().get::<Pool>().cloned().ok_or_else(|| {
            ServiceError::internal_server_error("Pool not found")
                .with_detail("Missing tenant pool in request extensions")
                .with_tag("tenant")
        })?;

        let tenant_id = req
            .extensions()
            .get::<TenantId>()
            .map(|t| t.0.clone())
            .ok_or_else(|| {
                ServiceError::unauthorized("Tenant not found")
                    .with_detail("Missing tenant ID in request extensions")
                    .with_tag("tenant")
            })?;

        Ok(CrudContext { pool, tenant_id })
    }

    /// Get database connection
    pub fn conn(
        &self,
    ) -> Result<
        diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
        ServiceError,
    > {
        self.pool.get().map_err(|e| {
            ServiceError::internal_server_error("Database connection failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })
    }
}

/// Generic CRUD operations trait
pub trait CrudOperations: Sized + Serialize {
    type CreateDto: for<'de> Deserialize<'de> + Validate;
    type UpdateDto: for<'de> Deserialize<'de> + Validate;

    fn create(
        dto: Self::CreateDto,
        tenant_id: &str,
        conn: &mut crate::config::db::Connection,
    ) -> Result<Self, DieselError>;
    fn find_all(
        tenant_id: &str,
        limit: i64,
        offset: i64,
        conn: &mut crate::config::db::Connection,
    ) -> Result<Vec<Self>, DieselError>;
    fn count(tenant_id: &str, conn: &mut crate::config::db::Connection)
        -> Result<i64, DieselError>;
    fn find_by_id(
        id: i32,
        tenant_id: &str,
        conn: &mut crate::config::db::Connection,
    ) -> Result<Self, DieselError>;
    fn update(
        id: i32,
        dto: Self::UpdateDto,
        tenant_id: &str,
        conn: &mut crate::config::db::Connection,
    ) -> Result<Self, DieselError>;
    fn delete(
        id: i32,
        tenant_id: &str,
        conn: &mut crate::config::db::Connection,
    ) -> Result<(), DieselError>;
}

/// Generic CRUD handler builder
pub struct CrudHandler<T: CrudOperations> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: CrudOperations> CrudHandler<T> {
    /// Generic create handler
    pub async fn create(
        req: HttpRequest,
        dto: web::Json<T::CreateDto>,
    ) -> Result<HttpResponse, ServiceError> {
        dto.validate().map_err(|e| {
            let error_messages: Vec<String> = e
                .field_errors()
                .iter()
                .map(|(field, errors)| {
                    let messages: Vec<String> = errors
                        .iter()
                        .filter_map(|err| err.message.as_ref().map(|m| m.to_string()))
                        .collect();
                    format!("{}: {}", field, messages.join(", "))
                })
                .collect();
            ServiceError::bad_request("Validation failed")
                .with_detail(error_messages.join("; "))
                .with_tag("validation")
        })?;

        let ctx = CrudContext::from_request(&req)?;
        let mut conn = ctx.conn()?;

        let entity = T::create(dto.into_inner(), &ctx.tenant_id, &mut conn).map_err(|e| {
            ServiceError::internal_server_error("Create failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

        Ok(HttpResponse::Created().json(json!({
            "message": "Created successfully",
            "data": entity
        })))
    }

    /// Generic list handler
    pub async fn find_all(
        req: HttpRequest,
        query: web::Query<std::collections::HashMap<String, String>>,
    ) -> Result<HttpResponse, ServiceError> {
        let ctx = CrudContext::from_request(&req)?;
        let mut conn = ctx.conn()?;

        let limit = query
            .get("limit")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(50)
            .clamp(1, 1000);
        let offset = query
            .get("offset")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0)
            .max(0);

        let entities = T::find_all(&ctx.tenant_id, limit, offset, &mut conn).map_err(|e| {
            ServiceError::internal_server_error("Find all failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

        let total = T::count(&ctx.tenant_id, &mut conn).map_err(|e| {
            ServiceError::internal_server_error("Count failed")
                .with_detail(e.to_string())
                .with_tag("database")
        })?;

        Ok(HttpResponse::Ok().json(json!({
            "message": "Records retrieved successfully",
            "data": entities,
            "pagination": {
                "limit": limit,
                "offset": offset,
                "total": total,
                "count": entities.len()
            }
        })))
    }

    /// Generic find by ID handler
    pub async fn find_by_id(
        req: HttpRequest,
        path: web::Path<i32>,
    ) -> Result<HttpResponse, ServiceError> {
        let id = path.into_inner();
        let ctx = CrudContext::from_request(&req)?;
        let mut conn = ctx.conn()?;

        let entity = T::find_by_id(id, &ctx.tenant_id, &mut conn).map_err(|e| match e {
            DieselError::NotFound => ServiceError::not_found("Record not found"),
            other => ServiceError::internal_server_error("Find failed")
                .with_detail(other.to_string())
                .with_tag("database"),
        })?;

        Ok(HttpResponse::Ok().json(json!({
            "message": "Record retrieved successfully",
            "data": entity
        })))
    }

    /// Generic update handler
    pub async fn update(
        req: HttpRequest,
        path: web::Path<i32>,
        dto: web::Json<T::UpdateDto>,
    ) -> Result<HttpResponse, ServiceError> {
        dto.validate().map_err(|e| {
            let error_messages: Vec<String> = e
                .field_errors()
                .iter()
                .map(|(field, errors)| {
                    let messages: Vec<String> = errors
                        .iter()
                        .filter_map(|err| err.message.as_ref().map(|m| m.to_string()))
                        .collect();
                    format!("{}: {}", field, messages.join(", "))
                })
                .collect();
            ServiceError::bad_request("Validation failed")
                .with_detail(error_messages.join("; "))
                .with_tag("validation")
        })?;

        let id = path.into_inner();
        let ctx = CrudContext::from_request(&req)?;
        let mut conn = ctx.conn()?;

        let entity =
            T::update(id, dto.into_inner(), &ctx.tenant_id, &mut conn).map_err(|e| match e {
                DieselError::NotFound => ServiceError::not_found("Record not found"),
                other => ServiceError::internal_server_error("Update failed")
                    .with_detail(other.to_string())
                    .with_tag("database"),
            })?;

        Ok(HttpResponse::Ok().json(json!({
            "message": "Updated successfully",
            "data": entity
        })))
    }

    /// Generic delete handler
    pub async fn delete(
        req: HttpRequest,
        path: web::Path<i32>,
    ) -> Result<HttpResponse, ServiceError> {
        let id = path.into_inner();
        let ctx = CrudContext::from_request(&req)?;
        let mut conn = ctx.conn()?;

        T::delete(id, &ctx.tenant_id, &mut conn).map_err(|e| match e {
            DieselError::NotFound => ServiceError::not_found("Record not found"),
            other => ServiceError::internal_server_error("Delete failed")
                .with_detail(other.to_string())
                .with_tag("database"),
        })?;

        Ok(HttpResponse::NoContent().finish())
    }
}
