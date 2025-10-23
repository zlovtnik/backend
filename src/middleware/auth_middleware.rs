use actix_service::forward_ready;
use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::{
    header::{HeaderName, HeaderValue},
    Method,
};
use actix_web::web::Data;
use actix_web::Error;
use actix_web::HttpMessage;
use actix_web::HttpResponse;
use futures::future::{ok, LocalBoxFuture, Ready};
use log::{error, info};

use crate::config::db::TenantPoolManager;
use crate::constants;
use crate::models::response::ResponseBody;
use crate::utils::token_utils;

pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddleware { service })
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    /// Authenticate a ServiceRequest and either forward it to the inner service or return a 401 Unauthorized response.
    ///
    /// On success, forwards the (possibly augmented) request to the wrapped service and returns its response as the left variant of `EitherBody`. On failure, returns a 401 Unauthorized JSON response as the right variant.
    ///
    /// # Returns
    ///
    /// A future resolving to a `ServiceResponse` whose body is `EitherBody<B>`: the left variant contains the inner service response when authentication succeeds, and the right variant contains a 401 Unauthorized JSON payload when authentication fails.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given `middleware` that wraps an inner service and a `req: ServiceRequest`:
    /// // let fut = middleware.call(req);
    /// // let res = futures::executor::block_on(fut).unwrap();
    /// // match res.into_body() {
    /// //     EitherBody::Left(body) => { /* authenticated path */ },
    /// //     EitherBody::Right(body) => { /* unauthorized response */ },
    /// // }
    /// ```
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let mut authenticate_pass: bool = false;

        // Let CORS middleware handle preflight requests without auth checks
        if Method::OPTIONS == *req.method() {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) });
        }

        // Check if route should be bypassed (no authentication required)
        let path = req.path();
        if constants::IGNORE_ROUTES
            .iter()
            .any(|route| path.starts_with(route))
        {
            authenticate_pass = true;
        }

        // Bypass some account routes
        let mut headers = req.headers().clone();
        headers.append(
            HeaderName::from_static("content-length"),
            HeaderValue::from_static("true"),
        );

        if !authenticate_pass {
            if let Some(manager) = req.app_data::<Data<TenantPoolManager>>() {
                if let Some(authen_header) = req.headers().get(constants::AUTHORIZATION) {
                    info!("Parsing authorization header...");
                    if let Ok(authen_str) = authen_header.to_str() {
                        if authen_str.starts_with("bearer") || authen_str.starts_with("Bearer") {
                            if authen_str.len() <= 7 {
                                error!("Authorization header missing bearer token");
                            } else {
                                let token = authen_str[7..].trim();
                                if let Ok(token_data) = token_utils::decode_token(token.to_string())
                                {
                                    info!("Decoding token...");
                                    if let Some(tenant_pool) =
                                        manager.get_tenant_pool(&token_data.claims.tenant_id)
                                    {
                                        if token_utils::verify_token(&token_data, &tenant_pool)
                                            .is_ok()
                                        {
                                            info!("Valid token");
                                            req.extensions_mut().insert(tenant_pool.clone());
                                            authenticate_pass = true;
                                        } else {
                                            error!("Invalid token");
                                        }
                                    } else {
                                        error!("Tenant not found");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !authenticate_pass {
            let (request, _pl) = req.into_parts();
            let response = HttpResponse::Unauthorized()
                .json(ResponseBody::new(
                    constants::MESSAGE_INVALID_TOKEN,
                    constants::EMPTY,
                ))
                .map_into_right_body();

            return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
        }

        let fut = self.service.call(req);

        Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) })
    }
}

// ===== FUNCTIONAL AUTHENTICATION MIDDLEWARE =====

#[cfg(feature = "functional")]
mod functional_auth {
    use super::*;

    use crate::functional::pure_function_registry::PureFunctionRegistry;
    use actix_web::body::BoxBody;
    use std::sync::Arc;

    /// Enhanced authentication middleware using functional programming patterns
    /// Provides composable middleware components with pure functions and zero-allocation processing
    pub struct FunctionalAuthentication {
        registry: Option<Arc<PureFunctionRegistry>>,
    }

    impl FunctionalAuthentication {
        /// Creates a FunctionalAuthentication with no registry configured.
        ///
        /// # Examples
        ///
        /// ```
        /// let auth = FunctionalAuthentication::new();
        /// assert!(auth.registry.is_none());
        /// ```
        pub fn new() -> Self {
            Self { registry: None }
        }

        /// Constructs a `FunctionalAuthentication` configured to use the provided pure-function registry.
        ///
        /// The created instance will use the given `registry` to register pure functions (if present)
        /// when authentication succeeds.
        ///
        /// # Parameters
        ///
        /// - `registry`: Shared `Arc` to a `PureFunctionRegistry` whose functions will be registered during authentication.
        ///
        /// # Returns
        ///
        /// A `FunctionalAuthentication` that will use the provided registry.
        ///
        /// # Examples
        ///
        /// ```
        /// use std::sync::Arc;
        /// // Assume PureFunctionRegistry::new() is available in scope.
        /// let registry = Arc::new(PureFunctionRegistry::new());
        /// let auth = FunctionalAuthentication::with_registry(registry.clone());
        /// ```
        pub fn with_registry(registry: Arc<PureFunctionRegistry>) -> Self {
            Self {
                registry: Some(registry),
            }
        }

        /// Returns true if this authentication has a registry configured.
        pub fn has_registry(&self) -> bool {
            self.registry.is_some()
        }
    }

    impl Default for FunctionalAuthentication {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<S, B> Transform<S, ServiceRequest> for FunctionalAuthentication
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
    {
        type Response = ServiceResponse<EitherBody<B>>;
        type Error = Error;
        type InitError = ();
        type Transform = FunctionalAuthenticationMiddleware<S>;
        type Future = Ready<Result<Self::Transform, Self::InitError>>;

        /// Create a FunctionalAuthentication middleware that wraps the provided inner service and clones the optional function registry.
        ///
        /// The returned ready future resolves to a `FunctionalAuthenticationMiddleware` which will forward requests to `service` and use a cloned copy of `self.registry` for optional pure-function registration.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use futures::executor::block_on;
        /// # use crate::middleware::auth::FunctionalAuthentication;
        /// let auth = FunctionalAuthentication::new();
        /// let service = /* an inner service */ ();
        /// let middleware = block_on(auth.new_transform(service)).unwrap();
        /// ```
        fn new_transform(&self, service: S) -> Self::Future {
            ok(FunctionalAuthenticationMiddleware {
                service,
                registry: self.registry.clone(),
            })
        }
    }

    pub struct FunctionalAuthenticationMiddleware<S> {
        service: S,
        registry: Option<Arc<PureFunctionRegistry>>,
    }

    impl<S, B> Service<ServiceRequest> for FunctionalAuthenticationMiddleware<S>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
    {
        type Response = ServiceResponse<EitherBody<B>>;
        type Error = Error;
        type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

        forward_ready!(service);

        /// Handles an incoming request through the middleware's functional authentication flow and forwards it to the inner service on success.
        ///
        /// The middleware skips authentication for OPTIONS and configured ignore routes. On successful authentication it inserts the tenant pool into the request's extensions, optionally registers pure functions when a registry is present, and forwards the request to the inner service. On failure or when required application data is missing it produces a 401 Unauthorized JSON response.
        ///
        /// # Returns
        ///
        /// A `Result` containing a `ServiceResponse` whose body is an `EitherBody`:
        /// - `Left` when the request was forwarded to the inner service (preserving the inner body),
        /// - `Right` when an unauthorized 401 JSON response was produced.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use actix_web::dev::ServiceRequest;
        /// # use std::sync::Arc;
        /// # async fn example(mut middleware_call: impl FnOnce(ServiceRequest) -> _ , req: ServiceRequest) {
        /// let fut = middleware_call(req);
        /// let _res = fut.await;
        /// # }
        /// ```
        fn call(&self, req: ServiceRequest) -> Self::Future {
            let registry = self.registry.clone();

            if Self::should_skip_authentication(&req) {
                if Method::OPTIONS == *req.method() {
                    let (request, _pl) = req.into_parts();
                    let response = HttpResponse::Ok().finish().map_into_right_body();
                    return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
                }

                info!("Skipping authentication for route: {}", req.path());
                let fut = self.service.call(req);
                return Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) });
            }

            let manager = match req.app_data::<Data<TenantPoolManager>>() {
                Some(mgr) => mgr.clone(),
                None => {
                    error!("TenantPoolManager not found in app data");
                    let (request, _pl) = req.into_parts();
                    let response = HttpResponse::Unauthorized()
                        .json(ResponseBody::new(
                            constants::MESSAGE_INVALID_TOKEN,
                            constants::EMPTY,
                        ))
                        .map_into_right_body();
                    return Box::pin(async move { Ok(ServiceResponse::new(request, response)) });
                }
            };

            let (tenant_id, user_id, tenant_pool) =
                match Self::process_authentication(&req, manager.get_ref()) {
                    Ok(data) => data,
                    Err(auth_error) => {
                        error!("Functional authentication failed: {:?}", auth_error);
                        let (request, _pl) = req.into_parts();
                        let response = HttpResponse::Unauthorized()
                            .json(ResponseBody::new(
                                constants::MESSAGE_INVALID_TOKEN,
                                constants::EMPTY,
                            ))
                            .map_into_right_body();
                        return Box::pin(
                            async move { Ok(ServiceResponse::new(request, response)) },
                        );
                    }
                };

            req.extensions_mut().insert(tenant_pool);
            info!(
                "Authentication successful for tenant: {}, user: {}",
                tenant_id, user_id
            );

            if let Some(reg) = registry.as_ref() {
                Self::register_auth_functions(reg);
            }

            let fut = self.service.call(req);
            Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) })
        }
    }

    impl<S> FunctionalAuthenticationMiddleware<S> {
        /// Pure function to determine if authentication should be skipped
        pub(crate) fn should_skip_authentication(req: &ServiceRequest) -> bool {
            // Skip OPTIONS preflight requests
            if Method::OPTIONS == *req.method() {
                return true;
            }

            // Check against configured ignore routes
            constants::IGNORE_ROUTES
                .iter()
                .any(|route| req.path().starts_with(route))
        }

        /// Functional pipeline for token extraction and validation
        fn process_authentication(
            req: &ServiceRequest,
            manager: &TenantPoolManager,
        ) -> Result<(String, String, crate::config::db::Pool), &'static str> {
            // Extract token using functional approach
            let token = Self::extract_token(req)?;

            // Decode and validate token
            let token_data =
                token_utils::decode_token(token.clone()).map_err(|_| "Token decode failed")?;

            let tenant_id = token_data.claims.tenant_id.clone();
            let user_id = token_data.claims.user.clone();

            // Verify token against tenant database
            let tenant_pool = manager
                .get_tenant_pool(&tenant_id)
                .ok_or("Tenant not found")?;

            token_utils::verify_token(&token_data, &tenant_pool)
                .map_err(|_| "Token verification failed")?;

            Ok((tenant_id, user_id, tenant_pool.clone()))
        }

        /// Extracts the bearer token from the `Authorization` header of the request.
        ///
        /// Returns the token string on success, or a static error message describing the failure.
        ///
        /// # Examples
        ///
        /// ```
        /// use actix_web::test::TestRequest;
        /// use actix_web::http::header;
        ///
        /// let req = TestRequest::default()
        ///     .insert_header((header::AUTHORIZATION, "Bearer abc.def.ghi"))
        ///     .to_srv_request();
        ///
        /// let token = extract_token(&req).unwrap();
        /// assert_eq!(token, "abc.def.ghi");
        /// ```
        pub fn extract_token(req: &ServiceRequest) -> Result<String, &'static str> {
            let auth_header = req
                .headers()
                .get(constants::AUTHORIZATION)
                .ok_or("Missing authorization header")?;

            let auth_str = auth_header
                .to_str()
                .map_err(|_| "Invalid header encoding")?;

            if !auth_str.to_lowercase().starts_with("bearer ") {
                return Err("Invalid authorization scheme");
            }

            let token = auth_str[7..].trim();
            if token.is_empty() {
                return Err("Empty token");
            }

            Ok(token.to_string())
        }

        /// Registers authentication-related pure functions into the provided registry.
        ///
        /// This function is a placeholder and does nothing; in a complete implementation it
        /// will add the pure functions used by authentication flows to `registry`.
        ///
        /// # Examples
        ///
        /// ```
        /// // Given an existing `PureFunctionRegistry` instance `reg`:
        /// // register_auth_functions(&reg);
        /// ```
        fn register_auth_functions(registry: &PureFunctionRegistry) {
            // Note: In a full implementation, we would register the pure functions
            // used in authentication here for reuse across the application
            let _ = registry; // Placeholder to avoid unused variable warning
        }

        /// Constructs a 401 Unauthorized ServiceResponse with the standardized invalid-token JSON payload.
        ///
        /// The response body is `ResponseBody::new(constants::MESSAGE_INVALID_TOKEN, constants::EMPTY)` mapped into the right variant of `EitherBody`.
        ///
        /// # Examples
        ///
        /// ```
        /// use actix_web::http::StatusCode;
        /// use actix_web::test::TestRequest;
        ///
        /// let req = TestRequest::default().to_srv_request();
        /// let res = crate::middleware::auth_middleware::create_unauthorized_response(req).unwrap();
        /// assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        /// ```
        fn create_unauthorized_response(
            req: ServiceRequest,
        ) -> Result<ServiceResponse<EitherBody<BoxBody>>, Error> {
            let (request, _pl) = req.into_parts();
            let response = HttpResponse::Unauthorized()
                .json(ResponseBody::new(
                    constants::MESSAGE_INVALID_TOKEN,
                    constants::EMPTY,
                ))
                .map_into_right_body();
            Ok(ServiceResponse::new(request, response))
        }
    }
}

#[cfg(all(test, feature = "functional"))]
mod tests {
    use super::functional_auth::{FunctionalAuthentication, FunctionalAuthenticationMiddleware};
    use crate::constants;
    use crate::functional::pure_function_registry::PureFunctionRegistry;
    use actix_web::http::StatusCode;
    use actix_web::{test, web, App, HttpResponse};
    use std::sync::Arc;

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().body("test")
    }

    #[actix_rt::test]
    async fn functional_auth_middleware_creates_default() {
        let middleware = FunctionalAuthentication::default();
        // Should create successfully
        assert!(!middleware.has_registry());
    }

    #[actix_rt::test]
    async fn functional_auth_middleware_with_registry() {
        let registry = Arc::new(PureFunctionRegistry::new());
        let middleware = FunctionalAuthentication::with_registry(registry.clone());

        assert!(middleware.has_registry());
    }

    #[actix_rt::test]
    async fn functional_auth_should_skip_options_request() {
        let app = test::init_service(
            App::new()
                .wrap(FunctionalAuthentication::new())
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::with_uri("/test")
            .method(actix_web::http::Method::OPTIONS)
            .to_request();

        let resp = test::call_service(&app, req).await;
        // OPTIONS should pass through without auth
        assert!(resp.status().is_success() || resp.status() == StatusCode::METHOD_NOT_ALLOWED);
    }

    #[actix_rt::test]
    async fn functional_auth_blocks_unauthorized_request() {
        let app = test::init_service(
            App::new()
                .wrap(FunctionalAuthentication::new())
                .route("/protected", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/protected").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn functional_auth_extract_token_missing_header() {
        let req = test::TestRequest::get().uri("/test").to_srv_request();
        let result = FunctionalAuthenticationMiddleware::<()>::extract_token(&req);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing authorization header");
    }

    #[actix_rt::test]
    async fn functional_auth_extract_token_invalid_scheme() {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header((constants::AUTHORIZATION, "Basic abc123"))
            .to_srv_request();

        let result = FunctionalAuthenticationMiddleware::<()>::extract_token(&req);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid authorization scheme");
    }

    #[actix_rt::test]
    async fn functional_auth_extract_token_empty_token() {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header((constants::AUTHORIZATION, "Bearer "))
            .to_srv_request();

        let result = FunctionalAuthenticationMiddleware::<()>::extract_token(&req);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Empty token");
    }

    #[actix_rt::test]
    async fn functional_auth_extract_token_success() {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header((constants::AUTHORIZATION, "Bearer valid_token_here"))
            .to_srv_request();

        let result = FunctionalAuthenticationMiddleware::<()>::extract_token(&req);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid_token_here");
    }

    #[actix_rt::test]
    async fn functional_auth_should_skip_health_endpoint() {
        let req = test::TestRequest::get().uri("/health").to_srv_request();

        let should_skip =
            FunctionalAuthenticationMiddleware::<()>::should_skip_authentication(&req);
        assert!(should_skip);
    }

    #[actix_rt::test]
    async fn functional_auth_should_skip_api_doc() {
        let req = test::TestRequest::get().uri("/api-doc").to_srv_request();

        let should_skip =
            FunctionalAuthenticationMiddleware::<()>::should_skip_authentication(&req);
        assert!(should_skip);
    }

    #[actix_rt::test]
    async fn functional_auth_should_not_skip_protected_route() {
        let req = test::TestRequest::get()
            .uri("/api/protected")
            .to_srv_request();

        let should_skip =
            FunctionalAuthenticationMiddleware::<()>::should_skip_authentication(&req);
        assert!(!should_skip);
    }
}
