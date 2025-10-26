//! Tenant Service - Functional Patterns for Tenant Operations
//!
//! Provides functional programming patterns for tenant management operations,
//! using QueryReader monads and composable validation.

use crate::{
    config::db::Pool,
    error::{ServiceError, ServiceResult},
    models::{
        tenant::{Tenant, TenantDTO, UpdateTenant},
        user::operations as user_ops,
    },
    services::functional_patterns::{self, QueryReader},
};
use diesel::result::Error as DieselError;
use serde::Serialize;

#[derive(Serialize)]
pub struct TenantStats {
    pub tenant_id: String,
    pub name: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct SystemStats {
    pub total_tenants: i64,
    pub active_tenants: i32,
    pub total_users: i64,
    pub logged_in_users: i64,
    pub tenant_stats: Vec<TenantStats>,
}

/// Build a QueryReader for fetching system statistics
pub fn system_stats_reader() -> QueryReader<SystemStats> {
    QueryReader::new(|conn| {
        // Get total tenant count
        let total_tenants = Tenant::count_all(conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to count tenants: {}", e))
                .with_tag("tenant")
        })?;

        // Get user counts
        let total_users = user_ops::count_all_users(conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to count users: {}", e))
                .with_tag("tenant")
        })?;

        let logged_in_users = user_ops::count_logged_in_users(conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to count logged in users: {}", e))
                .with_tag("tenant")
        })?;

        // Process tenants in chunks using functional iteration
        let mut tenant_stats = Vec::new();
        let mut active_count = 0;
        let page_size = 1000i64;
        let mut offset = 0i64;

        loop {
            let (tenants, _) = Tenant::list_paginated(offset, page_size, conn).map_err(|e| {
                ServiceError::internal_server_error(format!("Failed to fetch tenant page: {}", e))
                    .with_tag("tenant")
            })?;

            if tenants.is_empty() {
                break;
            }

            // Functional processing of tenant batch
            tenants.iter().for_each(|tenant| {
                // For now, assume all tenants are active (no active field in Tenant model)
                active_count += 1;
                let status = "active".to_string();

                tenant_stats.push(TenantStats {
                    tenant_id: tenant.id.clone(),
                    name: tenant.name.clone(),
                    status,
                });
            });

            offset += page_size;
        }

        Ok(SystemStats {
            total_tenants,
            active_tenants: active_count,
            total_users,
            logged_in_users,
            tenant_stats,
        })
    })
}

/// Build a QueryReader for listing tenants with pagination
pub fn list_tenants_reader(offset: i64, limit: i64) -> QueryReader<(Vec<Tenant>, i64)> {
    QueryReader::new(move |conn| {
        Tenant::list_paginated(offset, limit, conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to list tenants: {}", e))
                .with_tag("tenant")
        })
    })
}

/// Build a QueryReader for finding a tenant by ID
pub fn find_tenant_reader(tenant_id: String) -> QueryReader<Tenant> {
    QueryReader::new(move |conn| {
        Tenant::find_by_id(&tenant_id, conn).map_err(|e| match e {
            DieselError::NotFound => {
                ServiceError::not_found(format!("Tenant {} not found", tenant_id))
            }
            _ => ServiceError::internal_server_error(format!("Database error: {}", e))
                .with_tag("tenant"),
        })
    })
}

/// Build a QueryReader for creating a new tenant
pub fn create_tenant_reader(dto: TenantDTO) -> QueryReader<Tenant> {
    QueryReader::new(move |conn| {
        Tenant::create(dto.clone(), conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to create tenant: {}", e))
                .with_tag("tenant")
        })
    })
}

/// Build a QueryReader for updating a tenant
pub fn update_tenant_reader(tenant_id: String, dto: UpdateTenant) -> QueryReader<Tenant> {
    QueryReader::new(move |conn| {
        Tenant::update(&tenant_id, dto.clone(), conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to update tenant: {}", e))
                .with_tag("tenant")
        })
    })
}

/// Build a QueryReader for deleting a tenant
pub fn delete_tenant_reader(tenant_id: String) -> QueryReader<usize> {
    QueryReader::new(move |conn| {
        Tenant::delete(&tenant_id, conn).map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to delete tenant: {}", e))
                .with_tag("tenant")
        })
    })
}

/// Execute a QueryReader with a database pool (re-exported for convenience)
pub fn run_query<T>(reader: QueryReader<T>, pool: &Pool) -> ServiceResult<T> {
    functional_patterns::run_query(reader, pool)
}
