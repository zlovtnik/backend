use crate::models::response::ResponseBody;
use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use derive_more::{Display, Error};
use log::{debug, error as log_error, info as log_info, warn as log_warn, Level};
use serde::Serialize;
use serde_json::to_string as to_json_string;
use std::collections::{BTreeMap, BTreeSet};

pub type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ErrorContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_override: Option<String>,
}

impl ErrorContext {
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self.dedup_tags();
        self
    }

    #[must_use]
    pub fn with_tags<I>(mut self, tags: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        for tag in tags {
            self.tags.push(tag.into());
        }
        self.dedup_tags();
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code_override = Some(code.into());
        self
    }

    fn dedup_tags(&mut self) {
        let tags = std::mem::take(&mut self.tags);
        let set: BTreeSet<String> = tags.into_iter().collect();
        self.tags = set.into_iter().collect();
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErrorEnvelope {
    pub code: String,
    pub message: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl ErrorEnvelope {
    pub fn from_error(error: &ServiceError) -> Self {
        let context = error.context();
        let code = context
            .code_override
            .clone()
            .unwrap_or_else(|| error.default_code().to_string());
        Self {
            code,
            message: error.to_string(),
            timestamp: Utc::now(),
            status: error.http_status().as_u16(),
            detail: context.detail.clone(),
            correlation_id: context.correlation_id.clone(),
            tags: context.tags.clone(),
            metadata: context.metadata.clone(),
        }
    }
}

#[derive(Debug, Display, Error, Clone, PartialEq)]
pub enum ServiceError {
    #[display(fmt = "{error_message}")]
    Unauthorized {
        error_message: String,
        #[error(ignore)]
        context: ErrorContext,
    },
    #[display(fmt = "{error_message}")]
    InternalServerError {
        error_message: String,
        #[error(ignore)]
        context: ErrorContext,
    },
    #[display(fmt = "{error_message}")]
    BadRequest {
        error_message: String,
        #[error(ignore)]
        context: ErrorContext,
    },
    #[display(fmt = "{error_message}")]
    NotFound {
        error_message: String,
        #[error(ignore)]
        context: ErrorContext,
    },
    #[display(fmt = "{error_message}")]
    Conflict {
        error_message: String,
        #[error(ignore)]
        context: ErrorContext,
    },
}

impl ServiceError {
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            error_message: message.into(),
            context: ErrorContext::default(),
        }
    }

    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::InternalServerError {
            error_message: message.into(),
            context: ErrorContext::default(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            error_message: message.into(),
            context: ErrorContext::default(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            error_message: message.into(),
            context: ErrorContext::default(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            error_message: message.into(),
            context: ErrorContext::default(),
        }
    }

    pub fn with_context(mut self, updater: impl FnOnce(ErrorContext) -> ErrorContext) -> Self {
        match &mut self {
            ServiceError::Unauthorized { context, .. }
            | ServiceError::InternalServerError { context, .. }
            | ServiceError::BadRequest { context, .. }
            | ServiceError::NotFound { context, .. }
            | ServiceError::Conflict { context, .. } => {
                let current = std::mem::take(context);
                *context = updater(current);
            }
        }
        self
    }

    pub fn with_detail(self, detail: impl Into<String>) -> Self {
        self.with_context(|ctx| ctx.with_detail(detail))
    }

    pub fn with_metadata(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.with_context(|ctx| ctx.with_metadata(key, value))
    }

    pub fn with_tag(self, tag: impl Into<String>) -> Self {
        self.with_context(|ctx| ctx.with_tag(tag))
    }

    pub fn with_correlation_id(self, correlation_id: impl Into<String>) -> Self {
        self.with_context(|ctx| ctx.with_correlation_id(correlation_id))
    }

    pub fn context(&self) -> &ErrorContext {
        match self {
            ServiceError::Unauthorized { context, .. }
            | ServiceError::InternalServerError { context, .. }
            | ServiceError::BadRequest { context, .. }
            | ServiceError::NotFound { context, .. }
            | ServiceError::Conflict { context, .. } => context,
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            ServiceError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ServiceError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ServiceError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ServiceError::NotFound { .. } => StatusCode::NOT_FOUND,
            ServiceError::Conflict { .. } => StatusCode::CONFLICT,
        }
    }

    pub fn default_code(&self) -> &'static str {
        match self {
            ServiceError::Unauthorized { .. } => "AUTH-401",
            ServiceError::InternalServerError { .. } => "SRV-500",
            ServiceError::BadRequest { .. } => "REQ-400",
            ServiceError::NotFound { .. } => "REQ-404",
            ServiceError::Conflict { .. } => "REQ-409",
        }
    }

    fn default_log_level(&self) -> Level {
        match self {
            ServiceError::InternalServerError { .. } => Level::Error,
            ServiceError::Unauthorized { .. } => Level::Warn,
            ServiceError::Conflict { .. } => Level::Warn,
            ServiceError::BadRequest { .. } => Level::Info,
            ServiceError::NotFound { .. } => Level::Info,
        }
    }

    pub fn log(&self) {
        self.log_with_level(self.default_log_level());
    }

    pub fn log_with_level(&self, level: Level) {
        let envelope = ErrorEnvelope::from_error(self);
        let payload = to_json_string(&envelope).unwrap_or_else(|_| envelope.message.clone());
        match level {
            Level::Error => log_error!(target: "service_error", "{}", payload),
            Level::Warn => log_warn!(target: "service_error", "{}", payload),
            Level::Info => log_info!(target: "service_error", "{}", payload),
            Level::Debug | Level::Trace => debug!(target: "service_error", "{}", payload),
        }
    }
}

impl error::ResponseError for ServiceError {
    fn status_code(&self) -> StatusCode {
        self.http_status()
    }

    fn error_response(&self) -> HttpResponse {
        let envelope = ErrorEnvelope::from_error(self);
        self.log();
        HttpResponse::build(self.http_status())
            .insert_header(ContentType::json())
            .json(ResponseBody::new(&envelope.message.clone(), envelope))
    }
}

pub trait ErrorTransformer<T, E> {
    fn transform(&self, result: Result<T, E>) -> Result<T, E>;
}

pub trait ServiceResultExt<T> {
    fn map_service_error(self, mapper: impl Fn(ServiceError) -> ServiceError) -> ServiceResult<T>;

    fn log_on_error(self, level: Level) -> ServiceResult<T>;

    fn tap_error(self, tap: impl Fn(&ServiceError)) -> ServiceResult<T>;

    fn attach_context(self, updater: impl FnOnce(ErrorContext) -> ErrorContext)
        -> ServiceResult<T>;

    fn ensure<F>(self, predicate: F, error_fn: impl FnOnce() -> ServiceError) -> ServiceResult<T>
    where
        F: FnOnce(&T) -> bool;
}

impl<T> ServiceResultExt<T> for Result<T, ServiceError> {
    fn map_service_error(self, mapper: impl Fn(ServiceError) -> ServiceError) -> ServiceResult<T> {
        self.map_err(mapper)
    }

    fn log_on_error(self, level: Level) -> ServiceResult<T> {
        self.map_err(|err| {
            err.log_with_level(level);
            err
        })
    }

    fn tap_error(self, mut tap: impl FnMut(&ServiceError)) -> ServiceResult<T> {
        if let Err(ref err) = self {
            tap(err);
        }
        self
    }

    fn attach_context(
        self,
        updater: impl FnOnce(ErrorContext) -> ErrorContext,
    ) -> ServiceResult<T> {
        self.map_err(|err| err.with_context(updater))
    }

    fn ensure<F>(self, predicate: F, error_fn: impl FnOnce() -> ServiceError) -> ServiceResult<T>
    where
        F: FnOnce(&T) -> bool,
    {
        match self {
            Ok(value) if predicate(&value) => Ok(value),
            Ok(_) => Err(error_fn()),
            Err(err) => Err(err),
        }
    }
}

pub mod monadic {
    pub fn option_to_result<T, U, E, F, G>(
        option: Option<T>,
        f: F,
        default_error: G,
    ) -> Result<U, E>
    where
        F: FnOnce(T) -> U,
        G: FnOnce() -> E,
    {
        option.map(f).ok_or_else(default_error)
    }

    pub fn flatten_option<T, E, F, G>(
        result: Result<Option<T>, E>,
        error_mapper: F,
        none_error: G,
    ) -> Result<T, E>
    where
        F: FnOnce(E) -> E,
        G: FnOnce() -> E,
    {
        match result {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(none_error()),
            Err(e) => Err(error_mapper(e)),
        }
    }
}

pub mod error_pipeline {
    use super::ServiceResult;
    use std::sync::{Arc, Mutex};

    pub fn process_sequence<T, E, I, F>(initial: T, operations: I) -> Result<T, E>
    where
        I: IntoIterator<Item = F>,
        F: Fn(T) -> Result<T, E>,
    {
        operations
            .into_iter()
            .fold(Ok(initial), |acc, op| acc.and_then(op))
    }

    pub fn collect_successes<T, U, E, I, F>(results: I, transform: F) -> Vec<U>
    where
        I: IntoIterator<Item = Result<T, E>>,
        F: Fn(T) -> U,
    {
        results
            .into_iter()
            .filter_map(|r| r.ok())
            .map(transform)
            .collect()
    }

    pub fn build_error_reporter<E: Clone + Send + 'static>(
        errors: Arc<Mutex<Vec<E>>>,
        reporter: impl Fn(&E) + Send + 'static,
    ) -> impl FnMut(Result<(), E>) + Send + 'static {
        move |result: Result<(), E>| {
            if let Err(ref e) = result {
                reporter(e);
                if let Ok(mut vec) = errors.lock() {
                    vec.push(e.clone());
                }
            }
        }
    }

    /// A pipeline for composing error handling operations
    pub struct Pipeline<T, E> {
        steps: Vec<Box<dyn Fn(ServiceResult<T>) -> ServiceResult<T> + Send + Sync>>,
        _phantom: std::marker::PhantomData<E>,
    }

    impl<T, E> Pipeline<T, E> {
        /// Create a new pipeline
        pub fn new() -> Self {
            Self {
                steps: Vec::new(),
                _phantom: std::marker::PhantomData,
            }
        }

        /// Add a step to the pipeline
        pub fn step<F>(mut self, step: F) -> Self
        where
            F: Fn(ServiceResult<T>) -> ServiceResult<T> + Send + Sync + 'static,
        {
            self.steps.push(Box::new(step));
            self
        }

        /// Execute the pipeline with an initial result
        pub fn execute(self, initial: ServiceResult<T>) -> ServiceResult<T> {
            self.steps.into_iter().fold(initial, |acc, step| step(acc))
        }
    }

    impl<T, E> Default for Pipeline<T, E> {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod error_logging {
    use log::{debug, error, info, warn, Level};

    pub fn log_errors<T, E: std::fmt::Debug + Clone>(
        level: Level,
    ) -> impl FnMut(Result<T, E>) -> Result<T, E> {
        move |result: Result<T, E>| {
            if let Err(ref e) = result {
                match level {
                    Level::Error => error!("{:?}", e),
                    Level::Warn => warn!("{:?}", e),
                    Level::Info => info!("{:?}", e),
                    Level::Debug | Level::Trace => debug!("{:?}", e),
                }
            }
            result
        }
    }

    pub fn log_errors_if<T, E: std::fmt::Debug + Clone + 'static>(
        predicate: impl Fn(&E) -> bool + Send + Sync + 'static,
        level: Level,
    ) -> impl FnMut(Result<T, E>) -> Result<T, E> {
        let mut logger = log_errors::<T, E>(level);
        move |result: Result<T, E>| match result {
            Ok(_) => result,
            Err(ref e) if predicate(e) => logger(Err(e.clone())),
            Err(_) => result,
        }
    }

    pub fn compose_transformers<T, E, F, G>(
        first: F,
        second: G,
    ) -> impl FnMut(Result<T, E>) -> Result<T, E>
    where
        T: Send + 'static,
        E: Send + 'static,
        F: FnMut(Result<T, E>) -> Result<T, E> + Send + 'static,
        G: FnMut(Result<T, E>) -> Result<T, E> + Send + 'static,
    {
        let first = std::sync::Arc::new(std::sync::Mutex::new(first));
        let second = std::sync::Arc::new(std::sync::Mutex::new(second));

        move |result: Result<T, E>| {
            let intermediate = {
                let mut first_guard = first
                    .lock()
                    .expect("compose_transformers: first transformer mutex poisoned");
                (&mut *first_guard)(result)
            };

            let mut second_guard = second
                .lock()
                .expect("compose_transformers: second transformer mutex poisoned");

            (&mut *second_guard)(intermediate)
        }
    }

    pub fn chain_log_and_transform<T, E1, E2>(
        mut logger: impl FnMut(Result<T, E1>) -> Result<T, E1>,
        transformer: impl Fn(Result<T, E1>) -> Result<T, E2>,
    ) -> impl FnMut(Result<T, E1>) -> Result<T, E2> {
        move |result: Result<T, E1>| transformer(logger(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::ResponseError;
    use log::Level;
    use std::sync::{Arc, Mutex};

    #[test]
    fn service_error_context_builders() {
        let context = ErrorContext::default()
            .with_detail("missing field")
            .with_correlation_id("corr-123")
            .with_code("REQ-401")
            .with_metadata("field", "email")
            .with_tag("validation")
            .with_tags(["api", "validation"]);

        assert_eq!(context.detail.as_deref(), Some("missing field"));
        assert_eq!(context.correlation_id.as_deref(), Some("corr-123"));
        assert_eq!(context.code_override.as_deref(), Some("REQ-401"));
        assert!(context.metadata.get("field").is_some());
        assert!(context.tags.contains(&"validation".to_string()));
        assert!(context.tags.contains(&"api".to_string()));
    }

    #[test]
    fn service_error_with_context_applies_builder() {
        let error = ServiceError::bad_request("Invalid payload")
            .with_detail("email format is invalid")
            .with_correlation_id("abc-123")
            .with_metadata("field", "email")
            .with_tag("validation");

        let context = error.context();
        assert_eq!(context.detail.as_deref(), Some("email format is invalid"));
        assert_eq!(context.correlation_id.as_deref(), Some("abc-123"));
        assert_eq!(context.metadata.get("field"), Some(&"email".to_string()));
        assert!(context.tags.contains(&"validation".to_string()));
    }

    #[test]
    fn service_error_http_status_mapping() {
        assert_eq!(
            ServiceError::unauthorized("test").http_status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ServiceError::bad_request("test").http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ServiceError::internal_server_error("test").http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ServiceError::not_found("test").http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ServiceError::conflict("test").http_status(),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn service_result_ext_attach_context() {
        let result: ServiceResult<()> = Err(ServiceError::unauthorized("no token"));
        let enriched = result.attach_context(|ctx| ctx.with_detail("authorization header missing"));
        let error = enriched.unwrap_err();
        assert_eq!(
            error.context().detail.as_deref(),
            Some("authorization header missing")
        );
    }

    #[test]
    fn service_result_ext_ensure_predicate() {
        let result = Ok(10).ensure(
            |&value| value > 5,
            || ServiceError::bad_request("too small"),
        );
        assert!(result.is_ok());

        let failing = Ok(2).ensure(
            |&value| value > 5,
            || ServiceError::bad_request("too small"),
        );
        assert!(failing.is_err());
        assert_eq!(failing.unwrap_err().to_string(), "too small");
    }

    #[test]
    fn monadic_option_to_result_some() {
        let opt = Some(5);
        let result = monadic::option_to_result(opt, |x| x * 2, || "missing".to_string());
        assert_eq!(result, Ok(10));
    }

    #[test]
    fn monadic_option_to_result_none() {
        let opt: Option<i32> = None;
        let result = monadic::option_to_result(opt, |x| x * 2, || "missing".to_string());
        assert_eq!(result, Err("missing".to_string()));
    }

    #[test]
    fn monadic_flatten_option_success() {
        let result: Result<Option<i32>, String> = Ok(Some(42));
        let flattened = monadic::flatten_option(
            result,
            |e: String| format!("Error: {}", e),
            || "none".to_string(),
        );
        assert_eq!(flattened, Ok(42));
    }

    #[test]
    fn monadic_flatten_option_none() {
        let result: Result<Option<i32>, String> = Ok(None);
        let flattened = monadic::flatten_option(
            result,
            |e: String| format!("Error: {}", e),
            || "none".to_string(),
        );
        assert_eq!(flattened, Err("none".to_string()));
    }

    #[test]
    fn monadic_flatten_option_error() {
        let result: Result<Option<i32>, String> = Err("oops".to_string());
        let flattened = monadic::flatten_option(
            result,
            |e: String| format!("wrapped {}", e),
            || "none".to_string(),
        );
        assert_eq!(flattened, Err("wrapped oops".to_string()));
    }

    #[test]
    fn error_pipeline_process_sequence_success() {
        let operations: Vec<fn(i32) -> Result<i32, &'static str>> =
            vec![|x| Ok(x + 1), |x| Ok(x * 2), |x| Ok(x - 3)];
        let result = error_pipeline::process_sequence(5, operations);
        assert_eq!(result, Ok(9));
    }

    #[test]
    fn error_pipeline_process_sequence_failure() {
        let operations: Vec<fn(i32) -> Result<i32, &'static str>> =
            vec![|x| Ok(x + 1), |_| Err("fail"), |x| Ok(x * 2)];
        let result = error_pipeline::process_sequence(5, operations);
        assert_eq!(result, Err("fail"));
    }

    #[test]
    fn error_pipeline_collect_successes() {
        let results = vec![Ok(1), Err("bad"), Ok(2)];
        let successes = error_pipeline::collect_successes(results, |x| x * 10);
        assert_eq!(successes, vec![10, 20]);
    }

    #[test]
    fn error_pipeline_build_error_reporter() {
        let errors = Arc::new(Mutex::new(Vec::<String>::new()));
        let reported = Arc::new(Mutex::new(Vec::new()));
        let reporter = {
            let reported_clone = reported.clone();
            move |err: &String| {
                reported_clone.lock().unwrap().push(err.clone());
            }
        };
        let mut collect_fn = error_pipeline::build_error_reporter(errors.clone(), reporter);
        collect_fn(Err("err1".to_string()));
        collect_fn(Ok(()));
        collect_fn(Err("err2".to_string()));
        assert_eq!(errors.lock().unwrap().len(), 2);
        assert_eq!(reported.lock().unwrap().len(), 2);
    }

    #[test]
    fn error_pipeline_pipeline_executes_steps() {
        let pipeline = error_pipeline::Pipeline::<i32, ServiceError>::new()
            .step(|result: ServiceResult<i32>| result.map(|value| value + 1))
            .step(|result| result.map(|value| value * 2));
        let result = pipeline.execute(Ok(5));
        assert_eq!(result, Ok(12));
    }

    #[test]
    fn error_logging_log_errors_pass_through() {
        let mut logger = error_logging::log_errors::<i32, String>(Level::Error);
        let ok = logger(Ok(5));
        assert_eq!(ok, Ok(5));
        let err = logger(Err("boom".to_string()));
        assert_eq!(err, Err("boom".to_string()));
    }

    #[test]
    fn error_logging_chain_log_and_transform() {
        let logger = error_logging::log_errors::<i32, String>(Level::Error);
        let transformer = |r: Result<i32, String>| r.map_err(|e| format!("handled: {}", e));
        let mut combined = error_logging::chain_log_and_transform(logger, transformer);
        let err = combined(Err("boom".to_string()));
        assert_eq!(err, Err("handled: boom".to_string()));
    }

    #[test]
    fn response_error_envelope_contains_context() {
        let error = ServiceError::not_found("missing")
            .with_detail("resource abc not found")
            .with_correlation_id("corr-xyz")
            .with_metadata("resource", "abc");
        let response = error.error_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // Tests for Clone trait implementation on ServiceError
    #[test]
    fn service_error_clone_unauthorized() {
        let error = ServiceError::unauthorized("Invalid credentials");
        let cloned = error.clone();

        match (&error, &cloned) {
            (
                ServiceError::Unauthorized {
                    error_message: msg1,
                    ..
                },
                ServiceError::Unauthorized {
                    error_message: msg2,
                    ..
                },
            ) => {
                assert_eq!(msg1, msg2);
                assert_eq!(msg1, "Invalid credentials");
            }
            _ => panic!("Cloned error has different variant"),
        }
    }

    #[test]
    fn service_error_clone_bad_request() {
        let error = ServiceError::bad_request("Missing field");
        let cloned = error.clone();

        match (&error, &cloned) {
            (
                ServiceError::BadRequest {
                    error_message: msg1,
                    ..
                },
                ServiceError::BadRequest {
                    error_message: msg2,
                    ..
                },
            ) => {
                assert_eq!(msg1, msg2);
            }
            _ => panic!("Cloned error has different variant"),
        }
    }

    #[test]
    fn service_error_clone_not_found() {
        let error = ServiceError::not_found("Resource not found");
        let cloned = error.clone();

        match (&error, &cloned) {
            (
                ServiceError::NotFound {
                    error_message: msg1,
                    ..
                },
                ServiceError::NotFound {
                    error_message: msg2,
                    ..
                },
            ) => {
                assert_eq!(msg1, msg2);
            }
            _ => panic!("Cloned error has different variant"),
        }
    }

    #[test]
    fn service_error_clone_internal_server_error() {
        let error = ServiceError::internal_server_error("Database connection failed");
        let cloned = error.clone();

        match (&error, &cloned) {
            (
                ServiceError::InternalServerError {
                    error_message: msg1,
                    ..
                },
                ServiceError::InternalServerError {
                    error_message: msg2,
                    ..
                },
            ) => {
                assert_eq!(msg1, msg2);
            }
            _ => panic!("Cloned error has different variant"),
        }
    }

    #[test]
    fn service_error_clone_conflict() {
        let error = ServiceError::conflict("Duplicate entry");
        let cloned = error.clone();

        match (&error, &cloned) {
            (
                ServiceError::Conflict {
                    error_message: msg1,
                    ..
                },
                ServiceError::Conflict {
                    error_message: msg2,
                    ..
                },
            ) => {
                assert_eq!(msg1, msg2);
            }
            _ => panic!("Cloned error has different variant"),
        }
    }

    #[test]
    fn service_error_clone_independence() {
        let error = ServiceError::bad_request("original");
        let cloned = error.clone();

        // Modify original (by reassigning, since fields are private)
        let _modified = ServiceError::bad_request("modified");

        // Cloned should still have original value
        match cloned {
            ServiceError::BadRequest { error_message, .. } => {
                assert_eq!(error_message, "original");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn service_error_clone_in_vec() {
        let errors = vec![
            ServiceError::unauthorized("error1"),
            ServiceError::bad_request("error2"),
            ServiceError::not_found("error3"),
        ];

        let cloned_errors = errors.clone();

        assert_eq!(errors.len(), cloned_errors.len());
        for (orig, clone) in errors.iter().zip(cloned_errors.iter()) {
            match (orig, clone) {
                (
                    ServiceError::Unauthorized {
                        error_message: msg1,
                        ..
                    },
                    ServiceError::Unauthorized {
                        error_message: msg2,
                        ..
                    },
                ) => assert_eq!(msg1, msg2),
                (
                    ServiceError::BadRequest {
                        error_message: msg1,
                        ..
                    },
                    ServiceError::BadRequest {
                        error_message: msg2,
                        ..
                    },
                ) => assert_eq!(msg1, msg2),
                (
                    ServiceError::NotFound {
                        error_message: msg1,
                        ..
                    },
                    ServiceError::NotFound {
                        error_message: msg2,
                        ..
                    },
                ) => assert_eq!(msg1, msg2),
                _ => panic!("Mismatched variants"),
            }
        }
    }

    #[test]
    fn service_error_clone_in_result() {
        let result: Result<i32, ServiceError> = Err(ServiceError::internal_server_error("Failed"));
        let cloned_result = result.clone();

        assert!(result.is_err());
        assert!(cloned_result.is_err());

        if let (Err(orig), Err(clone)) = (result, cloned_result) {
            match (orig, clone) {
                (
                    ServiceError::InternalServerError {
                        error_message: msg1,
                        ..
                    },
                    ServiceError::InternalServerError {
                        error_message: msg2,
                        ..
                    },
                ) => {
                    assert_eq!(msg1, msg2);
                }
                _ => panic!("Mismatched errors"),
            }
        }
    }

    #[test]
    fn service_error_clone_preserves_response_error_trait() {
        let error = ServiceError::bad_request("test");
        let cloned = error.clone();

        // Both should implement ResponseError
        assert_eq!(error.status_code(), cloned.status_code());
        assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn service_error_clone_with_empty_message() {
        let error = ServiceError::unauthorized("");
        let cloned = error.clone();

        match cloned {
            ServiceError::Unauthorized { error_message, .. } => {
                assert_eq!(error_message, "");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn service_error_clone_with_unicode() {
        let error = ServiceError::bad_request("Erreur: données invalides 🚫");
        let cloned = error.clone();

        match cloned {
            ServiceError::BadRequest { error_message, .. } => {
                assert_eq!(error_message, "Erreur: données invalides 🚫");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn service_error_clone_with_long_message() {
        let long_message = "a".repeat(10000);
        let error = ServiceError::internal_server_error(&long_message);
        let cloned = error.clone();

        match cloned {
            ServiceError::InternalServerError { error_message, .. } => {
                assert_eq!(error_message, long_message);
                assert_eq!(error_message.len(), 10000);
            }
            _ => panic!("Wrong variant"),
        }
    }
}
