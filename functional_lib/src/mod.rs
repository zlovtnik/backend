//! Functional Programming Infrastructure
//!
//! This module provides advanced functional programming capabilities
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

pub mod backward_compatibility;
pub mod chain_builder;
pub mod concurrent_processing;
pub mod function_traits;
pub mod functional_tests;
pub mod immutable_state;
pub mod iterator_engine;
pub mod math_functions;
pub mod pagination;
pub mod parallel_iterators;
pub mod performance_monitoring;
pub mod prelude;
pub mod pure_function_registry;
pub mod query_builder;
pub mod query_builders;
pub mod query_composition;
pub mod response_transformers;
pub mod state_transitions;
pub mod validation_engine;
pub mod validation_integration;
pub mod validation_rules;

// Re-export commonly used types for convenience
// Commented out to avoid unused import warnings
// pub use chain_builder::ChainBuilder;
// pub use iterator_engine::{IteratorChain, IteratorEngine};
// pub use pure_function_registry::{PureFunctionRegistry, RegistryError, SharedRegistry};

// pub use immutable_state::{
//     ImmutableRef, ImmutableStateManager, PersistentHashMap, PersistentVector,
//     QueryResult, StateTransitionMetrics, TenantApplicationState
// };
// pub use state_transitions::{TransitionError, TransitionResult, build_login_transitions, build_logout_transitions};

// Re-export query composition types
// pub use query_builder::{
//     Column, Operator, Predicate, QueryFilter, TypeSafeQueryBuilder, LogicOperator,
//     equals, contains, compare, null_check
// };
// pub use query_composition::{
//     ComposablePredicate, FunctionalQueryComposer, LazyEvaluationConfig,
//     QueryPerformanceMetrics, ParameterSanitizer, QueryOptimizationEngine,
//     composable_predicate, field_filter_to_composable
// };

// Re-export validation types
// pub use validation_engine::{
//     ValidationEngine, ValidationConfig, ValidationContext, ValidationOutcome,
//     ValidationPipeline, ValidationPipelineResult, LazyValidationIterator,
//     validator, validator_with_config, validate_struct_field
// };
// pub use validation_rules::{
//     ValidationRule, ValidationResult, ValidationError,
//     Required, Length, Email, Range, Phone, Url, Custom, OneOf, Unique, MustBeTrue,
//     all, any, not, when, validate_collection, cross_field_validate
// };
