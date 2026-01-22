use crate::error::ServiceError;
use oracle::{Connection, Error as OracleError};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Oracle connection configuration parsed from connection string
#[derive(Clone, Debug)]
struct OracleConnectionConfig {
    connection_string: String,
    username: String,
    password: String,
}

impl OracleConnectionConfig {
    /// Parse Oracle connection string with embedded credentials
    /// Formats:
    /// - oracle://username:password@hostname:port/service_name
    /// - username/password@hostname:port/service_name  
    /// - hostname:port/service_name (credentials in env vars ORACLE_USER/ORACLE_PASSWORD)
    fn parse(url: &str) -> Result<Self, ServiceError> {
        // Try URL format with scheme
        if url.starts_with("oracle://") {
            let parsed = url::Url::parse(url).map_err(|e| {
                ServiceError::bad_request(format!("Invalid Oracle URL: {}", e))
            })?;
            
            let username = parsed.username().to_string();
            let password = parsed.password().unwrap_or("").to_string();
            let host = parsed.host_str().ok_or_else(|| {
                ServiceError::bad_request("Missing host in Oracle URL")
            })?;
            let port = parsed.port().unwrap_or(1521);
            let service_name = parsed.path().trim_start_matches('/');
            
            let connection_string = format!("{}:{}/{}", host, port, service_name);
            
            return Ok(OracleConnectionConfig {
                connection_string,
                username,
                password,
            });
        }
        
        // Try traditional format: username/password@host:port/service
        if let Some(at_pos) = url.find('@') {
            let (creds, conn_str) = url.split_at(at_pos);
            let connection_string = conn_str[1..].to_string(); // skip '@'
            
            if let Some(slash_pos) = creds.find('/') {
                let username = creds[..slash_pos].to_string();
                let password = creds[slash_pos + 1..].to_string();
                
                return Ok(OracleConnectionConfig {
                    connection_string,
                    username,
                    password,
                });
            }
        }
        
        // Fallback: hostname:port/service format, credentials from environment
        let username = std::env::var("ORACLE_USER").unwrap_or_else(|_| String::new());
        let password = std::env::var("ORACLE_PASSWORD").unwrap_or_else(|_| String::new());
        
        if username.is_empty() || password.is_empty() {
            return Err(ServiceError::bad_request(
                "Oracle credentials not found. Use oracle://user:pass@host:port/service or set ORACLE_USER/ORACLE_PASSWORD env vars"
            ));
        }
        
        Ok(OracleConnectionConfig {
            connection_string: url.to_string(),
            username,
            password,
        })
    }
}

/// RAII guard that automatically returns Oracle connection to pool on drop
pub struct OracleConnectionGuard {
    conn: Option<Connection>,
    pool: Arc<OracleConnectionPool>,
}

impl OracleConnectionGuard {
    fn new(conn: Connection, pool: Arc<OracleConnectionPool>) -> Self {
        OracleConnectionGuard {
            conn: Some(conn),
            pool,
        }
    }
    
    /// Get mutable reference to the underlying connection
    pub fn as_mut(&mut self) -> &mut Connection {
        self.conn.as_mut().expect("Connection already taken")
    }
    
    /// Check if connection is valid
    pub fn is_valid(&self) -> bool {
        self.conn.as_ref().map_or(false, |c| c.is_valid())
    }
}

impl Drop for OracleConnectionGuard {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.return_connection_internal(conn);
        }
    }
}

impl std::ops::Deref for OracleConnectionGuard {
    type Target = Connection;
    
    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().expect("Connection already taken")
    }
}

impl std::ops::DerefMut for OracleConnectionGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().expect("Connection already taken")
    }
}

/// Oracle connection wrapper with connection pooling simulation
/// Note: The `oracle` crate doesn't have native r2d2 support, so we manage connections manually
pub struct OracleConnectionPool {
    config: OracleConnectionConfig,
    max_connections: usize,
    active_connections: Arc<RwLock<Vec<Connection>>>,
}

impl OracleConnectionPool {
    /// Create a new Oracle connection pool
    pub fn new(connection_string: String, max_connections: usize) -> Result<Self, ServiceError> {
        let config = OracleConnectionConfig::parse(&connection_string)?;
        
        Ok(OracleConnectionPool {
            config,
            max_connections,
            active_connections: Arc::new(RwLock::new(Vec::with_capacity(max_connections))),
        })
    }

    /// Get or create a connection guard from the pool
    pub fn get(self: &Arc<Self>) -> Result<OracleConnectionGuard, ServiceError> {
        // Try to reuse an existing connection
        match self.active_connections.write() {
            Ok(mut conns) => {
                if let Some(conn) = conns.pop() {
                    if conn.is_valid() {
                        return Ok(OracleConnectionGuard::new(conn, Arc::clone(self)));
                    }
                }
            }
            Err(e) => {
                log::warn!("Lock poisoned in connection pool, creating new connection: {}", e);
            }
        }

        // Create new connection with parsed credentials
        let conn = Connection::connect(
            &self.config.connection_string,
            &self.config.username,
            &self.config.password,
        )
        .map_err(|e| ServiceError::internal_server_error(format!("Oracle connection failed: {}", e)))?;
        
        Ok(OracleConnectionGuard::new(conn, Arc::clone(self)))
    }

    /// Internal method to return a connection to the pool (called by Drop)
    fn return_connection_internal(&self, conn: Connection) {
        if let Ok(mut conns) = self.active_connections.write() {
            if conns.len() < self.max_connections {
                conns.push(conn);
            }
            // Otherwise let it drop and close
        }
    }
    
    /// Return a connection to the pool for reuse (deprecated - use OracleConnectionGuard instead)
    #[deprecated(note = "Use OracleConnectionGuard which returns connections automatically")]
    pub fn return_connection(&self, conn: Connection) {
        self.return_connection_internal(conn);
    }
}

/// Health status for Oracle connection pool
#[derive(Debug, Clone, Serialize)]
pub struct OraclePoolHealthStatus {
    pub tenant_id: String,
    pub active_connections: usize,
    pub max_connections: usize,
    pub is_healthy: bool,
    pub last_check: chrono::NaiveDateTime,
    pub error_message: Option<String>,
}

/// Manages Oracle database connections for tenants
#[derive(Clone)]
pub struct OracleTenantPoolManager {
    tenant_pools: Arc<RwLock<HashMap<String, Arc<OracleConnectionPool>>>>,
    tenant_connection_strings: Arc<RwLock<HashMap<String, String>>>,
}

impl OracleTenantPoolManager {
    pub fn new() -> Self {
        OracleTenantPoolManager {
            tenant_pools: Arc::new(RwLock::new(HashMap::new())),
            tenant_connection_strings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a tenant's Oracle connection pool
    pub fn add_tenant_pool(&self, tenant_id: String, connection_string: String) -> Result<(), ServiceError> {
        let pool = Arc::new(OracleConnectionPool::new(connection_string.clone(), 20)?);
        
        let mut pools = self.tenant_pools.write()
            .map_err(|_| ServiceError::internal_server_error("Lock poisoned: tenant_pools"))?;
        
        let mut conn_strings = self.tenant_connection_strings.write()
            .map_err(|_| ServiceError::internal_server_error("Lock poisoned: connection_strings"))?;

        pools.insert(tenant_id.clone(), pool);
        conn_strings.insert(tenant_id, connection_string);
        
        Ok(())
    }

    /// Get a tenant's Oracle connection pool
    pub fn get_tenant_pool(&self, tenant_id: &str) -> Option<Arc<OracleConnectionPool>> {
        self.tenant_pools.read()
            .ok()
            .and_then(|pools| pools.get(tenant_id).cloned())
    }

    /// Get an Oracle connection guard for a tenant
    pub fn get_connection(&self, tenant_id: &str) -> Result<OracleConnectionGuard, ServiceError> {
        let pool = self.get_tenant_pool(tenant_id)
            .ok_or_else(|| ServiceError::not_found(format!("Oracle pool not found for tenant: {}", tenant_id)))?;
        
        pool.get()
    }

    /// Get or create a tenant pool by querying the main PostgreSQL database
    pub fn get_or_create_pool(
        &self, 
        tenant_id: &str, 
        main_pg_pool: &crate::config::db::Pool
    ) -> Result<Arc<OracleConnectionPool>, ServiceError> {
        // Check if pool exists
        if let Some(pool) = self.get_tenant_pool(tenant_id) {
            return Ok(pool);
        }

        // Query tenant info from PostgreSQL
        use crate::models::tenant::Tenant;
        use crate::schema::tenants::dsl::*;
        use diesel::prelude::*;

        let mut conn = main_pg_pool.get()
            .map_err(|e| ServiceError::internal_server_error(format!("Failed to get PG connection: {}", e)))?;

        let tenant = tenants
            .filter(id.eq(tenant_id))
            .first::<Tenant>(&mut conn)
            .map_err(|e| ServiceError::not_found(format!("Tenant not found: {}", e)))?;

        // Assume tenant.db_url is in Oracle format: "hostname:port/service_name"
        // Or use a separate oracle_db_url field if you add it
        let oracle_conn_string = tenant.db_url;

        // Create the pool
        self.add_tenant_pool(tenant_id.to_string(), oracle_conn_string)?;
        
        self.get_tenant_pool(tenant_id)
            .ok_or_else(|| ServiceError::internal_server_error("Failed to retrieve newly created pool"))
    }

    /// Check health of all tenant pools
    pub fn health_check(&self) -> Vec<OraclePoolHealthStatus> {
        let pools = match self.tenant_pools.read() {
            Ok(p) => p,
            Err(_) => return vec![],
        };

        pools.iter().map(|(tenant_id, pool)| {
            let (is_healthy, error_msg) = match pool.get() {
                Ok(conn_guard) => {
                    // Connection guard automatically returns to pool on drop
                    let healthy = conn_guard.is_valid();
                    (healthy, None)
                },
                Err(e) => (false, Some(format!("{:?}", e))),
            };

            let active = pool.active_connections.read()
                .map(|conns| conns.len())
                .unwrap_or(0);

            OraclePoolHealthStatus {
                tenant_id: tenant_id.clone(),
                active_connections: active,
                max_connections: pool.max_connections,
                is_healthy,
                last_check: chrono::Utc::now().naive_utc(),
                error_message: error_msg,
            }
        }).collect()
    }

    /// Remove a tenant pool
    pub fn remove_tenant_pool(&self, tenant_id: &str) -> Result<bool, ServiceError> {
        let mut pools = self.tenant_pools.write()
            .map_err(|_| ServiceError::internal_server_error("Lock poisoned"))?;
        
        let mut conn_strings = self.tenant_connection_strings.write()
            .map_err(|_| ServiceError::internal_server_error("Lock poisoned"))?;

        let removed = pools.remove(tenant_id).is_some();
        conn_strings.remove(tenant_id);
        
        Ok(removed)
    }
}

impl Default for OracleTenantPoolManager {
    fn default() -> Self {
        Self::new()
    }
}
