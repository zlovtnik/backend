//! # Actix Web REST API with Multi-Tenant JWT and Functional Programming
//!
//! This library provides the core functionality for the multi-tenant REST API
//! with JWT authentication and advanced functional programming capabilities.

pub mod api;
pub mod config;
pub mod constants;
pub mod error;
pub mod types;
#[cfg(feature = "functional")]
pub use rcs_functional as functional;

#[cfg(not(feature = "functional"))]
pub mod functional {
    pub mod performance_monitoring {
        pub enum OperationType {
            Custom(String),
        }
        #[derive(serde::Serialize)]
        pub struct HealthSummary;
        pub fn get_performance_monitor() -> PerformanceMonitor {
            PerformanceMonitor
        }
        pub struct PerformanceMonitor;
        impl PerformanceMonitor {
            pub fn get_health_summary(&self) -> HealthSummary {
                HealthSummary
            }
        }
    }
    pub mod response_transformers {
        use actix_web::{http::StatusCode, HttpRequest, HttpResponse};
        use serde::Serialize;
        use std::borrow::Cow;

        pub struct ResponseTransformer<T> {
            data: T,
            message: Option<Cow<'static, str>>,
            status: StatusCode,
        }

        pub type ResponseTransformError = String;

        impl<T: Serialize> ResponseTransformer<T> {
            pub fn new(data: T) -> Self {
                Self {
                    data,
                    message: None,
                    status: StatusCode::OK,
                }
            }

            pub fn with_message(mut self, message: impl Into<Cow<'static, str>>) -> Self {
                self.message = Some(message.into());
                self
            }

            pub fn with_status(mut self, status: StatusCode) -> Self {
                self.status = status;
                self
            }

            pub fn try_with_metadata(
                self,
                _metadata: serde_json::Value,
            ) -> Result<Self, ResponseTransformError> {
                Ok(self)
            }

            pub fn respond_to(self, _req: &HttpRequest) -> HttpResponse {
                HttpResponse::build(self.status).json(serde_json::json!({
                    "data": self.data,
                    "message": self.message,
                }))
            }
        }
    }
    pub mod pagination {
        pub use crate::pagination::Pagination;
    }
    pub mod query_builder {
        pub struct Column<T, C>(std::marker::PhantomData<(T, C)>);
        impl<T, C> Column<T, C> {
            pub fn new(_table: String, _column: String) -> Self {
                Self(std::marker::PhantomData)
            }
        }
    }
    pub mod validation_engine {
        pub struct ValidationConfig;
        impl Default for ValidationConfig {
            fn default() -> Self {
                Self
            }
        }
        pub struct ValidationEngine<T> {
            pub errors: Vec<super::validation_rules::ValidationError>,
            _marker: std::marker::PhantomData<T>,
        }
        impl<T> Clone for ValidationEngine<T> {
            fn clone(&self) -> Self {
                Self {
                    errors: self.errors.clone(),
                    _marker: std::marker::PhantomData,
                }
            }
        }
        impl<T> ValidationEngine<T> {
            pub fn new() -> Self {
                Self {
                    errors: Vec::new(),
                    _marker: std::marker::PhantomData,
                }
            }
            pub fn with_config(_config: ValidationConfig) -> Self {
                Self::new()
            }
            pub fn validate_field(
                &self,
                _value: &T,
                _field: &str,
                _rules: Vec<super::validation_rules::Custom>,
            ) -> Self {
                self.clone()
            }
        }
    }
    pub mod validation_rules {
        pub struct Custom;
        impl Custom {
            pub fn new(_func: impl Fn(&String) -> bool, _code: &str, _message: &str) -> Self {
                Self
            }
        }
        #[derive(Clone)]
        pub struct ValidationError {
            pub message: String,
        }
    }
}

pub mod middleware;
pub mod models;
pub mod pagination; // Legacy index-based pagination
pub mod schema;
pub mod services;
#[cfg(feature = "functional")]
pub use functional::unified_pagination; // Recommended cursor-based pagination
pub mod utils;

#[cfg(test)]
mod integration_test_fixes;
