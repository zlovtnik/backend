//! Iterator-Based Validation Engine
//!
//! Core validation system that leverages iterators and higher-order functions
//! to create composable validation pipelines. Uses itertools for advanced
//! iterator operations and provides pure functional validation processing.

#![allow(dead_code)]

use std::collections::HashMap;

use crate::functional::validation_rules::{ValidationError, ValidationResult, ValidationRule};

/// Validation pipeline configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Stop on first validation error
    pub fail_fast: bool,
    /// Maximum number of validation errors to collect
    pub max_errors: Option<usize>,
    /// Enable parallel validation for large datasets
    pub parallel_validation: bool,
}

impl Default for ValidationConfig {
    /// Creates a ValidationConfig populated with sensible defaults.
    ///
    /// The defaults are:
    /// - `fail_fast = true`
    /// - `max_errors = Some(10)`
    /// - `parallel_validation = false`
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = ValidationConfig::default();
    /// assert!(cfg.fail_fast);
    /// assert_eq!(cfg.max_errors, Some(10));
    /// assert!(!cfg.parallel_validation);
    /// ```
    fn default() -> Self {
        Self {
            fail_fast: true,
            max_errors: Some(10),
            parallel_validation: false,
        }
    }
}

/// Validation context for tracking field paths and metadata
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Current field path (e.g., "user.address.street")
    pub field_path: String,
    /// Additional context data
    pub metadata: HashMap<String, String>,
}

impl ValidationContext {
    /// Creates a new ValidationContext for a given field path with no metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// let ctx = ValidationContext::new("user.address.street");
    /// assert_eq!(ctx.field_path, "user.address.street");
    /// assert!(ctx.metadata.is_empty());
    /// ```
    pub fn new(field_path: &str) -> Self {
        Self {
            field_path: field_path.to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Adds a metadata key-value pair to the validation context and returns the updated context.
    ///
    /// The provided key and value are stored as UTF-8 strings in the context's metadata map,
    /// replacing any existing value for the same key.
    ///
    /// # Examples
    ///
    /// ```
    /// let ctx = ValidationContext::new("user.email")
    ///     .with_metadata("source", "signup_form");
    /// assert_eq!(ctx.metadata.get("source"), Some(&"signup_form".to_string()));
    /// ```
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Extends the current context with a nested field name using dot notation.
    ///
    /// If the current `field_path` is empty, the returned context's `field_path` is
    /// set to `field_name`; otherwise the new path is `"{current}.{field_name}"`.
    /// The `metadata` is cloned into the new context.
    ///
    /// # Examples
    ///
    /// ```
    /// let ctx = ValidationContext::new("user");
    /// let child = ctx.child("address");
    /// assert_eq!(child.field_path, "user.address");
    /// let root = ValidationContext::new("");
    /// let direct = root.child("id");
    /// assert_eq!(direct.field_path, "id");
    /// ```
    pub fn child(&self, field_name: &str) -> Self {
        let new_path = if self.field_path.is_empty() {
            field_name.to_string()
        } else {
            format!("{}.{}", self.field_path, field_name)
        };

        Self {
            field_path: new_path,
            metadata: self.metadata.clone(),
        }
    }
}

/// Validation result with detailed error collection
#[derive(Debug, Clone)]
pub struct ValidationOutcome<T> {
    /// The validated value (if validation succeeded)
    pub value: Option<T>,
    /// Collection of validation errors
    pub errors: Vec<ValidationError>,
    /// Whether validation passed
    pub is_valid: bool,
}

impl<T> ValidationOutcome<T> {
    /// Creates a successful validation outcome containing the provided value.
    ///
    /// The returned outcome has `value` set to the given value, an empty `errors` vector, and `is_valid` set to `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// let o = ValidationOutcome::success(42);
    /// assert!(o.is_valid);
    /// assert_eq!(o.value, Some(42));
    /// assert!(o.errors.is_empty());
    /// ```
    pub fn success(value: T) -> Self {
        Self {
            value: Some(value),
            errors: Vec::new(),
            is_valid: true,
        }
    }

    /// Creates a failed validation outcome containing the provided errors and no value.
    ///
    /// The returned outcome has `is_valid` set to `false`, `value` set to `None`, and `errors` set
    /// to the given vector.
    ///
    /// # Examples
    ///
    /// ```
    /// let outcome = ValidationOutcome::<i32>::failure(vec![]);
    /// assert!(!outcome.is_valid);
    /// assert!(outcome.value.is_none());
    /// assert_eq!(outcome.errors.len(), 0);
    /// ```
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            value: None,
            errors,
            is_valid: false,
        }
    }

    /// Marks the outcome as failed by appending the provided error and clearing any successful value.
    ///
    /// The returned `ValidationOutcome` will have the error appended to its `errors` vector,
    /// `is_valid` set to `false`, and `value` set to `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Construct a successful outcome, then add an error to it.
    /// let outcome = ValidationOutcome::success(42);
    /// let err = ValidationError { code: "E001".into(), message: "Invalid value".into(), field: "age".into() };
    /// let failed = outcome.add_error(err);
    /// assert!(!failed.is_valid);
    /// assert!(failed.value.is_none());
    /// assert_eq!(failed.errors.len(), 1);
    /// ```
    pub fn add_error(mut self, error: ValidationError) -> Self {
        self.errors.push(error);
        self.is_valid = false;
        self.value = None;
        self
    }

    /// Merges another `ValidationOutcome` into this one, combining errors and updating validity and value.
    ///
    /// The resulting outcome contains all errors from both operands. If `other` is invalid, the result is marked invalid and its stored value is cleared.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = ValidationOutcome::success(42);
    /// let b: ValidationOutcome<i32> = ValidationOutcome::failure(vec![]);
    /// let combined = a.combine(b);
    /// assert!(!combined.is_valid);
    /// assert!(combined.value.is_none());
    /// assert_eq!(combined.errors.len(), 0);
    /// ```
    pub fn combine(mut self, other: ValidationOutcome<T>) -> Self {
        self.errors.extend(other.errors);
        if !other.is_valid {
            self.is_valid = false;
            self.value = None;
        }
        self
    }
}

/// Iterator-based validation engine
pub struct ValidationEngine<T> {
    config: ValidationConfig,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ValidationEngine<T> {
    /// Constructs a ValidationEngine using the default ValidationConfig.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = ValidationEngine::<i32>::new();
    /// // default configuration is applied (fail_fast = true by default)
    /// assert!(engine.config.fail_fast);
    /// ```
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Constructs a ValidationEngine using the provided configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = ValidationConfig {
    ///     fail_fast: false,
    ///     max_errors: Some(5),
    ///     parallel_validation: true,
    /// };
    /// /// let engine: ValidationEngine<String> = ValidationEngine::with_config(cfg);
    /// assert_eq!(engine.config.fail_fast, false);
    /// ```
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Validate a single field against an iterator of validation rules and collect any errors.
    ///
    /// This applies each provided rule to `value` using a context for `field_name`. Collected errors
    /// are returned if any rules fail; validation honors the engine configuration (stopping early if
    /// `fail_fast` is true and respecting `max_errors` when set).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let engine = ValidationEngine::<String>::new();
    /// let value = "example".to_string();
    /// // `rules` would be an iterable of objects implementing `ValidationRule<String>`.
    /// let outcome = engine.validate_field(&value, "username", Vec::<Box<dyn ValidationRule<String>>>::new());
    /// if outcome.is_valid {
    ///     println!("field valid: {}", outcome.value.unwrap());
    /// } else {
    ///     println!("errors: {:?}", outcome.errors);
    /// }
    /// ```
    pub fn validate_field<'a, I, R>(
        &self,
        value: &'a T,
        field_name: &str,
        rules: I,
    ) -> ValidationOutcome<&'a T>
    where
        I: IntoIterator<Item = R>,
        R: ValidationRule<T>,
    {
        let mut errors = Vec::new();
        let context = ValidationContext::new(field_name);

        for rule in rules {
            match rule.validate(value, &context.field_path) {
                Ok(()) => {
                    // Rule passed, continue
                }
                Err(error) => {
                    errors.push(error);

                    // Check if we should stop on first error
                    if self.config.fail_fast {
                        break;
                    }

                    // Check if we've reached the maximum error limit
                    if let Some(max) = self.config.max_errors {
                        if errors.len() >= max {
                            break;
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            ValidationOutcome::success(value)
        } else {
            ValidationOutcome::failure(errors)
        }
    }

    /// Validate multiple named fields and aggregate their outcomes.
    ///
    /// Iterates the provided (field_name, value, rules) tuples, validating each field with the given rules.
    /// On success returns a map of field names to their validated references; on any failures returns all collected validation errors.
    /// If the engine is configured with `fail_fast`, validation stops after the first failing field.
    ///
    /// # Examples
    ///
    /// ```
    /// /// let engine = validator::<i32>();
    /// let inputs = vec![(String::from("age"), &42, Vec::new::<()>())];
    /// let outcome = engine.validate_fields(inputs);
    /// assert!(outcome.is_valid);
    /// ```
    pub fn validate_fields<'a, I, R>(
        &self,
        field_validators: I,
    ) -> ValidationOutcome<HashMap<String, &'a T>>
    where
        I: IntoIterator<Item = (String, &'a T, Vec<R>)>,
        R: ValidationRule<T>,
    {
        let mut results = HashMap::new();
        let mut all_errors = Vec::new();
        let mut has_failures = false;

        for (field_name, value, rules) in field_validators {
            let field_result = self.validate_field(value, &field_name, rules);

            if field_result.is_valid {
                results.insert(field_name, value);
            } else {
                has_failures = true;
                all_errors.extend(field_result.errors);
            }

            // Stop if fail_fast is enabled and we have errors
            if self.config.fail_fast && has_failures {
                break;
            }
        }

        if has_failures {
            ValidationOutcome::failure(all_errors)
        } else {
            ValidationOutcome::success(results)
        }
    }
}

/// Creates a validation rule that applies the provided rules only when a predicate is true.
///
/// When `condition` returns `true` for a value, each rule in `rules` is evaluated; the combined rule
/// fails if any of those rules fail. When `condition` returns `false`, the combined rule succeeds
/// (validation is skipped).
///
/// # Parameters
///
/// - `condition`: Predicate that decides whether the supplied rules should be applied to a value.
/// - `rules`: Collection of validation rules to apply when the predicate is true.
///
/// # Returns
///
/// An `impl ValidationRule<T>` that performs conditional validation: it fails when the condition is
/// true and any of the provided rules produce an error, and succeeds otherwise.
///
/// # Examples
///
/// ```
/// # use crate::functional::validation_rules::Custom;
/// # use crate::functional::validation_engine::conditional_validate;
/// // A simple rule that fails for empty strings
/// let non_empty_rule = Custom::new(|v: &String| !v.is_empty(), "EMPTY", "Value is empty");
/// // Apply `non_empty_rule` only when the string starts with 'A'
/// let rule = conditional_validate(|s: &String| s.starts_with('A'), vec![non_empty_rule]);
///
/// let val = "Apple".to_string();
/// assert!(rule.validate(&val, "").is_ok());
///
/// let val2 = "Banana".to_string();
/// // Condition false -> validation skipped -> succeeds
/// assert!(rule.validate(&val2, "").is_ok());
/// ```
pub fn conditional_validate<T, C, R>(condition: C, rules: Vec<R>) -> impl ValidationRule<T>
where
    C: Fn(&T) -> bool,
    R: ValidationRule<T>,
{
    crate::functional::validation_rules::Custom::new(
        move |value: &T| {
            if condition(value) {
                // Apply all rules and collect errors
                let errors: Vec<_> = rules
                    .iter()
                    .filter_map(|rule| rule.validate(value, "").err())
                    .collect();

                if errors.is_empty() {
                    true
                } else {
                    false
                }
            } else {
                // Condition not met, skip validation
                true
            }
        },
        "CONDITIONAL_VALIDATION_FAILED",
        "Conditional validation failed",
    )
}

/// Creates a validation rule that applies the provided element rules to each item in a collection.
///
/// The returned rule validates a `Vec<T>` by running every `element_rule` against each element (with the
/// element's index included in the field path as `item[index]`). The collection rule fails if any element
/// fails any of the element rules. On failure the rule uses the code `"COLLECTION_VALIDATION_FAILED"`.
///
/// # Examples
///
/// ```
/// use std::vec::Vec;
/// // Build a simple element rule that requires a non-empty string.
/// let elem_rule = crate::functional::validation_rules::Custom::new(
///     |s: &String| !s.is_empty(),
///     "REQUIRED",
///     "must not be empty",
/// );
///
/// let rule = crate::functional::validation_engine::validate_collection(vec![elem_rule]);
///
/// let good = vec!["a".to_string(), "b".to_string()];
/// assert!(rule.validate(&good, "items").is_ok());
///
/// let bad = vec!["a".to_string(), "".to_string()];
/// assert!(rule.validate(&bad, "items").is_err());
/// ```
pub fn validate_collection<T, R>(element_rules: Vec<R>) -> impl ValidationRule<Vec<T>>
where
    R: ValidationRule<T> + Clone,
{
    crate::functional::validation_rules::Custom::new(
        move |collection: &Vec<T>| {
            // Use iterator to validate each element
            let has_errors = collection.iter().enumerate().any(|(index, item)| {
                element_rules
                    .iter()
                    .any(|rule| rule.validate(item, &format!("item[{}]", index)).is_err())
            });

            !has_errors
        },
        "COLLECTION_VALIDATION_FAILED",
        "One or more collection elements failed validation",
    )
}

/// Creates a validation rule that ensures a set of keys exist in a map and then applies a cross-field predicate.
///
/// The produced rule fails if any required key from `fields` is missing from the input map, or if `validator`
/// returns `false` when invoked with the full map.
///
/// # Parameters
/// - `fields`: list of keys that must be present in the map for the rule to run the predicate.
/// - `validator`: predicate invoked with the map when all required keys are present; returning `true` means the map passes.
///
/// # Returns
/// A `ValidationRule<HashMap<String, T>>` that enforces presence of the specified keys and the provided predicate.
/// The rule uses the error code `CROSS_FIELD_VALIDATION_FAILED` when it fails.
///
/// # Examples
///
/// ```no_run
/// use std::collections::HashMap;
/// // Require "start" and "end" keys and ensure start <= end
/// let rule = cross_field_validate(vec!["start".to_string(), "end".to_string()], |m: &HashMap<String, i32>| {
///     m.get("start").zip(m.get("end")).map_or(false, |(s, e)| s <= e)
/// });
///
/// let mut map = HashMap::new();
/// map.insert("start".to_string(), 1);
/// map.insert("end".to_string(), 3);
/// // `rule` should consider `map` valid (implementation-specific invocation omitted)
/// ```
pub fn cross_field_validate<T, F>(
    fields: Vec<String>,
    validator: F,
) -> impl ValidationRule<HashMap<String, T>>
where
    F: Fn(&HashMap<String, T>) -> bool,
{
    crate::functional::validation_rules::Custom::new(
        move |field_map: &HashMap<String, T>| {
            // Check if all required fields are present
            let all_present = fields.iter().all(|field| field_map.contains_key(field));

            if !all_present {
                return false;
            }

            // Apply cross-field validation
            validator(field_map)
        },
        "CROSS_FIELD_VALIDATION_FAILED",
        "Cross-field validation failed",
    )
}

/// Enhanced cross-field validation that validates field dependencies.
/// Requires that if field A is present, then field B must also be present.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// // If "password" is present, then "password_confirm" must also be present
/// let rule = require_field_if_present("password", "password_confirm");
/// ```
pub fn require_field_if_present<T>(
    conditional_field: &str,
    required_field: &str,
) -> impl ValidationRule<HashMap<String, T>> {
    let conditional_field = conditional_field.to_string();
    let required_field = required_field.to_string();
    let conditional_field_clone = conditional_field.clone();
    let required_field_clone = required_field.clone();

    crate::functional::validation_rules::Custom::new(
        move |field_map: &HashMap<String, T>| {
            // If conditional field is present, required field must also be present
            if field_map.contains_key(&conditional_field)
                && !field_map.contains_key(&required_field)
            {
                return false;
            }
            true
        },
        "MISSING_DEPENDENT_FIELD",
        &format!(
            "If {} is present, {} must also be present",
            conditional_field_clone, required_field_clone
        ),
    )
}

/// Validates that fields are mutually exclusive - at most one of the specified fields can be present.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// // Either "email" or "phone" can be present, but not both
/// let rule = mutually_exclusive_fields(vec!["email", "phone"]);
/// ```
pub fn mutually_exclusive_fields<T>(fields: Vec<&str>) -> impl ValidationRule<HashMap<String, T>> {
    let fields: Vec<String> = fields.into_iter().map(|s| s.to_string()).collect();

    crate::functional::validation_rules::Custom::new(
        move |field_map: &HashMap<String, T>| {
            let present_fields: Vec<_> = fields
                .iter()
                .filter(|field| field_map.contains_key(*field))
                .collect();

            // At most one field should be present
            present_fields.len() <= 1
        },
        "MUTUALLY_EXCLUSIVE_FIELDS",
        "Multiple mutually exclusive fields are present",
    )
}

/// Validates that exactly one of the specified fields must be present.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// // Either "email" or "phone" must be present (but not both)
/// let rule = require_exactly_one_of(vec!["email", "phone"]);
/// ```
pub fn require_exactly_one_of<T>(fields: Vec<&str>) -> impl ValidationRule<HashMap<String, T>> {
    let fields: Vec<String> = fields.into_iter().map(|s| s.to_string()).collect();

    crate::functional::validation_rules::Custom::new(
        move |field_map: &HashMap<String, T>| {
            let present_count = fields
                .iter()
                .filter(|field| field_map.contains_key(*field))
                .count();

            present_count == 1
        },
        "EXACTLY_ONE_FIELD_REQUIRED",
        "Exactly one of the specified fields must be present",
    )
}

/// Validates that if any field from a group is present, then all fields from that group must be present.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// // If any address field is present, all address fields must be present
/// let rule = require_all_or_none(vec!["street", "city", "zip_code"]);
/// ```
pub fn require_all_or_none<T>(fields: Vec<&str>) -> impl ValidationRule<HashMap<String, T>> {
    let fields: Vec<String> = fields.into_iter().map(|s| s.to_string()).collect();

    crate::functional::validation_rules::Custom::new(
        move |field_map: &HashMap<String, T>| {
            let present_count = fields
                .iter()
                .filter(|field| field_map.contains_key(*field))
                .count();

            // Either all fields are present, or none are present
            present_count == 0 || present_count == fields.len()
        },
        "ALL_OR_NONE_FIELDS",
        "Either all fields in the group must be present, or none",
    )
}

/// Validates field value relationships using a custom comparison function.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// // Ensure start_date <= end_date
/// let rule = compare_fields("start_date", "end_date", |start: &i64, end: &i64| start <= end);
/// ```
pub fn compare_fields<T, F>(
    field1: &str,
    field2: &str,
    comparator: F,
) -> impl ValidationRule<HashMap<String, T>>
where
    F: Fn(&T, &T) -> bool,
{
    let field1 = field1.to_string();
    let field2 = field2.to_string();
    let error_message = format!(
        "Field {} does not satisfy comparison with {}",
        field1, field2
    );

    crate::functional::validation_rules::Custom::new(
        move |field_map: &HashMap<String, T>| {
            if let (Some(val1), Some(val2)) = (field_map.get(&field1), field_map.get(&field2)) {
                comparator(val1, val2)
            } else {
                // If either field is missing, comparison can't fail
                true
            }
        },
        "FIELD_COMPARISON_FAILED",
        &error_message,
    )
}

/// Iterator-based validation pipeline for processing streams of data
pub struct ValidationPipeline<T, I>
where
    I: Iterator<Item = T>,
{
    iterator: I,
    validators: Vec<Box<dyn Fn(&T) -> ValidationResult<()>>>,
    config: ValidationConfig,
}

impl<T, I> ValidationPipeline<T, I>
where
    I: Iterator<Item = T>,
{
    /// Creates a new ValidationPipeline that will iterate over the provided iterator.
    ///
    /// The pipeline is created with no validators and uses the default ValidationConfig.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec!["a", "b", "c"];
    /// let result = ValidationPipeline::new(data.into_iter()).validate();
    /// assert_eq!(result.valid_items.len(), 3);
    /// ```
    pub fn new(iterator: I) -> Self {
        Self {
            iterator,
            validators: Vec::new(),
            config: ValidationConfig::default(),
        }
    }

    /// Add a validator to the pipeline and return the pipeline for chaining.
    ///
    /// The provided validator is applied to each item when the pipeline is executed. The validator
    /// receives a reference to an item of type `T` and must return a `ValidationResult<()>`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let data = vec!["a", "b", "c"];
    /// let pipeline = ValidationPipeline::new(data.into_iter())
    ///     .add_validator(|item: &str| {
    ///         // return Ok(()) for valid items or Err(...) with validation errors
    ///         Ok(())
    ///     });
    /// ```
    pub fn add_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&T) -> ValidationResult<()> + 'static,
    {
        self.validators.push(Box::new(validator));
        self
    }

    /// Applies the given validation configuration to the pipeline and returns the updated pipeline.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3].into_iter();
    /// /// let config = ValidationConfig { fail_fast: false, max_errors: Some(5), parallel_validation: false };
    /// let pipeline = ValidationPipeline::new(data).with_config(config);
    /// ```
    pub fn with_config(mut self, config: ValidationConfig) -> Self {
        self.config = config;
        self
    }

    /// Processes each item from the pipeline's iterator with all configured validators and collects passing items, failing items with their errors, and summary totals.
    ///
    /// The pipeline honors its `fail_fast` and `max_errors` configuration while validating items; items that pass all validators are returned in `valid_items`, items that fail are returned in `invalid_items` paired with their validation errors, and `total_processed`/`total_errors` report counts collected during execution.
    ///
    /// # Examples
    ///
    /// ```
    /// // Example validators: simple closures that succeed for positive values.
    /// let data = vec![1, 2, 3];
    /// let pipeline = ValidationPipeline::new(data.into_iter())
    ///     .add_validator(|v: &i32| if *v > 0 { Ok(()) } else { Err(ValidationError::new("non_positive")) });
    ///
    /// let result = pipeline.validate();
    /// assert_eq!(result.total_processed, 3);
    /// assert!(result.is_all_valid());
    /// ```
    pub fn validate(self) -> ValidationPipelineResult<T> {
        let mut valid_items = Vec::new();
        let mut invalid_items = Vec::new();
        let mut total_errors = 0;

        for item in self.iterator {
            let mut item_errors = Vec::new();

            // Apply all validators to this item
            for validator in &self.validators {
                match validator(&item) {
                    Ok(()) => {}
                    Err(error) => {
                        item_errors.push(error);

                        if self.config.fail_fast {
                            break;
                        }

                        if let Some(max) = self.config.max_errors {
                            if total_errors >= max {
                                break;
                            }
                        }
                    }
                }
            }

            if item_errors.is_empty() {
                valid_items.push(item);
            } else {
                total_errors += item_errors.len();
                invalid_items.push((item, item_errors));
            }

            // Check global error limit
            if let Some(max) = self.config.max_errors {
                if total_errors >= max {
                    break;
                }
            }
        }

        let total_processed = valid_items.len() + invalid_items.len();
        ValidationPipelineResult {
            valid_items,
            invalid_items,
            total_processed,
            total_errors,
        }
    }

    /// Apply the pipeline's validators to every item from the iterator and collect items that passed and items that failed.
    ///
    /// The pipeline's validators are executed for each item; items with no validation errors are returned in `valid_items`, and items with one or more errors are returned in `invalid_items` paired with their errors.
    ///
    /// # Returns
    ///
    /// `ValidationPipelineResult` containing:
    /// - `valid_items`: items that passed all validators,
    /// - `invalid_items`: pairs of items and their validation errors,
    /// - `total_processed`: number of items examined,
    /// - `total_errors`: total number of validation errors across all items.
    ///
    /// # Examples
    ///
    /// ```
    /// let pipeline = ValidationPipeline::new(vec![1, 2, 3].into_iter())
    ///     .add_validator(|_: &i32| Ok(()));
    ///
    /// let result = pipeline.validate_with_itertools();
    ///
    /// assert_eq!(result.valid_items.len(), 3);
    /// assert_eq!(result.total_errors, 0);
    /// ```
    pub fn validate_with_itertools(self) -> ValidationPipelineResult<T>
    where
        T: Clone + Eq + std::hash::Hash,
    {
        // Take ownership of validators before moving self.iterator
        let validators = self.validators;

        // Use itertools for advanced validation patterns
        let mut valid_items = Vec::new();
        let mut invalid_items = Vec::new();

        // Group items by validation status using itertools
        let grouped = self.iterator.map(|item| {
            let errors: Vec<_> = validators
                .iter()
                .filter_map(|validator| validator(&item).err())
                .collect();

            (item, errors)
        });

        for (item, errors) in grouped {
            if errors.is_empty() {
                valid_items.push(item);
            } else {
                invalid_items.push((item, errors));
            }
        }

        let total_processed = valid_items.len() + invalid_items.len();
        let total_errors: usize = invalid_items.iter().map(|(_, errors)| errors.len()).sum();

        ValidationPipelineResult {
            valid_items,
            invalid_items,
            total_processed,
            total_errors,
        }
    }
}

/// Result of running a validation pipeline
#[derive(Debug, Clone)]
pub struct ValidationPipelineResult<T> {
    /// Items that passed all validations
    pub valid_items: Vec<T>,
    /// Items that failed validation with their errors
    pub invalid_items: Vec<(T, Vec<ValidationError>)>,
    /// Total number of items processed
    pub total_processed: usize,
    /// Total number of validation errors
    pub total_errors: usize,
}

impl<T> ValidationPipelineResult<T> {
    /// Determines whether every processed item passed validation.
    ///
    /// # Returns
    ///
    /// `true` if there are no invalid items, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let result = ValidationPipelineResult {
    ///     valid_items: vec![1i32],
    ///     invalid_items: Vec::new(),
    ///     total_processed: 1,
    ///     total_errors: 0,
    /// };
    /// assert!(result.is_all_valid());
    /// ```
    pub fn is_all_valid(&self) -> bool {
        self.invalid_items.is_empty()
    }

    /// Compute the percentage of items that passed validation.
    ///
    /// Returns a value between `0.0` and `100.0` representing the share of processed
    /// items that are valid. If no items were processed, returns `0.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// let result = ValidationPipelineResult {
    ///     valid_items: vec![1, 2],
    ///     invalid_items: vec![],
    ///     total_processed: 2,
    ///     total_errors: 0,
    /// };
    /// assert_eq!(result.success_rate(), 100.0);
    /// ```
    pub fn success_rate(&self) -> f64 {
        if self.total_processed == 0 {
            0.0
        } else {
            (self.valid_items.len() as f64 / self.total_processed as f64) * 100.0
        }
    }

    /// Collects references to every `ValidationError` contained in the result's invalid items.
    ///
    /// Returns a vector of references to all errors from `invalid_items`, preserving iteration order.
    ///
    /// # Examples
    ///
    /// ```
    /// let result = ValidationPipelineResult {
    ///     valid_items: Vec::<i32>::new(),
    ///     invalid_items: vec![(1, Vec::<ValidationError>::new())],
    ///     total_processed: 1,
    ///     total_errors: 0,
    /// };
    /// assert!(result.all_errors().is_empty());
    /// ```
    pub fn all_errors(&self) -> Vec<&ValidationError> {
        self.invalid_items
            .iter()
            .flat_map(|(_, errors)| errors)
            .collect()
    }

    /// Group collected validation errors by their error code.
    ///
    /// Returns a map from error code to a list of references to `ValidationError`
    /// instances that share that code.
    ///
    /// # Examples
    ///
    /// ```
    /// // given a `result: ValidationPipelineResult<_>` with collected errors:
    /// let grouped = result.errors_by_code();
    /// // `grouped` maps error codes (String) to Vec<&ValidationError>
    /// assert!(grouped.is_empty() || grouped.values().next().is_some());
    /// ```
    pub fn errors_by_code(&self) -> HashMap<String, Vec<&ValidationError>> {
        let mut grouped = HashMap::new();

        for error in self.all_errors() {
            grouped
                .entry(error.code.clone())
                .or_insert_with(Vec::new)
                .push(error);
        }

        grouped
    }
}

/// Lazy validation iterator for processing large datasets
pub struct LazyValidationIterator<T, I>
where
    I: Iterator<Item = T>,
{
    iterator: I,
    validators: Vec<Box<dyn Fn(&T) -> ValidationResult<()>>>,
}

impl<T, I> LazyValidationIterator<T, I>
where
    I: Iterator<Item = T>,
{
    /// Creates a LazyValidationIterator that wraps the given iterator and starts with no validators.
    ///
    /// # Examples
    ///
    /// ```
    /// let it = vec![1, 2, 3].into_iter();
    /// let mut lazy = LazyValidationIterator::new(it);
    /// // Add a no-op validator that always succeeds
    /// lazy = lazy.add_validator(|_: &i32| Ok(()));
    /// let results: Vec<_> = lazy.collect();
    /// assert_eq!(results.len(), 3);
    /// ```
    pub fn new(iterator: I) -> Self {
        Self {
            iterator,
            validators: Vec::new(),
        }
    }

    /// Add a validator to the pipeline and return the pipeline for chaining.
    ///
    /// The provided validator is applied to each item when the pipeline is executed. The validator
    /// receives a reference to an item of type `T` and must return a `ValidationResult<()>`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let data = vec!["a", "b", "c"];
    /// let pipeline = ValidationPipeline::new(data.into_iter())
    ///     .add_validator(|item: &str| {
    ///         // return Ok(()) for valid items or Err(...) with validation errors
    ///         Ok(())
    ///     });
    /// ```
    pub fn add_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&T) -> ValidationResult<()> + 'static,
    {
        self.validators.push(Box::new(validator));
        self
    }
}

impl<T, I> Iterator for LazyValidationIterator<T, I>
where
    I: Iterator<Item = T>,
{
    type Item = ValidationOutcome<T>;

    /// Advances the underlying iterator, validates the next item with all configured validators, and yields a `ValidationOutcome` describing success or collected errors.
    ///
    /// The method returns `None` when the underlying iterator is exhausted.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3];
    /// let mut iter = LazyValidationIterator::new(data.into_iter())
    ///     .add_validator(|_: &i32| Ok(()));
    ///
    /// let first = iter.next().unwrap();
    /// assert!(first.is_valid);
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|item| {
            let mut errors = Vec::new();

            for validator in &self.validators {
                if let Err(error) = validator(&item) {
                    errors.push(error);
                }
            }

            if errors.is_empty() {
                ValidationOutcome::success(item)
            } else {
                ValidationOutcome::failure(errors)
            }
        })
    }
}

/// Creates a default ValidationEngine for type `T`.

///

/// Returns a `ValidationEngine<T>` configured with the library's default `ValidationConfig`.

///

/// # Examples

///

/// ```

/// /// let engine = validator::<i32>();

/// // default configuration applies (fail_fast = true, max_errors = Some(10), parallel_validation = false)

/// assert!(engine.config.fail_fast);

/// ```
pub fn validator<T>() -> ValidationEngine<T> {
    ValidationEngine::new()
}

/// Creates a ValidationEngine configured with the given ValidationConfig.
///
/// # Examples
///
/// ```
/// /// let config = ValidationConfig { fail_fast: false, max_errors: Some(5), parallel_validation: false };
/// /// let engine: ValidationEngine<String> = validator_with_config(config);
/// ```
pub fn validator_with_config<T>(config: ValidationConfig) -> ValidationEngine<T> {
    ValidationEngine::with_config(config)
}

/// Validate a single struct field using the provided validation rules.
///
/// # Returns
///
/// A `ValidationOutcome` that contains the original field reference when validation succeeds, or the collected `ValidationError` entries when validation fails.
///
/// # Examples
///
/// ```
/// // Assume `MyType` and a set of rules implementing `ValidationRule<MyType>` exist.
/// // This demonstrates calling `validate_struct_field` with an engine and rules.
/// /// let engine = validator::<MyType>();
/// let value = MyType::default();
/// let rules = vec![]; // populate with rules implementing `ValidationRule<MyType>`
/// /// let outcome = validate_struct_field(&engine, &value, "my_field", rules);
/// // `outcome` carries the validated reference on success or errors on failure.
/// ```
pub fn validate_struct_field<'a, T, R>(
    engine: &ValidationEngine<T>,
    value: &'a T,
    field_name: &str,
    rules: Vec<R>,
) -> ValidationOutcome<&'a T>
where
    R: ValidationRule<T>,
{
    engine.validate_field(value, field_name, rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functional::validation_rules::{Email, Required};
    use std::collections::HashMap;

    // Tests using concrete types for validation rules

    #[test]
    fn test_single_field_validation_success() {
        let engine = ValidationEngine::new();
        let value = "test".to_string();
        let rules = vec![Required];

        let result = engine.validate_field(&value, "field", rules);
        assert!(result.is_valid);
    }

    /// Verifies that validating an empty string with the `Required` rule produces a failure.
    ///
    /// This test constructs a default `ValidationEngine`, validates an empty `String` for the field named `"field"` using the `Required` rule, and asserts the outcome is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = ValidationEngine::new();
    /// let value = "".to_string();
    /// let rules = vec![Required];
    /// let result = engine.validate_field(&value, "field", rules);
    /// assert!(!result.is_valid);
    /// ```
    #[test]
    fn test_single_field_validation_failure() {
        let engine = ValidationEngine::new();
        let value = "".to_string();
        let rules = vec![Required];

        let result = engine.validate_field(&value, "field", rules);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_multiple_field_validation() {
        let engine = ValidationEngine::new();
        let value = "test@example.com".to_string();
        // Test each rule separately since they are different types

        let result1 = engine.validate_field(&value, "email", vec![Required]);
        assert!(result1.is_valid);

        let result2 = engine.validate_field(&value, "email", vec![Email]);
        assert!(result2.is_valid);
    }

    #[test]
    fn test_validation_pipeline() {
        let data = vec![
            "valid@example.com".to_string(),
            "invalid-email".to_string(),
            "another@valid.com".to_string(),
        ];

        let pipeline = ValidationPipeline::new(data.into_iter())
            .add_validator(|email: &String| Email.validate(email, "email"));

        let result = pipeline.validate();
        assert_eq!(result.valid_items.len(), 2);
        assert_eq!(result.invalid_items.len(), 1);
        assert_eq!(result.total_errors, 1);
    }

    /// Demonstrates validating items lazily with `LazyValidationIterator`, producing a `ValidationOutcome` per element.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec!["test".to_string(), "".to_string(), "another".to_string()];
    ///
    /// let lazy_iter = LazyValidationIterator::new(data.into_iter())
    ///     .add_validator(|s: &String| Required.validate(s, "field"));
    ///
    /// let results: Vec<_> = lazy_iter.collect();
    /// assert_eq!(results.len(), 3);
    /// assert!(results[0].is_valid);
    /// assert!(!results[1].is_valid);
    /// assert!(results[2].is_valid);
    /// ```
    #[test]
    fn test_lazy_validation_iterator() {
        let data = vec!["test".to_string(), "".to_string(), "another".to_string()];

        let lazy_iter = LazyValidationIterator::new(data.into_iter())
            .add_validator(|s: &String| Required.validate(s, "field"));

        let results: Vec<_> = lazy_iter.collect();
        assert_eq!(results.len(), 3);
        assert!(results[0].is_valid);
        assert!(!results[1].is_valid);
        assert!(results[2].is_valid);
    }

    // Cross-field validation tests

    #[test]
    fn test_require_field_if_present_valid() {
        let rule = require_field_if_present("password", "password_confirm");
        let mut data = HashMap::new();
        data.insert("password".to_string(), "secret123".to_string());
        data.insert("password_confirm".to_string(), "secret123".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_field_if_present_invalid_missing_required() {
        let rule = require_field_if_present("password", "password_confirm");
        let mut data = HashMap::new();
        data.insert("password".to_string(), "secret123".to_string());
        // password_confirm is missing

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, "MISSING_DEPENDENT_FIELD");
        assert!(error.message.contains("If password is present, password_confirm must also be present"));
    }

    #[test]
    fn test_require_field_if_present_valid_conditional_missing() {
        let rule = require_field_if_present("password", "password_confirm");
        let mut data = HashMap::new();
        // password is missing, so password_confirm doesn't need to be present
        data.insert("username".to_string(), "user".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mutually_exclusive_fields_valid_none_present() {
        let rule = mutually_exclusive_fields(vec!["email", "phone"]);
        let mut data = HashMap::new();
        data.insert("username".to_string(), "user".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mutually_exclusive_fields_valid_one_present() {
        let rule = mutually_exclusive_fields(vec!["email", "phone"]);
        let mut data = HashMap::new();
        data.insert("email".to_string(), "user@example.com".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mutually_exclusive_fields_invalid_both_present() {
        let rule = mutually_exclusive_fields(vec!["email", "phone"]);
        let mut data = HashMap::new();
        data.insert("email".to_string(), "user@example.com".to_string());
        data.insert("phone".to_string(), "123-456-7890".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, "MUTUALLY_EXCLUSIVE_FIELDS");
        assert!(error.message.contains("Multiple mutually exclusive fields are present"));
    }

    #[test]
    fn test_mutually_exclusive_fields_invalid_three_fields_both_present() {
        let rule = mutually_exclusive_fields(vec!["email", "phone", "sms"]);
        let mut data = HashMap::new();
        data.insert("email".to_string(), "user@example.com".to_string());
        data.insert("phone".to_string(), "123-456-7890".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
    }

    #[test]
    fn test_mutually_exclusive_fields_valid_empty_fields_list() {
        let rule = mutually_exclusive_fields(vec![]);
        let mut data = HashMap::new();
        data.insert("any_field".to_string(), "value".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_exactly_one_of_valid() {
        let rule = require_exactly_one_of(vec!["email", "phone"]);
        let mut data = HashMap::new();
        data.insert("email".to_string(), "user@example.com".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_exactly_one_of_invalid_none_present() {
        let rule = require_exactly_one_of(vec!["email", "phone"]);
        let mut data = HashMap::new();
        data.insert("username".to_string(), "user".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, "EXACTLY_ONE_FIELD_REQUIRED");
        assert!(error.message.contains("Exactly one of the specified fields must be present"));
    }

    #[test]
    fn test_require_exactly_one_of_invalid_both_present() {
        let rule = require_exactly_one_of(vec!["email", "phone"]);
        let mut data = HashMap::new();
        data.insert("email".to_string(), "user@example.com".to_string());
        data.insert("phone".to_string(), "123-456-7890".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, "EXACTLY_ONE_FIELD_REQUIRED");
    }

    #[test]
    fn test_require_exactly_one_of_valid_three_fields() {
        let rule = require_exactly_one_of(vec!["email", "phone", "sms"]);
        let mut data = HashMap::new();
        data.insert("phone".to_string(), "123-456-7890".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_all_or_none_valid_all_present() {
        let rule = require_all_or_none(vec!["street", "city", "zip_code"]);
        let mut data = HashMap::new();
        data.insert("street".to_string(), "123 Main St".to_string());
        data.insert("city".to_string(), "Anytown".to_string());
        data.insert("zip_code".to_string(), "12345".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_all_or_none_valid_none_present() {
        let rule = require_all_or_none(vec!["street", "city", "zip_code"]);
        let mut data = HashMap::new();
        data.insert("country".to_string(), "USA".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_all_or_none_invalid_partial_present() {
        let rule = require_all_or_none(vec!["street", "city", "zip_code"]);
        let mut data = HashMap::new();
        data.insert("street".to_string(), "123 Main St".to_string());
        data.insert("city".to_string(), "Anytown".to_string());
        // zip_code is missing

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, "ALL_OR_NONE_FIELDS");
        assert!(error.message.contains("Either all fields in the group must be present, or none"));
    }

    #[test]
    fn test_require_all_or_none_invalid_partial_present_two_fields() {
        let rule = require_all_or_none(vec!["street", "city", "zip_code"]);
        let mut data = HashMap::new();
        data.insert("street".to_string(), "123 Main St".to_string());
        // city and zip_code are missing

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
    }

    #[test]
    fn test_require_all_or_none_valid_empty_fields_list() {
        let rule = require_all_or_none(vec![]);
        let mut data = HashMap::new();
        data.insert("any_field".to_string(), "value".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compare_fields_valid_string_equality() {
        let rule = compare_fields("password", "password_confirm", |a: &String, b: &String| a == b);
        let mut data = HashMap::new();
        data.insert("password".to_string(), "secret123".to_string());
        data.insert("password_confirm".to_string(), "secret123".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compare_fields_invalid_string_inequality() {
        let rule = compare_fields("password", "password_confirm", |a: &String, b: &String| a == b);
        let mut data = HashMap::new();
        data.insert("password".to_string(), "secret123".to_string());
        data.insert("password_confirm".to_string(), "different".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, "FIELD_COMPARISON_FAILED");
        assert!(error.message.contains("Field password does not satisfy comparison with password_confirm"));
    }

    #[test]
    fn test_compare_fields_valid_numeric_comparison() {
        let rule = compare_fields("start_date", "end_date", |start: &i64, end: &i64| start <= end);
        let mut data = HashMap::new();
        data.insert("start_date".to_string(), 1000i64);
        data.insert("end_date".to_string(), 2000i64);

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compare_fields_invalid_numeric_comparison() {
        let rule = compare_fields("start_date", "end_date", |start: &i64, end: &i64| start <= end);
        let mut data = HashMap::new();
        data.insert("start_date".to_string(), 2000i64);
        data.insert("end_date".to_string(), 1000i64);

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_err());
    }

    #[test]
    fn test_compare_fields_valid_missing_fields() {
        let rule = compare_fields("optional1", "optional2", |a: &String, b: &String| a == b);
        let mut data = HashMap::new();
        data.insert("other_field".to_string(), "value".to_string());
        // Both optional1 and optional2 are missing

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok()); // Missing fields should not cause comparison failure
    }

    #[test]
    fn test_compare_fields_valid_one_field_missing() {
        let rule = compare_fields("field1", "field2", |a: &String, b: &String| a == b);
        let mut data = HashMap::new();
        data.insert("field1".to_string(), "value".to_string());
        // field2 is missing

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok()); // Missing field should not cause comparison failure
    }

    #[test]
    fn test_compare_fields_edge_case_empty_strings() {
        let rule = compare_fields("field1", "field2", |a: &String, b: &String| a.len() == b.len());
        let mut data = HashMap::new();
        data.insert("field1".to_string(), "".to_string());
        data.insert("field2".to_string(), "".to_string());

        let result = rule.validate(&data, "cross_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cross_field_validation_composition() {
        // Test combining multiple cross-field rules
        let rule1 = require_field_if_present("has_address", "street");
        let rule2 = require_all_or_none(vec!["street", "city", "zip_code"]);

        let mut data = HashMap::new();
        data.insert("has_address".to_string(), "true".to_string());
        data.insert("street".to_string(), "123 Main St".to_string());
        data.insert("city".to_string(), "Anytown".to_string());
        data.insert("zip_code".to_string(), "12345".to_string());

        // Both rules should pass
        let result1 = rule1.validate(&data, "cross_field");
        assert!(result1.is_ok());

        let result2 = rule2.validate(&data, "cross_field");
        assert!(result2.is_ok());
    }

    #[test]
    fn test_cross_field_validation_composition_failure() {
        // Test combining multiple cross-field rules with failure
        let rule1 = require_field_if_present("has_address", "street");
        let rule2 = require_all_or_none(vec!["street", "city", "zip_code"]);

        let mut data = HashMap::new();
        data.insert("has_address".to_string(), "true".to_string());
        data.insert("street".to_string(), "123 Main St".to_string());
        // city and zip_code are missing, violating rule2

        let result1 = rule1.validate(&data, "cross_field");
        assert!(result1.is_ok()); // rule1 passes because street is present

        let result2 = rule2.validate(&data, "cross_field");
        assert!(result2.is_err()); // rule2 fails because not all address fields are present
    }
}
