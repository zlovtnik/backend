use std::default::Default;
use std::{env, io};

use actix_cors::Cors;
use actix_session::config::PersistentSession;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::time::Duration;
use actix_web::cookie::SameSite;
use actix_web::web;
use actix_web::{http, App, HttpServer};
use rcs::config;
use rcs::utils::ws_logger::{init_websocket_logging, LogBroadcaster};
use std::sync::Arc;
/// Application entry point.
///
/// Reads required environment variables (APP_HOST, APP_PORT, DATABASE_URL, REDIS_URL),
/// initialises the main DB pool and Redis client, registers tenant pools, configures
/// CORS and middleware, and starts the Actix HTTP server.
///
/// # Examples
///
/// ```no_run
/// // Start the application (requires appropriate environment variables).
/// // let _ = futures::executor::block_on(crate::main());
/// ```
#[actix_rt::main]
async fn main() -> io::Result<()> {
    if let Err(e) = dotenv::dotenv() {
        match e {
            dotenv::Error::Io(io_err) if io_err.kind() == std::io::ErrorKind::NotFound => {
                log::warn!(".env file not found, environment variables will be read from system environment");
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Failed to read .env file: {}", e),
                ));
            }
        }
    }
    // Set RUST_LOG default before any threads are spawned.
    // NOTE: env::set_var is unsound if called after threads exist; this call is safe
    // here because it happens before HttpServer::new spawns worker threads.
    if env::var("RUST_LOG").is_err() {
        // Safety: no threads have been spawned at this point in main()
        #[allow(unsafe_code)]
        unsafe { env::set_var("RUST_LOG", "info") };
    }

    // Read WebSocket log buffer size from environment or use default of 1000
    let ws_log_buffer_size = match env::var("WS_LOG_BUFFER_SIZE") {
        Ok(s) => match s.parse::<usize>() {
            Ok(size) => size,
            Err(_) => {
                log::warn!(
                    "Invalid WS_LOG_BUFFER_SIZE value '{}': failed to parse as usize. Using default 1000",
                    s
                );
                1000
            }
        },
        Err(_) => 1000,
    };

    // Initialize WebSocket-based logging with tracing
    let log_broadcaster = LogBroadcaster::new(ws_log_buffer_size);
    init_websocket_logging(log_broadcaster.clone()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to initialize logging: {}", e),
        )
    })?;

    // Validate cursor encryption key at startup to fail fast on misconfiguration
    #[cfg(feature = "functional")]
    if let Err(e) = rcs::unified_pagination::validate_cursor_encryption_key() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Invalid CURSOR_ENCRYPTION_KEY configuration: {}. Ensure it is a Base64-encoded 32-byte key.",
                e
            ),
        ));
    }

    let app_host = env::var("APP_HOST").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("APP_HOST not found: {}", e),
        )
    })?;
    let app_port = env::var("APP_PORT").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("APP_PORT not found: {}", e),
        )
    })?;
    let app_url = format!("{}:{}", &app_host, &app_port);

    let db_url = env::var("DATABASE_URL").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("DATABASE_URL not found: {}", e),
        )
    })?;
    // Redis is optional - if REDIS_URL is not set or connection fails, cache features will be disabled
    let redis_url = env::var("REDIS_URL").ok();

    let main_pool = config::db::init_db_pool(&db_url);
    let mut conn = main_pool.get().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to get database connection for migration: {}", e),
        )
    })?;
    config::db::run_migration(&mut conn).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Database migration failed: {}", e),
        )
    })?;
    
    // Try to initialize Redis client - distinguish between "not configured" and "connection failed"
    let redis_client: Option<config::cache::Pool> = match &redis_url {
        None => {
            log::warn!("Redis not configured (REDIS_URL missing). Cache features will be disabled.");
            None
        }
        Some(url) => {
            match config::cache::try_init_redis_client(url) {
                Some(pool) => Some(pool),
                None => {
                    log::warn!("Redis connection failed for configured URL. Cache features will be disabled.");
                    None
                }
            }
        }
    };

    // Initialize Keycloak client
    let keycloak_config = rcs::utils::keycloak::KeycloakConfig {
        issuer_url: {
            let default_issuer = "http://localhost:8080/realms/middleware";
            match env::var("KEYCLOAK_ISSUER_URL") {
                Ok(url) => url,
                Err(_) => {
                    let is_dev = env::var("APP_ENV").map(|v| v == "dev").unwrap_or(false);
                    if is_dev {
                        log::warn!("KEYCLOAK_ISSUER_URL not set. Using development default localhost. DO NOT use in production.");
                        default_issuer.to_string()
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "KEYCLOAK_ISSUER_URL must be set in production. Set APP_ENV=dev to use development defaults.",
                        ));
                    }
                }
            }
        },
        client_id: env::var("KEYCLOAK_CLIENT_ID").unwrap_or_else(|_| "middleware-app".to_string()),
        client_secret: {
            use secrecy::SecretString;
            let secret = env::var("KEYCLOAK_MIDDLEWARE_APP_SECRET");
            match secret {
                Ok(s) => SecretString::new(s.into_boxed_str()),
                Err(_) => {
                    let is_dev = env::var("APP_ENV").map(|v| v == "dev").unwrap_or(false);
                    if is_dev {
                        log::warn!("KEYCLOAK_MIDDLEWARE_APP_SECRET not set. Using development default. DO NOT use in production.");
                        SecretString::new("middleware-app-secret-dev".to_string().into_boxed_str())
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "KEYCLOAK_MIDDLEWARE_APP_SECRET must be set in production. Set APP_ENV=dev to use development defaults.",
                        ));
                    }
                }
            }
        },
        redirect_url: env::var("KEYCLOAK_REDIRECT_URL")
            .unwrap_or_else(|_| "http://localhost:8000/api/callback".to_string()),
    };
    let keycloak_client = web::Data::new(
        rcs::utils::keycloak::KeycloakClient::new(keycloak_config)
            .await
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to initialize Keycloak client: {}", e),
                )
            })?,
    );

    // Load or generate session encryption key for OAuth session state storage
    // In production, load from a secure key management system (e.g., AWS KMS, HashiCorp Vault)
    let session_key = {
        use base64::Engine;
        
        match env::var("SESSION_ENCRYPTION_KEY") {
            Ok(key_b64) => {
                // Attempt to decode the base64-encoded 64-byte key
                let key_vec = base64::engine::general_purpose::STANDARD
                    .decode(&key_b64)
                    .map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("Failed to decode SESSION_ENCRYPTION_KEY from base64: {}", e),
                        )
                    })?;
                
                // Convert Vec<u8> to [u8; 64]
                let key_bytes: [u8; 64] = key_vec.try_into().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "SESSION_ENCRYPTION_KEY must be exactly 64 bytes when decoded from base64".to_string(),
                    )
                })?;
                
                actix_web::cookie::Key::from(&key_bytes)
            }
            Err(_) => {
                // Fallback to generation only in development
                let is_dev = env::var("APP_ENV").map(|v| v == "dev").unwrap_or(false);
                if is_dev {
                    log::warn!("SESSION_ENCRYPTION_KEY not set. Generating random key for development. DO NOT use in production.");
                    actix_web::cookie::Key::generate()
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "SESSION_ENCRYPTION_KEY must be set in production. Set APP_ENV=dev to generate a key for development.",
                    ));
                }
            }
        }
    };

    let manager = config::db::TenantPoolManager::new(main_pool.clone());
    // Hardcoded demo tenant; in production, load from the database
    manager
        .add_tenant_pool("tenant1".to_string(), main_pool.clone())
        .expect("Failed to add tenant pool");

    // Clone log_broadcaster for use in main server
    let main_broadcaster = log_broadcaster.clone();

    // Create and share a PureFunctionRegistry to encourage functional usage across middleware
    #[cfg(feature = "functional")]
    let pure_registry =
        Arc::new(rcs::functional::pure_function_registry::PureFunctionRegistry::new());

    // Compute allowed origins once; the closure clones the Vec per worker thread
    let allowed_origins = rcs::middleware::ws_security::get_allowed_origins();

    // Start the main HTTP server
    HttpServer::new(move || {
        let mut cors_builder = Cors::default();
        for origin in &allowed_origins {
            cors_builder = cors_builder.allowed_origin(origin);
        }

        // Configure allowed HTTP methods and headers
        cors_builder = cors_builder
            .allowed_methods(vec![
                http::Method::GET,
                http::Method::POST,
                http::Method::PUT,
                http::Method::DELETE,
                http::Method::OPTIONS,
            ])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
                http::header::HeaderName::from_static("x-tenant-id"),
            ])
            .expose_headers(vec![
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
                http::header::HeaderName::from_static("x-tenant-id"),
            ])
            .max_age(3600);

        // Apply credentials flag if configured
        let cors = if env::var("CORS_ALLOW_CREDENTIALS")
            .map(|v| v == "true")
            .unwrap_or(false)
        {
            cors_builder.supports_credentials()
        } else {
            cors_builder
        };

        let app = App::new()
            // Configure secure session middleware for OAuth state storage
            // Uses HttpOnly, Secure (HTTPS only in production), SameSite=Strict cookies
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), session_key.clone())
                    .session_lifecycle(
                        PersistentSession::default()
                            .session_ttl(Duration::seconds(600)) // 10 minute OAuth session TTL
                    )
                    .cookie_name("oauth_session".to_string())
                    .cookie_path("/api".to_string()) // Covers /api/auth/* and /api/callback
                    .cookie_http_only(true) // Prevent JavaScript access
                    .cookie_same_site(SameSite::Strict) // Prevent CSRF
                    // In production, set to true and ensure HTTPS
                    .cookie_secure(env::var("SESSION_COOKIE_SECURE")
                        .map(|v| v != "false")
                        .unwrap_or(true))
                    .build()
            )
            .app_data(web::Data::new(manager.clone()))
            .app_data(web::Data::new(main_pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .app_data(web::Data::new(main_broadcaster.clone()))
            .app_data(keycloak_client.clone())
            .wrap(tracing_actix_web::TracingLogger::default());

        #[cfg(feature = "functional")]
        let app = app.wrap(rcs::middleware::auth_middleware::functional_auth::FunctionalAuthentication::with_registry(pure_registry.clone()));

        // CORS must be applied LAST so it wraps all other middleware and executes FIRST
        // This ensures CORS headers are added to ALL responses including auth errors (401/403)
        app.configure(config::app::config_services)
            .wrap(cors)
    })
    .bind(&app_url)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use actix_cors::Cors;
    use actix_web::dev::Service;
    use actix_web::web;
    use actix_web::{http, App, HttpServer};
    use futures::FutureExt;
    use testcontainers::clients;
    use testcontainers::images::postgres::Postgres;
    use testcontainers::Container;

    use rcs::config;
    use rcs::utils::ws_logger::{init_websocket_logging, LogBroadcaster};

    fn try_run_postgres<'a>(docker: &'a clients::Cli) -> Option<Container<'a, Postgres>> {
        catch_unwind(AssertUnwindSafe(|| docker.run(Postgres::default()))).ok()
    }

    #[actix_web::test]
    #[cfg(feature = "functional")]
    async fn test_startup_ok() {
        use std::sync::Arc;
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_startup_ok because Docker is unavailable");
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
        config::db::run_migration(&mut pool.get().unwrap());

        // Initialize logging for tests
        let log_broadcaster = LogBroadcaster::new(100);
        init_websocket_logging(log_broadcaster.clone())
            .expect("failed to initialize websocket logging in test_startup_ok");

        let test_registry =
            Arc::new(rcs::functional::pure_function_registry::PureFunctionRegistry::new());

        HttpServer::new(move || {
            App::new()
                .wrap(
                    Cors::default() // allowed_origin return access-control-allow-origin: * by default
                        // .allowed_origin("http://127.0.0.1:8080")
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(log_broadcaster.clone()))
                .wrap(tracing_actix_web::TracingLogger::default())
                .wrap(rcs::middleware::auth_middleware::functional_auth::FunctionalAuthentication::with_registry(test_registry.clone()))
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(config::app::config_services)
        })
        .bind("localhost:8000".to_string())
        .unwrap()
        .run();

        // Test passes if server starts without panicking - HTTP binding success is the verification
    }

    /// Starts an Actix HTTP server configured with CORS and a database pool to verify it can start without authentication middleware.
    ///
    /// # Examples
    ///
    /// ```
    /// // This test starts a PostgreSQL test container, runs DB migrations,
    /// // and launches an Actix server bound to localhost:8001 with permissive CORS
    /// // and no authentication middleware to ensure startup succeeds.
    /// #[actix_web::test]
    /// async fn test_startup_without_auth_middleware_ok() {
    ///     // setup test Postgres, pool, migrations, and start server...
    /// }
    /// ```
    #[actix_web::test]
    async fn test_startup_without_auth_middleware_ok() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!(
                    "Skipping test_startup_without_auth_middleware_ok because Docker is unavailable"
                );
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
        config::db::run_migration(&mut pool.get().unwrap());

        // Initialize logging for tests
        let log_broadcaster = LogBroadcaster::new(100);
        init_websocket_logging(log_broadcaster.clone()).expect(
            "failed to initialize websocket logging in test_startup_without_auth_middleware_ok",
        );

        let _ = HttpServer::new(move || {
            App::new()
                .wrap(
                    Cors::default() // allowed_origin return access-control-allow-origin: * by default
                        // .allowed_origin("http://127.0.0.1:8080")
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(log_broadcaster.clone()))
                .wrap(tracing_actix_web::TracingLogger::default())
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(config::app::config_services)
        })
        .bind("localhost:8001".to_string())
        .unwrap()
        .run()
        .await;
    }

        // Test passes if server starts without panicking - HTTP binding success is the verification
}
