use std::sync::Arc;
use std::time::Duration;

use actix_cors::Cors;
use actix_session::config::PersistentSession;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::SameSite;
use actix_web::{http, web, App, HttpServer};

use crate::app_error::AppError;
use crate::config;
use crate::state::AppState;

pub async fn run_http_server(
    state: AppState,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), AppError> {
    let server_cfg = state.config().server.clone();
    let bind_addr = server_cfg.bind_addr();
    let app_state = state.clone();
    let app_server_cfg = server_cfg.clone();

    #[cfg(feature = "functional")]
    let pure_registry =
        Arc::new(crate::functional::pure_function_registry::PureFunctionRegistry::new());

    let http_server = HttpServer::new(move || {
        let state = app_state.clone();
        let allowed_origins = crate::middleware::ws_security::get_allowed_origins();
        let mut cors_builder = Cors::default();

        for origin in allowed_origins {
            cors_builder = cors_builder.allowed_origin(&origin);
        }

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

        let cors = if app_server_cfg.cors_allow_credentials {
            cors_builder.supports_credentials()
        } else {
            cors_builder
        };

        let app = App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), state.session_key())
                    .session_lifecycle(
                        PersistentSession::default()
                            .session_ttl(actix_web::cookie::time::Duration::seconds(
                                state.config().session.ttl_seconds,
                            )),
                    )
                    .cookie_name("oauth_session".to_string())
                    .cookie_path("/api".to_string())
                    .cookie_http_only(true)
                    .cookie_same_site(SameSite::Strict)
                    .cookie_secure(state.config().session.cookie_secure)
                    .build(),
            )
            .app_data(web::Data::new(state.tenant_manager()))
            .app_data(web::Data::new(state.main_pool()))
            .app_data(web::Data::new(state.redis_pool()))
            .app_data(web::Data::new(state.log_broadcaster()))
            .app_data(web::Data::new(state.keycloak_client()))
            .wrap(tracing_actix_web::TracingLogger::default());

        #[cfg(feature = "functional")]
        let app = app.wrap(
            crate::middleware::auth_middleware::functional_auth::FunctionalAuthentication::with_registry(
                pure_registry.clone(),
            ),
        );

        app.configure(config::app::config_services).wrap(cors)
    })
    .workers(server_cfg.workers.unwrap_or_else(num_cpus::get))
    .keep_alive(Duration::from_secs(server_cfg.keep_alive_seconds))
    .client_request_timeout(Duration::from_secs(
        server_cfg.client_request_timeout_seconds,
    ))
    .bind(&bind_addr)
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
    .run();

    let handle = http_server.handle();
    tokio::spawn(async move {
        if *shutdown.borrow() {
            handle.stop(true).await;
            return;
        }

        if shutdown.changed().await.is_err() {
            tracing::warn!("http shutdown watcher channel closed before change notification");
        }

        handle.stop(true).await;
    });

    http_server
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}
