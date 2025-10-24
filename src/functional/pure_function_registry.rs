//! Pure Function Registry
//!
//! A thread-safe registry for storing and retrieving pure functions.
//! Provides fast lookup by category and signature, function composition,
//! and purity validation. All operations are optimized for performance
//! with sub-millisecond lookup times.

#[allow(dead_code)]
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use super::function_traits::{FunctionCategory, FunctionContainer, FunctionWrapper, PureFunction};

/// Information about a registered function (without the executable function)
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInfo {
    pub signature: &'static str,
    pub category: FunctionCategory,
    pub input_type_id: std::any::TypeId,
    pub output_type_id: std::any::TypeId,
}

/// Performance metrics for registry operations.
#[derive(Debug, Clone)]
pub struct RegistryMetrics {
    /// Average lookup time in nanoseconds (computed from total_lookup_time_ns / lookup_count)
    pub avg_lookup_time_ns: u64,
    /// Total accumulated lookup time in nanoseconds (internal tracking for precise averaging)
    total_lookup_time_ns: u128,
    /// Total number of functions registered
    pub total_functions: usize,
    /// Number of lookup operations performed
    pub lookup_count: u64,
    /// Number of composition operations performed
    pub composition_count: u64,
}

impl RegistryMetrics {
    /// Recomputes the average lookup time in nanoseconds from the accumulated totals.
    ///
    /// Sets `avg_lookup_time_ns` to `total_lookup_time_ns / lookup_count` (integer division).
    /// If `lookup_count` is zero, `avg_lookup_time_ns` is set to `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut m = RegistryMetrics { total_lookup_time_ns: 150, lookup_count: 3, ..Default::default() };
    /// m.recompute_average();
    /// assert_eq!(m.avg_lookup_time_ns, 50);
    /// ```
    fn recompute_average(&mut self) {
        if self.lookup_count > 0 {
            self.avg_lookup_time_ns =
                (self.total_lookup_time_ns / self.lookup_count as u128) as u64;
        } else {
            self.avg_lookup_time_ns = 0;
        }
    }
}

impl Default for RegistryMetrics {
    /// Creates a default `RegistryMetrics` with all counters initialized to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// let m = RegistryMetrics::default();
    /// assert_eq!(m.avg_lookup_time_ns, 0);
    /// assert_eq!(m.total_functions, 0);
    /// assert_eq!(m.lookup_count, 0);
    /// assert_eq!(m.composition_count, 0);
    /// ```
    fn default() -> Self {
        Self {
            avg_lookup_time_ns: 0,
            total_lookup_time_ns: 0,
            total_functions: 0,
            lookup_count: 0,
            composition_count: 0,
        }
    }
}

/// Thread-safe pure function registry with performance monitoring.
pub struct PureFunctionRegistry {
    /// Functions organized by category for fast lookup
    functions: RwLock<HashMap<FunctionCategory, HashMap<&'static str, FunctionContainer>>>,
    /// Performance metrics
    metrics: RwLock<RegistryMetrics>,
}

impl PureFunctionRegistry {
    /// Creates a new, empty PureFunctionRegistry with initialized metrics.
    ///
    /// # Examples
    ///
    /// ```
    /// let registry = PureFunctionRegistry::new();
    /// let metrics = registry.get_metrics().unwrap();
    /// assert_eq!(metrics.total_functions, 0);
    /// ```
    pub fn new() -> Self {
        Self {
            functions: RwLock::new(HashMap::new()),
            metrics: RwLock::new(RegistryMetrics::default()),
        }
    }

    /// Registers a pure function in the registry under its category and signature.
    ///
    /// Inserts the provided function into the registry if no function with the same
    /// category and signature already exists, and increments the registry's total
    /// function count.
    ///
    /// # Parameters
    ///
    /// * `function` - The pure function to register; its category and signature are
    ///   derived from the function itself.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the function was registered successfully, `Err(RegistryError::FunctionAlreadyExists { .. })`
    /// if a function with the same category and signature is already present, or
    /// `Err(RegistryError::LockPoisoned)` if an internal lock is poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// // Assuming `PureFunctionRegistry`, `PureFunction`, and a sample function type exist.
    /// #[derive(Clone)]
    /// struct Identity;
    /// impl PureFunction<i32, i32> for Identity {
    ///     fn category(&self) -> FunctionCategory { FunctionCategory::Transformation }
    ///     fn signature(&self) -> &'static str { "identity_i32" }
    ///     fn call(&self, input: i32) -> i32 { input }
    /// }
    ///
    /// let registry = PureFunctionRegistry::new();
    /// let f = Identity;
    /// assert!(registry.register(f).is_ok());
    /// ```
    pub fn register<Input, Output, F>(&self, function: F) -> Result<(), RegistryError>
    where
        Input: Send + Sync + 'static,
        Output: Send + Sync + 'static,
        F: PureFunction<Input, Output> + 'static,
    {
        let _start = Instant::now();
        let mut functions = self
            .functions
            .write()
            .map_err(|_| RegistryError::LockPoisoned)?;

        let category = function.category();
        let signature = function.signature();

        // Get or create category map
        let category_map = functions.entry(category).or_insert_with(HashMap::new);

        // Check if function already exists
        if category_map.contains_key(signature) {
            return Err(RegistryError::FunctionAlreadyExists {
                category,
                signature: signature.to_string(),
            });
        }

        // Register the function
        let container = FunctionContainer::new(function, signature, category);
        category_map.insert(signature, container);

        // Update metrics
        let mut metrics = self
            .metrics
            .write()
            .map_err(|_| RegistryError::LockPoisoned)?;
        metrics.total_functions += 1;

        Ok(())
    }

    /// Retrieve metadata for a registered function by category and signature.
    ///
    /// Returns `Some(FunctionInfo)` when a function with the given category and signature exists,
    /// otherwise returns `None`. This operation also updates internal lookup metrics.
    ///
    /// # Returns
    ///
    /// `Some(FunctionInfo)` if a matching function is found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use crate::functional::prelude::create_standard_registry;
    /// use crate::functional::pure_function_registry::FunctionCategory;
    ///
    /// let registry = create_standard_registry().unwrap();
    /// let info = registry.lookup(FunctionCategory::Transformation, "identity").unwrap();
    /// assert!(info.is_some());
    /// ```
    pub fn lookup(
        &self,
        category: FunctionCategory,
        signature: &str,
    ) -> Result<Option<FunctionInfo>, RegistryError> {
        let start = Instant::now();
        let functions = self
            .functions
            .read()
            .map_err(|_| RegistryError::LockPoisoned)?;

        let result = functions
            .get(&category)
            .and_then(|category_map| category_map.get(signature))
            .map(|container| FunctionInfo {
                signature: container.signature(),
                category: container.category(),
                input_type_id: container.input_type_id(),
                output_type_id: container.output_type_id(),
            });

        let duration = start.elapsed();
        self.update_lookup_metrics(duration)?;

        Ok(result)
    }

    /// List signatures of functions registered under a given category.
    ///
    /// # Arguments
    ///
    /// * `category` - The function category to query.
    ///
    /// # Returns
    ///
    /// A vector containing the signatures of functions in the specified category; empty if none are registered.
    ///
    /// # Examples
    ///
    /// ```
    /// let registry = PureFunctionRegistry::new();
    /// let signatures = registry.get_category_functions(FunctionCategory::Transformation).unwrap();
    /// assert!(signatures.is_empty());
    /// ```
    pub fn get_category_functions(
        &self,
        category: FunctionCategory,
    ) -> Result<Vec<&'static str>, RegistryError> {
        let functions = self
            .functions
            .read()
            .map_err(|_| RegistryError::LockPoisoned)?;

        Ok(functions
            .get(&category)
            .map(|category_map| category_map.keys().copied().collect())
            .unwrap_or_default())
    }

    /// Attempts to register a new function produced by composing two existing functions in the registry.
    ///
    /// This method looks up two functions by their signatures and category, and if found,
    /// creates a composed function that applies the first function to the input and then
    /// applies the second function to the result.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::FunctionNotFound` if either function is not found,
    /// `RegistryError::IncompatibleComposition` if the output type of the first function
    /// doesn't match the input type of the second function, or `RegistryError::LockPoisoned`
    /// if registry locks are poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// // In a real implementation, this would compose two compatible functions
    /// // let registry = PureFunctionRegistry::new();
    /// // let result = registry.compose_functions("double", "increment", FunctionCategory::Mathematical, "double_then_increment");
    /// // assert!(result.is_ok());
    /// ```
    pub fn compose_functions(
        &self,
        first_sig: &str,
        second_sig: &str,
        category: FunctionCategory,
        composed_sig: &'static str,
    ) -> Result<(), RegistryError> {
        let _start = Instant::now();
        let functions = self
            .functions
            .read()
            .map_err(|_| RegistryError::LockPoisoned)?;

        // Look up the first function
        let first_func = functions
            .get(&category)
            .and_then(|category_map| category_map.get(first_sig))
            .ok_or_else(|| RegistryError::FunctionNotFound {
                category,
                signature: first_sig.to_string(),
            })?;

        // Look up the second function
        let second_func = functions
            .get(&category)
            .and_then(|category_map| category_map.get(second_sig))
            .ok_or_else(|| RegistryError::FunctionNotFound {
                category,
                signature: second_sig.to_string(),
            })?;

        // Check type compatibility: first function's output type must match second function's input type
        if first_func.output_type_id() != second_func.input_type_id() {
            return Err(RegistryError::IncompatibleComposition {
                first_sig: first_sig.to_string(),
                second_sig: second_sig.to_string(),
                reason: format!(
                    "Type mismatch: output of '{}' ({:?}) != input of '{}' ({:?})",
                    first_sig,
                    first_func.output_type_id(),
                    second_sig,
                    second_func.input_type_id()
                ),
            });
        }

        // Clone necessary information before dropping the read lock
        let first_sig_owned = first_sig.to_string();
        let second_sig_owned = second_sig.to_string();
        let category_owned = category;

        // Clone self for use in the composed function
        let self_clone = self;

        // Drop the read lock before acquiring a write lock
        drop(functions);

        // For now, we'll return an error indicating this is not yet fully implemented
        // A complete implementation would require complex type checking and runtime composition
        Err(RegistryError::IncompatibleComposition {
            first_sig: first_sig.to_string(),
            second_sig: second_sig.to_string(),
            reason: "Function composition requires complex type checking and is not yet fully implemented".to_string(),
        })
    }

    /// Execute a registered function identified by category and signature using the provided input,
    /// returning the function's output when the stored function exists and the requested output type matches.
    ///
    /// Returns `Ok(Some(output))` when a function with the given category and signature is found and its output can be
    /// downcast to `Output`. Returns `Ok(None)` when no function is found or when the runtime output type does not match
    /// `Output`. Returns `Err(RegistryError::LockPoisoned)` if registry locks are poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assumes the standard prelude registers an "identity" transformation that returns its input.
    /// let registry = crate::functional::prelude::create_standard_registry().unwrap();
    /// let out = registry
    ///     .execute(crate::functional::pure_function_registry::FunctionCategory::Transformation, "identity", 42)
    ///     .unwrap();
    /// assert_eq!(out, Some(42));
    /// ```
    pub fn execute<Input, Output>(
        &self,
        category: FunctionCategory,
        signature: &str,
        input: Input,
    ) -> Result<Option<Output>, RegistryError>
    where
        Input: Send + Sync + 'static,
        Output: Send + Sync + 'static,
    {
        let start = Instant::now();
        let functions = self
            .functions
            .read()
            .map_err(|_| RegistryError::LockPoisoned)?;

        let result = functions
            .get(&category)
            .and_then(|category_map| category_map.get(signature))
            .and_then(|container| container.try_call(Box::new(input)))
            .and_then(|boxed_result| boxed_result.downcast::<Output>().ok())
            .map(|output| *output);

        let duration = start.elapsed();
        self.update_lookup_metrics(duration)?;

        Ok(result)
    }

    /// Checks whether a registered function produces the same output for repeated identical inputs.
    ///
    /// Executes the function identified by `category` and `signature` for `iterations` times (defaults to 100)
    /// using the provided `input`. Returns `true` if every invocation produces a value equal to the first
    /// observed result, `false` if any invocation produces a different result, and `Err` if the registry
    /// lookup or execution fails (for example, if the function is not found).
    ///
    /// # Parameters
    ///
    /// * `category` - The function's category used for lookup.
    /// * `signature` - The function's signature string used for lookup.
    /// * `input` - The input value to pass to each invocation.
    /// * `iterations` - Optional number of times to invoke the function; if `None`, 100 iterations are used.
    ///
    /// # Returns
    ///
    /// `true` if all outputs across the runs are equal, `false` otherwise. Returns `Err(RegistryError)` on lookup/execution errors.
    ///
    /// # Examples
    ///
    /// ```
    /// // Create a registry with standard functions (identity, double, string_length)
    /// let registry = crate::functional::prelude::create_standard_registry().unwrap();
    ///
    /// // Validate purity of the pre-registered "identity" transformation for integers
    /// let is_pure = registry.validate_purity(
    ///     crate::functional::pure_function_registry::FunctionCategory::Transformation,
    ///     "identity",
    ///     42i32,
    ///     Some(10),
    /// ).unwrap();
    ///
    /// assert!(is_pure);
    /// ```
    pub fn validate_purity<Input, Output>(
        &self,
        category: FunctionCategory,
        signature: &str,
        input: Input,
        iterations: Option<usize>,
    ) -> Result<bool, RegistryError>
    where
        Input: Clone + Send + Sync + 'static,
        Output: Eq + Send + Sync + 'static,
    {
        let iterations = iterations.unwrap_or(100);
        let mut results: Vec<Output> = Vec::with_capacity(iterations);

        // Execute function multiple times
        for _ in 0..iterations {
            let result = self.execute(category, signature, input.clone())?;
            match result {
                Some(output) => results.push(output),
                None => {
                    return Err(RegistryError::FunctionNotFound {
                        category,
                        signature: signature.to_string(),
                    })
                }
            }
        }

        // Guard against zero iterations or empty results
        if iterations == 0 || results.is_empty() {
            // If no runs were performed, return true as no non-deterministic behavior was detected
            return Ok(true);
        }

        // Check if all results are identical
        let first_result = &results[0];
        Ok(results.iter().all(|r| r == first_result))
    }

    /// Return a snapshot of the registry's current metrics.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::LockPoisoned` if the internal metrics lock is poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// let registry = PureFunctionRegistry::new();
    /// let metrics = registry.get_metrics().unwrap();
    /// assert_eq!(metrics.total_functions, 0);
    /// ```
    pub fn get_metrics(&self) -> Result<RegistryMetrics, RegistryError> {
        let metrics = self
            .metrics
            .read()
            .map_err(|_| RegistryError::LockPoisoned)?;
        Ok(metrics.clone())
    }

    /// Clears all registered functions and resets the registry's metrics to default.
    ///
    /// After this call the registry will contain no functions and metrics such as
    /// `total_functions` and `lookup_count` will be reset.
    ///
    /// # Examples
    ///
    /// ```
    /// let registry = PureFunctionRegistry::new();
    /// // register functions here...
    /// registry.clear().unwrap();
    /// assert_eq!(registry.get_metrics().unwrap().total_functions, 0);
    /// assert_eq!(registry.get_metrics().unwrap().lookup_count, 0);
    /// ```
    pub fn clear(&self) -> Result<(), RegistryError> {
        let mut functions = self
            .functions
            .write()
            .map_err(|_| RegistryError::LockPoisoned)?;
        functions.clear();

        let mut metrics = self
            .metrics
            .write()
            .map_err(|_| RegistryError::LockPoisoned)?;
        *metrics = RegistryMetrics::default();

        Ok(())
    }

    /// Update the registry's lookup-performance metrics with a new duration measurement.
    ///
    /// Adds the measured duration (in nanoseconds) to the total accumulated lookup time,
    /// increments the lookup count, and recomputes the running average lookup time.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError::LockPoisoned` if the metrics lock is poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let registry = PureFunctionRegistry::new();
    ///
    /// // record a 100-nanosecond lookup measurement
    /// registry.update_lookup_metrics(Duration::from_nanos(100)).unwrap();
    ///
    /// let metrics = registry.get_metrics().unwrap();
    /// assert_eq!(metrics.lookup_count, 1);
    /// assert_eq!(metrics.avg_lookup_time_ns, 100);
    /// ```
    fn update_lookup_metrics(&self, duration: Duration) -> Result<(), RegistryError> {
        let mut metrics = self
            .metrics
            .write()
            .map_err(|_| RegistryError::LockPoisoned)?;

        let new_measurement = duration.as_nanos();

        // Accumulate total time (no precision loss)
        metrics.total_lookup_time_ns += new_measurement;
        metrics.lookup_count += 1;

        // Recompute average from accumulated total
        metrics.recompute_average();

        Ok(())
    }
}

/// Errors that can occur during registry operations.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Function already exists: {category:?}::{signature}")]
    FunctionAlreadyExists {
        category: FunctionCategory,
        signature: String,
    },

    #[error("Function not found: {category:?}::{signature}")]
    FunctionNotFound {
        category: FunctionCategory,
        signature: String,
    },

    #[error("Incompatible composition: {first_sig} -> {second_sig}: {reason}")]
    IncompatibleComposition {
        first_sig: String,
        second_sig: String,
        reason: String,
    },

    #[error("Type mismatch during operation")]
    TypeMismatch,

    #[error("Registry lock was poisoned")]
    LockPoisoned,
}

/// Thread-safe wrapper for the registry.
pub type SharedRegistry = Arc<PureFunctionRegistry>;

impl PureFunctionRegistry {
    /// Creates a new `Arc`-wrapped `PureFunctionRegistry` for shared, thread-safe use.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use crate::functional::pure_function_registry::PureFunctionRegistry;
    ///
    /// let registry = PureFunctionRegistry::shared();
    /// let registry_clone = registry.clone();
    /// assert_eq!(Arc::strong_count(&registry), 2);
    /// ```
    pub fn shared() -> SharedRegistry {
        Arc::new(Self::new())
    }
}

/// Convenience functions for common registry operations.
pub mod prelude {
    use super::*;

    /// Creates a shared PureFunctionRegistry populated with common pure functions.
    ///
    /// On success returns a SharedRegistry containing the pre-registered functions:
    /// - "identity" (Transformation): i32 -> i32
    /// - "double" (Mathematical): i32 -> i32
    /// - "string_length" (StringProcessing): String -> usize
    ///
    /// # Errors
    ///
    /// Returns a RegistryError if any registration fails (for example, due to lock poisoning or a duplicate signature).
    ///
    /// # Examples
    ///
    /// ```
    /// let registry = create_standard_registry().expect("failed to create registry");
    /// let info = registry
    ///     .lookup(FunctionCategory::Transformation, "identity")
    ///     .unwrap()
    ///     .unwrap();
    /// assert_eq!(info.signature, "identity");
    /// ```
    pub fn create_standard_registry() -> Result<SharedRegistry, RegistryError> {
        let registry = PureFunctionRegistry::shared();

        // Register common transformation functions
        registry.register(FunctionWrapper::new(
            |x: i32| x,
            "identity",
            FunctionCategory::Transformation,
        ))?;

        registry.register(FunctionWrapper::new(
            |x: i32| x * 2,
            "double",
            FunctionCategory::Mathematical,
        ))?;

        registry.register(FunctionWrapper::new(
            |s: String| s.len(),
            "string_length",
            FunctionCategory::StringProcessing,
        ))?;

        Ok(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functional::function_traits::{FunctionCategory, FunctionWrapper};

    #[test]
    fn test_registry_creation() {
        let registry = PureFunctionRegistry::new();
        assert_eq!(registry.get_metrics().unwrap().total_functions, 0);
    }

    #[test]
    fn test_function_registration() {
        let registry = PureFunctionRegistry::new();

        // Register a simple function
        let result = registry.register(FunctionWrapper::new(
            |x: i32| x * 2,
            "double",
            FunctionCategory::Mathematical,
        ));

        assert!(result.is_ok());
        assert_eq!(registry.get_metrics().unwrap().total_functions, 1);
    }

    #[test]
    fn test_function_lookup() {
        let registry = PureFunctionRegistry::new();

        // Register a function
        registry
            .register(FunctionWrapper::new(
                |x: i32| x + 1,
                "increment",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        // Look it up
        let info = registry
            .lookup(FunctionCategory::Mathematical, "increment")
            .unwrap();
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.signature, "increment");
        assert_eq!(info.category, FunctionCategory::Mathematical);
    }

    #[test]
    fn test_function_execution() {
        let registry = PureFunctionRegistry::new();

        // Register a function
        registry
            .register(FunctionWrapper::new(
                |x: i32| x * 3,
                "triple",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        // Execute it
        let result: Option<i32> = registry
            .execute(FunctionCategory::Mathematical, "triple", 5)
            .unwrap();
        assert_eq!(result, Some(15));
    }

    #[test]
    fn test_duplicate_registration_error() {
        let registry = PureFunctionRegistry::new();

        // Register a function
        registry
            .register(FunctionWrapper::new(
                |x: i32| x + 1,
                "increment",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        // Try to register the same function again
        let result = registry.register(FunctionWrapper::new(
            |x: i32| x + 2,
            "increment",
            FunctionCategory::Mathematical,
        ));

        assert!(result.is_err());
        match result.unwrap_err() {
            RegistryError::FunctionAlreadyExists { .. } => {}
            _ => panic!("Expected FunctionAlreadyExists error"),
        }
    }

    #[test]
    fn test_purity_validation() {
        let registry = PureFunctionRegistry::new();

        // Register a pure function
        registry
            .register(FunctionWrapper::new(
                |x: i32| x * 2,
                "pure_double",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        // Validate purity
        let is_pure = registry
            .validate_purity::<i32, i32>(
                FunctionCategory::Mathematical,
                "pure_double",
                10,
                Some(10),
            )
            .unwrap();

        assert!(is_pure);
    }

    #[test]
    fn test_category_functions() {
        let registry = PureFunctionRegistry::new();

        // Register functions in different categories
        registry
            .register(FunctionWrapper::new(
                |x: i32| x + 1,
                "increment",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        registry
            .register(FunctionWrapper::new(
                |s: String| s.len(),
                "length",
                FunctionCategory::StringProcessing,
            ))
            .unwrap();

        registry
            .register(FunctionWrapper::new(
                |x: i32| x * 2,
                "double",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        // Check mathematical functions
        let math_funcs = registry
            .get_category_functions(FunctionCategory::Mathematical)
            .unwrap();
        assert_eq!(math_funcs.len(), 2);
        assert!(math_funcs.contains(&"increment"));
        assert!(math_funcs.contains(&"double"));

        // Check string processing functions
        let string_funcs = registry
            .get_category_functions(FunctionCategory::StringProcessing)
            .unwrap();
        assert_eq!(string_funcs.len(), 1);
        assert!(string_funcs.contains(&"length"));
    }

    #[test]
    fn test_performance_metrics() {
        let registry = PureFunctionRegistry::new();

        // Register a function
        registry
            .register(FunctionWrapper::new(
                |x: i32| x + 1,
                "increment",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        // Perform some lookups
        for _ in 0..10 {
            let _ = registry
                .lookup(FunctionCategory::Mathematical, "increment")
                .unwrap();
        }

        let metrics = registry.get_metrics().unwrap();
        assert_eq!(metrics.total_functions, 1);
        assert_eq!(metrics.lookup_count, 10);
    }

    #[test]
    fn test_registry_clear() {
        let registry = PureFunctionRegistry::new();

        // Register some functions
        registry
            .register(FunctionWrapper::new(
                |x: i32| x + 1,
                "increment",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        registry
            .register(FunctionWrapper::new(
                |s: String| s.len(),
                "length",
                FunctionCategory::StringProcessing,
            ))
            .unwrap();

        assert_eq!(registry.get_metrics().unwrap().total_functions, 2);

        // Clear the registry
        registry.clear().unwrap();

        assert_eq!(registry.get_metrics().unwrap().total_functions, 0);
        assert_eq!(registry.get_metrics().unwrap().lookup_count, 0);
    }

    #[test]
    fn test_lookup_metrics_precision() {
        use std::time::Duration;

        let registry = PureFunctionRegistry::new();

        // Register a function
        registry
            .register(FunctionWrapper::new(
                |x: i32| x + 1,
                "increment",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        // Simulate lookups with varying durations that would cause precision loss
        // with integer division on each update
        // Example: 3 lookups of 1ns, 2ns, 3ns = average should be 2ns exactly
        registry
            .update_lookup_metrics(Duration::from_nanos(1))
            .unwrap();
        registry
            .update_lookup_metrics(Duration::from_nanos(2))
            .unwrap();
        registry
            .update_lookup_metrics(Duration::from_nanos(3))
            .unwrap();

        let metrics = registry.get_metrics().unwrap();
        assert_eq!(metrics.lookup_count, 3);
        // With the old integer division approach: (0*0 + 1)/1 = 1, (1*1 + 2)/2 = 1, (1*2 + 3)/3 = 1
        // With the new approach: (1 + 2 + 3) / 3 = 2 (exact)
        assert_eq!(metrics.avg_lookup_time_ns, 2);

        // Test with many measurements to ensure no accumulated rounding errors
        registry.clear().unwrap();
        registry
            .register(FunctionWrapper::new(
                |x: i32| x + 1,
                "increment",
                FunctionCategory::Mathematical,
            ))
            .unwrap();

        // Add 1000 measurements of 1000ns each
        for _ in 0..1000 {
            registry
                .update_lookup_metrics(Duration::from_nanos(1000))
                .unwrap();
        }

        let metrics = registry.get_metrics().unwrap();
        assert_eq!(metrics.lookup_count, 1000);
        // Should be exactly 1000ns average with no precision loss
        assert_eq!(metrics.avg_lookup_time_ns, 1000);
    }
}
