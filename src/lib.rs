//! # Actix Web REST API with Multi-Tenant JWT and Functional Programming
//!
//! This library provides the core functionality for the multi-tenant REST API
//! with JWT authentication and advanced functional programming capabilities.

pub mod api;
pub mod config;
pub mod constants;
pub mod error;
pub mod functional;
pub mod middleware;
pub mod models;
pub mod pagination;
pub mod schema;
pub mod services;
pub mod utils;

#[cfg(test)]
mod integration_test_fixes;
