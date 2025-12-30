//! # RCS Functional Programming Library
//!
//! This library provides advanced functional programming capabilities
//! for the Actix Web REST API, leveraging itertools and Rust's
//! functional programming features to create efficient, composable
//! data processing pipelines.
//!
//! Key components:
//! - Iterator Engine: Core iterator chain processing with itertools integration
//! - Chain Builder: Fluent API for building complex iterator chains
//! - Pure Function Registry: Storage and composition of pure functions
//! - Immutable State Management: Functional state handling with structural sharing
//! - State Transitions: High-level functional state transition operations
//! - Query Composition: Type-safe functional query building
//! - Validation Engine: Iterator-based validation pipelines
//! - Lazy Evaluation: Deferred computation patterns
//! - Concurrent Processing: Parallel functional operations
//! - Response Transformers: Composable API response formatting
//! - Error Handling: Monadic error processing
//! - Pagination: Iterator-based pagination
//! - Performance Monitoring: Functional pipeline metrics

pub mod chain_builder;
pub mod concurrent_processing;
pub mod constants;
pub mod function_traits;
pub mod functional_tests;
pub mod immutable_state;
pub mod iterator_engine;
pub mod math_functions;
pub mod models;
pub mod pagination;
pub mod parallel_iterators;
pub mod performance_monitoring;
pub mod prelude;
pub mod pure_function_registry;
pub mod query_builder;
pub mod query_builders;
pub mod query_composition;
pub mod response_transformers;
pub mod schema;
pub mod state_transitions;
pub mod validation_engine;
pub mod validation_integration;
pub mod validation_rules;

// Re-export commonly used types for convenience
pub use chain_builder::ChainBuilder;
pub use iterator_engine::{IteratorChain, IteratorEngine};
pub use pure_function_registry::{PureFunctionRegistry, RegistryError, SharedRegistry};
