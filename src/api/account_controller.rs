use actix_web::http::StatusCode;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use log::info;
use serde_json::json;
use std::borrow::Cow;

use crate::{
    config::db::{Pool, TenantPoolManager},
    constants,
    error::ServiceError,
    functional::response_transformers::{ResponseTransformError, ResponseTransformer},
    models::user::{LoginDTO, SignupDTO, UserDTO},
    services::{
        account_service::{self, RefreshTokenRequest},
        functional_service_base::FunctionalErrorHandling,
    },
};

fn response_composition_error(err: ResponseTransformError) -> ServiceError {
    ServiceError::internal_server_error(constants::MESSAGE_INTERNAL_SERVER_ERROR)
        .with_tag("response")
        .with_detail(err.to_string())
}

fn respond_empty(req: &HttpRequest, status: StatusCode, message: &str) -> HttpResponse {
    ResponseTransformer::new(constants::EMPTY)
        .with_message(Cow::Owned(message.to_string()))
        .with_status(status)
        .respond_to(req)
}

/// Extracts the tenant Pool stored in the request's extensions.
///
/// # Returns
///
/// `Ok(pool)` with a clone of the tenant `Pool` if present in the request extensions, `Err(ServiceError::BadRequest)` with message `"Tenant not found"` otherwise.
///
/// # Examples
///
/// ```no_run
/// use actix_web::HttpRequest;
/// // assume `Pool` and `ServiceError` are in scope
/// // let req: HttpRequest = /* request with Pool inserted into extensions */ ;
/// // let pool = extract_tenant_pool(&req)?;
/// ```
fn extract_tenant_pool(req: &HttpRequest) -> Result<Pool, ServiceError> {
    match req.extensions().get::<Pool>() {
        Some(pool) => Ok(pool.clone()),
        None => Err(ServiceError::bad_request("Tenant not found")
            .with_tag("tenant")
            .with_detail("Missing tenant pool in request extensions")),
    }
}

/// Process a tenant-scoped user signup and produce an HTTP response.
///
/// On success returns an `HttpResponse::Ok` with a JSON `ResponseBody` containing the signup message and an empty payload.
/// Returns `Err(ServiceError)` when the tenant cannot be found or when the account service returns an error.
///
/// # Examples
///
/// ```no_run
/// use actix_web::web;
///
/// // Assume `signup_dto` and `manager` are prepared appropriately in an async context.
/// // let resp = signup(web::Json(signup_dto), web::Data::new(manager)).await;
/// ```
pub async fn signup(
    user_dto: web::Json<SignupDTO>,
    manager: web::Data<TenantPoolManager>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    info!("Processing signup request");

    let signup_payload = user_dto.into_inner();
    let tenant_id = signup_payload.tenant_id.clone();
    let user_db = UserDTO {
        username: signup_payload.username,
        email: signup_payload.email,
        password: signup_payload.password,
        active: true, // New users are active by default
    };

    match manager.get_tenant_pool(&tenant_id) {
        Some(pool) => {
            let tenant_metadata = tenant_id.clone();
            account_service::signup(user_db, &pool)
                .log_error("account_controller::signup")
                .and_then(|message| {
                    ResponseTransformer::new(constants::EMPTY)
                        .with_message(Cow::Owned(message))
                        .try_with_metadata(json!({ "tenant_id": tenant_metadata }))
                        .map(|transformer| transformer.respond_to(&req))
                        .map_err(response_composition_error)
                })
        }
        None => Err(ServiceError::bad_request("Tenant not found")
            .with_metadata("tenant_id", tenant_id)
            .with_tag("tenant")),
    }
}

// POST api/auth/login
pub async fn login(
    login_dto: web::Json<LoginDTO>,
    manager: web::Data<TenantPoolManager>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let login_payload = login_dto.into_inner();
    let tenant_id = login_payload.tenant_id.clone();

    if let Some(pool) = manager.get_tenant_pool(&tenant_id) {
        let tenant_metadata = tenant_id.clone();
        account_service::login(login_payload, &pool)
            .log_error("account_controller::login")
            .and_then(|token_res| {
                ResponseTransformer::new(token_res)
                    .with_message(Cow::Borrowed(constants::MESSAGE_LOGIN_SUCCESS))
                    .try_with_metadata(json!({ "tenant_id": tenant_metadata }))
                    .map(|transformer| transformer.respond_to(&req))
                    .map_err(response_composition_error)
            })
    } else {
        Err(ServiceError::bad_request("Tenant not found")
            .with_tag("tenant")
            .with_detail("Tenant pool missing for login request"))
    }
}

// POST api/auth/logout
pub async fn logout(req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    if let Some(authen_header) = req.headers().get(constants::AUTHORIZATION) {
        let pool = extract_tenant_pool(&req)?;
        account_service::logout(authen_header, &pool)
            .log_error("account_controller::logout")
            .map(|_| respond_empty(&req, StatusCode::OK, constants::MESSAGE_LOGOUT_SUCCESS))
    } else {
        Err(ServiceError::bad_request(constants::MESSAGE_TOKEN_MISSING)
            .with_tag("auth")
            .with_detail("Authorization header missing"))
    }
}

/// Refresh the authentication state and produce updated login information.
///
/// Requires an `Authorization` header on `req` and a tenant `Pool` stored in the request's extensions.
/// On success this returns an `HttpResponse` with a JSON body containing the refreshed `LoginInfo`.
/// If the `Authorization` header is missing the function yields `ServiceError::BadRequest`; other `ServiceError`s
/// returned by the refresh operation are propagated.
///
/// # Examples
///
/// ```rust
/// use actix_web::test::TestRequest;
/// # async fn run() {
/// let req = TestRequest::default().to_http_request();
/// let _ = crate::handlers::refresh(req).await;
/// # }
/// ```
pub async fn refresh(req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    if let Some(authen_header) = req.headers().get(constants::AUTHORIZATION) {
        let pool = extract_tenant_pool(&req)?;
        account_service::refresh(authen_header, &pool)
            .log_error("account_controller::refresh")
            .map(|login_info| {
                ResponseTransformer::new(login_info)
                    .with_message(Cow::Borrowed(constants::MESSAGE_OK))
                    .respond_to(&req)
            })
    } else {
        Err(ServiceError::bad_request(constants::MESSAGE_TOKEN_MISSING)
            .with_tag("auth")
            .with_detail("Authorization header missing"))
    }
}

// POST api/auth/refresh-token
/// Refreshes access and refresh tokens using a valid refresh token.
///
/// Requires a JSON body with `refresh_token` and `tenant_id`. On success returns an HTTP 200 response
/// with a JSON body containing new access_token and refresh_token.
/// If the refresh token is invalid or expired, returns an unauthorized error.
///
/// # Examples
///
/// ```no_run
/// use actix_web::web;
/// use serde_json::json;
///
/// // POST /api/auth/refresh-token with body: {"refresh_token": "token", "tenant_id": "tenant1"}
/// ```
pub async fn refresh_token(
    refresh_dto: web::Json<RefreshTokenRequest>,
    manager: web::Data<TenantPoolManager>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    log::debug!("refresh_token controller called");
    let refresh_payload = refresh_dto.into_inner();
    let tenant_id = refresh_payload.tenant_id;

    if let Some(pool) = manager.get_tenant_pool(&tenant_id) {
        account_service::refresh_with_token(&refresh_payload.refresh_token, &tenant_id, &pool)
            .log_error("account_controller::refresh_token")
            .and_then(|token_res| {
                ResponseTransformer::new(token_res)
                    .with_message(Cow::Borrowed(constants::MESSAGE_OK))
                    .try_with_metadata(json!({ "tenant_id": tenant_id }))
                    .map(|transformer| transformer.respond_to(&req))
                    .map_err(response_composition_error)
            })
    } else {
        Err(ServiceError::bad_request("Tenant not found")
            .with_tag("tenant")
            .with_detail("Tenant pool missing for refresh token request"))
    }
}

// GET api/auth/me
/// Returns the authenticated user's login information from the incoming request.
///
/// Requires an `Authorization` header and a tenant `Pool` stored in the request extensions. On success returns an HTTP 200 response with a JSON `ResponseBody` whose message is `constants::MESSAGE_OK` and whose payload is the user's login information.
///
/// # Errors
///
/// Returns a `ServiceError` if the authorization token is missing, the tenant pool cannot be resolved, or the account service returns an error.
///
/// # Examples
///
/// ```no_run
/// use actix_web::HttpRequest;
///
/// // Prepare an HttpRequest containing an Authorization header and a tenant Pool in extensions,
/// // then call `me(req).await` to retrieve the current user's login info.
/// // (Test setup and tenant pool insertion are omitted for brevity.)
///
/// // let resp = actix_web::rt::System::new().block_on(async { me(req).await });
/// ```
pub async fn me(req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    if let Some(authen_header) = req.headers().get(constants::AUTHORIZATION) {
        let pool = extract_tenant_pool(&req)?;
        account_service::me(authen_header, &pool)
            .log_error("account_controller::me")
            .map(|login_info| {
                ResponseTransformer::new(login_info)
                    .with_message(Cow::Borrowed(constants::MESSAGE_OK))
                    .respond_to(&req)
            })
    } else {
        Err(ServiceError::bad_request(constants::MESSAGE_TOKEN_MISSING)
            .with_tag("auth")
            .with_detail("Authorization header missing"))
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use actix_cors::Cors;
    use actix_web::dev::Service;
    use actix_web::web;
    use actix_web::{http, http::StatusCode, test};
    use futures::FutureExt;
    use http::header;
    use testcontainers::clients;
    use testcontainers::images::postgres::Postgres;
    use testcontainers::Container;

    use crate::config;
    use crate::config::db::{Pool, TenantPoolManager};
    use actix_web::App;

    fn try_run_postgres<'a>(docker: &'a clients::Cli) -> Option<Container<'a, Postgres>> {
        catch_unwind(AssertUnwindSafe(|| docker.run(Postgres::default()))).ok()
    }

    fn ensure_migrations(pool: &Pool, test_name: &str) -> bool {
        match pool.get() {
            Ok(mut conn) => match config::db::run_migration(&mut conn) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("Skipping {test_name} because migration failed: {e}");
                    false
                }
            },
            Err(e) => {
                eprintln!("Skipping {test_name} because DB pool unavailable: {e}");
                false
            }
        }
    }

    #[actix_web::test]
    async fn test_signup_ok() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_signup_ok because Docker is unavailable");
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
        match pool.get() {
            Ok(mut conn) => {
                if let Err(e) = config::db::run_migration(&mut conn) {
                    eprintln!("Skipping test: Migration failed: {}", e);
                    return;
                }
            }
            Err(e) => {
                eprintln!("Skipping test: DB pool unavailable: {}", e);
                return;
            }
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("test".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        let resp = test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"TestPass123","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        // let data = test::read_body(resp).await;

        // println!("{:#?}", &data);
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_signup_duplicate_user() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_signup_duplicate_user because Docker is unavailable");
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
        match pool.get() {
            Ok(mut conn) => {
                if let Err(e) = config::db::run_migration(&mut conn) {
                    eprintln!("Skipping test: Migration failed: {}", e);
                    return;
                }
            }
            Err(e) => {
                eprintln!("Skipping test: DB pool unavailable: {}", e);
                return;
            }
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("test".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"123456","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        let resp = test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"123456","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        // let data = test::read_body(resp).await;

        // println!("{:#?}", &data);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_login_ok_with_username() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_login_ok_with_username because Docker is unavailable");
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
        match pool.get() {
            Ok(mut conn) => {
                if let Err(e) = config::db::run_migration(&mut conn) {
                    eprintln!("Skipping test: Migration failed: {}", e);
                    return;
                }
            }
            Err(e) => {
                eprintln!("Skipping test: DB pool unavailable: {}", e);
                return;
            }
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("test".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"TestPass123","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        let resp = test::TestRequest::post()
            .uri("/api/auth/login")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username_or_email":"admin","password":"TestPass123","tenant_id":"test"}"#
                    .as_bytes(),
            )
            .send_request(&app)
            .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_login_ok_with_email() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_login_ok_with_email because Docker is unavailable");
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
        if !ensure_migrations(&pool, "test_signup_ok") {
            return;
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("test".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"TestPass123","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        let resp = test::TestRequest::post()
            .uri("/api/auth/login")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username_or_email":"admin@gmail.com","password":"TestPass123","tenant_id":"test"}"#
                    .as_bytes(),
            )
            .send_request(&app)
            .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_login_password_incorrect_with_username() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!(
                    "Skipping test_login_password_incorrect_with_username because Docker is unavailable"
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
        if !ensure_migrations(&pool, "test_login_password_incorrect_with_username") {
            return;
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("test".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"123456","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        let resp = test::TestRequest::post()
            .uri("/api/auth/login")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username_or_email":"admin","password":"password","tenant_id":"test"}"#
                    .as_bytes(),
            )
            .send_request(&app)
            .await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_login_password_incorrect_with_email() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!(
                    "Skipping test_login_password_incorrect_with_email because Docker is unavailable"
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
        if !ensure_migrations(&pool, "test_login_password_incorrect_with_email") {
            return;
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("test".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"123456","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        let resp = test::TestRequest::post()
            .uri("/api/auth/login")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username_or_email":"admin@gmail.com","password":"password","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_login_user_not_found_with_username() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!(
                    "Skipping test_login_user_not_found_with_username because Docker is unavailable"
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
        if !ensure_migrations(&pool, "test_login_user_not_found_with_username") {
            return;
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("test".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"password","tenant_id":"test"}"#
                    .as_bytes(),
            )
            .send_request(&app)
            .await;

        let resp = test::TestRequest::post()
            .uri("/api/auth/login")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username_or_email":"abc","password":"123456","tenant_id":"test"}"#.as_bytes(),
            )
            .send_request(&app)
            .await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_login_user_not_found_with_email() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!(
                    "Skipping test_login_user_not_found_with_email because Docker is unavailable"
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
        if !ensure_migrations(&pool, "test_login_user_not_found_with_email") {
            return;
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("test".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        test::TestRequest::post()
            .uri("/api/auth/signup")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username":"admin","email":"admin@gmail.com","password":"password","tenant_id":"test"}"#
                    .as_bytes(),
            )
            .send_request(&app)
            .await;

        let resp = test::TestRequest::post()
            .uri("/api/auth/login")
            .insert_header(header::ContentType::json())
            .set_payload(
                r#"{"username_or_email":"abc@gmail.com","password":"123456","tenant_id":"test"}"#
                    .as_bytes(),
            )
            .send_request(&app)
            .await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
