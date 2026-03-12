use crate::{
    config::{
        db::{Pool as PgPool, TenantPoolManager as PgTenantPoolManager},
        hybrid_db::DbConnection,
        oracle_db::OracleTenantPoolManager,
    },
    error::ServiceError,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Cached tenant database type with expiry
#[derive(Clone, Debug)]
struct CachedTenantDbType {
    db_type: TenantDbType,
    expires_at: Instant,
}

/// Hybrid database manager that routes between PostgreSQL (admin/NFe) and Oracle (tenant data)
#[derive(Clone)]
pub struct HybridDatabaseManager {
    /// PostgreSQL for admin operations, users, tenants table, and NFe schema
    pub pg_admin_pool: PgPool,

    /// PostgreSQL tenant pools (if some tenants still use PG)
    pub pg_tenant_manager: PgTenantPoolManager,

    /// Oracle tenant pools (for production tenant data)
    pub oracle_tenant_manager: OracleTenantPoolManager,

    /// TTL cache for tenant database types (reduces admin DB queries)
    tenant_db_type_cache: Arc<RwLock<HashMap<String, CachedTenantDbType>>>,

    /// Cache TTL duration (default 5 minutes)
    cache_ttl: Duration,
}

/// Tenant database type configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantDbType {
    /// Tenant uses PostgreSQL
    Postgres,
    /// Tenant uses Oracle
    Oracle,
}

impl HybridDatabaseManager {
    /// Create a new hybrid database manager with default 5-minute cache TTL
    pub fn new(pg_admin_pool: PgPool) -> Self {
        Self::with_cache_ttl(pg_admin_pool, Duration::from_secs(300))
    }

    /// Create a new hybrid database manager with custom cache TTL
    pub fn with_cache_ttl(pg_admin_pool: PgPool, cache_ttl: Duration) -> Self {
        let pg_tenant_manager = PgTenantPoolManager::new(pg_admin_pool.clone());
        let oracle_tenant_manager = OracleTenantPoolManager::new();

        HybridDatabaseManager {
            pg_admin_pool,
            pg_tenant_manager,
            oracle_tenant_manager,
            tenant_db_type_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
        }
    }

    /// Get admin pool (always PostgreSQL) for users, tenants, login_history
    pub fn get_admin_pool(&self) -> PgPool {
        self.pg_admin_pool.clone()
    }

    /// Get tenant database connection (routes to appropriate database)
    pub fn get_tenant_connection(&self, tenant_id: &str) -> Result<DbConnection, ServiceError> {
        // Check which database type this tenant uses
        let db_type = self.get_tenant_db_type(tenant_id)?;

        match db_type {
            TenantDbType::Postgres => {
                // Use PostgreSQL tenant pool
                let pool = self
                    .pg_tenant_manager
                    .get_tenant_pool(tenant_id)
                    .or_else(|| {
                        // Try to create pool from tenants table
                        match self
                            .pg_tenant_manager
                            .get_or_create_pool_functional(tenant_id)
                        {
                            crate::services::functional_patterns::Either::Right(pool) => Some(pool),
                            crate::services::functional_patterns::Either::Left(err) => {
                                log::error!("Failed to create PG tenant pool: {}", err);
                                None
                            }
                        }
                    })
                    .ok_or_else(|| {
                        ServiceError::not_found(format!(
                            "PostgreSQL pool not found for tenant: {}",
                            tenant_id
                        ))
                    })?;

                let conn = pool.get().map_err(|e| {
                    ServiceError::internal_server_error(format!(
                        "Failed to get PG connection: {}",
                        e
                    ))
                })?;

                Ok(DbConnection::Postgres(Box::new(conn)))
            }
            TenantDbType::Oracle => {
                // Use Oracle tenant pool
                let conn = self
                    .oracle_tenant_manager
                    .get_or_create_pool(tenant_id, &self.pg_admin_pool)
                    .and_then(|pool| pool.get())?;

                Ok(DbConnection::Oracle(conn))
            }
        }
    }

    /// Determine which database type a tenant uses by parsing the db_url (with caching)
    fn get_tenant_db_type(&self, tenant_id: &str) -> Result<TenantDbType, ServiceError> {
        let now = Instant::now();

        // Check cache first
        if let Ok(cache) = self.tenant_db_type_cache.read() {
            if let Some(cached) = cache.get(tenant_id) {
                if cached.expires_at > now {
                    return Ok(cached.db_type);
                }
            }
        }

        // Cache miss or expired - query database
        use crate::models::tenant::Tenant;
        use crate::schema::tenants::dsl::*;
        use diesel::prelude::*;

        let mut conn = self.pg_admin_pool.get().map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to get admin connection: {}", e))
        })?;

        let tenant = tenants
            .filter(id.eq(tenant_id))
            .first::<Tenant>(&mut conn)
            .map_err(|e| ServiceError::not_found(format!("Tenant not found: {}", e)))?;

        let db_type = Self::parse_database_type(&tenant.db_url)?;

        // Update cache
        if let Ok(mut cache) = self.tenant_db_type_cache.write() {
            cache.insert(
                tenant_id.to_string(),
                CachedTenantDbType {
                    db_type,
                    expires_at: now + self.cache_ttl,
                },
            );
        }

        Ok(db_type)
    }

    /// Parse database type from connection URL with explicit validation
    fn parse_database_type(db_url: &str) -> Result<TenantDbType, ServiceError> {
        // Try to parse as URL with scheme
        if let Ok(parsed_url) = url::Url::parse(db_url) {
            match parsed_url.scheme() {
                "postgres" | "postgresql" | "pg" => return Ok(TenantDbType::Postgres),
                "oracle" => return Ok(TenantDbType::Oracle),
                "mysql" | "mariadb" => {
                    return Err(ServiceError::bad_request(format!(
                        "Unsupported database type: {}. Only PostgreSQL and Oracle are supported.",
                        parsed_url.scheme()
                    )));
                }
                scheme => {
                    return Err(ServiceError::bad_request(format!(
                        "Unknown database scheme: {}. Expected postgres:// or oracle://",
                        scheme
                    )));
                }
            }
        }

        // Handle scheme-less URLs by checking common patterns
        let url_lower = db_url.to_lowercase();

        // PostgreSQL patterns without scheme
        if url_lower.contains("@localhost")
            || url_lower.contains("@127.0.0.1")
            || url_lower.contains("@::1")
        {
            // Likely PostgreSQL connection string format: user:pass@host/db
            if url_lower.contains("/") && !url_lower.contains(":") {
                return Ok(TenantDbType::Postgres);
            }
        }

        // Oracle patterns: typically hostname:port/service_name or user/pass@host:port/service
        if db_url.contains(":") && db_url.contains("/") {
            // Check if it looks like Oracle format (has port number)
            if db_url.matches(':').count() == 1 {
                let parts: Vec<&str> = db_url.split(':').collect();
                if parts.len() == 2 && parts[1].chars().next().map_or(false, |c| c.is_numeric()) {
                    return Ok(TenantDbType::Oracle);
                }
            }
        }

        // If we can't determine, return error rather than assuming
        Err(ServiceError::bad_request(
            format!(
                "Cannot determine database type from URL: '{}'. Use explicit scheme (postgres://, oracle://) or valid connection string format.",
                db_url
            )
        ))
    }

    /// Add a PostgreSQL tenant pool explicitly
    pub fn add_pg_tenant_pool(&self, tenant_id: String, pool: PgPool) -> Result<(), ServiceError> {
        self.pg_tenant_manager.add_tenant_pool(tenant_id, pool)
    }

    /// Add an Oracle tenant pool explicitly
    pub fn add_oracle_tenant_pool(
        &self,
        tenant_id: String,
        connection_string: String,
    ) -> Result<(), ServiceError> {
        self.oracle_tenant_manager
            .add_tenant_pool(tenant_id, connection_string)
    }

    /// Health check for all databases
    pub fn health_check_all(&self) -> HybridHealthStatus {
        // Check PostgreSQL admin pool
        let pg_admin_healthy = self.pg_admin_pool.get().is_ok();

        // Check all PG tenant pools
        let pg_tenant_health = self
            .pg_tenant_manager
            .list_all_tenant_pools()
            .unwrap_or_default()
            .into_iter()
            .map(|tenant_id| {
                let healthy = self
                    .pg_tenant_manager
                    .get_tenant_pool(&tenant_id)
                    .and_then(|pool| pool.get().ok())
                    .is_some();
                (tenant_id, healthy)
            })
            .collect();

        // Check all Oracle tenant pools
        let oracle_tenant_health = self.oracle_tenant_manager.health_check();

        HybridHealthStatus {
            pg_admin_healthy,
            pg_tenant_health,
            oracle_tenant_health,
        }
    }
}

/// Combined health status for hybrid database setup
#[derive(Debug, serde::Serialize)]
pub struct HybridHealthStatus {
    pub pg_admin_healthy: bool,
    pub pg_tenant_health: Vec<(String, bool)>,
    pub oracle_tenant_health: Vec<crate::config::oracle_db::OraclePoolHealthStatus>,
}
