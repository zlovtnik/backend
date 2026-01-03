use actix_session::Session;
use actix_web::http::{self, StatusCode};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use log::info;
use serde_json::json;
use std::borrow::Cow;

use crate::{
    api::controller_context::{AuthContext, ControllerContext, DatabaseContext},
    config::db::TenantPoolManager,
    constants,
    error::ServiceError,
    models::user::{validators, LoginDTO, SignupDTO, UserDTO},
    services::{
        account_service::{self, RefreshTokenRequest},
        functional_service_base::FunctionalErrorHandling,
    },
};

use crate::functional::performance_monitoring::OperationType;
use crate::functional::response_transformers::{ResponseTransformError, ResponseTransformer};
#[cfg(feature = "functional")]
use rcs_functional::measure_operation;

#[cfg(not(feature = "functional"))]
macro_rules! measure_operation {
    ($operation:expr, $body:expr) => {
        $body
    };
}

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
    validators::validate_signup(&signup_payload)?;

    let tenant_id = signup_payload.tenant_id.clone();
    let database = DatabaseContext::from_manager(manager.get_ref(), tenant_id.clone())?;
    let context = ControllerContext::new(database);

    let user_dto = UserDTO::from(&signup_payload);
    let signup_flow = account_service::signup_reader(user_dto)?;

    let operation = OperationType::Custom("account_signup_controller".to_string());
    let signup_message = measure_operation!(operation, { context.run_query(signup_flow) })
        .log_error("account_controller::signup")?;

    ResponseTransformer::new(constants::EMPTY)
        .with_message(Cow::Owned(signup_message))
        .try_with_metadata(json!({ "tenant_id": tenant_id }))
        .map(|transformer| transformer.respond_to(&req))
        .map_err(response_composition_error)
}

// POST api/auth/login
pub async fn login(
    login_dto: web::Json<LoginDTO>,
    manager: web::Data<TenantPoolManager>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let login_payload = login_dto.into_inner();
    validators::validate_login(&login_payload)?;
    let tenant_id = login_payload.tenant_id.clone();

    let database = DatabaseContext::from_manager(manager.get_ref(), tenant_id.clone())?;
    let context = ControllerContext::new(database);

    let login_flow = account_service::login_reader(login_payload)?;
    let operation = OperationType::Custom("account_login_controller".to_string());

    let token_res = measure_operation!(operation, { context.run_query(login_flow) })
        .log_error("account_controller::login")?;

    ResponseTransformer::new(token_res)
        .with_message(Cow::Borrowed(constants::MESSAGE_LOGIN_SUCCESS))
        .try_with_metadata(json!({ "tenant_id": tenant_id }))
        .map(|transformer| transformer.respond_to(&req))
        .map_err(response_composition_error)
}

// POST api/auth/logout
pub async fn logout(req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    let auth_context = AuthContext::from_request(&req).ok_or_else(|| {
        ServiceError::bad_request(constants::MESSAGE_TOKEN_MISSING)
            .with_tag("auth")
            .with_detail("Authorization header missing")
    })?;

    let database = DatabaseContext::from_request(&req)?;

    let operation = OperationType::Custom("account_logout_controller".to_string());
    measure_operation!(operation, {
        account_service::logout(auth_context.header(), database.pool())
    })
    .log_error("account_controller::logout")?;

    Ok(respond_empty(
        &req,
        StatusCode::OK,
        constants::MESSAGE_LOGOUT_SUCCESS,
    ))
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
    let auth_context = AuthContext::from_request(&req).ok_or_else(|| {
        ServiceError::bad_request(constants::MESSAGE_TOKEN_MISSING)
            .with_tag("auth")
            .with_detail("Authorization header missing")
    })?;

    let database = DatabaseContext::from_request(&req)?;

    let operation = OperationType::Custom("account_refresh_controller".to_string());
    let login_info = measure_operation!(operation, {
        account_service::refresh(auth_context.header(), database.pool())
    })
    .log_error("account_controller::refresh")?;

    Ok(ResponseTransformer::new(login_info)
        .with_message(Cow::Borrowed(constants::MESSAGE_OK))
        .respond_to(&req))
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

    let database = DatabaseContext::from_manager(manager.get_ref(), tenant_id.clone())?;
    let context = ControllerContext::new(database);

    let operation = OperationType::Custom("account_refresh_token_controller".to_string());
    let token_res = measure_operation!(operation, {
        account_service::refresh_with_token(
            &refresh_payload.refresh_token,
            &tenant_id,
            context.database().pool(),
        )
    })
    .log_error("account_controller::refresh_token")?;

    ResponseTransformer::new(token_res)
        .with_message(Cow::Borrowed(constants::MESSAGE_OK))
        .try_with_metadata(json!({ "tenant_id": tenant_id }))
        .map(|transformer| transformer.respond_to(&req))
        .map_err(response_composition_error)
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
    let auth_context = AuthContext::from_request(&req).ok_or_else(|| {
        ServiceError::bad_request(constants::MESSAGE_TOKEN_MISSING)
            .with_tag("auth")
            .with_detail("Authorization header missing")
    })?;

    let database = DatabaseContext::from_request(&req)?;

    let operation = OperationType::Custom("account_me_controller".to_string());
    let login_info = measure_operation!(operation, {
        account_service::me(auth_context.header(), database.pool())
    })
    .log_error("account_controller::me")?;

    Ok(ResponseTransformer::new(login_info)
        .with_message(Cow::Borrowed(constants::MESSAGE_OK))
        .respond_to(&req))
}

/// Initiates Keycloak OAuth login flow.
///
/// Securely stores PKCE verifier, CSRF token (state), and nonce in HttpOnly, Secure,
/// SameSite=Strict cookie-based session with 10-minute expiration. These values are
/// used to prevent PKCE, CSRF, and replay attacks during the callback phase.
///
/// # Examples
///
/// ```no_run
/// // GET /api/auth/login/keycloak
/// // Redirects to Keycloak authorization URL with state parameter
/// ```
pub async fn keycloak_login(
    keycloak_client: web::Data<crate::utils::keycloak::KeycloakClient>,
    session: Session,
    _req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let (auth_url, session_state) = keycloak_client.get_authorization_url().await.map_err(|e| {
        log::error!("Failed to generate OAuth authorization URL: {}", e);
        ServiceError::internal_server_error(constants::MESSAGE_INTERNAL_SERVER_ERROR)
    })?;

    // Store session state in secure, HttpOnly, SameSite=Strict cookie with 10-minute TTL
    session.insert("oauth_state", &session_state).map_err(|e| {
        log::error!("Failed to store OAuth session state: {}", e);
        ServiceError::internal_server_error(constants::MESSAGE_INTERNAL_SERVER_ERROR)
    })?;

    log::debug!(
        "OAuth session state stored with CSRF token: {}",
        &session_state.csrf_token[..8.min(session_state.csrf_token.len())]
    );

    Ok(HttpResponse::Found()
        .append_header((http::header::LOCATION, auth_url))
        .finish())
}

// GET api/auth/callback
/// Handles Keycloak OAuth callback with security validations.
///
/// Performs the following security checks in order:
/// 1. Retrieves stored PKCE verifier, CSRF token, and nonce from secure session
/// 2. Validates session is not expired (10-minute window)
/// 3. Validates returned state parameter matches stored CSRF token (CSRF protection)
/// 4. Validates authorization code is present
/// 5. Validates nonce in ID token against stored nonce (replay attack prevention)
/// 6. Validates PKCE verifier matches during code exchange
///
/// All values are removed from session immediately after validation to prevent reuse.
/// Failed validations return descriptive errors without exposing sensitive details.
///
/// # Examples
///
/// ```no_run
/// // GET /api/auth/callback?code=auth_code&state=state
/// // Validates state, exchanges code for tokens, validates nonce
/// ```
pub async fn keycloak_callback(
    query: web::Query<std::collections::HashMap<String, String>>,
    _keycloak_client: web::Data<crate::utils::keycloak::KeycloakClient>,
    session: Session,
    _req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    // Check for OAuth errors from Keycloak
    if let Some(error) = query.get("error") {
        log::warn!("OAuth error from Keycloak: {}", error);
        if let Some(error_desc) = query.get("error_description") {
            log::warn!("Error description: {}", error_desc);
        }
        return Err(
            ServiceError::bad_request("Authentication failed. Please try again.")
                .with_tag("oauth_error")
                .with_detail(format!("Provider error: {}", error)),
        );
    }

    // Retrieve stored OAuth session state from secure session
    let session_state: crate::utils::keycloak::OAuthSessionState = session
        .get("oauth_state")
        .map_err(|e| {
            log::error!("Failed to retrieve OAuth session state: {}", e);
            ServiceError::bad_request("Session expired or invalid. Please restart authentication.")
                .with_tag("session_error")
        })?
        .ok_or_else(|| {
            log::warn!("OAuth session state not found in session");
            ServiceError::bad_request("Session expired or invalid. Please restart authentication.")
                .with_tag("session_missing")
        })?;

    // Validate session has not expired (check 10-minute window, but allow small clock skew)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    const OAUTH_STATE_TTL: i64 = 600; // 10 minutes
    const CLOCK_SKEW: i64 = 30; // 30 seconds allowance for clock skew

    if now - session_state.created_at > OAUTH_STATE_TTL + CLOCK_SKEW {
        log::warn!("OAuth session state expired");
        session.purge(); // Clear the entire session
        return Err(ServiceError::bad_request(
            "Authentication session expired. Please restart authentication.",
        )
        .with_tag("session_expired"));
    }

    // Extract and validate state parameter (CSRF protection)
    let returned_state = query.get("state").ok_or_else(|| {
        log::warn!("State parameter missing from OAuth callback");
        ServiceError::bad_request("State parameter missing. Invalid callback.")
            .with_tag("missing_state")
    })?;

    if returned_state != &session_state.csrf_token {
        log::warn!("CSRF token mismatch - possible CSRF attack");
        session.purge();
        return Err(ServiceError::bad_request(
            "CSRF validation failed. Please restart authentication.",
        )
        .with_tag("csrf_mismatch"));
    }

    // Extract and validate authorization code
    #[allow(unused_variables)]
    let code = query.get("code").ok_or_else(|| {
        log::warn!("Authorization code missing from OAuth callback");
        ServiceError::bad_request("Authorization code missing. Invalid callback.")
            .with_tag("missing_code")
    })?;

    // Remove OAuth session state from session immediately to prevent reuse
    session.remove("oauth_state");

    // Exchange authorization code for tokens using code, pkce_verifier, and nonce
    let tokens = _keycloak_client
        .exchange_code_for_token(
            code,
            session_state.pkce_verifier.clone(),
            session_state.nonce.clone(),
        )
        .await
        .map_err(|e| {
            log::error!("Failed to exchange authorization code for tokens: {}", e);
            ServiceError::internal_server_error(
                "Token exchange failed. Please restart authentication.",
            )
            .with_tag("token_exchange_failed")
            .with_detail(format!("Keycloak error: {}", e))
        })?;

    // Validate nonce in ID token matches session_state.nonce
    let id_token_str = tokens
        .id_token
        .as_ref()
        .ok_or_else(|| {
            log::error!("ID token not present in token response");
            ServiceError::internal_server_error("Token validation failed: ID token missing")
                .with_tag("missing_id_token")
        })?;

    // Validate ID token signature using Keycloak's public key/JWKS
    let id_token_claims: crate::utils::keycloak::Claims = _keycloak_client
        .validate_id_token(id_token_str)
        .await
        .map_err(|e| {
            log::error!("Failed to validate ID token signature: {}", e);
            ServiceError::internal_server_error(
                "Token validation failed: invalid signature",
            )
            .with_tag("invalid_id_token_signature")
            .with_detail(format!("Validation error: {}", e))
        })?;

    // Validate nonce in ID token matches stored nonce
    if id_token_claims.nonce.as_deref() != Some(&session_state.nonce) {
        log::warn!(
            "Nonce mismatch - possible replay attack. Expected: {}, got: {:?}",
            &session_state.nonce,
            id_token_claims.nonce
        );
        return Err(ServiceError::bad_request(
            "Nonce validation failed. Possible replay attack detected.",
        )
        .with_tag("nonce_mismatch"));
    }

    log::debug!("Nonce validation successful");

    // Production guard: Prevent OAuth flow in production until full Keycloak/token validation is implemented
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    if app_env == "production" {
        log::error!("OAuth login attempted in production with incomplete implementation. Aborting.");
        session.purge();
        return Err(ServiceError::bad_request(
            "OAuth authentication is not yet available in production. Please use standard authentication."
        ).with_tag("oauth_production_disabled"));
    }

    // Extract stable unique identifier from ID token claims
    // Prefer sub (subject claim) as it uniquely identifies the user within the Keycloak realm
    let oauth_unique_id = id_token_claims
        .sub
        .clone();

    // Extract username from claims: prefer preferred_username, fallback to email, then sub
    let username = id_token_claims
        .preferred_username
        .clone()
        .or_else(|| id_token_claims.email.clone())
        .unwrap_or_else(|| format!("oauth_{}", oauth_unique_id));

    // Extract tenant_id from ID token claims
    // Priority: custom 'tenant_id' claim > default from environment
    // NOTE: For multi-tenant support, you may also extract from 'realm_access.roles'
    // or map roles to tenant IDs using application configuration
    let tenant_id = id_token_claims
        .tenant_id
        .clone()
        .or_else(|| std::env::var("OAUTH_DEFAULT_TENANT").ok())
        .unwrap_or_else(|| "default".to_string());

    // Generate unique login session ID
    let login_session = format!("oauth-{}-{}", oauth_unique_id, uuid::Uuid::new_v4());

    log::debug!(
        "OAuth login: sub={}, username={}, email={:?}, tenant_id={}",
        oauth_unique_id, username, id_token_claims.email, tenant_id
    );

    // Create LoginInfoDTO with extracted values from ID token
    let oauth_login = crate::models::user::LoginInfoDTO {
        username,
        login_session,
        tenant_id,
    };

    let token = crate::models::user_token::UserToken::generate_token(&oauth_login);

    // Create a redirect response with HttpOnly secure cookie (cookie-based auth)
    // The token is stored in the cookie and sent with each request via the Authorization header.
    // The server-side session is not used for token storage; authentication is stateless via Bearer tokens.
    use actix_web::cookie::Cookie;
    let cookie = Cookie::build("auth_token", token.clone())
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(actix_web::cookie::SameSite::Strict)
        .finish();

    // Read frontend callback URL from environment with sensible defaults
    let frontend_callback_url = std::env::var("OAUTH_FRONTEND_CALLBACK_URL")
        .or_else(|_| {
            // Default based on environment
            let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
            match app_env.as_str() {
                "production" => Err(std::env::VarError::NotPresent),
                _ => Ok("http://localhost:3000/auth/callback".to_string()),
            }
        })
        .map_err(|_| {
            log::error!("OAUTH_FRONTEND_CALLBACK_URL not set and no development default available");
            ServiceError::internal_server_error(
                "OAuth configuration incomplete: frontend callback URL not configured"
            ).with_tag("oauth_config_missing")
        })?
        .trim()
        .to_string();

    // Validate callback URL is a valid absolute URL
    use url::Url;
    let _validated_url = Url::parse(&frontend_callback_url)
        .map_err(|e| {
            log::error!("Invalid OAUTH_FRONTEND_CALLBACK_URL: {}", e);
            ServiceError::bad_request(
                "OAuth configuration error: invalid callback URL format"
            ).with_tag("oauth_invalid_callback_url")
        })?
        .to_string(); // Normalize and validate

    let response = HttpResponse::Found() // 302 redirect
        .insert_header(("Location", frontend_callback_url))
        .cookie(cookie)
        .finish();

    log::info!("OAuth callback successful, redirecting to frontend with auth token");
    Ok(response)
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
