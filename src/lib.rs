//! # Actix Web REST API with Multi-Tenant JWT and Functional Programming
//!
//! This library provides the core functionality for the multi-tenant REST API
//! with JWT authentication and advanced functional programming capabilities.

pub mod types;
pub mod api;
pub mod config;
pub mod constants;
pub mod error;
#[cfg(feature = "functional")]
pub use rcs_functional as functional;
pub mod middleware;
pub mod models;
pub mod pagination;
pub mod schema;
pub mod services;
pub mod unified_pagination;
pub mod utils;

#[cfg(test)]
mod integration_test_fixes;
