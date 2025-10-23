//! Functional Middleware Pipeline
//!
//! This module implements composable middleware components using functional programming patterns.
//! Middleware functions are pure, composable, and support zero-allocation processing for
//! performance-critical paths. The system enables reusable middleware components through
//! functional composition and immutable request/response transformations.

#[cfg(feature = "functional")]
pub mod functional_middleware_impl {
    use actix_service::{forward_ready, Service, Transform};
    use actix_web::body::{BoxBody, EitherBody, MessageBody};
    use actix_web::dev::{ServiceRequest, ServiceResponse};
    use actix_web::web::Data;
    use actix_web::{Error, HttpMessage, HttpResponse};
    use futures::future::{ok, LocalBoxFuture, Ready};
    use log::{error, info};
    use std::marker::PhantomData;
    use std::sync::Arc;

    #[cfg(feature = "functional")]
    use crate::config::db::TenantPoolManager;
    #[cfg(feature = "functional")]
    use crate::constants;
    #[cfg(feature = "functional")]
    use crate::functional::function_traits::{FunctionCategory, PureFunction};
    #[cfg(feature = "functional")]
    use crate::functional::pure_function_registry::PureFunctionRegistry;
    #[cfg(feature = "functional")]
    use crate::models::response::ResponseBody;
    #[cfg(feature = "functional")]
    use crate::utils::token_utils;

    /// Result type for middleware operations
    pub type MiddlewareResult<T> = Result<T, MiddlewareError>;

    /// Error types for middleware operations
    #[derive(Debug, Clone)]
    pub enum MiddlewareError {
        AuthenticationFailed(String),
        TenantNotFound(String),
        TokenInvalid(String),
        InternalError(String),
    }

    /// Context passed through middleware pipeline
    #[derive(Debug, Clone)]
    pub struct MiddlewareContext {
        pub tenant_id: Option<String>,
        pub user_id: Option<String>,
        pub is_authenticated: bool,
        pub skip_auth: bool,
    }

    impl Default for MiddlewareContext {
        /// Create a MiddlewareContext with no tenant or user and with authentication disabled.
        ///
        /// The returned context has `tenant_id` and `user_id` set to `None`, and `is_authenticated` and `skip_auth` set to `false`.
        ///
        /// # Examples
        ///
        /// ```
        /// let ctx = MiddlewareContext::default();
        /// assert!(ctx.tenant_id.is_none());
        /// assert!(ctx.user_id.is_none());
        /// assert_eq!(ctx.is_authenticated, false);
        /// assert_eq!(ctx.skip_auth, false);
        /// ```
        fn default() -> Self {
            Self {
                tenant_id: None,
                user_id: None,
                is_authenticated: false,
                skip_auth: false,
            }
        }
    }

    #[cfg(feature = "functional")]
    /// Pure function for extracting authentication token from request
    pub struct TokenExtractor;

    impl PureFunction<&ServiceRequest, MiddlewareResult<String>> for TokenExtractor {
        /// Extracts the Bearer token from the request's Authorization header.
        ///
        /// Returns the token string if the header exists, is valid ASCII/UTF-8, and begins with the case-insensitive prefix `Bearer `; otherwise returns `MiddlewareError::AuthenticationFailed`.
        ///
        /// # Examples
        ///
        /// ```
        /// use actix_web::http::header;
        /// use actix_web::test::TestRequest;
        ///
        /// // Construct a ServiceRequest with an Authorization header
        /// let req = TestRequest::default()
        ///     .insert_header((header::AUTHORIZATION, "Bearer token123"))
        ///     .to_srv_request();
        ///
        /// let extractor = crate::TokenExtractor;
        /// let result = extractor.call(&req);
        /// assert_eq!(result.unwrap(), "token123".to_string());
        /// ```
        fn call(&self, req: &ServiceRequest) -> MiddlewareResult<String> {
            if let Some(auth_header) = req.headers().get(constants::AUTHORIZATION) {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.to_lowercase().starts_with("bearer ") {
                        let token = auth_str[7..].trim().to_string();
                        if !token.is_empty() {
                            return Ok(token);
                        }
                    }
                }
            }
            Err(MiddlewareError::AuthenticationFailed(
                "Missing or invalid authorization header".to_string(),
            ))
        }

        /// Canonical signature string for this pure function.
        ///
        /// The returned string describes the function's expected input and output types:
        /// `"fn(&ServiceRequest) -> MiddlewareResult<String>"`.
        ///
        /// # Examples
        ///
        /// ```
        /// let sig = TokenExtractor.signature(&TokenExtractor);
        /// assert_eq!(sig, "fn(&ServiceRequest) -> MiddlewareResult<String>");
        /// ```
        fn signature(&self) -> &'static str {
            "fn(&ServiceRequest) -> MiddlewareResult<String>"
        }

        /// Indicates this pure function belongs to the middleware category.
        ///
        /// # Returns
        ///
        /// `FunctionCategory::Middleware`
        ///
        /// # Examples
        ///
        /// ```
        /// #[derive(PartialEq, Debug)]
        /// enum FunctionCategory { Middleware }
        ///
        /// struct Dummy;
        ///
        /// impl Dummy {
        ///     fn category(&self) -> FunctionCategory {
        ///         FunctionCategory::Middleware
        ///     }
        /// }
        ///
        /// let d = Dummy;
        /// assert_eq!(d.category(), FunctionCategory::Middleware);
        /// ```
        fn category(&self) -> FunctionCategory {
            FunctionCategory::BusinessLogic
        }
    }

    /// Pure function for validating JWT token and extracting claims
    pub struct TokenValidator;

    impl PureFunction<(String, &TenantPoolManager), MiddlewareResult<(String, String)>>
        for TokenValidator
    {
        /// Validate a token and return the tenant and user IDs extracted from its claims.
        ///
        /// Decodes the provided token and verifies it against the tenant pool obtained from
        /// the given `TenantPoolManager`. On success returns the tenant ID and user ID
        /// extracted from the token claims.
        ///
        /// # Returns
        ///
        /// `Ok((tenant_id, user_id))` with IDs extracted from the token claims on success.
        /// `Err(MiddlewareError::TokenInvalid(_))` if token decoding or verification fails.
        /// `Err(MiddlewareError::TenantNotFound(_))` if no tenant pool exists for the token's tenant.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// // Requires a real TenantPoolManager and a valid token to run.
        /// // let validator = TokenValidator;
        /// // let manager: TenantPoolManager = /* ... */;
        /// // let token = "eyJ...".to_string();
        /// // let result = validator.call((token, &manager));
        /// ```
        fn call(&self, input: (String, &TenantPoolManager)) -> MiddlewareResult<(String, String)> {
            let (token, manager) = input;

            let token_data = token_utils::decode_token(token).map_err(|e| {
                MiddlewareError::TokenInvalid(format!("Token decode failed: {}", e))
            })?;

            let tenant_id = token_data.claims.tenant_id.clone();
            let user_id = token_data.claims.user.clone();

            // Verify token against tenant database
            if let Some(tenant_pool) = manager.get_tenant_pool(&tenant_id) {
                token_utils::verify_token(&token_data, &tenant_pool).map_err(|e| {
                    MiddlewareError::TokenInvalid(format!("Token verification failed: {}", e))
                })?;
            } else {
                return Err(MiddlewareError::TenantNotFound(format!(
                    "Tenant '{}' not found",
                    tenant_id
                )));
            }

            Ok((tenant_id, user_id))
        }

        /// Textual signature identifying this pure function's input and output types for registry lookups.
        ///
        /// # Returns
        ///
        /// A static string equal to
        /// `"fn((String, &TenantPoolManager)) -> MiddlewareResult<(String, String)>"`.
        ///
        /// # Examples
        ///
        /// ```
        /// let sig = TokenValidator {}.signature();
        /// assert_eq!(sig, "fn((String, &TenantPoolManager)) -> MiddlewareResult<(String, String)>");
        /// ```
        fn signature(&self) -> &'static str {
            "fn((String, &TenantPoolManager)) -> MiddlewareResult<(String, String)>"
        }

        /// Indicates this pure function belongs to the middleware category.
        ///
        /// # Returns
        ///
        /// `FunctionCategory::Middleware`
        ///
        /// # Examples
        ///
        /// ```
        /// #[derive(PartialEq, Debug)]
        /// enum FunctionCategory { Middleware }
        ///
        /// struct Dummy;
        ///
        /// impl Dummy {
        ///     fn category(&self) -> FunctionCategory {
        ///         FunctionCategory::Middleware
        ///     }
        /// }
        ///
        /// let d = Dummy;
        /// assert_eq!(d.category(), FunctionCategory::Middleware);
        /// ```
        fn category(&self) -> FunctionCategory {
            FunctionCategory::BusinessLogic
        }
    }

    /// Pure function for checking if request should skip authentication
    pub struct AuthSkipChecker;

    impl PureFunction<&ServiceRequest, bool> for AuthSkipChecker {
        /// Determine if authentication should be skipped for the given request.
        ///
        /// Skips authentication when the request method is OPTIONS or when the request path
        /// starts with any entry in `constants::IGNORE_ROUTES`.
        ///
        /// # Examples
        ///
        /// ```
        /// use actix_web::http::Method;
        /// use actix_web::test::TestRequest;
        ///
        /// // OPTIONS requests should skip authentication.
        /// let req = TestRequest::default().method(Method::OPTIONS).to_srv_request();
        /// let checker = AuthSkipChecker;
        /// assert!(checker.call(&req));
        ///
        /// // Paths that match configured ignore routes should skip authentication.
        /// let req = TestRequest::default().uri("/healthz").to_srv_request();
        /// let checker = AuthSkipChecker;
        /// // assuming "/healthz" is listed in `constants::IGNORE_ROUTES`
        /// // assert!(checker.call(&req));
        /// ```
        ///
        /// # Returns
        ///
        /// `true` if the request should bypass authentication, `false` otherwise.
        fn call(&self, req: &ServiceRequest) -> bool {
            // Skip OPTIONS requests
            if req.method().as_str() == "OPTIONS" {
                return true;
            }

            // Check against ignore routes
            constants::IGNORE_ROUTES
                .iter()
                .any(|route| req.path().starts_with(route))
        }

        fn signature(&self) -> &'static str {
            "fn(&ServiceRequest) -> bool"
        }

        /// Indicates this pure function belongs to the middleware category.
        ///
        /// # Returns
        ///
        /// `FunctionCategory::Middleware`
        ///
        /// # Examples
        ///
        /// ```
        /// #[derive(PartialEq, Debug)]
        /// enum FunctionCategory { Middleware }
        ///
        /// struct Dummy;
        ///
        /// impl Dummy {
        ///     fn category(&self) -> FunctionCategory {
        ///         FunctionCategory::Middleware
        ///     }
        /// }
        ///
        /// let d = Dummy;
        /// assert_eq!(d.category(), FunctionCategory::Middleware);
        /// ```
        fn category(&self) -> FunctionCategory {
            FunctionCategory::BusinessLogic
        }
    }

    /// Composable middleware transformer that supports functional composition
    pub struct FunctionalMiddleware<T, F> {
        function: Arc<F>,
        registry: Arc<PureFunctionRegistry>,
        _phantom: PhantomData<T>,
    }

    impl<T, F> FunctionalMiddleware<T, F> {
        /// Wraps a function and a pure-function registry into a new `FunctionalMiddleware` with shared ownership.
        ///
        /// # Examples
        ///
        /// ```
        /// use std::sync::Arc;
        /// use std::marker::PhantomData;
        /// // Assume `PureFunctionRegistry` and `FunctionalMiddleware` are in scope.
        ///
        /// let registry = Arc::new(PureFunctionRegistry::new());
        /// let func = |req: &ServiceRequest| -> MiddlewareResult<&ServiceRequest> { Ok(req) };
        /// let middleware = FunctionalMiddleware::new(func, registry.clone());
        ///
        /// // `middleware` now holds `func` and `registry` behind `Arc`s.
        /// ```
        pub fn new(function: F, registry: Arc<PureFunctionRegistry>) -> Self {
            Self {
                function: Arc::new(function),
                registry,
                _phantom: PhantomData,
            }
        }
    }

    /// Reader monad for middleware context
    pub struct MiddlewareReader<T> {
        run: Box<dyn Fn(&MiddlewareContext) -> MiddlewareResult<T> + Send + Sync>,
    }

    impl<T> MiddlewareReader<T> {
        /// Create a new MiddlewareReader from a function
        pub fn new<F>(f: F) -> Self
        where
            F: Fn(&MiddlewareContext) -> MiddlewareResult<T> + Send + Sync + 'static,
        {
            Self { run: Box::new(f) }
        }

        /// Execute the reader with the provided context
        pub fn run(&self, ctx: &MiddlewareContext) -> MiddlewareResult<T> {
            (self.run)(ctx)
        }

        /// Map the result of this reader to a new type
        pub fn map<U, F>(self, f: F) -> MiddlewareReader<U>
        where
            F: Fn(T) -> U + Send + Sync + 'static,
            T: 'static,
        {
            MiddlewareReader::new(move |ctx| self.run(ctx).map(&f))
        }

        /// Chain another reader operation that depends on the result of this one
        pub fn and_then<U, F>(self, f: F) -> MiddlewareReader<U>
        where
            F: Fn(T) -> MiddlewareReader<U> + Send + Sync + 'static,
            T: 'static,
        {
            MiddlewareReader::new(move |ctx| {
                let result = self.run(ctx)?;
                f(result).run(ctx)
            })
        }

        /// Add validation logic before executing the reader
        pub fn validate<F>(self, validator: F) -> MiddlewareReader<T>
        where
            F: Fn(&T) -> MiddlewareResult<()> + Send + Sync + 'static,
            T: Clone + 'static,
        {
            MiddlewareReader::new(move |ctx| {
                let result = self.run(ctx)?;
                validator(&result)?;
                Ok(result)
            })
        }
    }

    /// Execute a MiddlewareReader with a context
    pub fn run_middleware_reader<T>(
        reader: MiddlewareReader<T>,
        context: &MiddlewareContext,
    ) -> MiddlewareResult<T> {
        reader.run(context)
    }

    /// Authentication middleware using functional composition
    pub struct FunctionalAuthentication {
        registry: Arc<PureFunctionRegistry>,
    }

    impl FunctionalAuthentication {
        /// Constructs a new FunctionalAuthentication using the provided pure-function registry.
        ///
        /// The registry supplies pure function implementations used by the authentication middleware.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use std::sync::Arc;
        /// // assuming PureFunctionRegistry and FunctionalAuthentication are in scope
        /// let registry = Arc::new(PureFunctionRegistry::default());
        /// let auth = FunctionalAuthentication::new(registry.clone());
        /// ```
        pub fn new(registry: Arc<PureFunctionRegistry>) -> Self {
            Self { registry }
        }

        /// Returns a reference to the registry (for testing purposes)
        #[cfg(test)]
        pub fn registry(&self) -> &Arc<PureFunctionRegistry> {
            &self.registry
        }
    }

    impl<S, B> Transform<S, ServiceRequest> for FunctionalAuthentication
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: MessageBody + 'static,
    {
        type Response = ServiceResponse<EitherBody<BoxBody>>;
        type Error = Error;
        type InitError = ();
        type Transform = FunctionalAuthenticationMiddleware<S>;
        type Future = Ready<Result<Self::Transform, Self::InitError>>;

        /// Create a transform future that produces a `FunctionalAuthenticationMiddleware` wrapping the provided service.
        ///
        /// The future resolves to a `FunctionalAuthenticationMiddleware` that contains the given `service` and a cloned
        /// reference to the middleware registry.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use std::sync::Arc;
        /// use futures::executor::block_on;
        ///
        /// // Assume `registry` is an Arc<PureFunctionRegistry> and `service` implements the Actix `Service` trait.
        /// // let registry: Arc<PureFunctionRegistry> = Arc::new(...);
        /// // let auth = FunctionalAuthentication::new(registry.clone());
        /// // let service = /* some service */;
        /// // let fut = auth.new_transform(service);
        /// // let middleware = block_on(fut).unwrap();
        /// ```
        fn new_transform(&self, service: S) -> Self::Future {
            ok(FunctionalAuthenticationMiddleware { service })
        }
    }

    pub struct FunctionalAuthenticationMiddleware<S> {
        service: S,
    }

    impl<S, B> Service<ServiceRequest> for FunctionalAuthenticationMiddleware<S>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: MessageBody + 'static,
    {
        type Response = ServiceResponse<EitherBody<BoxBody>>;
        type Error = Error;
        type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

        forward_ready!(service);

        /// Authenticate the incoming `ServiceRequest`, inject tenant context on success, and forward it to the inner service or return a standardized HTTP error response.
        ///
        /// On success the request is forwarded to the inner service and its `ServiceResponse` is returned with the left `EitherBody`. If authentication is skipped for the request, it is forwarded unchanged. If any required setup or validation fails (missing tenant manager, token extraction or validation failure, or missing tenant pool), a standardized HTTP error response is produced.
        ///
        /// # Returns
        ///
        /// A `Future` that resolves to the inner service's `ServiceResponse` mapped into the left `EitherBody` on success, or an `actix_web::Error` representing an HTTP error response on failure.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// // Illustrative usage; running this requires an Actix runtime and a constructed middleware instance.
        /// # use actix_web::dev::ServiceRequest;
        /// #
        /// // let middleware: FunctionalAuthenticationMiddleware<_> = /* constructed middleware */ ;
        /// // let req: ServiceRequest = /* construct request */ ;
        /// // let fut = middleware.call(req);
        /// // let res = futures::executor::block_on(fut);
        /// ```
        fn call(&self, req: ServiceRequest) -> Self::Future {
            let skip_checker = AuthSkipChecker;

            if skip_checker.call(&req) {
                info!("Skipping authentication for route: {}", req.path());
                let fut = self.service.call(req);
                return Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_boxed_body().map_into_left_body())
                });
            }

            let manager = match req.app_data::<Data<TenantPoolManager>>() {
                Some(mgr) => mgr.clone(),
                None => {
                    error!("TenantPoolManager not found in app data");
                    return Box::pin(async move {
                        Self::create_error_response(req, constants::MESSAGE_INTERNAL_SERVER_ERROR)
                    });
                }
            };

            let token_extractor = TokenExtractor;
            let token = match token_extractor.call(&req) {
                Ok(token) => token,
                Err(err) => {
                    error!("Token extraction failed: {:?}", err);
                    return Box::pin(async move {
                        Self::create_error_response(req, constants::MESSAGE_INVALID_TOKEN)
                    });
                }
            };

            let validator = TokenValidator;
            let (tenant_id, user_id) = match validator.call((token, manager.get_ref())) {
                Ok(ids) => ids,
                Err(err) => {
                    error!("Token validation failed: {:?}", err);
                    return Box::pin(async move {
                        Self::create_error_response(req, constants::MESSAGE_INVALID_TOKEN)
                    });
                }
            };

            let tenant_pool = match manager.get_tenant_pool(&tenant_id) {
                Some(pool) => pool,
                None => {
                    error!("Tenant pool not found for tenant: {}", tenant_id);
                    return Box::pin(async move {
                        Self::create_error_response(req, constants::MESSAGE_INVALID_TOKEN)
                    });
                }
            };

            req.extensions_mut().insert(tenant_pool.clone());
            info!(
                "Authentication successful for tenant: {}, user: {}",
                tenant_id, user_id
            );

            let fut = self.service.call(req);
            Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_boxed_body().map_into_left_body())
            })
        }
    }

    impl<S> FunctionalAuthenticationMiddleware<S> {
        /// Create a 401 Unauthorized `ServiceResponse` whose JSON body is a `ResponseBody` containing the given `message` and an empty data payload.
        ///
        /// The `req` is converted into the response's request parts; the function constructs an `HttpResponse::Unauthorized`
        /// with `ResponseBody::new(message, constants::EMPTY)` and wraps it as a `ServiceResponse`.
        ///
        /// # Returns
        ///
        /// `Ok(ServiceResponse)` containing a 401 Unauthorized response with the JSON error body, or `Err(Error)` if constructing the response fails.
        ///
        /// # Examples
        ///
        /// ```
        /// // Given a `ServiceRequest` named `req`:
        /// let resp = create_error_response(req, "Unauthorized access").unwrap();
        /// assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
        /// ```
        fn create_error_response(
            req: ServiceRequest,
            message: &str,
        ) -> Result<ServiceResponse<EitherBody<BoxBody>>, Error> {
            let (request, _pl) = req.into_parts();
            let response = HttpResponse::InternalServerError()
                .json(ResponseBody::new(message, constants::EMPTY))
                .map_into_right_body();
            Ok(ServiceResponse::new(request, response))
        }
    }

    /// Builder for composing middleware pipelines
    pub struct MiddlewarePipelineBuilder {
        registry: Arc<PureFunctionRegistry>,
        middleware_stack: Vec<Box<dyn MiddlewareComponent>>,
    }

    impl MiddlewarePipelineBuilder {
        /// Constructs a new MiddlewarePipelineBuilder that uses the provided registry and starts with an empty middleware stack.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use std::sync::Arc;
        /// # use crate::functional::PureFunctionRegistry;
        ///
        /// let registry = Arc::new(PureFunctionRegistry::default());
        /// let builder = MiddlewarePipelineBuilder::new(registry);
        /// ```
        pub fn new(registry: Arc<PureFunctionRegistry>) -> Self {
            Self {
                registry,
                middleware_stack: Vec::new(),
            }
        }

        /// Adds the functional authentication middleware to the builder's middleware stack.
        ///
        /// The method clones a `FunctionalAuthentication` component from the builder's registry,
        /// appends it to the internal middleware stack, and returns the updated builder for chaining.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use std::sync::Arc;
        ///
        /// // assume `registry` is an Arc<PureFunctionRegistry> obtained elsewhere
        /// let registry: Arc<_> = Arc::new(PureFunctionRegistry::new());
        /// let builder = MiddlewarePipelineBuilder::new(registry).with_auth();
        /// ```
        pub fn with_auth(mut self) -> Self {
            self.middleware_stack
                .push(Box::new(FunctionalAuthentication::new(Arc::clone(
                    &self.registry,
                ))));
            self
        }

        /// Returns the number of middleware components in the stack (for testing purposes)
        #[cfg(test)]
        pub fn stack_len(&self) -> usize {
            self.middleware_stack.len()
        }

        /// Adds a custom middleware component to the pipeline builder (test-only helper).
        #[cfg(test)]
        pub fn with_component<T>(mut self, component: T) -> Self
        where
            T: MiddlewareComponent + 'static,
        {
            self.middleware_stack.push(Box::new(component));
            self
        }

        pub fn build<S, B>(self, service: S) -> ComposedMiddleware<S>
        where
            S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
            S::Future: 'static,
            B: MessageBody + 'static,
        {
            ComposedMiddleware {
                service: Arc::new(service),
                middleware_stack: self.middleware_stack,
            }
        }
    }

    /// Trait for composable middleware components
    pub trait MiddlewareComponent: Send + Sync {
        fn process(&self, req: &mut ServiceRequest) -> Result<(), MiddlewareError>;
    }

    /// Composed middleware that chains multiple middleware components
    pub struct ComposedMiddleware<S> {
        service: Arc<S>,
        middleware_stack: Vec<Box<dyn MiddlewareComponent>>,
    }

    impl<S, B> Service<ServiceRequest> for ComposedMiddleware<S>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static,
        B: MessageBody + 'static,
    {
        type Response = ServiceResponse<EitherBody<BoxBody>>;
        type Error = Error;
        type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

        forward_ready!(service);

        /// Apply the middleware stack to `req` in order, then invoke the wrapped service with the final request.
        ///
        /// If any middleware returns an error, this method produces and returns an HTTP 500 Internal Server Error
        /// response immediately. If all middleware succeed, the resulting request is forwarded to the inner service
        /// and that service's response is returned (mapped into the left branch of `EitherBody`).
        ///
        /// # Examples
        ///
        /// ```
        /// // Illustrative example: a ComposedMiddleware is created elsewhere with a middleware stack,
        /// // and `call` applies those middleware before delegating to the inner service.
        /// // The real integration demonstrates actual ServiceRequest/ServiceResponse flows.
        /// // let response = composed.call(service_request).await;
        /// ```
        fn call(&self, req: ServiceRequest) -> Self::Future {
            let mut current_req = req;

            for middleware in &self.middleware_stack {
                if let Err(e) = middleware.process(&mut current_req) {
                    error!("Middleware processing failed: {:?}", e);
                    return Box::pin(async move {
                        Self::create_error_response(
                            current_req,
                            constants::MESSAGE_INTERNAL_SERVER_ERROR,
                        )
                    });
                }
            }

            let service = Arc::clone(&self.service);
            Box::pin(async move {
                let res = service.call(current_req).await?;
                Ok(res.map_into_boxed_body().map_into_left_body())
            })
        }
    }

    impl<S> ComposedMiddleware<S> {
        /// Creates a 500 Internal Server Error response with a JSON body containing `message`.
        ///
        /// The function consumes the provided `ServiceRequest` and returns a `ServiceResponse` whose
        /// body is the right branch of an `EitherBody`, serialized from `ResponseBody::new(message, constants::EMPTY)`.
        ///
        /// # Examples
        ///
        /// ```
        /// use actix_web::{dev::ServiceRequest, http::StatusCode};
        /// // Assume `req` is a valid ServiceRequest constructed in a test context.
        /// // let req: ServiceRequest = ...;
        /// // let res = create_error_response(req, "internal failure").unwrap();
        /// // assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        /// ```
        /// Build a 500 Internal Server Error JSON response using the provided request's parts.
        ///
        /// The resulting `ServiceResponse` contains a JSON body `ResponseBody::new(message, constants::EMPTY)`
        /// and is mapped into the `EitherBody` right variant.
        ///
        /// # Parameters
        ///
        /// - `req`: the original `ServiceRequest` to consume when constructing the `ServiceResponse`.
        /// - `message`: the error message to include in the JSON response body.
        ///
        /// # Returns
        ///
        /// `Ok(ServiceResponse)` wrapping the original request parts and a 500 JSON error response, or an `Error` if response construction fails.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # use actix_web::dev::{ServiceRequest, ServiceResponse};
        /// # use actix_web::Error;
        /// // Assume `req` is a valid ServiceRequest obtained earlier in a handler or middleware.
        /// # let req: ServiceRequest = unimplemented!();
        /// let resp: ServiceResponse<_> = create_error_response(req, "internal failure").unwrap();
        /// ```
        fn create_error_response(
            req: ServiceRequest,
            message: &str,
        ) -> Result<ServiceResponse<EitherBody<BoxBody>>, Error> {
            let (request, _pl) = req.into_parts();
            let response = HttpResponse::InternalServerError()
                .json(ResponseBody::new(message, constants::EMPTY))
                .map_into_right_body();
            Ok(ServiceResponse::new(request, response))
        }
    }

    impl MiddlewareComponent for FunctionalAuthentication {
        fn process(&self, req: &mut ServiceRequest) -> Result<(), MiddlewareError> {
            let _registry = &self.registry;
            let _ = req;
            Ok(())
        }
    }
}

#[cfg(all(test, feature = "functional"))]
mod tests {
    use super::functional_middleware_impl::{
        AuthSkipChecker, FunctionalAuthentication, MiddlewareComponent, MiddlewareContext,
        MiddlewareError, MiddlewarePipelineBuilder, MiddlewareReader, MiddlewareResult,
        TokenExtractor, TokenValidator, run_middleware_reader,
    };
    use crate::constants;
    use crate::functional::function_traits::{FunctionCategory, PureFunction};
    use crate::functional::pure_function_registry::PureFunctionRegistry;
    use actix_service::{fn_service, Service, Transform};
    use actix_web::body::to_bytes;
    use actix_web::dev::ServiceRequest;
    use actix_web::http::{Method, StatusCode};
    use actix_web::test::TestRequest;
    use actix_web::HttpResponse;
    use serde_json::Value;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn middleware_context_default() {
        let context = MiddlewareContext::default();

        assert!(context.tenant_id.is_none());
        assert!(context.user_id.is_none());
        assert!(!context.is_authenticated);
        assert!(!context.skip_auth);
    }

    #[test]
    fn middleware_context_with_tenant() {
        let context = MiddlewareContext {
            tenant_id: Some("tenant123".to_string()),
            user_id: Some("user456".to_string()),
            is_authenticated: true,
            skip_auth: false,
        };

        assert_eq!(context.tenant_id.unwrap(), "tenant123");
        assert_eq!(context.user_id.unwrap(), "user456");
        assert!(context.is_authenticated);
        assert!(!context.skip_auth);
    }

    #[test]
    fn middleware_error_variants() {
        let auth_err = MiddlewareError::AuthenticationFailed("test".to_string());
        let tenant_err = MiddlewareError::TenantNotFound("tenant1".to_string());
        let token_err = MiddlewareError::TokenInvalid("invalid".to_string());
        let internal_err = MiddlewareError::InternalError("error".to_string());

        match auth_err {
            MiddlewareError::AuthenticationFailed(msg) => assert_eq!(msg, "test"),
            _ => panic!("Wrong variant"),
        }

        match tenant_err {
            MiddlewareError::TenantNotFound(msg) => assert_eq!(msg, "tenant1"),
            _ => panic!("Wrong variant"),
        }

        match token_err {
            MiddlewareError::TokenInvalid(msg) => assert_eq!(msg, "invalid"),
            _ => panic!("Wrong variant"),
        }

        match internal_err {
            MiddlewareError::InternalError(msg) => assert_eq!(msg, "error"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn middleware_error_clone() {
        let error = MiddlewareError::AuthenticationFailed("test".to_string());
        let cloned = error.clone();

        match cloned {
            MiddlewareError::AuthenticationFailed(msg) => assert_eq!(msg, "test"),
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn token_extractor_signature() {
        let extractor = TokenExtractor;
        assert_eq!(
            extractor.signature(),
            "fn(&ServiceRequest) -> MiddlewareResult<String>"
        );
    }

    #[test]
    fn token_extractor_category() {
        let extractor = TokenExtractor;
        assert_eq!(extractor.category(), FunctionCategory::BusinessLogic);
    }

    #[test]
    fn token_validator_signature() {
        let validator = TokenValidator;
        assert_eq!(
            validator.signature(),
            "fn((String, &TenantPoolManager)) -> MiddlewareResult<(String, String)>"
        );
    }

    #[test]
    fn token_validator_category() {
        let validator = TokenValidator;
        assert_eq!(validator.category(), FunctionCategory::BusinessLogic);
    }

    #[test]
    fn auth_skip_checker_signature() {
        let checker = AuthSkipChecker;
        assert_eq!(checker.signature(), "fn(&ServiceRequest) -> bool");
    }

    #[test]
    fn auth_skip_checker_category() {
        let checker = AuthSkipChecker;
        assert_eq!(checker.category(), FunctionCategory::BusinessLogic);
    }

    #[test]
    fn auth_skip_checker_skips_options() {
        let checker = AuthSkipChecker;
        let req = TestRequest::default()
            .method(Method::OPTIONS)
            .uri("/test")
            .to_srv_request();

        assert!(checker.call(&req));
    }

    #[test]
    fn auth_skip_checker_skips_health() {
        let checker = AuthSkipChecker;
        let req = TestRequest::get().uri("/health").to_srv_request();

        assert!(checker.call(&req));
    }

    #[test]
    fn auth_skip_checker_does_not_skip_api() {
        let checker = AuthSkipChecker;
        let req = TestRequest::get().uri("/api/users").to_srv_request();

        assert!(!checker.call(&req));
    }

    #[test]
    fn token_extractor_missing_header() {
        let extractor = TokenExtractor;
        let req = TestRequest::get().uri("/test").to_srv_request();

        let result = extractor.call(&req);
        assert!(result.is_err());

        match result {
            Err(MiddlewareError::AuthenticationFailed(msg)) => {
                assert!(msg.contains("Missing or invalid authorization header"));
            }
            _ => panic!("Expected AuthenticationFailed error"),
        }
    }

    #[test]
    fn token_extractor_invalid_scheme() {
        let extractor = TokenExtractor;
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((constants::AUTHORIZATION, "Basic token123"))
            .to_srv_request();

        let result = extractor.call(&req);
        assert!(result.is_err());
    }

    #[test]
    fn token_extractor_empty_token() {
        let extractor = TokenExtractor;
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((constants::AUTHORIZATION, "Bearer "))
            .to_srv_request();

        let result = extractor.call(&req);
        assert!(result.is_err());
    }

    #[test]
    fn token_extractor_valid_token() {
        let extractor = TokenExtractor;
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((constants::AUTHORIZATION, "Bearer valid_token"))
            .to_srv_request();

        let result = extractor.call(&req);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid_token");
    }

    #[test]
    fn token_extractor_case_insensitive() {
        let extractor = TokenExtractor;
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((constants::AUTHORIZATION, "BEARER token123"))
            .to_srv_request();

        let result = extractor.call(&req);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "token123");
    }

    #[actix_rt::test]
    async fn functional_auth_creates_with_registry() {
        let registry = Arc::new(PureFunctionRegistry::new());
        let auth = FunctionalAuthentication::new(registry.clone());

        // Should create successfully - registry should be stored
        assert!(Arc::ptr_eq(auth.registry(), &registry));
    }

    #[actix_rt::test]
    async fn middleware_pipeline_builder_creates() {
        let registry = Arc::new(PureFunctionRegistry::new());
        let builder = MiddlewarePipelineBuilder::new(registry);

        // Should create with empty stack
        assert_eq!(builder.stack_len(), 0);
    }

    #[actix_rt::test]
    async fn middleware_pipeline_builder_with_auth() {
        let registry = Arc::new(PureFunctionRegistry::new());
        let builder = MiddlewarePipelineBuilder::new(registry).with_auth();

        // Should add auth middleware
        assert_eq!(builder.stack_len(), 1);
    }

    #[test]
    fn token_extractor_trims_surrounding_whitespace() {
        let extractor = TokenExtractor;
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((constants::AUTHORIZATION, "Bearer    spaced-token  "))
            .to_srv_request();

        let result = extractor.call(&req).expect("token should be extracted");
        assert_eq!(result, "spaced-token");
    }

    #[actix_rt::test]
    async fn functional_auth_middleware_skips_auth_for_options_requests() {
        let registry = Arc::new(PureFunctionRegistry::new());
        let auth = FunctionalAuthentication::new(registry);
        let inner = fn_service(|req: ServiceRequest| async move {
            Ok(req.into_response(HttpResponse::Ok().finish()))
        });

        let middleware = auth
            .new_transform(inner)
            .await
            .expect("transform construction must succeed");

        let req = TestRequest::default()
            .method(Method::OPTIONS)
            .uri("/any-route")
            .to_srv_request();

        let res = middleware
            .call(req)
            .await
            .expect("middleware call should succeed");

        assert_eq!(res.status(), StatusCode::OK);

        let body = to_bytes(res.into_body()).await.expect("body to bytes");
        assert!(body.is_empty());
    }

    #[actix_rt::test]
    async fn functional_auth_middleware_returns_error_without_pool_manager() {
        let registry = Arc::new(PureFunctionRegistry::new());
        let auth = FunctionalAuthentication::new(registry);
        let inner = fn_service(|req: ServiceRequest| async move {
            Ok(req.into_response(HttpResponse::Ok().finish()))
        });

        let middleware = auth
            .new_transform(inner)
            .await
            .expect("transform construction must succeed");

        let req = TestRequest::default()
            .method(Method::GET)
            .uri("/api/secure")
            .to_srv_request();

        let res = middleware
            .call(req)
            .await
            .expect("middleware call should succeed");

        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(res.into_body()).await.expect("body to bytes");
        let json: Value = serde_json::from_slice(&body).expect("valid json");
        assert_eq!(
            json.get("message").unwrap(),
            constants::MESSAGE_INTERNAL_SERVER_ERROR
        );
    }

    #[actix_rt::test]
    async fn composed_middleware_successfully_invokes_inner_service() {
        struct RecordingMiddleware {
            hits: Arc<AtomicUsize>,
        }

        impl MiddlewareComponent for RecordingMiddleware {
            fn process(&self, _req: &mut ServiceRequest) -> Result<(), MiddlewareError> {
                self.hits.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let hits = Arc::new(AtomicUsize::new(0));
        let registry = Arc::new(PureFunctionRegistry::new());
        let builder =
            MiddlewarePipelineBuilder::new(registry).with_component(RecordingMiddleware {
                hits: Arc::clone(&hits),
            });

        let inner = fn_service(|req: ServiceRequest| async move {
            Ok(req.into_response(HttpResponse::Ok().finish()))
        });

        let middleware = builder.build(inner);

        let req = TestRequest::default()
            .method(Method::GET)
            .uri("/api/data")
            .to_srv_request();

        let res = middleware
            .call(req)
            .await
            .expect("middleware call should succeed");

        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(res.status(), StatusCode::OK);

        let body = to_bytes(res.into_body()).await.expect("body to bytes");
        assert!(body.is_empty());
    }

    #[actix_rt::test]
    async fn composed_middleware_returns_internal_error_on_failure() {
        struct FailingMiddleware;

        impl MiddlewareComponent for FailingMiddleware {
            fn process(&self, _req: &mut ServiceRequest) -> Result<(), MiddlewareError> {
                Err(MiddlewareError::InternalError("boom".to_string()))
            }
        }

        let registry = Arc::new(PureFunctionRegistry::new());
        let builder = MiddlewarePipelineBuilder::new(registry).with_component(FailingMiddleware);

        let inner = fn_service(|req: ServiceRequest| async move {
            Ok(req.into_response(HttpResponse::Ok().finish()))
        });

        let middleware = builder.build(inner);

        let req = TestRequest::default()
            .method(Method::GET)
            .uri("/api/data")
            .to_srv_request();

        let res = middleware
            .call(req)
            .await
            .expect("middleware call should succeed");

        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(res.into_body()).await.expect("body to bytes");
        let json: Value = serde_json::from_slice(&body).expect("valid json");
        assert_eq!(
            json.get("message").unwrap(),
            constants::MESSAGE_INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn middleware_result_ok() {
        let result: MiddlewareResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn middleware_result_err() {
        let result: MiddlewareResult<i32> = Err(MiddlewareError::InternalError("test".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn middleware_context_skip_auth() {
        let context = MiddlewareContext {
            tenant_id: None,
            user_id: None,
            is_authenticated: false,
            skip_auth: true,
        };

        assert!(context.skip_auth);
        assert!(!context.is_authenticated);
    }

    #[test]
    fn middleware_context_authenticated() {
        let context = MiddlewareContext {
            tenant_id: Some("t1".to_string()),
            user_id: Some("u1".to_string()),
            is_authenticated: true,
            skip_auth: false,
        };

        assert!(context.is_authenticated);
        assert!(!context.skip_auth);
        assert!(context.tenant_id.is_some());
        assert!(context.user_id.is_some());
    }

    #[test]
    fn middleware_error_debug_format() {
        let error = MiddlewareError::TokenInvalid("test token".to_string());
        let debug_str = format!("{:?}", error);

        assert!(debug_str.contains("TokenInvalid"));
        assert!(debug_str.contains("test token"));
    }

    #[test]
    fn pure_function_implementations_are_consistent() {
        let extractor = TokenExtractor;
        let validator = TokenValidator;
        let checker = AuthSkipChecker;

        // All should be categorized as business logic functions
        assert_eq!(extractor.category(), FunctionCategory::BusinessLogic);
        assert_eq!(validator.category(), FunctionCategory::BusinessLogic);
        assert_eq!(checker.category(), FunctionCategory::BusinessLogic);

        // All should have non-empty signatures
        assert!(!extractor.signature().is_empty());
        assert!(!validator.signature().is_empty());
        assert!(!checker.signature().is_empty());
    }

    #[test]
    fn middleware_reader_basic() {
        let reader = MiddlewareReader::new(|ctx: &MiddlewareContext| {
            if ctx.is_authenticated {
                Ok("authenticated".to_string())
            } else {
                Ok("unauthenticated".to_string())
            }
        });

        let ctx = MiddlewareContext {
            tenant_id: None,
            user_id: None,
            is_authenticated: true,
            skip_auth: false,
        };

        let result = run_middleware_reader(reader, &ctx);
        assert_eq!(result.unwrap(), "authenticated");
    }

    #[test]
    fn middleware_reader_map() {
        let reader = MiddlewareReader::new(|ctx: &MiddlewareContext| {
            if ctx.is_authenticated {
            Ok("authenticated".to_string())
            } else {
                Ok("unauthenticated".to_string())
            }
        });

        let mapped_reader = reader.map(|s| s.len());
        let ctx = MiddlewareContext {
            tenant_id: None,
            user_id: None,
            is_authenticated: true,
            skip_auth: false,
        };

        let result = run_middleware_reader(mapped_reader, &ctx);
        assert_eq!(result.unwrap(), 13); // length of "authenticated"
    }

    #[test]
    fn middleware_reader_and_then() {
        let reader = MiddlewareReader::new(|ctx: &MiddlewareContext| {
            if ctx.is_authenticated {
                Ok("authenticated")
            } else {
                Ok("unauthenticated")
            }
        });

        let chained_reader = reader.and_then(|s| {
            MiddlewareReader::new(move |_ctx| {
                if s == "authenticated" {
                    Ok(1)
                } else {
                    Ok(0)
                }
            })
        });

        let ctx = MiddlewareContext {
            tenant_id: None,
            user_id: None,
            is_authenticated: true,
            skip_auth: false,
        };

        let result = run_middleware_reader(chained_reader, &ctx);
        assert_eq!(result.unwrap(), 1);
    }
}
