//! Models Module with Functional Programming Support
//!
//! This module provides database models with integrated functional programming
//! capabilities from the functional crate. Models leverage type-safe query builders,
//! iterator-based validation, and composable data processing pipelines.
//!
//! Key Features:
//! - Functional query composition with lazy evaluation
//! - Iterator-based validation engines
//! - Type-safe column references and predicates
//! - Pure function registries for data transformations
//! - Performance monitoring for database operations

pub mod filters;
pub mod login_history;
pub mod nfe_cofins;
pub mod nfe_document;
pub mod nfe_emitter;
pub mod nfe_icms;
pub mod nfe_ipi;
pub mod nfe_item;
pub mod nfe_pis;
pub mod nfe_product;
pub mod nfe_recipient;
pub mod pagination;
pub mod person;
pub mod refresh_token;
pub mod response;
pub mod tenant;
pub mod user;
pub mod user_token;

// Re-export functional programming utilities for model operations
pub use crate::functional::{
    query_builder::Column,
    validation_engine::{ValidationConfig, ValidationEngine},
    validation_rules::{Custom, Email, Length, Phone, Range, ValidationError},
};

// Re-export commonly used functional traits

// Functional model utilities
pub mod functional_utils {
    //! Functional utilities specifically for model operations

    use super::*;

    /// Create a type-safe column reference for functional queries.
    ///
    /// The returned `Column<T, C>` carries the target row type `T` and the column value type `C`,
    /// and is identified by the provided table and column names.
    ///
    /// # Examples
    ///
    /// ```
    /// let _col: crate::functional::query_builder::Column<i32, String> = column("users", "id");
    /// ```
    pub fn column<T, C>(table: &str, column: &str) -> Column<T, C> {
        Column::new(table.to_string(), column.to_string())
    }

    /// Creates a validation engine with the default configuration for the target type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// let _engine = crate::models::functional_utils::validation_engine::<i32>();
    /// ```
    pub fn validation_engine<T>() -> ValidationEngine<T> {
        ValidationEngine::new()
    }

    /// Creates a validation engine configured for the target type `T`.
    ///
    /// The provided `config` customizes validation rules and behavior used by the returned engine.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::models::functional_utils::{validation_engine_with_config, ValidationConfig};
    /// struct User { name: String }
    /// let cfg = ValidationConfig::default();
    /// let engine = validation_engine_with_config::<User>(cfg);
    /// ```
    pub fn validation_engine_with_config<T>(config: ValidationConfig) -> ValidationEngine<T> {
        ValidationEngine::with_config(config)
    }

    /// Return the list of validation errors with their full structured data.
    ///
    /// Returns a `Vec<ValidationError>` containing the full details of each validation error in the same order.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::models::functional_utils::{to_error_messages, ValidationError};
    ///
    /// let errors: Vec<ValidationError> = vec![/* ... */];
    /// let structured_errors = to_error_messages(errors);
    /// // Each error contains field, code, and message.
    /// ```
    pub fn to_error_messages(errors: Vec<ValidationError>) -> Vec<String> {
        errors.into_iter().map(|error| error.message).collect()
    }
}
