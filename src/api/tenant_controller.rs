use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use log::info;
use serde::Serialize;
use std::collections::HashMap;

use crate::{
    config::db::{Pool as DatabasePool, TenantPoolManager},
    constants,
    error::ServiceError,
    models::filters::TenantFilter,
    models::response::ResponseBody,
    models::tenant::{Tenant, TenantDTO, UpdateTenant},
    models::user::operations as user_ops,
};

#[derive(Serialize)]
struct TenantStats {
    tenant_id: String,
    name: String,
    status: String,
}

#[derive(Serialize)]
struct SystemStats {
    total_tenants: i64,
    active_tenants: i32,
    total_users: i64,
    logged_in_users: i64,
    tenant_stats: Vec<TenantStats>,
}

#[derive(Serialize)]
struct TenantHealth {
    tenant_id: String,
    name: String,
    status: bool,
    error_message: Option<String>,
}

#[derive(Serialize)]
struct PaginatedTenantResponse {
    data: Vec<Tenant>,
    total: i64,
    offset: i64,
    limit: i64,
    count: i64,
    next_cursor: Option<i64>,
}

/// Collects system-wide metrics and per-tenant connection status.
///
/// Gathers totals for tenants and users and reports each tenant's connection state, returning an HTTP 200 response containing the serialized `SystemStats`.
///
/// # Examples
///
/// ```no_run
/// use actix_web::{web, HttpResponse};
/// // `pool` and `manager` would be provided by application setup
/// // let resp: Result<HttpResponse, _> = get_system_stats(web::Data::new(pool), web::Data::new(manager)).await;
/// ```
pub async fn get_system_stats(
    pool: web::Data<DatabasePool>,
    manager: web::Data<TenantPoolManager>,
) -> Result<HttpResponse, ServiceError> {
    info!("Fetching tenant statistics");

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "get_system_stats")
    })?;

    // Get total tenant count first without loading all tenants
    let total_tenants = Tenant::count_all(&mut conn).map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to count tenants: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "get_system_stats")
    })?;

    // Get total users and logged in count (global, since no tenant_id in users)
    let total_users = user_ops::count_all_users(&mut conn).map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to count users: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "get_system_stats")
    })?;
    let logged_in_users = user_ops::count_logged_in_users(&mut conn).map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to count logged in users: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "get_system_stats")
    })?;

    let mut tenant_stats = Vec::new();
    let mut active_count = 0;
    let page_size = 1000i64; // Process tenants in chunks of 1000
    let mut offset = 0i64;

    // Process tenants in paginated chunks to avoid memory issues
    loop {
        let (tenants, _) = Tenant::list_paginated(offset, page_size, &mut conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to fetch tenant page: {}", e))
                .with_tag("tenant")
                .with_metadata("operation", "get_system_stats")
        })?;

        if tenants.is_empty() {
            break; // No more tenants to process
        }

        for tenant in &tenants {
            // Check tenant health (database connection)
            let status = match manager.get_tenant_pool(&tenant.id) {
                Some(pool) => match pool.get() {
                    Ok(_) => {
                        active_count += 1;
                        true
                    }
                    Err(_) => false,
                },
                None => false,
            };

            // For per-tenant, since users don't have tenant_id, using global for simplicity
            tenant_stats.push(TenantStats {
                tenant_id: tenant.id.clone(),
                name: tenant.name.clone(),
                status: if status {
                    "active".to_string()
                } else {
                    "inactive".to_string()
                },
            });
        }

        offset += page_size;

        // Safety check to prevent infinite loops
        if offset >= total_tenants {
            break;
        }
    }

    let stats = SystemStats {
        total_tenants,
        active_tenants: active_count,
        total_users,
        logged_in_users,
        tenant_stats,
    };

    Ok(HttpResponse::Ok().json(stats))
}

/// Get detailed health status of all tenants (admin only)
pub async fn get_tenant_health(
    pool: web::Data<DatabasePool>,
    manager: web::Data<TenantPoolManager>,
) -> Result<HttpResponse, ServiceError> {
    info!("Fetching tenant health status");

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "get_tenant_health")
    })?;

    let mut tenant_health_status = Vec::new();
    let page_size = 1000i64; // Process tenants in chunks
    let mut offset = 0i64;

    // Process tenants in paginated chunks to avoid memory issues
    loop {
        let (tenants, _) = Tenant::list_paginated(offset, page_size, &mut conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to fetch tenant page: {}", e))
                .with_tag("tenant")
                .with_metadata("operation", "get_tenant_health")
        })?;

        if tenants.is_empty() {
            break; // No more tenants to process
        }

        for tenant in tenants {
            let (status, error_msg) = match manager.get_tenant_pool(&tenant.id) {
                Some(pool) => {
                    match pool.get() {
                        Ok(mut conn) => {
                            // Simple health check: SELECT 1
                            match diesel::sql_query("SELECT 1").execute(&mut conn) {
                                Ok(_) => (true, None),
                                Err(e) => (false, Some(format!("DB query failed: {}", e))),
                            }
                        }
                        Err(e) => (false, Some(format!("Pool connection failed: {}", e))),
                    }
                }
                None => (false, Some("No connection pool configured".to_string())),
            };

            tenant_health_status.push(TenantHealth {
                tenant_id: tenant.id,
                name: tenant.name,
                status,
                error_message: error_msg,
            });
        }

        offset += page_size;
    }

    Ok(HttpResponse::Ok().json(tenant_health_status))
}

/// Return a map of tenant IDs to their connection status.
///
/// Fetches all tenants and reports, for each tenant ID, whether a tenant-specific
/// connection pool is currently available and able to produce a connection. On
/// success this handler returns an HTTP 200 response with a JSON object where
/// keys are tenant IDs and values are booleans (`true` = connected, `false` = not connected).
///
/// # Examples
///
/// ```no_run
/// use actix_web::web;
/// # async fn example() {
/// let db_pool = web::Data::new(/* DatabasePool */ unimplemented!());
/// let manager = web::Data::new(/* TenantPoolManager */ unimplemented!());
/// let _ = get_tenant_status(db_pool, manager).await;
/// # }
/// ```
pub async fn get_tenant_status(
    pool: web::Data<DatabasePool>,
    manager: web::Data<TenantPoolManager>,
) -> Result<HttpResponse, ServiceError> {
    info!("Fetching tenant connection status");

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "get_tenant_status")
    })?;

    let mut status_map = HashMap::new();
    let page_size = 1000i64; // Process tenants in chunks
    let mut offset = 0i64;

    // Process tenants in paginated chunks to avoid memory issues
    loop {
        let (tenants, _) = Tenant::list_paginated(offset, page_size, &mut conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to fetch tenant page: {}", e))
                .with_tag("tenant")
                .with_metadata("operation", "get_tenant_status")
        })?;

        if tenants.is_empty() {
            break; // No more tenants to process
        }

        for tenant in tenants {
            let connected = match manager.get_tenant_pool(&tenant.id) {
                Some(pool) => pool.get().is_ok(),
                None => false,
            };
            status_map.insert(tenant.id.clone(), connected);
        }

        offset += page_size;
    }

    Ok(HttpResponse::Ok().json(status_map))
}

// CRUD operations for tenants

/// Retrieves a paginated list of tenants along with pagination metadata.
///
/// The JSON response body contains a `ResponseBody` wrapping a `PaginatedTenantResponse` with:
/// - `data`: list of tenants for the current page,
/// - `total`: total number of matching tenants,
/// - `offset` and `limit`: pagination parameters used,
/// - `count`: number of tenants returned in this page,
/// - `next_cursor`: optional next offset (`i64`) when more pages are available.
///
/// # Returns
///
/// `Ok(HttpResponse)` with a `ResponseBody` containing `PaginatedTenantResponse` on success, or
/// `Err(ServiceError::InternalServerError)` for database connection/query failures.
///
/// # Examples
///
/// ```rust,no_run
/// use std::collections::HashMap;
/// use actix_web::web;
///
/// // Build a query map with cursor and limit
/// let mut q = HashMap::new();
/// q.insert("cursor".to_string(), "0".to_string());
/// q.insert("limit".to_string(), "100".to_string());
/// let query = web::Query::from(q);
///
/// // Calling the handler requires an application DatabasePool; omitted here.
/// // The handler returns an `HttpResponse` containing a `PaginatedTenantResponse`.
/// ```
pub async fn find_all(
    query: web::Query<HashMap<String, String>>,
    pool: web::Data<DatabasePool>,
) -> Result<HttpResponse, ServiceError> {
    // Parse pagination parameters
    let offset = query
        .get("offset")
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| query.get("cursor").and_then(|s| s.parse::<i64>().ok()))
        .unwrap_or(0);

    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50);
    let limit = limit.min(500); // Max limit

    info!(
        "Fetching tenants with pagination - offset: {}, limit: {}",
        offset, limit
    );

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "find_all")
    })?;

    let (tenants, total) = Tenant::list_paginated(offset, limit, &mut conn).map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to fetch tenants: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "find_all")
            .with_metadata("offset", offset.to_string())
            .with_metadata("limit", limit.to_string())
    })?;

    let count = tenants.len();

    let response = PaginatedTenantResponse {
        data: tenants,
        total,
        offset,
        limit,
        count: count as i64,
        next_cursor: if offset + limit < total {
            Some(offset + limit)
        } else {
            None
        },
    };

    info!("Returning {} tenants out of {} total", count, total);

    Ok(HttpResponse::Ok().json(ResponseBody::new(constants::MESSAGE_OK, response)))
}

/// Parse query-encoded field filters and optional pagination and return matching tenants.
///
/// This handler accepts query parameters in two formats:
///
/// 1. Complex format for multiple/advanced filters:
///    - `filters[N][field]`, `filters[N][operator]`, `filters[N][value]` which are parsed into a `TenantFilter`
///    - `cursor` and `page_size` for pagination
///
/// 2. Simple format for basic filtering:
///    - Direct field parameters: `name`, `id`, `db_url` (uses "contains" for name/db_url, "equals" for id)
///    - `page_num` (treated as cursor, 0-based) and `page_size` for pagination
///
/// Values that fail numeric parsing are ignored (treated as absent).
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
///
/// // Complex format:
/// let mut q = HashMap::new();
/// q.insert("filters[0][field]".to_string(), "name".to_string());
/// q.insert("filters[0][operator]".to_string(), "eq".to_string());
/// q.insert("filters[0][value]".to_string(), "acme".to_string());
/// q.insert("page_size".to_string(), "25".to_string());
///
/// // Simple format:
/// let mut q2 = HashMap::new();
/// q2.insert("name".to_string(), "acme".to_string());
/// q2.insert("page_num".to_string(), "0".to_string());
/// q2.insert("page_size".to_string(), "10".to_string());
///
/// // Both produce a TenantFilter with appropriate filters and pagination.
/// ```
pub async fn filter(
    query: web::Query<std::collections::HashMap<String, String>>,
    pool: web::Data<DatabasePool>,
) -> Result<HttpResponse, ServiceError> {
    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "filter")
    })?;

    // Parse filters from query parameters
    let mut filters = Vec::new();
    let mut cursor = None;
    let mut page_size = None;

    for (key, value) in query.iter() {
        if key.starts_with("filters[") && key.ends_with("][field]") {
            // Extract index from filters[index][field]
            if let Some(index_str) = key
                .strip_prefix("filters[")
                .and_then(|s| s.strip_suffix("][field]"))
            {
                if let Ok(index) = index_str.parse::<usize>() {
                    // Get corresponding operator and value
                    let operator_key = format!("filters[{}][operator]", index);
                    let value_key = format!("filters[{}][value]", index);
                    if let (Some(operator), Some(field_val)) =
                        (query.get(&operator_key), query.get(&value_key))
                    {
                        filters.push(crate::models::filters::FieldFilter {
                            field: value.clone(),
                            operator: operator.clone(),
                            value: field_val.clone(),
                        });
                    }
                }
            }
        } else if key == "cursor" {
            cursor = value.parse().ok();
        } else if key == "page_num" {
            // Treat page_num as cursor (0-based)
            cursor = value.parse().ok();
        } else if key == "page_size" {
            page_size = value.parse().ok();
        } else if key == "name" {
            // Direct field parameter: name with contains operator
            filters.push(crate::models::filters::FieldFilter {
                field: "name".to_string(),
                operator: "contains".to_string(),
                value: value.clone(),
            });
        } else if key == "id" {
            // Direct field parameter: id with equals operator
            filters.push(crate::models::filters::FieldFilter {
                field: "id".to_string(),
                operator: "equals".to_string(),
                value: value.clone(),
            });
        } else if key == "db_url" {
            // Direct field parameter: db_url with contains operator
            filters.push(crate::models::filters::FieldFilter {
                field: "db_url".to_string(),
                operator: "contains".to_string(),
                value: value.clone(),
            });
        }
    }

    let filter = TenantFilter {
        filters,
        cursor,
        page_size,
    };

    let tenants = Tenant::filter(filter, &mut conn).map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to filter tenants: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "filter")
    })?;

    Ok(HttpResponse::Ok().json(tenants))
}

/// Fetches a tenant by ID and returns it wrapped in the standard `ResponseBody`.
///
/// On success returns HTTP 200 with a `ResponseBody` containing the tenant. If no tenant
/// matches the provided ID, the function returns `ServiceError::NotFound`. Other failures
/// produce `ServiceError::InternalServerError`.
///
/// # Examples
///
/// ```
/// // Example (handler-level): requesting tenant "tenant-123" should return 200 with that tenant.
/// // let resp = find_by_id(web::Path::from("tenant-123".to_string()), pool).await;
/// // assert_eq!(resp.status(), StatusCode::OK);
/// ```
pub async fn find_by_id(
    id: web::Path<String>,
    pool: web::Data<DatabasePool>,
) -> Result<HttpResponse, ServiceError> {
    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "find_by_id")
            .with_metadata("tenant_id", id.to_string())
    })?;

    let tenant = match Tenant::find_by_id(&id, &mut conn) {
        Ok(t) => t,
        Err(diesel::result::Error::NotFound) => {
            return Err(ServiceError::not_found(format!("Tenant not found: {}", id))
                .with_tag("tenant")
                .with_metadata("operation", "find_by_id")
                .with_metadata("tenant_id", id.to_string()))
        }
        Err(e) => {
            return Err(ServiceError::internal_server_error(format!(
                "Failed to find tenant: {}",
                e
            ))
            .with_tag("tenant")
            .with_metadata("operation", "find_by_id")
            .with_metadata("tenant_id", id.to_string()))
        }
    };

    Ok(HttpResponse::Ok().json(ResponseBody::new(constants::MESSAGE_OK, tenant)))
}

/// Creates a new tenant from the provided `TenantDTO`.
///
/// On success returns an HTTP 201 Created response containing a `ResponseBody` with the created `Tenant`.
///
/// # Errors
///
/// Returns `ServiceError::BadRequest` if input validation fails.
/// Returns `ServiceError::Conflict` if a tenant with the same id or name already exists.
/// Returns `ServiceError::InternalServerError` for database connection or creation failures.
///
/// # Examples
///
/// ```
/// // Construct a TenantDTO and call the handler (framework wiring and runtime omitted).
/// let dto = TenantDTO { id: "tenant1".into(), name: "Tenant One".into(), /* ... */ };
/// // let resp = create(web::Json(dto), pool).await.unwrap();
/// // assert_eq!(resp.status(), 201);
/// ```
pub async fn create(
    tenant_dto: web::Json<TenantDTO>,
    pool: web::Data<DatabasePool>,
) -> Result<HttpResponse, ServiceError> {
    let mut dto = tenant_dto.into_inner();

    // Auto-generate tenant ID if not provided
    if dto.id.is_empty() {
        dto.id = crate::utils::generate_tenant_id();
    }

    // Validate input data format and required fields
    if let Err(validation_error) = Tenant::validate_tenant_dto(&dto) {
        return Err(ServiceError::bad_request(validation_error.to_string())
            .with_tag("tenant")
            .with_metadata("operation", "create"));
    }

    let tenant_name = dto.name.clone();
    let tenant_id = dto.id.clone();

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "create")
            .with_metadata("tenant_name", tenant_name.clone())
            .with_metadata("tenant_id", tenant_id.clone())
    })?;

    // Create the tenant, relying on DB unique constraints to prevent duplicates
    let tenant = match Tenant::create(dto, &mut conn) {
        Ok(t) => t,
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            info,
        )) => {
            return Err(ServiceError::conflict(format!(
                "Tenant unique constraint violated: {}",
                info.message()
            ))
            .with_tag("tenant")
            .with_metadata("operation", "create")
            .with_metadata("tenant_id", tenant_id.clone()))
        }
        Err(e) => {
            return Err(ServiceError::internal_server_error(format!(
                "Failed to create tenant: {}",
                e
            ))
            .with_tag("tenant")
            .with_metadata("operation", "create")
            .with_metadata("tenant_id", tenant_id.clone()))
        }
    };

    Ok(HttpResponse::Created().json(ResponseBody::new(constants::MESSAGE_OK, tenant)))
}

/// Updates an existing tenant identified by `id`.
///
/// Attempts to apply `update_dto` and returns the updated tenant wrapped in an HTTP 200 response.
/// Returns `ServiceError::NotFound` if the tenant does not exist; other failures map to
/// `ServiceError::InternalServerError`.
///
/// # Examples
///
/// ```no_run
/// use actix_web::web;
///
/// // In an async test or handler, prepare `id`, `update_dto`, and `pool` appropriately:
/// // let id = web::Path::from("tenant-id".to_string());
/// // let update = web::Json(UpdateTenant { /* fields */ });
/// // let pool = web::Data::new(database_pool);
/// // let resp = update(id, update, pool).await;
/// ```
pub async fn update(
    id: web::Path<String>,
    update_dto: web::Json<UpdateTenant>,
    pool: web::Data<DatabasePool>,
) -> Result<HttpResponse, ServiceError> {
    let dto = update_dto.into_inner();

    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "update")
            .with_metadata("tenant_id", id.to_string())
    })?;

    let tenant = match Tenant::update(&id, dto, &mut conn) {
        Ok(t) => t,
        Err(diesel::result::Error::NotFound) => {
            return Err(ServiceError::not_found(format!("Tenant not found: {}", id))
                .with_tag("tenant")
                .with_metadata("operation", "update")
                .with_metadata("tenant_id", id.to_string()))
        }
        Err(e) => {
            return Err(ServiceError::internal_server_error(format!(
                "Failed to update tenant: {}",
                e
            ))
            .with_tag("tenant")
            .with_metadata("operation", "update")
            .with_metadata("tenant_id", id.to_string()))
        }
    };

    Ok(HttpResponse::Ok().json(ResponseBody::new(constants::MESSAGE_OK, tenant)))
}

/// Delete a tenant by its identifier.
///
/// On success returns HTTP 200 with a standardized empty payload and message. Returns
/// `ServiceError::NotFound` if the tenant does not exist, or `ServiceError::InternalServerError`
/// for database or connection errors.
///
/// # Examples
///
/// ```no_run
/// // Called from an async context (e.g., an Actix handler or async test)
/// // let resp = delete(web::Path::from(String::from("tenant-id")), pool).await?;
/// // assert_eq!(resp.status(), http::StatusCode::OK);
/// ```
pub async fn delete(
    id: web::Path<String>,
    pool: web::Data<DatabasePool>,
) -> Result<HttpResponse, ServiceError> {
    let mut conn = pool.get().map_err(|e| {
        ServiceError::internal_server_error(format!("Failed to get db connection: {}", e))
            .with_tag("tenant")
            .with_metadata("operation", "delete")
            .with_metadata("tenant_id", id.to_string())
    })?;

    match Tenant::delete(&id, &mut conn) {
        Ok(_) => (),
        Err(diesel::result::Error::NotFound) => {
            return Err(ServiceError::not_found(format!("Tenant not found: {}", id))
                .with_tag("tenant")
                .with_metadata("operation", "delete")
                .with_metadata("tenant_id", id.to_string()))
        }
        Err(e) => {
            return Err(ServiceError::internal_server_error(format!(
                "Failed to delete tenant: {}",
                e
            ))
            .with_tag("tenant")
            .with_metadata("operation", "delete")
            .with_metadata("tenant_id", id.to_string()))
        }
    };

    Ok(HttpResponse::Ok().json(ResponseBody::new(constants::MESSAGE_OK, constants::EMPTY)))
}
