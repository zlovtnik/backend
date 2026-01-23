use actix_web::{get, web, HttpRequest, HttpResponse};
use serde::Serialize;
use tokio::time::{timeout, Duration};

use crate::config::cache::Pool as RedisPool;
use crate::config::db::{Pool as DatabasePool, TenantPoolManager};
use crate::constants;
use crate::error::ServiceError;
use crate::models::response::ResponseBody;
use crate::models::tenant::Tenant;

use chrono::Utc;
use diesel::prelude::*;
use log::{error, info};
use redis;

use crate::functional::performance_monitoring::{
    get_performance_monitor, HealthSummary as PerformanceHealthSummary, OperationType,
};

#[derive(Serialize, Clone)]
enum Status {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "unhealthy")]
    Unhealthy,
    #[serde(rename = "unavailable")]
    Unavailable,
}

impl Status {
    fn is_healthy(&self) -> bool {
        matches!(self, Status::Healthy)
    }
    
    fn is_available(&self) -> bool {
        !matches!(self, Status::Unavailable)
    }
}

#[derive(Serialize)]
struct HealthStatus {
    database: Status,
    cache: Status,
}

#[derive(Serialize)]
struct HealthResponse {
    status: Status,
    timestamp: String,
    components: HealthStatus,
    tenants: Option<Vec<TenantHealth>>,
    #[cfg(feature = "functional")]
    performance: Option<PerformanceHealthSummary>,
}

#[derive(Serialize)]
struct TenantHealth {
    tenant_id: String,
    name: String,
    status: Status,
}

/// Check whether the database accepts a simple health query using the provided connection pool.
///
/// Returns `Ok(())` if a basic query succeeds and the database connection is healthy, `Err` with
/// the underlying error otherwise.
///
/// # Examples
///
/// ```
/// # async fn example(pool: actix_web::web::Data<DatabasePool>) {
/// let result = check_database_health_async(pool).await;
/// assert!(result.is_ok());
/// # }
/// ```
async fn check_database_health_async(
    pool: web::Data<DatabasePool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    tokio::task::spawn_blocking(move || check_database_health(pool)).await?
}

/// Checks whether the Redis cache responds to a PING.
///
/// Returns `Ok(())` if the cache responds to a PING, `Err(...)` if the probe fails.
///
/// # Examples
///
/// ```
/// # use actix_web::web;
/// # async fn demo(pool: web::Data<crate::RedisPool>) {
/// let result = crate::check_cache_health_async(pool).await;
/// assert!(result.is_ok() || result.is_err());
/// # }
/// ```
async fn check_cache_health_async(
    redis_pool: web::Data<RedisPool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    tokio::task::spawn_blocking(move || check_cache_health(&redis_pool))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>)?
}

/// Return a JSON health summary for the service.
///
/// Includes the overall `Status`, an RFC3339 `timestamp`, and component statuses
/// for `database` and `cache`. The `tenants` field is omitted.
///
/// # Examples
///
/// ```no_run
/// use actix_web::{test, App};
///
/// # async fn example() {
/// let app = test::init_service(App::new().service(crate::health)).await;
/// let req = test::TestRequest::get().uri("/health").to_request();
/// let resp = test::call_service(&app, req).await;
/// assert!(resp.status().is_success());
/// # }
/// ```
#[get("/health")]
async fn health(
    pool: web::Data<DatabasePool>,
    redis_pool: Option<web::Data<Option<RedisPool>>>,
) -> Result<HttpResponse, ServiceError> {
    info!("Health check requested");

    // Check database with timeout
    let db_status = match timeout(Duration::from_secs(5), check_database_health_async(pool)).await {
        Ok(Ok(())) => Status::Healthy,
        Ok(Err(e)) => {
            error!("Database health check failed: {}", e);
            Status::Unhealthy
        }
        Err(_) => {
            error!("Database health check timeout");
            Status::Unhealthy
        }
    };

    // Check cache with timeout - handle optional Redis gracefully
    let cache_status = match redis_pool.as_ref().and_then(|d| d.get_ref().as_ref()) {
        Some(pool) => {
            let pool_clone = pool.clone();
            let pool_data = web::Data::new(pool_clone);
            match timeout(Duration::from_secs(3), check_cache_health_async(pool_data)).await {
                Ok(Ok(())) => Status::Healthy,
                Ok(Err(e)) => {
                    error!("Cache health check failed: {}", e);
                    Status::Unhealthy
                }
                Err(_) => {
                    error!("Cache health check timeout");
                    Status::Unhealthy
                }
            }
        }
        None => {
            info!("Redis cache is not configured - marking as unavailable");
            Status::Unavailable
        }
    };

    // System is healthy if database is healthy and cache is either healthy or unavailable (not configured)
    let overall_status = if db_status.is_healthy() && (cache_status.is_healthy() || !cache_status.is_available()) {
        Status::Healthy
    } else {
        Status::Unhealthy
    };

    let response = HealthResponse {
        status: overall_status,
        timestamp: Utc::now().to_rfc3339(),
        components: HealthStatus {
            database: db_status,
            cache: cache_status,
        },
        tenants: None,
        #[cfg(feature = "functional")]
        performance: None,
    };

    Ok(HttpResponse::Ok().json(ResponseBody::new(constants::MESSAGE_OK, response)))
}

/// Produces a detailed health report that includes database, cache, and per-tenant statuses.
///
/// The response body is a JSON-encoded `HealthResponse` containing:
/// - `status`: overall system status,
/// - `timestamp`: RFC3339 timestamp of the check,
/// - `components`: individual `database` and `cache` statuses,
/// - `tenants`: optional list of `TenantHealth` entries when tenant pools are available.
///
/// # Examples
///
/// ```
/// use actix_web::test::{self, TestRequest};
/// use actix_web::http::StatusCode;
///
/// // Build a simple request and call the handler (integration tests should set up app data).
/// let req = TestRequest::with_uri("/health/detailed").to_http_request();
/// // In real tests, provide `pool`, `redis_pool`, and `main_conn` as `web::Data` in app state.
/// // Here we only demonstrate the call shape; integration tests should assert the JSON body.
/// let resp = actix_rt::System::new().block_on(async {
///     // health_detailed(req, pool, redis_pool, main_conn).await
///     // -> HttpResponse
///     HttpResponse::Ok()
/// });
/// assert_eq!(resp.status(), StatusCode::OK);
/// ```
#[get("/health/detailed")]
async fn health_detailed(
    req: HttpRequest,
    pool: web::Data<DatabasePool>,
    redis_pool: Option<web::Data<Option<RedisPool>>>,
    main_conn: web::Data<DatabasePool>,
) -> Result<HttpResponse, ServiceError> {
    let manager = req.app_data::<web::Data<TenantPoolManager>>();
    info!("Detailed health check requested");

    // Check database with timeout
    let db_status = match timeout(Duration::from_secs(5), check_database_health_async(pool)).await {
        Ok(Ok(())) => Status::Healthy,
        Ok(Err(e)) => {
            error!("Database health check failed: {}", e);
            Status::Unhealthy
        }
        Err(_) => {
            error!("Database health check timeout");
            Status::Unhealthy
        }
    };

    // Check cache with timeout - handle optional Redis gracefully
    let cache_status = match redis_pool.as_ref().and_then(|d| d.get_ref().as_ref()) {
        Some(rpool) => {
            let pool_clone = rpool.clone();
            let pool_data = web::Data::new(pool_clone);
            match timeout(Duration::from_secs(3), check_cache_health_async(pool_data)).await {
                Ok(Ok(())) => Status::Healthy,
                Ok(Err(e)) => {
                    error!("Cache health check failed: {}", e);
                    Status::Unhealthy
                }
                Err(_) => {
                    error!("Cache health check timeout");
                    Status::Unhealthy
                }
            }
        }
        None => {
            info!("Redis cache is not configured - marking as unavailable");
            Status::Unavailable
        }
    };

    // Check tenant health if tenant manager is available
    let tenants = if let Some(manager_ref) = manager {
        let manager_data = manager_ref.clone();
        match tokio::task::spawn_blocking(move || {
            let mut main_conn = main_conn
                .get()
                .map_err(|e| format!("Failed to get db connection: {}", e))?;
            let tenants = Tenant::list_all(&mut main_conn).unwrap_or_else(|_| Vec::new());
            let mut tenant_healths = Vec::new();

            for tenant in tenants {
                let status = match manager_data.get_tenant_pool(&tenant.id) {
                    Some(pool) => match pool.get() {
                        Ok(mut conn) => match diesel::sql_query("SELECT 1").execute(&mut conn) {
                            Ok(_) => Status::Healthy,
                            Err(_) => Status::Unhealthy,
                        },
                        Err(_) => Status::Unhealthy,
                    },
                    None => Status::Unhealthy,
                };
                tenant_healths.push(TenantHealth {
                    tenant_id: tenant.id,
                    name: tenant.name,
                    status,
                });
            }
            Ok::<Vec<TenantHealth>, String>(tenant_healths)
        })
        .await
        {
            Ok(Ok(healths)) if !healths.is_empty() => Some(healths),
            _ => None,
        }
    } else {
        None
    };

    // System is healthy if:
    // - database is healthy
    // - cache is healthy OR unavailable (not configured)
    // - all tenants are healthy (if any)
    let overall_status = if db_status.is_healthy()
        && (cache_status.is_healthy() || !cache_status.is_available())
        && tenants
            .as_ref()
            .map_or(true, |t| t.iter().all(|th| th.status.is_healthy()))
    {
        Status::Healthy
    } else {
        Status::Unhealthy
    };

    // Get performance monitoring health summary
    #[cfg(feature = "functional")]
    let performance_summary = get_performance_monitor().get_health_summary();

    let response = HealthResponse {
        status: overall_status,
        timestamp: Utc::now().to_rfc3339(),
        components: HealthStatus {
            database: db_status,
            cache: cache_status,
        },
        tenants,
        #[cfg(feature = "functional")]
        performance: Some(performance_summary),
    };

    Ok(HttpResponse::Ok().json(ResponseBody::new(constants::MESSAGE_OK, response)))
}

/// Checks database connectivity by acquiring a connection from the pool and executing `SELECT 1`.
///
/// Returns `Ok(())` if a connection is acquired and the validation query succeeds, `Err` with an error otherwise.
///
/// # Examples
///
/// ```rust
/// use actix_web::web;
/// // Assuming `pool: web::Data<crate::DatabasePool>`
/// # fn example(pool: web::Data<crate::DatabasePool>) {
/// let _ = crate::check_database_health(pool);
/// # }
/// ```
fn check_database_health(
    pool: web::Data<DatabasePool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    match pool.get() {
        Ok(mut conn) => {
            diesel::sql_query("SELECT 1").execute(&mut conn)?;
            Ok(())
        }
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get database connection: {}", e),
        ))),
    }
}

/// Verifies Redis cache responsiveness by sending a `PING` command.
///
/// Uses the provided Redis connection pool to obtain a connection and issues a `PING`.
///
/// # Parameters
///
/// * `redis_pool` - Connection pool used to acquire a Redis connection for the health check.
///
/// # Returns
///
/// `Ok(())` if Redis responds to `PING`, `Err` with the underlying error otherwise.
///
/// # Examples
///
/// ```
/// // Acquire or construct a RedisPool appropriate for your application.
/// // let redis_pool = RedisPool::new("redis://127.0.0.1").unwrap();
/// // assert!(check_cache_health(&redis_pool).is_ok());
/// ```
fn check_cache_health(
    redis_pool: &RedisPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut conn = redis_pool
        .get()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>)?;
    redis::cmd("PING")
        .query::<()>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>)?;
    Ok(())
}

/// **DEPRECATED**: Use WebSocket endpoint `/api/ws/logs` instead.
///
/// Streams the application's log file to clients over Server-Sent Events (SSE).
///
/// This endpoint is deprecated in favor of the WebSocket-based log streaming
/// at `/api/ws/logs` which provides real-time log messages without file I/O.
///
/// When `ENABLE_LOG_STREAM` is set to `"true"` and the file at `LOG_FILE` (defaults to
/// `/var/log/app.log`) exists, this handler returns an `HttpResponse` that continuously
/// streams new log lines as SSE `data:` frames. If streaming is disabled, the handler
/// responds with `405 MethodNotAllowed`. If the configured log file does not exist, the
/// handler responds with `404 NotFound`.
///
/// # Examples
///
/// ```
/// use actix_web::{App, test};
/// use std::env;
/// use std::fs;
///
/// # async fn run_example() {
/// env::set_var("ENABLE_LOG_STREAM", "true");
/// env::set_var("LOG_FILE", "/tmp/app.log");
/// let _ = fs::write("/tmp/app.log", ""); // ensure file exists
///
/// let app = test::init_service(App::new().service(crate::logs)).await;
/// let req = test::TestRequest::get().uri("/logs").to_request();
/// let resp = test::call_service(&app, req).await;
/// assert!(resp.status().is_success());
/// # }
/// ```
#[get("/logs")]
async fn logs() -> Result<HttpResponse, ServiceError> {
    // Return deprecation notice
    Ok(HttpResponse::Gone().json(serde_json::json!({
        "message": "This endpoint is deprecated. Please use WebSocket endpoint /api/ws/logs instead",
        "websocket_url": "/api/ws/logs",
        "deprecation_version": "0.2.0",
        "more_info": "WebSocket provides real-time log streaming without file I/O"
    })))
}

// Legacy log streaming implementation removed in commit: git log -1 --format=%H
// See: WebSocket endpoint /api/ws/logs for current real-time log streaming
// The legacy SSE-based implementation has been superseded by the WebSocket-based solution.

/// Retrieves performance monitoring data and metrics for functional programming operations.
///
/// Returns current performance statistics including execution counts, timing data,
/// memory usage, and threshold violations for different operation types.
///
/// # Parameters
///
/// - `req` - HTTP request containing optional query parameters for filtering
///
/// # Query Parameters
///
/// - `operation_type` - Filter metrics by operation type (e.g., "iterator", "validation", "query")
/// - `include_history` - Include historical data in response (default: false)
/// - `reset_counters` - Reset performance counters after reading (default: false)
///
/// # Returns
///
/// Returns a JSON response containing performance metrics and health summary:
/// - Overall performance health status
/// - Per-operation type metrics (execution count, average duration, memory usage)
/// - Threshold violations and performance warnings
/// - Memory allocation patterns and garbage collection stats
///
/// # Examples
///
/// ```rust
/// // Basic performance metrics
/// GET /health/performance
///
/// // Filter by operation type
/// GET /health/performance?operation_type=iterator
///
/// // Include historical data
/// GET /health/performance?include_history=true
///
/// // Reset counters after reading
/// GET /health/performance?reset_counters=true
/// ```
///
/// # Integration Testing
///
/// ```rust
/// use actix_web::test;
/// use actix_web::http::StatusCode;
///
/// let app = test::init_service(
///     App::new()
///         .service(performance_metrics)
///         .wrap(crate::middleware::auth_middleware::Authentication)
/// ).await;
///
/// let req = test::TestRequest::get()
///     .uri("/health/performance")
///     .to_request();
/// let resp = test::call_service(&app, req).await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// ```
#[cfg(feature = "performance_monitoring")]
#[get("/health/performance")]
async fn performance_metrics(req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    info!("Performance metrics requested");

    // Parse query parameters
    let query =
        web::Query::<std::collections::HashMap<String, String>>::from_query(req.query_string())
            .unwrap_or_else(|_| web::Query(std::collections::HashMap::new()));

    let operation_type_filter = query.get("operation_type").cloned();
    let include_history = query
        .get("include_history")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);
    let reset_counters = query
        .get("reset_counters")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    // Get performance monitor instance
    let monitor = get_performance_monitor();

    // Generate comprehensive performance report
    let performance_summary = monitor.get_health_summary();
    let all_metrics = monitor.get_all_metrics();

    // Filter metrics by operation type if specified
    let filtered_metrics = if let Some(op_type_str) = operation_type_filter {
        let operation_type = match op_type_str.as_str() {
            "iterator_chain" => Some(OperationType::IteratorChain),
            "validation_pipeline" => Some(OperationType::ValidationPipeline),
            "query_composition" => Some(OperationType::QueryComposition),
            "response_transformation" => Some(OperationType::ResponseTransformation),
            "concurrent_processing" => Some(OperationType::ConcurrentProcessing),
            "state_transition" => Some(OperationType::StateTransition),
            "lazy_pipeline" => Some(OperationType::LazyPipeline),
            "pure_function_call" => Some(OperationType::PureFunctionCall),
            _ => None,
        };

        if let Some(op_type) = operation_type {
            all_metrics
                .into_iter()
                .filter(|(key, _)| *key == op_type)
                .collect()
        } else {
            all_metrics
        }
    } else {
        all_metrics
    };

    // Build response data
    let total_operations: u64 = filtered_metrics.values().map(|m| m.operation_count).sum();
    let total_duration: f64 = filtered_metrics
        .values()
        .map(|m| m.avg_execution_time.as_secs_f64() * 1000.0)
        .sum();
    let count = filtered_metrics.len();
    let average_duration_ms = if count > 0 {
        total_duration / count as f64
    } else {
        0.0
    };
    let total_memory_allocated_mb = filtered_metrics
        .values()
        .map(|m| m.memory_stats.total_allocated)
        .sum::<u64>()
        / (1024 * 1024);

    let operations_by_type: Vec<serde_json::Value> = filtered_metrics.iter().map(|(op_type, metrics)| {
        serde_json::json!({
            "operation": format!("{:?}", op_type),
            "execution_count": metrics.operation_count,
            "average_duration_ms": metrics.avg_execution_time.as_secs_f64() * 1000.0,
            "min_duration_ms": metrics.min_execution_time.as_secs_f64() * 1000.0,
            "max_duration_ms": metrics.max_execution_time.as_secs_f64() * 1000.0,
            "memory_allocated_mb": metrics.memory_stats.total_allocated / (1024 * 1024),
            "memory_peak_mb": metrics.memory_stats.peak_memory_bytes / (1024 * 1024),
            "success_rate": if metrics.operation_count > 0 {
                ((metrics.operation_count - metrics.error_count) as f64 / metrics.operation_count as f64) * 100.0
            } else { 100.0 },
            "error_count": metrics.error_count,
            "last_execution": chrono::DateTime::<chrono::Utc>::from(
                std::time::UNIX_EPOCH + metrics.last_updated.elapsed()
            ).to_rfc3339(),
        })
    }).collect();

    let mut response_data = serde_json::json!({
        "performance_health": performance_summary,
        "metrics_summary": {
            "total_operations": total_operations,
            "average_duration_ms": average_duration_ms,
            "total_memory_allocated_mb": total_memory_allocated_mb,
            "operations_by_type": operations_by_type
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    // Add historical data if requested
    if include_history {
        response_data["historical_data"] = serde_json::json!({
            "note": "Historical data tracking not yet implemented",
            "future_enhancements": [
                "Time-series performance data",
                "Performance trend analysis",
                "Bottleneck identification",
                "Capacity planning metrics"
            ]
        });
    }

    // Reset counters if requested
    if reset_counters {
        monitor.reset_metrics();
        response_data["counters_reset"] = serde_json::Value::Bool(true);
    }

    Ok(HttpResponse::Ok().json(ResponseBody::new(constants::MESSAGE_OK, response_data)))
}

#[cfg(not(feature = "performance_monitoring"))]
#[get("/health/performance")]
async fn performance_metrics(_req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    Ok(HttpResponse::ServiceUnavailable().json(ResponseBody::new(
        "Performance monitoring feature not enabled",
        serde_json::json!({
            "error": "Performance monitoring is not compiled into this build",
            "suggestion": "Rebuild with --features performance_monitoring",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    )))
}

#[cfg(test)]
mod tests {
    //! Integration tests for health and logging endpoints.
    //!
    //! **Important**: Tests involving log streaming (`test_logs_*`) use global environment
    //! variables which can cause race conditions when tests run in parallel.
    //! To avoid test failures, run with: `cargo test -- --test-threads=1`
    //!
    //! Consider using the `serial_test` crate in the future for better test isolation.

    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use actix_cors::Cors;
    use actix_web::web::Data;
    use actix_web::{http::StatusCode, test};
    use testcontainers::clients;
    use testcontainers::images::postgres::Postgres;
    use testcontainers::images::redis::Redis;
    use testcontainers::Container;

    use crate::config;

    fn try_run_postgres<'a>(docker: &'a clients::Cli) -> Option<Container<'a, Postgres>> {
        catch_unwind(AssertUnwindSafe(|| docker.run(Postgres::default()))).ok()
    }

    fn try_run_redis<'a>(docker: &'a clients::Cli) -> Option<Container<'a, Redis>> {
        catch_unwind(AssertUnwindSafe(|| docker.run(Redis))).ok()
    }

    /// Verifies that the /api/health endpoint returns HTTP 200 when PostgreSQL and Redis are available.
    ///
    /// Spawns PostgreSQL and Redis test containers, initializes the database and cache clients, mounts the application,
    /// and asserts the health endpoint responds with status 200.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Run the integration test with:
    /// // cargo test --test integration_tests -- --nocapture
    /// ```
    #[actix_web::test]
    async fn test_health_ok() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_health_ok because Docker is unavailable");
                return;
            }
        };
        let redis = match try_run_redis(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_health_ok because Redis container could not start");
                return;
            }
        };

        let pool = config::db::init_db_pool(
            format!(
                "postgres://postgres:postgres@127.0.0.1:{}/postgres",
                postgres.get_host_port_ipv4(5432)
            )
            .as_str(),
        );
        config::db::run_migration(&mut pool.get().unwrap())
            .expect("DB migration failed in test setup");

        let redis_client = config::cache::init_redis_client(
            format!("redis://127.0.0.1:{}", redis.get_host_port_ipv4(6379)).as_str(),
        );

        let app = test::init_service(
            actix_web::App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(actix_web::http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(Data::new(pool))
                .app_data(Data::new(redis_client))
                .wrap(crate::middleware::auth_middleware::Authentication)
                .configure(config::app::config_services),
        )
        .await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        // You can parse the JSON and check fields
    }

    /// Verifies that the `/api/logs` endpoint returns HTTP 410 Gone with deprecation notice.
    ///
    /// The SSE-based log streaming endpoint is deprecated in favor of the WebSocket endpoint
    /// at `/api/ws/logs`. This test confirms that the deprecated endpoint returns 410 (Gone)
    /// with a JSON response body containing the new WebSocket endpoint URL and deprecation message.
    ///
    /// # Examples
    ///
    /// ```
    /// // The test performs a GET request to /api/logs and validates:
    /// // 1. Response status is 410 Gone
    /// // 2. Response Content-Type is application/json
    /// // 3. JSON body contains websocket_url and deprecation fields
    /// ```
    #[actix_web::test]
    async fn test_logs_deprecated() {
        let app = test::init_service(
            actix_web::App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(actix_web::http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .wrap(crate::middleware::auth_middleware::Authentication)
                .configure(config::app::config_services),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/logs").to_request();
        let resp = test::call_service(&app, req).await;

        // Verify the response is 410 Gone
        assert_eq!(
            resp.status(),
            StatusCode::GONE,
            "Expected 410 Gone for deprecated /api/logs endpoint"
        );

        // Verify the response contains deprecation JSON
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            "application/json"
        );

        let body_bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        // Verify JSON contains expected fields
        assert!(
            body_str.contains("websocket_url"),
            "Response should contain websocket_url field. Body: {}",
            body_str
        );
        assert!(
            body_str.contains("deprecated"),
            "Response should contain deprecation message. Body: {}",
            body_str
        );
    }

    /// Verifies that the /api/health/performance endpoint returns performance metrics data.
    ///
    /// Tests that the performance monitoring endpoint responds with HTTP 200 and returns
    /// valid JSON containing performance metrics and health summary.
    ///
    /// # Test Cases
    ///
    /// - Basic performance metrics request
    /// - Query parameter filtering (operation_type)
    /// - Include history flag
    /// - Reset counters functionality
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Run the integration test with:
    /// // cargo test test_performance_metrics_ok -- --nocapture
    /// ```
    #[cfg(feature = "performance_monitoring")]
    #[actix_web::test]
    async fn test_performance_metrics_ok() {
        use crate::functional::performance_monitoring::{get_performance_monitor, OperationType};
        use actix_web::{http::StatusCode, test};
        use std::time::Duration as StdDuration;

        // Generate some test metrics data
        let monitor = get_performance_monitor();
        monitor.record_operation(
            OperationType::IteratorChain,
            StdDuration::from_millis(100),
            1024,
            false,
        );
        monitor.record_operation(
            OperationType::ValidationPipeline,
            StdDuration::from_millis(50),
            512,
            false,
        );

        let app = test::init_service(
            actix_web::App::new().service(performance_metrics).wrap(
                Cors::default()
                    .send_wildcard()
                    .allowed_methods(vec!["GET"])
                    .allowed_header(actix_web::http::header::CONTENT_TYPE)
                    .max_age(3600),
            ),
        )
        .await;

        // Test basic performance metrics request
        let req = test::TestRequest::get()
            .uri("/health/performance")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        // Verify response structure
        assert!(json["data"]["performance_health"].is_object());
        assert!(json["data"]["metrics_summary"].is_object());
        assert!(json["data"]["metrics_summary"]["total_operations"].is_number());
        assert!(json["data"]["timestamp"].is_string());

        // Test with operation type filter
        let req = test::TestRequest::get()
            .uri("/health/performance?operation_type=iterator_chain")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Test with include history flag
        let req = test::TestRequest::get()
            .uri("/health/performance?include_history=true")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(json["data"]["historical_data"].is_object());

        // Test with reset counters
        let req = test::TestRequest::get()
            .uri("/health/performance?reset_counters=true")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["data"]["counters_reset"], true);
    }

    #[cfg(not(feature = "performance_monitoring"))]
    #[actix_web::test]
    async fn test_performance_metrics_disabled() {
        use actix_web::{http::StatusCode, test};

        let app = test::init_service(actix_web::App::new().service(performance_metrics)).await;

        let req = test::TestRequest::get()
            .uri("/health/performance")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(json["message"].as_str().unwrap().contains("not enabled"));
    }
}
