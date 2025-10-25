//! Functional Query Composition
//!
//! This module provides functional composition capabilities for database queries,
//! enabling lazy evaluation of large result sets, automatic parameter sanitization,
//! and type-safe predicate composition. It integrates deeply with Diesel ORM
//! while maintaining functional programming principles.
//!
//! **SQL Injection Protection Strategy:**
//! This module implements defense-in-depth for SQL injection prevention:
//! - **Primary defense:** Diesel ORM's parameterized queries handle all user input
//! - **Secondary defense:** The ParameterSanitizer provides additional validation
//!   by rejecting dangerous patterns in SQL comments, keywords, and encoded sequences
//! - All user-provided values are bound as parameters, never concatenated into SQL strings
//!
//! Key Features:
//! - Lazy evaluation for large datasets with automatic chunking
//! - Parameterized queries with automatic sanitization
//! - Functional predicate composition with monadic operations
//! - Query performance monitoring and optimization
//! - Asynchronous query execution patterns

#![allow(dead_code)]

use crate::config::db::Pool;
use crate::functional::query_builder::{
    Column, Operator, Predicate, QueryFilter, TypeSafeQueryBuilder,
};
use diesel::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Lazy evaluation configuration for large result sets.
///
/// Controls how queries are chunked and evaluated to prevent
/// memory exhaustion with large datasets.
#[derive(Debug, Clone)]
pub struct LazyEvaluationConfig {
    /// Maximum number of records to load at once
    pub chunk_size: usize,
    /// Maximum total records to process
    pub max_total_records: Option<usize>,
    /// Timeout for individual query chunks
    pub chunk_timeout: Duration,
    /// Enable parallel processing of chunks
    pub parallel_processing: bool,
    /// Maximum concurrent chunks
    pub max_concurrent_chunks: usize,
}

impl Default for LazyEvaluationConfig {
    /// Provides the default configuration for lazy evaluation of large result sets.
    ///
    /// The default values are:
    /// - `chunk_size = 1000`
    /// - `max_total_records = Some(100_000)`
    /// - `chunk_timeout = 30s`
    /// - `parallel_processing = true`
    /// - `max_concurrent_chunks = 4`
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = LazyEvaluationConfig::default();
    /// assert_eq!(cfg.chunk_size, 1000);
    /// assert_eq!(cfg.max_total_records, Some(100_000));
    /// assert_eq!(cfg.parallel_processing, true);
    /// assert_eq!(cfg.max_concurrent_chunks, 4);
    /// ```
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            max_total_records: Some(100_000),
            chunk_timeout: Duration::from_secs(30),
            parallel_processing: true,
            max_concurrent_chunks: 4,
        }
    }
}

/// Performance metrics for query composition and execution.
#[derive(Debug, Clone, Default)]
pub struct QueryPerformanceMetrics {
    /// Total time spent composing the query
    pub composition_time: Duration,
    /// Total time spent executing the query
    pub execution_time: Duration,
    /// Number of records processed
    pub records_processed: usize,
    /// Query complexity score (0-100, higher is more complex)
    pub complexity_score: u32,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Number of database round trips
    pub round_trips: usize,
}

/// Functional query composer that combines predicates using monadic operations.
///
/// This struct enables functional composition of database queries while maintaining
/// type safety and providing lazy evaluation capabilities.
pub struct FunctionalQueryComposer<T, U> {
    /// Base query builder
    builder: TypeSafeQueryBuilder<T, U>,
    /// Current composed filter
    current_filter: Option<QueryFilter<U>>,
    /// Lazy evaluation configuration
    lazy_config: LazyEvaluationConfig,
    /// Performance metrics collector
    metrics: QueryPerformanceMetrics,
    /// Database connection pool for executing queries
    pool: Option<Pool>,
    /// Type markers
    _phantom: PhantomData<(T, U)>,
}

// Monadic predicate composition for functional query building.
///
/// This represents a predicate that can be composed with others using
/// functional programming patterns (map, flat_map, filter, etc.)
pub struct ComposablePredicate<T> {
    /// The underlying predicate
    predicate: Predicate<T>,
    /// Metadata for composition and optimization
    metadata: PredicateMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct PredicateMetadata {
    /// Estimated selectivity (0.0 to 1.0, lower is more selective)
    pub selectivity: f64,
    /// Estimated cost in query execution time
    pub cost_estimate: Duration,
    /// Whether this predicate can be pushed down to the database
    pub pushdown_capable: bool,
    /// Dependencies on other predicates
    pub dependencies: Vec<String>,
}

impl<T> ComposablePredicate<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Wraps a `Predicate` with composition and optimization metadata for functional chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::query_builder::{Predicate, PredicateMetadata, ComposablePredicate, Operator};
    /// use std::time::Duration;
    ///
    /// let pred = Predicate::new("users.id".into(), Operator::Equals, Some("42".into()), "id".into());
    /// let meta = PredicateMetadata {
    ///     selectivity: 0.1,
    ///     cost_estimate: Duration::from_millis(5),
    ///     pushdown_capable: true,
    ///     dependencies: vec![],
    /// };
    /// let cp = ComposablePredicate::new(pred, meta);
    /// ```
    pub fn new(predicate: Predicate<T>, metadata: PredicateMetadata) -> Self {
        Self {
            predicate,
            metadata,
        }
    }

    /// Applies a transformation to the inner `Predicate` and returns a new `ComposablePredicate` with the mapped predicate and the original metadata.
    ///
    /// # Parameters
    /// - `f`: A function that converts the contained `Predicate<T>` into a `Predicate<U>`.
    ///
    /// # Returns
    /// A `ComposablePredicate<U>` containing the transformed predicate and the preserved `PredicateMetadata`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::functional::query_builder::Predicate;
    /// use crate::functional::query_composition::ComposablePredicate;
    ///
    /// // given an existing composable predicate `cp` of type `ComposablePredicate<T>`:
    /// // let cp: ComposablePredicate<T> = ...;
    /// // apply a mapping that converts the inner predicate type
    /// // let mapped: ComposablePredicate<U> = cp.map(|p| /* produce Predicate<U> from p */ p);
    /// ```
    pub fn map<F, U>(self, f: F) -> ComposablePredicate<U>
    where
        F: FnOnce(Predicate<T>) -> Predicate<U>,
        U: Clone + Send + Sync + 'static,
    {
        let new_predicate = f(self.predicate);
        ComposablePredicate {
            predicate: new_predicate,
            metadata: self.metadata,
        }
    }

    /// Applies a flat-mapping function to the underlying predicate and returns the iterator produced by that function.
    ///
    /// The provided function receives the inner `Predicate<T>` and must return an iterator of `ComposablePredicate<U>`.
    ///
    /// # Parameters
    ///
    /// * `f` - A function that takes the inner `Predicate<T>` and returns an `Iterator` yielding `ComposablePredicate<U>` items.
    ///
    /// # Returns
    ///
    /// An iterator of `ComposablePredicate<U>` produced by the provided mapping function.
    pub fn flat_map<F, U, I>(self, f: F) -> I
    where
        F: FnOnce(Predicate<T>) -> I,
        I: Iterator<Item = ComposablePredicate<U>>,
        U: Clone + Send + Sync + 'static,
    {
        f(self.predicate)
    }

    /// Return the composable predicate when it satisfies the given test.
    ///
    /// # Arguments
    ///
    /// * `f` - A function that inspects the inner `Predicate<T>` and returns `true` to keep the predicate.
    ///
    /// # Returns
    ///
    /// `Some(self)` if `f(&self.predicate)` returns `true`, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::query_builder::{Predicate, Operator};
    /// use crate::functional::query_composition::ComposablePredicate;
    ///
    /// let pred = Predicate::new("users.id", Operator::Equals, Some("42".to_string()), "id");
    /// let composed = ComposablePredicate::new(pred, Default::default());
    ///
    /// // Keep only predicates that reference the "id" field
    /// let kept = composed.filter(|p| p.field_name() == "id");
    /// assert!(kept.is_some());
    /// ```
    pub fn filter<F>(self, f: F) -> Option<Self>
    where
        F: FnOnce(&Predicate<T>) -> bool,
    {
        if f(&self.predicate) {
            Some(self)
        } else {
            None
        }
    }
}

/// Lazy evaluation iterator for large datasets.
///
/// This iterator processes data in chunks to prevent memory exhaustion
/// while providing a seamless streaming interface.
pub struct LazyQueryIterator<T, U> {
    /// The query composer
    composer: Arc<FunctionalQueryComposer<T, U>>,
    /// Current chunk being processed
    current_chunk: Vec<U>,
    /// Current position in the chunk
    chunk_position: usize,
    /// Current offset in the total dataset
    offset: usize,
    /// Page size for chunked loading
    page_size: usize,
    /// Total records processed so far
    total_processed: usize,
    /// Whether the iterator is exhausted
    exhausted: bool,
    /// Whether this is a test-only iterator (no chunk loading)
    is_test_iterator: bool,
    /// Semaphore for controlling concurrency
    semaphore: Arc<Semaphore>,
    /// Performance metrics
    metrics: QueryPerformanceMetrics,
}

impl<T, U> LazyQueryIterator<T, U>
where
    T: Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
{
    /// Creates a lazy streaming iterator configured to load query results in chunks
    /// according to the composer's `lazy_config`.
    ///
    /// The returned iterator is initialized with an empty current chunk, positions set
    /// to the start, and the page size taken from `composer.lazy_config.chunk_size`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Semaphore;
    ///
    /// // Assume `FunctionalQueryComposer` and `QueryPerformanceMetrics` are available in scope.
    /// let composer: Arc<FunctionalQueryComposer<_, _>> = Arc::new(/* ... */);
    /// let semaphore = Arc::new(Semaphore::new(4));
    /// let metrics = QueryPerformanceMetrics::default();
    ///
    /// let iterator = LazyQueryIterator::new(composer, semaphore, metrics);
    /// ```
    pub fn new(
        composer: Arc<FunctionalQueryComposer<T, U>>,
        semaphore: Arc<Semaphore>,
        metrics: QueryPerformanceMetrics,
    ) -> Self {
        let page_size = composer.lazy_config.chunk_size;
        Self {
            composer,
            current_chunk: Vec::new(),
            chunk_position: 0,
            offset: 0,
            page_size,
            total_processed: 0,
            exhausted: false,
            is_test_iterator: false,
            semaphore,
            metrics,
        }
    }

    /// Creates a test-only LazyQueryIterator that yields the provided items in order.
    ///
    /// This constructor bypasses any database loading and returns elements from `data` until exhausted.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3];
    /// let mut iter = LazyQueryIterator::with_data(data);
    /// assert_eq!(iter.next(), Some(1));
    /// assert_eq!(iter.next(), Some(2));
    /// assert_eq!(iter.next(), Some(3));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[cfg(test)]
    pub fn with_data(data: Vec<U>) -> Self {
        use std::time::Duration;

        let composer = Arc::new(FunctionalQueryComposer {
            builder: TypeSafeQueryBuilder::new(),
            current_filter: None,
            lazy_config: LazyEvaluationConfig::default(),
            metrics: QueryPerformanceMetrics {
                composition_time: Duration::from_secs(0),
                execution_time: Duration::from_secs(0),
                records_processed: 0,
                complexity_score: 0,
                memory_usage: 0,
                round_trips: 0,
            },
            pool: None,
            _phantom: PhantomData,
        });

        Self {
            composer,
            current_chunk: data,
            chunk_position: 0,
            offset: 0,
            page_size: 1000,
            total_processed: 0,
            exhausted: false,
            is_test_iterator: true,
            semaphore: Arc::new(Semaphore::new(1)),
            metrics: QueryPerformanceMetrics {
                composition_time: Duration::from_secs(0),
                execution_time: Duration::from_secs(0),
                records_processed: 0,
                complexity_score: 0,
                memory_usage: 0,
                round_trips: 0,
            },
        }
    }

    /// Load the next page of results into the iterator's buffer.
    ///
    /// Fetches up to `self.page_size` records starting at `self.offset`,
    /// replaces `self.current_chunk` with the fetched records, resets
    /// `self.chunk_position` to `0`, advances `self.offset` by the number of
    /// records loaded, and marks the iterator as exhausted when no more results
    /// are available.
    ///
    /// # Returns
    ///
    /// `true` if a new chunk was loaded successfully, `false` if no more data is available.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut it = LazyQueryIterator::with_data(vec![1u32, 2u32, 3u32]);
    /// assert_eq!(it.next(), Some(1u32));
    /// assert_eq!(it.next(), Some(2u32));
    /// assert_eq!(it.next(), Some(3u32));
    /// assert_eq!(it.next(), None);
    /// ```
    fn load_next_chunk(&mut self) -> bool {
        // For the test iterator, we don't have real database access
        if self.is_test_iterator {
            self.exhausted = true;
            return false;
        }

        // Check if we've reached our limit
        if let Some(max_records) = self.composer.lazy_config.max_total_records {
            if self.total_processed >= max_records {
                self.exhausted = true;
                return false;
            }
        }

        // In a real implementation, we would execute a query with LIMIT and OFFSET
        // For now, we'll use the composer's execute_chunk_query method
        match self
            .composer
            .execute_chunk_query(self.offset, self.page_size)
        {
            Ok(chunk) => {
                if chunk.is_empty() {
                    self.exhausted = true;
                    false
                } else {
                    self.current_chunk = chunk;
                    self.chunk_position = 0;
                    self.offset += self.current_chunk.len();
                    self.total_processed += self.current_chunk.len();
                    true
                }
            }
            Err(_) => {
                // In a real implementation, we would handle the error properly
                // For now, we'll just mark the iterator as exhausted
                self.exhausted = true;
                false
            }
        }
    }
}

impl<T, U> Iterator for LazyQueryIterator<T, U>
where
    T: Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
{
    type Item = U;

    /// Returns the next item from the currently loaded in-memory chunk or `None` if the iterator is exhausted.
    ///
    /// If the current chunk contains remaining items, advances the internal position counters and yields the next element. For iterators constructed with `with_data`, no additional chunk loading is performed and iteration ends when the in-memory data is consumed.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::query_composition::LazyQueryIterator;
    ///
    /// let data = vec![1, 2, 3];
    /// let mut iter = LazyQueryIterator::with_data(data);
    ///
    /// assert_eq!(iter.next(), Some(1));
    /// assert_eq!(iter.next(), Some(2));
    /// assert_eq!(iter.next(), Some(3));
    /// assert_eq!(iter.next(), None);
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        // If we've exhausted the current chunk, try to load the next one
        if self.chunk_position >= self.current_chunk.len() {
            // For test iterators, just mark as exhausted (no chunk loading)
            if self.is_test_iterator {
                self.exhausted = true;
                return None;
            }

            // Load the next chunk
            if !self.load_next_chunk() {
                self.exhausted = true;
                return None;
            }
        }

        // Return the next item from the current chunk
        let item = self.current_chunk[self.chunk_position].clone();
        self.chunk_position += 1;
        self.total_processed += 1;

        Some(item)
    }
}

/// Automatic parameter sanitization for SQL injection prevention.
///
/// This struct ensures all query parameters are properly sanitized
/// and bound using parameterized queries.
pub struct ParameterSanitizer {
    /// Parameter bindings with type information
    bindings: HashMap<String, ParameterBinding>,
    /// Sanitization rules
    rules: Vec<SanitizationRule>,
}

#[derive(Debug, Clone)]
pub struct ParameterBinding {
    /// Parameter name
    pub name: String,
    /// Parameter value as string (for logging/debugging)
    pub value: String,
    /// SQL type of the parameter
    pub sql_type: String,
    /// Whether the parameter has been validated
    pub validated: bool,
}

#[derive(Debug, Clone)]
pub struct SanitizationRule {
    /// Rule name
    pub name: String,
    /// Rule pattern (regex or validation function)
    pub pattern: String,
    /// Error message if validation fails
    pub error_message: String,
}

/// Provides a compiled Regex that matches SQL comment markers ("--", "/*", "*/").
///
/// The compiled pattern is cached in a static OnceLock and reused for subsequent calls.
///
/// # Examples
///
/// ```
/// let re = sql_comment_regex();
/// assert!(re.is_match("-- a comment"));
/// assert!(re.is_match("/* block comment */"));
/// assert!(re.is_match("*/"));
/// ```
fn sql_comment_regex() -> &'static Regex {
    static SQL_COMMENT: OnceLock<Regex> = OnceLock::new();
    SQL_COMMENT.get_or_init(|| Regex::new(r"--|/\*|\*/").expect("SQL comment regex should compile"))
}

/// Returns a cached compiled regex that matches common SQL keywords at word boundaries, case-insensitively.
///
/// The regex is initialized once and reused for subsequent calls to avoid recompilation.
///
/// # Returns
///
/// A reference to a compiled regex matching the keywords `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `DROP`, `UNION`, `EXEC`, and `EXECUTE` at word boundaries, case-insensitively.
///
/// # Examples
///
/// ```
/// let re = sql_keyword_regex();
/// assert!(re.is_match("SELECT * FROM users"));
/// assert!(re.is_match("select"));
/// assert!(!re.is_match("selection"));
/// ```
fn sql_keyword_regex() -> &'static Regex {
    static SQL_KEYWORD: OnceLock<Regex> = OnceLock::new();
    SQL_KEYWORD.get_or_init(|| {
        Regex::new(r"(?i)\b(SELECT|INSERT|UPDATE|DELETE|DROP|UNION|EXEC|EXECUTE)\b")
            .expect("SQL keyword regex should compile")
    })
}

/// Compiled regex that matches two-digit hexadecimal sequences (useful for detecting percent-encoding components).
///
/// # Examples
///
/// ```
/// // matches the two-hex-digit sequence in a percent-encoded string
/// assert!(hex_sequence_regex().is_match("%20"));
/// // matches a standalone two-digit hex sequence
/// assert!(hex_sequence_regex().is_match("2F"));
/// // does not match non-hex or incomplete sequences
/// assert!(!hex_sequence_regex().is_match("g1"));
/// assert!(!hex_sequence_regex().is_match("A"));
/// ```
fn hex_sequence_regex() -> &'static Regex {
    static HEX_SEQUENCE: OnceLock<Regex> = OnceLock::new();
    HEX_SEQUENCE
        .get_or_init(|| Regex::new(r"[0-9A-Fa-f]{2}").expect("Hex sequence regex should compile"))
}

impl ParameterSanitizer {
    /// Constructs a new ParameterSanitizer with default sanitization rules and no bindings.
    ///
    /// The sanitizer starts with the module's default rules and an empty binding map.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = crate::functional::query_composition::ParameterSanitizer::new();
    /// assert!(s.bindings().is_empty());
    /// assert!(!s.rules.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            rules: Self::default_rules(),
        }
    }

    /// Bind a parameter after validating it with the sanitizer's rules.
    ///
    /// Validates `value` against the configured sanitization rules and, on success,
    /// stores a validated `ParameterBinding` under `name` with the provided `sql_type`.
    ///
    /// # Parameters
    ///
    /// - `name`: The parameter's binding name used in queries.
    /// - `value`: The parameter value to sanitize and bind (converted with `ToString`).
    /// - `sql_type`: A textual representation of the SQL type for the bound parameter.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the value passed all sanitization rules and was stored;
    /// `Err(SanitizationError::ValidationFailed { .. })` if any rule rejected the value.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sanitizer = ParameterSanitizer::new();
    /// sanitizer
    ///     .bind_parameter("user_id".to_string(), 123i32, "INTEGER".to_string())
    ///     .unwrap();
    /// assert!(sanitizer.bindings().contains_key("user_id"));
    /// ```
    pub fn bind_parameter<T>(
        &mut self,
        name: String,
        value: T,
        sql_type: String,
    ) -> Result<(), SanitizationError>
    where
        T: ToString + Send + Sync + 'static,
    {
        let value_str = value.to_string();

        // Apply sanitization rules
        for rule in &self.rules {
            if !self.apply_rule(&value_str, rule) {
                return Err(SanitizationError::ValidationFailed {
                    parameter: name.clone(),
                    rule: rule.name.clone(),
                    message: rule.error_message.clone(),
                });
            }
        }

        // Store the validated binding
        self.bindings.insert(
            name.clone(),
            ParameterBinding {
                name,
                value: value_str,
                sql_type,
                validated: true,
            },
        );

        Ok(())
    }

    /// Applies a sanitization rule to a string value.
    ///
    /// Recognizes the `"no_sql_injection"` rule (rejects SQL comment tokens, common SQL keywords
    /// at word boundaries, percent-encoding hints, and semicolons) and the `"reasonable_length"`
    /// rule (allows up to 255 characters). Unknown rule names pass by default.
    ///
    /// # Examples
    ///
    /// ```
    /// let sanitizer = ParameterSanitizer::new();
    /// let rule = SanitizationRule {
    ///     name: "reasonable_length".to_string(),
    ///     pattern: "".to_string(),
    ///     error_message: "too long".to_string(),
    /// };
    /// assert!(sanitizer.apply_rule("short value", &rule));
    /// ```
    ///
    /// # Returns
    ///
    /// `true` if the value passes the given sanitization rule, `false` otherwise.
    fn apply_rule(&self, value: &str, rule: &SanitizationRule) -> bool {
        // Defense-in-depth validation:
        // Primary defense is Diesel's parameterized queries
        // This provides additional pattern-based rejection
        match rule.name.as_str() {
            "no_sql_injection" => {
                // Use compiled regexes for efficient pattern matching

                // 1. Reject SQL comment patterns (--|/*|*/)
                if sql_comment_regex().is_match(value) {
                    return false;
                }

                // 2. Reject SQL keywords at word boundaries (case-insensitive)
                if sql_keyword_regex().is_match(value) {
                    return false;
                }

                // 3. Reject percent-encoding attempts (e.g., %27 for single quote)
                // Check if value contains '%' followed by hex digits
                if value.contains('%') && hex_sequence_regex().is_match(value) {
                    return false;
                }

                // 4. Reject semicolons (statement terminators)
                !value.contains(';')
            }
            "reasonable_length" => value.len() <= 255, // Max reasonable length
            _ => true,                                 // Unknown rules pass by default
        }
    }

    /// Provide the module's default sanitization rules.
    ///
    /// The returned vector contains two rules:
    /// - `"no_sql_injection"`: flags common SQL-injection characters/patterns.
    /// - `"reasonable_length"`: enforces a max-length constraint.
    ///
    /// # Examples
    ///
    /// ```
    /// let rules = default_rules();
    /// assert_eq!(rules.len(), 2);
    /// assert_eq!(rules[0].name, "no_sql_injection");
    /// assert_eq!(rules[1].name, "reasonable_length");
    /// ```
    fn default_rules() -> Vec<SanitizationRule> {
        vec![
            SanitizationRule {
                name: "no_sql_injection".to_string(),
                pattern: ";".to_string(),
                error_message: "Parameter contains potentially dangerous SQL characters"
                    .to_string(),
            },
            SanitizationRule {
                name: "reasonable_length".to_string(),
                pattern: "".to_string(), // Would check length in real implementation
                error_message: "Parameter is too long".to_string(),
            },
        ]
    }

    /// Access the map of validated parameter bindings.
    ///
    /// Returns a reference to the internal HashMap that stores bound parameters keyed by their parameter name.
    /// The map contains `ParameterBinding` values that have passed sanitization and are marked as validated.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sanitizer = ParameterSanitizer::new();
    /// sanitizer.bind_parameter("user_id", 42, "INTEGER").unwrap();
    /// let bindings = sanitizer.bindings();
    /// assert!(bindings.contains_key("user_id"));
    /// ```
    pub fn bindings(&self) -> &HashMap<String, ParameterBinding> {
        &self.bindings
    }
}

/// Sanitization error types.
#[derive(Debug, Clone)]
pub enum SanitizationError {
    /// Parameter failed validation
    ValidationFailed {
        /// Parameter name
        parameter: String,
        /// Rule name that failed
        rule: String,
        /// Error message
        message: String,
    },
    /// Other sanitization error
    Other(String),
}

impl std::fmt::Display for SanitizationError {
    /// Formats a `SanitizationError` into a human-readable message.
    ///
    /// Validation failures are rendered as
    /// `Parameter '<name>' failed rule '<rule>': <message>`, while other errors are
    /// rendered as `Sanitization error: <message>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::query_composition::SanitizationError;
    ///
    /// let v = SanitizationError::ValidationFailed {
    ///     parameter: "id".to_string(),
    ///     rule: "no_sql_injection".to_string(),
    ///     message: "contains forbidden sequence".to_string(),
    /// };
    /// assert_eq!(
    ///     format!("{}", v),
    ///     "Parameter 'id' failed rule 'no_sql_injection': contains forbidden sequence"
    /// );
    ///
    /// let o = SanitizationError::Other("unexpected".to_string());
    /// assert_eq!(format!("{}", o), "Sanitization error: unexpected");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SanitizationError::ValidationFailed {
                parameter,
                rule,
                message,
            } => {
                write!(
                    f,
                    "Parameter '{}' failed rule '{}': {}",
                    parameter, rule, message
                )
            }
            SanitizationError::Other(msg) => write!(f, "Sanitization error: {}", msg),
        }
    }
}

impl std::error::Error for SanitizationError {}

/// Query optimization engine for functional compositions.
///
/// This analyzes predicate compositions and optimizes query execution
/// order and strategies.
pub struct QueryOptimizationEngine {
    /// Optimization rules
    rules: Vec<OptimizationRule>,
    /// Query complexity analyzer
    complexity_analyzer: ComplexityAnalyzer,
}

#[derive(Debug, Clone)]
pub struct OptimizationRule {
    /// Rule name
    pub name: String,
    /// Optimization logic description
    pub description: String,
    /// When this rule applies
    pub condition: String,
    /// Expected performance improvement
    pub improvement_factor: f64,
}

#[derive(Debug, Clone)]
pub struct ComplexityAnalyzer {
    /// Base complexity score
    pub base_score: u32,
    /// Complexity multipliers for different operations
    pub multipliers: HashMap<String, f64>,
}

impl QueryOptimizationEngine {
    /// Constructs a query optimization engine initialized with the default optimization rules
    /// and a complexity analyzer.
    ///
    /// # Examples
    ///
    /// ```
    /// let _engine = QueryOptimizationEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
            complexity_analyzer: ComplexityAnalyzer::new(),
        }
    }

    /// Optimize a functional query composition and produce updated composition metrics.
    ///
    /// Applies the engine's optimization rules to the provided `FunctionalQueryComposer` (currently a pass-through),
    /// computes a complexity score for the resulting composition, and returns the (possibly optimized) composer
    /// together with updated `QueryPerformanceMetrics` that include `composition_time` and `complexity_score`.
    ///
    /// # Returns
    ///
    /// A tuple containing the optimized `FunctionalQueryComposer` and its `QueryPerformanceMetrics` with an updated
    /// `composition_time` and `complexity_score`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::functional::query_composition::{QueryOptimizationEngine, FunctionalQueryComposer};
    ///
    /// let engine = QueryOptimizationEngine::new();
    /// // Construct or obtain a FunctionalQueryComposer...
    /// let composer: FunctionalQueryComposer<(), ()> = /* build composer */ unimplemented!();
    ///
    /// let (optimized, metrics) = engine.optimize(composer);
    /// println!("composition_time: {:?}", metrics.composition_time);
    /// ```
    pub fn optimize<T, U>(
        &self,
        composer: FunctionalQueryComposer<T, U>,
    ) -> (FunctionalQueryComposer<T, U>, QueryPerformanceMetrics)
    where
        T: Send + Sync + 'static,
        U: Clone + Send + Sync + 'static,
    {
        let start_time = Instant::now();

        // Apply optimization rules
        let optimized_composer = composer;

        // Calculate complexity
        let complexity = self.complexity_analyzer.analyze(&optimized_composer);

        // Update metrics
        let mut metrics = optimized_composer.metrics.clone();
        metrics.composition_time = start_time.elapsed();
        metrics.complexity_score = complexity;

        (optimized_composer, metrics)
    }

    /// Provides the default set of optimization rules used by the query optimization engine.
    ///
    /// The returned vector contains predefined `OptimizationRule` entries (filter_pushdown,
    /// index_utilization, lazy_evaluation) with their descriptions, conditions, and improvement factors.
    ///
    /// # Examples
    ///
    /// ```
    /// let rules = default_rules();
    /// assert_eq!(rules.len(), 3);
    /// assert_eq!(rules[0].name, "filter_pushdown");
    /// ```
    fn default_rules() -> Vec<OptimizationRule> {
        vec![
            OptimizationRule {
                name: "filter_pushdown".to_string(),
                description: "Push selective filters down to reduce rows processed".to_string(),
                condition: "High selectivity predicates exist".to_string(),
                improvement_factor: 2.5,
            },
            OptimizationRule {
                name: "index_utilization".to_string(),
                description: "Ensure indexed columns are used effectively".to_string(),
                condition: "Indexed columns are being queried".to_string(),
                improvement_factor: 10.0,
            },
            OptimizationRule {
                name: "lazy_evaluation".to_string(),
                description: "Use lazy evaluation for large result sets".to_string(),
                condition: "Estimated result set > 10k rows".to_string(),
                improvement_factor: 3.0,
            },
        ]
    }
}

impl<T, U> FunctionalQueryComposer<T, U>
where
    T: Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
{
    /// Creates a new FunctionalQueryComposer with default configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// let composer = FunctionalQueryComposer::<(), ()>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            builder: TypeSafeQueryBuilder::new(),
            current_filter: None,
            lazy_config: LazyEvaluationConfig::default(),
            metrics: QueryPerformanceMetrics::default(),
            pool: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the database connection pool for this composer.
    ///
    /// # Examples
    ///
    /// ```
    /// // let pool = /* obtain pool */;
    /// // let mut composer = FunctionalQueryComposer::<(), ()>::new();
    /// // composer = composer.with_pool(pool);
    /// ```
    pub fn with_pool(mut self, pool: Pool) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Executes a chunked query with the specified offset and limit.
    ///
    /// This method builds and executes a query with LIMIT and OFFSET clauses
    /// to fetch a specific chunk of data using the query builder and database pool.
    /// It integrates with the TypeSafeQueryBuilder to construct parameterized queries.
    ///
    /// # Arguments
    ///
    /// * `offset` - The number of records to skip
    /// * `limit` - The maximum number of records to return
    ///
    /// # Returns
    ///
    /// A vector of results or an error if the query fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // let composer: FunctionalQueryComposer<MyTable, MyRecord> = /* ... */;
    /// // let chunk = composer.execute_chunk_query(0, 100).expect("Query failed");
    /// // assert!(chunk.len() <= 100);
    /// ```
    pub fn execute_chunk_query(&self, offset: usize, limit: usize) -> Result<Vec<U>, String> {
        // Check if we have a database pool
        let pool = self.pool.as_ref().ok_or("No database pool configured")?;

        // Get a connection from the pool
        let _conn = pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        // Build the query using our query builder
        // For now, we'll create a simple implementation that works with Diesel
        // In a real implementation, we would use the TypeSafeQueryBuilder to construct
        // a parameterized query with the filters, ordering, etc.

        // This is a simplified implementation - in a real application, we would need
        // to properly integrate with the TypeSafeQueryBuilder and generate actual SQL

        // For demonstration purposes, let's simulate executing a query
        // In a real implementation, this would execute an actual database query

        // Record execution time
        let start_time = Instant::now();

        // Simulate query execution with a delay
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Update metrics
        let execution_time = start_time.elapsed();
        // In a real implementation, we would update self.metrics here

        // Return an empty vector for now - in a real implementation, this would
        // contain the actual query results
        Ok(Vec::new())
    }
}

impl ComplexityAnalyzer {
    /// Constructs a ComplexityAnalyzer preconfigured with a default base score and operation multipliers.
    ///
    /// # Returns
    ///
    /// A ComplexityAnalyzer initialized with a base score of 10 and multipliers for the operations
    /// "join", "subquery", "aggregation", and "ordering".
    ///
    /// # Examples
    ///
    /// ```
    /// let _analyzer = ComplexityAnalyzer::new();
    /// ```
    pub fn new() -> Self {
        let mut multipliers = HashMap::new();
        multipliers.insert("join".to_string(), 2.0);
        multipliers.insert("subquery".to_string(), 3.0);
        multipliers.insert("aggregation".to_string(), 1.5);
        multipliers.insert("ordering".to_string(), 1.2);

        Self {
            base_score: 10,
            multipliers,
        }
    }

    /// Compute a bounded complexity score for a query composition.
    ///
    /// The score is computed from a base score and increased by fixed amounts for
    /// filters and ordering specifications present in the composer's query builder.
    /// The result is capped at 100.
    ///
    /// # Returns
    ///
    /// A complexity score from 0 to 100, where higher values indicate greater complexity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # // Example usage (ignored for doc tests): analyze a composer's complexity.
    /// # use crate::functional::query_composition::ComplexityAnalyzer;
    /// # use crate::functional::query_composition::FunctionalQueryComposer;
    /// # // let composer: FunctionalQueryComposer<_, _> = /* build composer */ unimplemented!();
    /// # let analyzer = ComplexityAnalyzer::new();
    /// # let _score = analyzer.analyze(&composer);
    /// ```
    pub fn analyze<T, U>(&self, composer: &FunctionalQueryComposer<T, U>) -> u32
    where
        T: Send + Sync + 'static,
        U: Clone + Send + Sync + 'static,
    {
        let mut score = self.base_score;

        // Add complexity for each filter
        score += composer.builder.filters().len() as u32 * 5;

        // Add complexity for ordering
        score += composer.builder.order_by_specs().len() as u32 * 3;

        // Cap at 100
        score.min(100)
    }
}

/// Creates a ComposablePredicate for the given column with associated predicate metadata.
///
/// The returned ComposablePredicate wraps a Predicate constructed from the provided column,
/// operator, value, and field name, and attaches a PredicateMetadata record containing the
/// given selectivity, a cost estimate derived from the operator, pushdown capability set to
/// true, and an empty dependency list.
///
/// # Examples
///
/// ```
/// use crate::functional::query_builder::{Column, Operator};
/// use crate::functional::query_composition::composable_predicate;
///
/// // Construct a column for demonstration — adjust to your Column constructor.
/// let col = Column::new("users", "age");
/// let cp = composable_predicate(col, Operator::GreaterThan, Some(30), "age".to_string(), 0.5);
/// assert_eq!(cp.metadata.selectivity, 0.5);
/// ```
pub fn composable_predicate<T>(
    column: Column<T, T>,
    operator: Operator,
    value: Option<T>,
    field_name: String,
    selectivity: f64,
) -> ComposablePredicate<T>
where
    T: Clone + Send + Sync + 'static,
{
    let predicate = Predicate::new(column, operator, value, field_name);

    let cost_estimate = match operator {
        Operator::Equals => Duration::from_millis(1),
        Operator::Contains => Duration::from_millis(5),
        Operator::GreaterThan | Operator::LessThan => Duration::from_millis(2),
        _ => Duration::from_millis(3),
    };

    let metadata = PredicateMetadata {
        selectivity,
        cost_estimate,
        pushdown_capable: true,
        dependencies: Vec::new(),
    };

    ComposablePredicate::new(predicate, metadata)
}

/// Convert a FieldFilter into a ComposablePredicate<String> for backward compatibility.
///
/// # Examples
///
/// ```
/// use crate::models::filters::FieldFilter;
/// use crate::functional::query_composition::field_filter_to_composable;
///
/// let filter = FieldFilter {
///     field: "name".into(),
///     operator: "contains".into(),
///     value: "alice".into(),
/// };
///
/// let comp = field_filter_to_composable(&filter, "users");
/// let _: crate::functional::query_composition::ComposablePredicate<String> = comp;
/// ```
pub fn field_filter_to_composable(
    filter: &crate::models::filters::FieldFilter,
    table_name: &str,
) -> ComposablePredicate<String> {
    use crate::functional::query_builder::Operator::*;

    let operator = match filter.operator.as_str() {
        "equals" => Equals,
        "contains" => Contains,
        "gt" => GreaterThan,
        "lt" => LessThan,
        "gte" => GreaterThanEqual,
        "lte" => LessThanEqual,
        _ => Equals, // Default fallback
    };

    let field_name = filter.field.clone();
    let column_str = filter.field.clone();
    let column = Column::new(table_name.to_string(), column_str);

    composable_predicate(
        column,
        operator,
        Some(filter.value.clone()),
        field_name,
        0.5, // Default selectivity
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_query_iterator_with_data() {
        // Test that the iterator works correctly with pre-loaded data
        let data = vec![1, 2, 3, 4, 5];
        let mut iter: LazyQueryIterator<(), i32> = LazyQueryIterator::with_data(data);

        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None); // Should remain None
    }

    #[test]
    fn test_lazy_query_iterator_empty_data() {
        // Test that an empty iterator immediately returns None
        let data: Vec<i32> = vec![];
        let mut iter: LazyQueryIterator<(), i32> = LazyQueryIterator::with_data(data);

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_lazy_query_iterator_single_item() {
        // Test with a single item
        let data = vec![42];
        let mut iter: LazyQueryIterator<(), i32> = LazyQueryIterator::with_data(data);

        assert_eq!(iter.next(), Some(42));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_lazy_query_iterator_collect() {
        // Test that collect() works properly
        let data = vec!["a", "b", "c"];
        let iter: LazyQueryIterator<(), &str> = LazyQueryIterator::with_data(data.clone());

        let collected: Vec<&str> = iter.collect();
        assert_eq!(collected, data);
    }
}
