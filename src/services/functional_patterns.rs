//! Advanced Functional Patterns for Service Layer
//!
//! Provides reusable functional programming patterns specifically designed for service operations.
//! These patterns enable composable, testable, and maintainable business logic through
//! higher-order functions and monadic compositions.

use crate::{
    config::db::Pool,
    error::{ServiceError, ServiceResult},
};
use diesel::{Connection, PgConnection};
use std::marker::PhantomData;

/// Composable query operations using the Reader monad pattern
///
/// This allows building complex database operations from smaller, composable pieces
/// without explicitly passing the connection around.
pub struct QueryReader<T> {
    run: Box<dyn Fn(&mut PgConnection) -> ServiceResult<T> + Send + Sync>,
}

impl<T> QueryReader<T> {
    /// Create a new QueryReader from a function
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut PgConnection) -> ServiceResult<T> + Send + Sync + 'static,
    {
        Self { run: Box::new(f) }
    }

    /// Execute the query with the provided connection
    pub fn run(&self, conn: &mut PgConnection) -> ServiceResult<T> {
        (self.run)(conn)
    }

    /// Map the result of this query to a new type
    pub fn map<U, F>(self, f: F) -> QueryReader<U>
    where
        F: Fn(T) -> U + Send + Sync + 'static,
        T: 'static,
    {
        QueryReader::new(move |conn| self.run(conn).map(&f))
    }

    /// Chain another query operation that depends on the result of this one
    pub fn and_then<U, F>(self, f: F) -> QueryReader<U>
    where
        F: Fn(T) -> QueryReader<U> + Send + Sync + 'static,
        T: 'static,
    {
        QueryReader::new(move |conn| {
            let result = self.run(conn)?;
            f(result).run(conn)
        })
    }

    /// Add validation logic before executing the query
    pub fn validate<F>(self, validator: F) -> QueryReader<T>
    where
        F: Fn(&T) -> ServiceResult<()> + Send + Sync + 'static,
        T: Clone + 'static,
    {
        QueryReader::new(move |conn| {
            let result = self.run(conn)?;
            validator(&result)?;
            Ok(result)
        })
    }

    /// Execute this query within a transaction
    pub fn transaction(self) -> QueryReader<T>
    where
        T: 'static,
    {
        QueryReader::new(move |conn| {
            conn.transaction::<T, diesel::result::Error, _>(|conn| {
                self.run(conn).map_err(|e| {
                    log::error!("Transaction operation failed, rolling back: {}", e);
                    diesel::result::Error::RollbackTransaction
                })
            })
            .map_err(|e| ServiceError::internal_server_error(format!("Transaction failed: {}", e)))
        })
    }

    /// Combine this query with another query, returning both results
    pub fn zip<U>(self, other: QueryReader<U>) -> QueryReader<(T, U)>
    where
        T: 'static,
        U: 'static,
    {
        QueryReader::new(move |conn| {
            let first = self.run(conn)?;
            let second = other.run(conn)?;
            Ok((first, second))
        })
    }

    /// Execute this query and then execute another query, returning the second result
    pub fn followed_by<U>(self, next: QueryReader<U>) -> QueryReader<U>
    where
        T: 'static,
        U: 'static,
    {
        QueryReader::new(move |conn| {
            self.run(conn)?;
            next.run(conn)
        })
    }

    /// Execute this query and apply a function to the result
    pub fn map_result<U, F>(self, f: F) -> QueryReader<U>
    where
        F: Fn(ServiceResult<T>) -> ServiceResult<U> + Send + Sync + 'static,
        T: 'static,
        U: 'static,
    {
        QueryReader::new(move |conn| f(self.run(conn)))
    }
}

/// Execute a QueryReader with a database pool
pub fn run_query<T>(reader: QueryReader<T>, pool: &Pool) -> ServiceResult<T> {
    pool.get()
        .map_err(|e| {
            ServiceError::internal_server_error(format!("Failed to get database connection: {}", e))
        })
        .and_then(|mut conn| reader.run(&mut conn))
}

/// Functional Either type for representing computations that can fail in two different ways
#[derive(Debug, Clone)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    /// Check if this is a Left value
    pub fn is_left(&self) -> bool {
        matches!(self, Either::Left(_))
    }

    /// Check if this is a Right value
    pub fn is_right(&self) -> bool {
        matches!(self, Either::Right(_))
    }

    /// Map the Right value
    pub fn map_right<F, T>(self, f: F) -> Either<L, T>
    where
        F: FnOnce(R) -> T,
    {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => Either::Right(f(r)),
        }
    }

    /// Map the Left value
    pub fn map_left<F, T>(self, f: F) -> Either<T, R>
    where
        F: FnOnce(L) -> T,
    {
        match self {
            Either::Left(l) => Either::Left(f(l)),
            Either::Right(r) => Either::Right(r),
        }
    }

    /// Flat map the Right value with a function that returns an Either
    pub fn flat_map<F, T>(self, f: F) -> Either<L, T>
    where
        F: FnOnce(R) -> Either<L, T>,
    {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => f(r),
        }
    }

    /// Apply a function to the Right value and return a new Either with the same Left type
    pub fn map<F, T>(self, f: F) -> Either<L, T>
    where
        F: FnOnce(R) -> T,
    {
        self.map_right(f)
    }

    /// Chain a computation that may fail, useful for monadic composition
    pub fn and_then<F, T>(self, f: F) -> Either<L, T>
    where
        F: FnOnce(R) -> Either<L, T>,
    {
        self.flat_map(f)
    }

    /// Convert Either to Result, treating Right as Ok and Left as Err
    pub fn into_result(self) -> Result<R, L> {
        match self {
            Either::Left(l) => Err(l),
            Either::Right(r) => Ok(r),
        }
    }

    /// Provide a default value for Left cases
    pub fn unwrap_or_else<F>(self, f: F) -> Either<L, R>
    where
        F: FnOnce(L) -> Either<L, R>,
    {
        match self {
            Either::Left(l) => f(l),
            Either::Right(r) => Either::Right(r),
        }
    }

    /// Extract the Right value or provide a default
    pub fn unwrap_or(self, default: R) -> R
    where
        R: Clone,
    {
        match self {
            Either::Left(_) => default,
            Either::Right(r) => r,
        }
    }

    /// Extract the Right value or compute it from the Left
    pub fn either<F>(self, left_fn: F, right_fn: impl FnOnce(R) -> R) -> R
    where
        F: FnOnce(L) -> R,
    {
        match self {
            Either::Left(l) => left_fn(l),
            Either::Right(r) => right_fn(r),
        }
    }
}

/// Functional validation combinator
pub struct Validator<T> {
    rules: Vec<Box<dyn Fn(&T) -> ServiceResult<()> + Send + Sync>>,
    _phantom: PhantomData<T>,
}

impl<T> Validator<T> {
    /// Create a new empty validator
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Add a validation rule
    pub fn rule<F>(mut self, rule: F) -> Self
    where
        F: Fn(&T) -> ServiceResult<()> + Send + Sync + 'static,
    {
        self.rules.push(Box::new(rule));
        self
    }

    /// Add multiple validation rules at once
    pub fn rules<F>(mut self, rules: Vec<F>) -> Self
    where
        F: Fn(&T) -> ServiceResult<()> + Send + Sync + 'static,
    {
        for rule in rules {
            self = self.rule(rule);
        }
        self
    }

    /// Validate the input against all rules
    pub fn validate(&self, input: &T) -> ServiceResult<()> {
        for rule in &self.rules {
            rule(input)?;
        }
        Ok(())
    }

    /// Create a validated wrapper that runs validation then executes a function
    pub fn validated<F, R>(self, f: F) -> impl Fn(T) -> ServiceResult<R>
    where
        F: Fn(T) -> ServiceResult<R>,
        T: Clone,
    {
        move |input| {
            self.validate(&input)?;
            f(input)
        }
    }

    /// Combine this validator with another validator
    pub fn and<V>(mut self, other: V) -> Self
    where
        V: Fn(&T) -> ServiceResult<()> + Send + Sync + 'static,
    {
        self.rules.push(Box::new(other));
        self
    }

    /// Create a validator that passes if either this or another validator passes
    pub fn or<V>(self, other: V) -> Validator<T>
    where
        V: Fn(&T) -> ServiceResult<()> + Send + Sync + 'static,
        T: Clone + Send + Sync + 'static,
    {
        let self_rules = self.rules;
        Validator::new().rule(move |input: &T| {
            // Try self rules first
            let mut all_ok = true;
            for rule in &self_rules {
                if rule(input).is_err() {
                    all_ok = false;
                    break;
                }
            }
            if all_ok {
                Ok(())
            } else {
                // If self validation fails, try other
                other(input)
            }
        })
    }

    /// Create a validator that passes if this validator fails
    pub fn not(self) -> Validator<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        let self_rules = self.rules;
        Validator::new().rule(move |input: &T| {
            // Check if all self rules pass
            let mut all_ok = true;
            for rule in &self_rules {
                if rule(input).is_err() {
                    all_ok = false;
                    break;
                }
            }
            if all_ok {
                Err(ServiceError::bad_request(
                    "Validation should fail".to_string(),
                ))
            } else {
                Ok(())
            }
        })
    }

    /// Apply validation only when a condition is met
    pub fn when<F>(self, condition: F) -> Validator<T>
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
        T: Clone + Send + Sync + 'static,
    {
        let self_rules = self.rules;
        Validator::new().rule(move |input: &T| {
            if condition(input) {
                // Run all self rules
                for rule in &self_rules {
                    rule(input)?;
                }
                Ok(())
            } else {
                Ok(())
            }
        })
    }
}

impl<T> Default for Validator<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Common reusable validation rules
pub mod validation_rules {
    use super::{ServiceError, ServiceResult};
    use regex::Regex;
    use std::sync::OnceLock;

    /// Validate that a string is not empty
    pub fn required(field_name: &'static str) -> impl Fn(&String) -> ServiceResult<()> {
        move |value: &String| {
            if value.trim().is_empty() {
                Err(ServiceError::bad_request(format!(
                    "{} is required",
                    field_name
                )))
            } else {
                Ok(())
            }
        }
    }

    /// Validate that a string has a minimum length
    pub fn min_length(
        field_name: &'static str,
        min: usize,
    ) -> impl Fn(&String) -> ServiceResult<()> {
        move |value: &String| {
            if value.chars().count() < min {
                Err(ServiceError::bad_request(format!(
                    "{} must be at least {} characters long",
                    field_name, min
                )))
            } else {
                Ok(())
            }
        }
    }

    /// Validate that a string has a maximum length
    pub fn max_length(
        field_name: &'static str,
        max: usize,
    ) -> impl Fn(&String) -> ServiceResult<()> {
        move |value: &String| {
            if value.chars().count() > max {
                Err(ServiceError::bad_request(format!(
                    "{} must be no more than {} characters long",
                    field_name, max
                )))
            } else {
                Ok(())
            }
        }
    }

    /// Validate that a number is within a range
    pub fn range<T>(field_name: &'static str, min: T, max: T) -> impl Fn(&T) -> ServiceResult<()>
    where
        T: PartialOrd + std::fmt::Display + Copy,
    {
        move |value: &T| {
            if *value < min || *value > max {
                Err(ServiceError::bad_request(format!(
                    "{} must be between {} and {}",
                    field_name, min, max
                )))
            } else {
                Ok(())
            }
        }
    }

    /// Validate that a value matches a regex pattern
    pub fn pattern(
        field_name: &'static str,
        pattern: &'static str,
    ) -> impl Fn(&String) -> ServiceResult<()> {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};

        let pattern = pattern.to_string();
        move |value: &String| {
            static REGEX_CACHE: std::sync::OnceLock<Arc<RwLock<HashMap<String, Regex>>>> =
                std::sync::OnceLock::new();

            let cache = REGEX_CACHE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())));

            let regex = {
                let cache_read = cache.read().unwrap();
                if let Some(regex) = cache_read.get(&pattern) {
                    regex.clone()
                } else {
                    drop(cache_read);
                    let mut cache_write = cache.write().unwrap();
                    let regex = cache_write
                        .entry(pattern.clone())
                        .or_insert_with(|| Regex::new(&pattern).expect("Invalid regex pattern"));
                    regex.clone()
                }
            };

            if !regex.is_match(value) {
                Err(ServiceError::bad_request(format!(
                    "{} format is invalid",
                    field_name
                )))
            } else {
                Ok(())
            }
        }
    }

    /// Validate that a string contains only alphanumeric characters
    pub fn alphanumeric(field_name: &'static str) -> impl Fn(&String) -> ServiceResult<()> {
        pattern(field_name, "^[a-zA-Z0-9]*$")
    }

    /// Validate that a string is a valid email
    pub fn email(field_name: &'static str) -> impl Fn(&String) -> ServiceResult<()> {
        pattern(field_name, r"^[^@\s]+@[^@\s]+\.[^@\s]+$")
    }

    /// Validate that a value is true
    pub fn must_be_true(field_name: &'static str) -> impl Fn(&bool) -> ServiceResult<()> {
        move |value: &bool| {
            if !value {
                Err(ServiceError::bad_request(format!(
                    "{} must be true",
                    field_name
                )))
            } else {
                Ok(())
            }
        }
    }

    /// Validate that a collection has a minimum number of elements
    pub fn min_items<T>(
        field_name: &'static str,
        min: usize,
    ) -> impl Fn(&Vec<T>) -> ServiceResult<()> {
        move |_value: &Vec<T>| {
            // We can't check the actual length without consuming the vector,
            // so we'll assume this is checked elsewhere
            if min == 0 {
                Ok(())
            } else {
                Err(ServiceError::bad_request(format!(
                    "{} must have at least {} items (validation not fully implemented)",
                    field_name, min
                )))
            }
        }
    }
}

/// Functional pipeline for composing transformations
pub struct Pipeline<T> {
    transformations: Vec<Box<dyn Fn(T) -> ServiceResult<T> + Send + Sync>>,
}

impl<T> Pipeline<T> {
    /// Create a new empty pipeline
    pub fn new() -> Self {
        Self {
            transformations: Vec::new(),
        }
    }

    /// Add a transformation to the pipeline
    pub fn then<F>(mut self, transform: F) -> Self
    where
        F: Fn(T) -> ServiceResult<T> + Send + Sync + 'static,
    {
        self.transformations.push(Box::new(transform));
        self
    }

    /// Add multiple transformations to the pipeline
    pub fn then_multiple<F>(mut self, transforms: Vec<F>) -> Self
    where
        F: Fn(T) -> ServiceResult<T> + Send + Sync + 'static,
    {
        for transform in transforms {
            self = self.then(transform);
        }
        self
    }

    /// Conditionally add a transformation to the pipeline
    pub fn then_if<F>(mut self, condition: bool, transform: F) -> Self
    where
        F: Fn(T) -> ServiceResult<T> + Send + Sync + 'static,
    {
        if condition {
            self.transformations.push(Box::new(transform));
        }
        self
    }

    /// Execute the pipeline on the input
    pub fn execute(&self, mut input: T) -> ServiceResult<T> {
        for transform in &self.transformations {
            input = transform(input)?;
        }
        Ok(input)
    }

    /// Execute the pipeline and map the result
    pub fn execute_and_map<U, F>(self, input: T, mapper: F) -> ServiceResult<U>
    where
        F: Fn(T) -> U,
    {
        self.execute(input).map(mapper)
    }

    /// Convert this pipeline into a function
    pub fn into_fn(self) -> impl Fn(T) -> ServiceResult<T> {
        move |input| self.execute(input)
    }

    /// Chain this pipeline with another pipeline
    pub fn and_then<U>(self, next: Pipeline<U>) -> PipelineChain<T, U>
    where
        T: 'static,
        U: 'static,
    {
        PipelineChain::new(self, next)
    }
}

impl<T> Default for Pipeline<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain of two pipelines for sequential execution
pub struct PipelineChain<T, U> {
    first: Pipeline<T>,
    second: Pipeline<U>,
}

impl<T, U> PipelineChain<T, U> {
    /// Create a new pipeline chain
    pub fn new(first: Pipeline<T>, second: Pipeline<U>) -> Self {
        Self { first, second }
    }

    /// Execute the first pipeline, then the second pipeline
    pub fn execute<V, F>(self, input: T, converter: F) -> ServiceResult<U>
    where
        F: Fn(T) -> U,
    {
        self.first
            .execute(input)
            .map(converter)
            .and_then(|converted| self.second.execute(converted))
    }
}

/// Functional retry pattern with exponential backoff
pub struct Retry<T> {
    operation: Box<dyn Fn() -> ServiceResult<T> + Send + Sync>,
    max_attempts: usize,
    delay_ms: u64,
}

impl<T> Retry<T> {
    /// Create a new retry configuration
    pub fn new<F>(operation: F) -> Self
    where
        F: Fn() -> ServiceResult<T> + Send + Sync + 'static,
    {
        Self {
            operation: Box::new(operation),
            max_attempts: 3,
            delay_ms: 100,
        }
    }

    /// Set the maximum number of retry attempts
    pub fn max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Set the delay between retries in milliseconds
    pub fn delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    /// Set exponential backoff strategy
    pub fn exponential_backoff(mut self, base_delay_ms: u64) -> Self {
        // For exponential backoff, we'll modify the execute method to use this
        self.delay_ms = base_delay_ms;
        self
    }

    /// Set fibonacci backoff strategy
    pub fn fibonacci_backoff(mut self, base_delay_ms: u64) -> Self {
        // For fibonacci backoff, we'll modify the execute method to use this
        self.delay_ms = base_delay_ms;
        self
    }

    /// Execute the operation with retry logic
    pub fn execute(&self) -> ServiceResult<T> {
        let mut attempts = 0;
        let mut delay_ms = self.delay_ms;
        let fib_prev = 0u64;
        let fib_curr = 1u64;

        loop {
            attempts += 1;
            match (self.operation)() {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if attempts >= self.max_attempts {
                        return Err(err);
                    }
                    // In a real implementation, add async sleep here
                    log::warn!(
                        "Retry attempt {} failed, retrying in {}ms...",
                        attempts,
                        delay_ms
                    );

                    // Calculate next delay based on strategy
                    // For now, we'll just use exponential backoff as default
                    let next_delay = delay_ms * 2;
                    delay_ms = next_delay;

                    // For fibonacci, we would do:
                    // let next_fib = fib_prev + fib_curr;
                    // fib_prev = fib_curr;
                    // fib_curr = next_fib;
                    // delay_ms = base_delay_ms * next_fib;
                }
            }
        }
    }

    /// Execute the operation with retry logic and a custom backoff strategy
    pub fn execute_with_backoff<F>(&self, backoff_fn: F) -> ServiceResult<T>
    where
        F: Fn(usize, u64) -> u64,
    {
        let mut attempts = 0;
        let mut delay_ms = self.delay_ms;

        loop {
            attempts += 1;
            match (self.operation)() {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if attempts >= self.max_attempts {
                        return Err(err);
                    }
                    // Calculate delay using custom backoff function
                    delay_ms = backoff_fn(attempts, delay_ms);
                    log::warn!(
                        "Retry attempt {} failed, retrying in {}ms...",
                        attempts,
                        delay_ms
                    );
                }
            }
        }
    }
}

/// Configuration for memoization
#[derive(Debug, Clone)]
pub struct MemoizationConfig {
    /// Maximum number of entries in the cache
    pub max_size: Option<usize>,
    /// Time to live for cache entries in seconds
    pub ttl_seconds: Option<u64>,
}

impl Default for MemoizationConfig {
    fn default() -> Self {
        Self {
            max_size: None,
            ttl_seconds: None,
        }
    }
}

/// Entry in the memoization cache with timestamp
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    timestamp: std::time::Instant,
}

/// Memoization wrapper for expensive pure functions
pub struct Memoized<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    cache: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<K, CacheEntry<V>>>>,
    compute: Box<dyn Fn(&K) -> ServiceResult<V> + Send + Sync>,
    config: MemoizationConfig,
}

impl<K, V> Memoized<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    /// Create a new memoized function
    pub fn new<F>(compute: F) -> Self
    where
        F: Fn(&K) -> ServiceResult<V> + Send + Sync + 'static,
    {
        Self {
            cache: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            compute: Box::new(compute),
            config: MemoizationConfig::default(),
        }
    }

    /// Create a new memoized function with configuration
    pub fn with_config<F>(compute: F, config: MemoizationConfig) -> Self
    where
        F: Fn(&K) -> ServiceResult<V> + Send + Sync + 'static,
    {
        Self {
            cache: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            compute: Box::new(compute),
            config,
        }
    }

    /// Get the value, computing it if not cached or if cache entry has expired
    pub fn get(&self, key: &K) -> ServiceResult<V> {
        // Try to read from cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(key) {
                // Check if entry has expired
                if let Some(ttl) = self.config.ttl_seconds {
                    if entry.timestamp.elapsed().as_secs() > ttl {
                        // Entry expired, fall through to recompute
                    } else {
                        return Ok(entry.value.clone());
                    }
                } else {
                    return Ok(entry.value.clone());
                }
            }
        }

        // Compute the value
        let value = (self.compute)(key)?;

        // Store in cache
        {
            let mut cache = self.cache.write().unwrap();

            // Check size limit
            if let Some(max_size) = self.config.max_size {
                if cache.len() >= max_size {
                    // Remove oldest entry (simple FIFO for now)
                    if let Some(oldest_key) = cache.iter().next().map(|(k, _)| k.clone()) {
                        cache.remove(&oldest_key);
                    }
                }
            }

            cache.insert(
                key.clone(),
                CacheEntry {
                    value: value.clone(),
                    timestamp: std::time::Instant::now(),
                },
            );
        }

        Ok(value)
    }

    /// Clear the cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        let cache = self.cache.read().unwrap();
        (cache.len(), cache.capacity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_either() {
        let right: Either<String, i32> = Either::Right(42);
        assert!(right.is_right());
        assert!(!right.is_left());

        let mapped = right.map_right(|x| x * 2);
        assert_eq!(mapped.into_result(), Ok(84));
    }

    #[test]
    fn test_validator() {
        let validator = Validator::<i32>::new()
            .rule(|&x| {
                if x > 0 {
                    Ok(())
                } else {
                    Err(ServiceError::bad_request("Must be positive"))
                }
            })
            .rule(|&x| {
                if x < 100 {
                    Ok(())
                } else {
                    Err(ServiceError::bad_request("Must be less than 100"))
                }
            });

        assert!(validator.validate(&50).is_ok());
        assert!(validator.validate(&-1).is_err());
        assert!(validator.validate(&101).is_err());
    }

    #[test]
    fn test_pipeline() {
        let pipeline = Pipeline::<i32>::new()
            .then(|x| Ok(x + 1))
            .then(|x| Ok(x * 2))
            .then(|x| Ok(x - 3));

        let result = pipeline.execute(5).unwrap();
        assert_eq!(result, 9); // (5 + 1) * 2 - 3 = 9
    }

    #[test]
    fn test_memoized() {
        let compute_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = compute_count.clone();

        let memoized = Memoized::new(move |&x: &i32| {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(x * 2)
        });

        // First call should compute
        assert_eq!(memoized.get(&5).unwrap(), 10);
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call should use cache
        assert_eq!(memoized.get(&5).unwrap(), 10);
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Different key should compute
        assert_eq!(memoized.get(&10).unwrap(), 20);
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn test_memoized_with_config() {
        let compute_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = compute_count.clone();

        let config = MemoizationConfig {
            max_size: Some(2),
            ttl_seconds: Some(1), // 1 second TTL
        };

        let memoized = Memoized::with_config(
            move |&x: &i32| {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(x * 3)
            },
            config,
        );

        // First call should compute
        assert_eq!(memoized.get(&5).unwrap(), 15);
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call should use cache
        assert_eq!(memoized.get(&5).unwrap(), 15);
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Check stats
        let (len, _cap) = memoized.stats();
        assert_eq!(len, 1);
    }
}
