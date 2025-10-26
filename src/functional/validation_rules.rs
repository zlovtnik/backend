//! Composable Validation Rules
//!
//! This module provides pure, composable validation functions that can be
//! combined using functional programming patterns. All validation rules
//! are pure functions that return Results for easy chaining and composition.

#![allow(dead_code)]

use chrono;
use once_cell::sync::Lazy;
use regex::Regex;
use rust_decimal;
use std::collections::HashSet;
use uuid;

/// Cached regex patterns for validation
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap());
static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[\d\s\-\(\)\+]{7,20}$").unwrap());

/// Validation result type for composable validation chains
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validation error with detailed information
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub code: String,
    pub message: String,
}

impl ValidationError {
    /// Creates a ValidationError with the provided field name, error code, and message.
    ///
    /// # Examples
    ///
    /// ```
    /// let err = ValidationError::new("email", "INVALID_EMAIL", "Email format is invalid");
    /// assert_eq!(err.field, "email");
    /// assert_eq!(err.code, "INVALID_EMAIL");
    /// assert_eq!(err.message, "Email format is invalid");
    /// ```
    pub fn new(field: &str, code: &str, message: &str) -> Self {
        Self {
            field: field.to_string(),
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

/// Core validation rule trait for composable validation
pub trait ValidationRule<T> {
    fn validate(&self, value: &T, field_name: &str) -> ValidationResult<()>;
}

/// Required field validation - ensures value is not empty/default
pub struct Required;

impl<T: Default + PartialEq> ValidationRule<T> for Required {
    /// Ensures the provided value is not equal to its type's default.
    ///
    /// If the value equals T::default(), validation fails and a `ValidationError` is returned
    /// with code `"REQUIRED"` and a message of the form `"<field_name> is required"`.
    ///
    /// # Parameters
    ///
    /// - `value`: the value to validate.
    /// - `field_name`: name used in the error's `field` and interpolated into the error message.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the value is not the default, `Err(ValidationError)` with code `"REQUIRED"` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let rule = Required;
    /// let val = String::from("hello");
    /// assert!(rule.validate(&val, "greeting").is_ok());
    ///
    /// let empty: String = String::default();
    /// let err = rule.validate(&empty, "greeting").unwrap_err();
    /// assert_eq!(err.code, "REQUIRED");
    /// assert_eq!(err.message, "greeting is required");
    /// ```
    fn validate(&self, value: &T, field_name: &str) -> ValidationResult<()> {
        if *value == T::default() {
            return Err(ValidationError::new(
                field_name,
                "REQUIRED",
                &format!("{} is required", field_name),
            ));
        }
        Ok(())
    }
}

/// String length validation
pub struct Length {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

impl ValidationRule<String> for Length {
    /// Validates that a string's length falls within the rule's optional minimum and maximum bounds.
    ///
    /// If `min` is set and the string has fewer than `min` characters, validation fails with code
    /// `TOO_SHORT`. If `max` is set and the string has more than `max` characters, validation fails
    /// with code `TOO_LONG`. Error messages include `field_name`.
    ///
    /// # Parameters
    ///
    /// - `field_name`: Name of the field used in the returned `ValidationError`'s `field` and message.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the string length satisfies the configured bounds, `Err(ValidationError)` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::{Length, ValidationRule};
    ///
    /// let rule = Length { min: Some(2), max: Some(4) };
    /// assert!(rule.validate(&"hi".to_string(), "name").is_ok());
    /// assert!(rule.validate(&"h".to_string(), "name").is_err()); // TOO_SHORT
    /// assert!(rule.validate(&"hello".to_string(), "name").is_err()); // TOO_LONG
    /// ```
    fn validate(&self, value: &String, field_name: &str) -> ValidationResult<()> {
        let len = value.len();

        if let Some(min) = self.min {
            if len < min {
                return Err(ValidationError::new(
                    field_name,
                    "TOO_SHORT",
                    &format!("{} must be at least {} characters", field_name, min),
                ));
            }
        }

        if let Some(max) = self.max {
            if len > max {
                return Err(ValidationError::new(
                    field_name,
                    "TOO_LONG",
                    &format!("{} must be at most {} characters", field_name, max),
                ));
            }
        }

        Ok(())
    }
}

/// Email format validation using regex
pub struct Email;

impl ValidationRule<String> for Email {
    /// Validates that a string is a well-formed email address using a simple pattern.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the value matches a simple email pattern; `Err(ValidationError)` with code `INVALID_EMAIL` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let rule = Email;
    /// assert!(rule.validate(&"user@example.com".to_string(), "email").is_ok());
    /// assert!(rule.validate(&"not-an-email".to_string(), "email").is_err());
    /// ```
    fn validate(&self, value: &String, field_name: &str) -> ValidationResult<()> {
        // Simple email regex - in production you might want a more comprehensive one
        if !EMAIL_REGEX.is_match(value) {
            return Err(ValidationError::new(
                field_name,
                "INVALID_EMAIL",
                &format!("{} must be a valid email address", field_name),
            ));
        }

        Ok(())
    }
}

/// Numeric range validation
pub struct Range {
    pub min: Option<i32>,
    pub max: Option<i32>,
}

impl ValidationRule<i32> for Range {
    /// Validates that an integer falls within the configured inclusive range.
    ///
    /// Returns `Ok(())` if `value` is greater than or equal to `min` (when `min` is set)
    /// and less than or equal to `max` (when `max` is set). Returns `Err(ValidationError)`
    /// with code `"TOO_SMALL"` when `value` is less than `min`, or `"TOO_LARGE"` when
    /// `value` is greater than `max`. The error message includes the `field_name` and the
    /// violated bound.
    ///
    /// # Examples
    ///
    /// ```
    /// let range = Range { min: Some(0), max: Some(10) };
    /// assert!(range.validate(&5, "count").is_ok());
    /// let err = range.validate(&-1, "count").unwrap_err();
    /// assert_eq!(err.code, "TOO_SMALL");
    /// ```
    fn validate(&self, value: &i32, field_name: &str) -> ValidationResult<()> {
        if let Some(min) = self.min {
            if *value < min {
                return Err(ValidationError::new(
                    field_name,
                    "TOO_SMALL",
                    &format!("{} must be at least {}", field_name, min),
                ));
            }
        }

        if let Some(max) = self.max {
            if *value > max {
                return Err(ValidationError::new(
                    field_name,
                    "TOO_LARGE",
                    &format!("{} must be at most {}", field_name, max),
                ));
            }
        }

        Ok(())
    }
}

/// Decimal range validation with optional precision checking
#[derive(Clone)]
pub struct DecimalRange {
    pub min: Option<rust_decimal::Decimal>,
    pub max: Option<rust_decimal::Decimal>,
    pub max_scale: Option<u32>, // Maximum decimal places
}

impl DecimalRange {
    /// Creates a new DecimalRange validator
    ///
    /// # Arguments
    /// * `min` - Optional minimum value (inclusive)
    /// * `max` - Optional maximum value (inclusive)
    /// * `max_scale` - Optional maximum decimal places
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_decimal::Decimal;
    /// let range = DecimalRange::new(
    ///     Some(Decimal::ZERO),
    ///     Some(Decimal::from(1000)),
    ///     Some(2)
    /// );
    /// ```
    pub fn new(
        min: Option<rust_decimal::Decimal>,
        max: Option<rust_decimal::Decimal>,
        max_scale: Option<u32>,
    ) -> Self {
        Self {
            min,
            max,
            max_scale,
        }
    }
}

impl ValidationRule<rust_decimal::Decimal> for DecimalRange {
    fn validate(&self, value: &rust_decimal::Decimal, field_name: &str) -> ValidationResult<()> {
        if let Some(min) = self.min {
            if *value < min {
                return Err(ValidationError::new(
                    field_name,
                    "DECIMAL_TOO_SMALL",
                    &format!("{} must be at least {}", field_name, min),
                ));
            }
        }

        if let Some(max) = self.max {
            if *value > max {
                return Err(ValidationError::new(
                    field_name,
                    "DECIMAL_TOO_LARGE",
                    &format!("{} must be at most {}", field_name, max),
                ));
            }
        }

        if let Some(max_scale) = self.max_scale {
            let scale = value.scale();
            if scale > max_scale {
                return Err(ValidationError::new(
                    field_name,
                    "DECIMAL_PRECISION_TOO_HIGH",
                    &format!(
                        "{} must have at most {} decimal places",
                        field_name, max_scale
                    ),
                ));
            }
        }

        Ok(())
    }
}

/// Validates that a decimal is positive (greater than zero)
pub struct PositiveDecimal;

impl ValidationRule<rust_decimal::Decimal> for PositiveDecimal {
    fn validate(&self, value: &rust_decimal::Decimal, field_name: &str) -> ValidationResult<()> {
        if *value <= rust_decimal::Decimal::ZERO {
            return Err(ValidationError::new(
                field_name,
                "DECIMAL_NOT_POSITIVE",
                &format!("{} must be positive", field_name),
            ));
        }
        Ok(())
    }
}

/// Validates that a decimal is non-negative (greater than or equal to zero)
pub struct NonNegativeDecimal;

impl ValidationRule<rust_decimal::Decimal> for NonNegativeDecimal {
    fn validate(&self, value: &rust_decimal::Decimal, field_name: &str) -> ValidationResult<()> {
        if *value < rust_decimal::Decimal::ZERO {
            return Err(ValidationError::new(
                field_name,
                "DECIMAL_NEGATIVE",
                &format!("{} must not be negative", field_name),
            ));
        }
        Ok(())
    }
}

/// Validates that a NaiveDateTime is in the past
pub struct PastDateTime;

impl ValidationRule<chrono::NaiveDateTime> for PastDateTime {
    fn validate(&self, value: &chrono::NaiveDateTime, field_name: &str) -> ValidationResult<()> {
        let now = chrono::Utc::now().naive_utc();
        if *value >= now {
            return Err(ValidationError::new(
                field_name,
                "DATETIME_NOT_PAST",
                &format!("{} must be in the past", field_name),
            ));
        }
        Ok(())
    }
}

/// Validates that a NaiveDateTime is in the future
pub struct FutureDateTime;

impl ValidationRule<chrono::NaiveDateTime> for FutureDateTime {
    fn validate(&self, value: &chrono::NaiveDateTime, field_name: &str) -> ValidationResult<()> {
        let now = chrono::Utc::now().naive_utc();
        if *value <= now {
            return Err(ValidationError::new(
                field_name,
                "DATETIME_NOT_FUTURE",
                &format!("{} must be in the future", field_name),
            ));
        }
        Ok(())
    }
}

/// Validates that a DateTime<Utc> is in the past
pub struct PastDateTimeUtc;

impl ValidationRule<chrono::DateTime<chrono::Utc>> for PastDateTimeUtc {
    fn validate(
        &self,
        value: &chrono::DateTime<chrono::Utc>,
        field_name: &str,
    ) -> ValidationResult<()> {
        let now = chrono::Utc::now();
        if *value >= now {
            return Err(ValidationError::new(
                field_name,
                "DATETIME_NOT_PAST",
                &format!("{} must be in the past", field_name),
            ));
        }
        Ok(())
    }
}

/// Validates that a DateTime<Utc> is in the future
pub struct FutureDateTimeUtc;

impl ValidationRule<chrono::DateTime<chrono::Utc>> for FutureDateTimeUtc {
    fn validate(
        &self,
        value: &chrono::DateTime<chrono::Utc>,
        field_name: &str,
    ) -> ValidationResult<()> {
        let now = chrono::Utc::now();
        if *value <= now {
            return Err(ValidationError::new(
                field_name,
                "DATETIME_NOT_FUTURE",
                &format!("{} must be in the future", field_name),
            ));
        }
        Ok(())
    }
}

/// Validates that a NaiveDateTime falls within a specified range
#[derive(Clone)]
pub struct DateTimeRange {
    pub min: Option<chrono::NaiveDateTime>,
    pub max: Option<chrono::NaiveDateTime>,
}

impl DateTimeRange {
    /// Creates a new DateTimeRange validator
    ///
    /// # Arguments
    /// * `min` - Optional minimum datetime (inclusive)
    /// * `max` - Optional maximum datetime (inclusive)
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
    /// let end = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap().and_hms_opt(23, 59, 59).unwrap();
    /// let range = DateTimeRange::new(Some(start), Some(end));
    /// ```
    pub fn new(min: Option<chrono::NaiveDateTime>, max: Option<chrono::NaiveDateTime>) -> Self {
        Self { min, max }
    }
}

impl ValidationRule<chrono::NaiveDateTime> for DateTimeRange {
    fn validate(&self, value: &chrono::NaiveDateTime, field_name: &str) -> ValidationResult<()> {
        if let Some(min) = self.min {
            if *value < min {
                return Err(ValidationError::new(
                    field_name,
                    "DATETIME_TOO_EARLY",
                    &format!("{} must be at or after {}", field_name, min),
                ));
            }
        }

        if let Some(max) = self.max {
            if *value > max {
                return Err(ValidationError::new(
                    field_name,
                    "DATETIME_TOO_LATE",
                    &format!("{} must be at or before {}", field_name, max),
                ));
            }
        }

        Ok(())
    }
}

/// Validates UUID format (any valid UUID)
pub struct UuidFormat;

impl ValidationRule<uuid::Uuid> for UuidFormat {
    fn validate(&self, _value: &uuid::Uuid, _field_name: &str) -> ValidationResult<()> {
        // uuid::Uuid is guaranteed to be valid by construction, so this always passes
        Ok(())
    }
}

/// Validates UUID version (v1, v3, v4, v5, etc.)
#[derive(Clone)]
pub struct UuidVersion {
    pub required_version: usize,
}

impl UuidVersion {
    /// Creates a new UuidVersion validator
    ///
    /// # Arguments
    /// * `version` - Required UUID version (1, 3, 4, 5, etc.)
    ///
    /// # Examples
    ///
    /// ```
    /// let v4_validator = UuidVersion::new(4); // Require UUID v4
    /// ```
    pub fn new(version: u8) -> Self {
        Self {
            required_version: version as usize,
        }
    }
}

impl ValidationRule<uuid::Uuid> for UuidVersion {
    fn validate(&self, value: &uuid::Uuid, field_name: &str) -> ValidationResult<()> {
        let actual_version = value.get_version_num() as usize;
        if actual_version != self.required_version {
            return Err(ValidationError::new(
                field_name,
                "UUID_WRONG_VERSION",
                &format!(
                    "{} must be UUID version {}, got version {}",
                    field_name, self.required_version, actual_version
                ),
            ));
        }
        Ok(())
    }
}

/// Validates that a UUID is not nil (all zeros)
pub struct UuidNotNil;

impl ValidationRule<uuid::Uuid> for UuidNotNil {
    fn validate(&self, value: &uuid::Uuid, field_name: &str) -> ValidationResult<()> {
        if value.is_nil() {
            return Err(ValidationError::new(
                field_name,
                "UUID_IS_NIL",
                &format!("{} cannot be a nil UUID", field_name),
            ));
        }
        Ok(())
    }
}

/// Validates UUID string format (when UUID is provided as string)
pub struct UuidString;

impl ValidationRule<String> for UuidString {
    fn validate(&self, value: &String, field_name: &str) -> ValidationResult<()> {
        match uuid::Uuid::parse_str(value) {
            Ok(_) => Ok(()),
            Err(_) => Err(ValidationError::new(
                field_name,
                "INVALID_UUID_FORMAT",
                &format!("{} must be a valid UUID format", field_name),
            )),
        }
    }
}

/// Phone number format validation (basic)
pub struct Phone;

impl ValidationRule<String> for Phone {
    /// Validates that a string is a phone number containing only digits, spaces, dashes, parentheses, or `+`, with length between 7 and 20 characters.
    ///
    /// Returns an `Err(ValidationError)` with code `"INVALID_PHONE"` when the value does not match the expected phone format.
    ///
    /// # Examples
    ///
    /// ```
    /// let phone = Phone;
    /// assert!(phone.validate(&"123-456-7890".to_string(), "contact_phone").is_ok());
    /// assert!(phone.validate(&"invalid_phone!".to_string(), "contact_phone").is_err());
    /// ```
    fn validate(&self, value: &String, field_name: &str) -> ValidationResult<()> {
        // Basic phone regex - allows digits, spaces, dashes, parentheses, plus
        if !PHONE_REGEX.is_match(value) {
            return Err(ValidationError::new(
                field_name,
                "INVALID_PHONE",
                &format!("{} must be a valid phone number", field_name),
            ));
        }

        Ok(())
    }
}

/// Custom validation using a predicate function
pub struct Custom<F> {
    predicate: F,
    error_code: String,
    error_message: String,
}

impl<F> Custom<F> {
    /// Creates a predicate-based custom validation rule.
    ///
    /// The `predicate` should return `true` when the value is considered valid. `error_code` and
    /// `error_message` are stored and used to construct a `ValidationError` when the predicate
    /// returns `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// let rule = Custom::new(|v: &i32| *v > 0, "TOO_SMALL", "must be greater than 0");
    /// assert!(rule.validate(&5, "age").is_ok());
    /// assert!(rule.validate(&0, "age").is_err());
    /// ```
    pub fn new(predicate: F, error_code: &str, error_message: &str) -> Self {
        Self {
            predicate,
            error_code: error_code.to_string(),
            error_message: error_message.to_string(),
        }
    }
}

impl<F, T> ValidationRule<T> for Custom<F>
where
    F: Fn(&T) -> bool,
{
    /// Validates a value with the rule's predicate and produces a ValidationError when the predicate fails.
    ///
    /// The `field_name` is interpolated into the rule's error message where `{}` appears.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the predicate returns `true`, `Err(ValidationError)` with the rule's code and interpolated message otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let rule = Custom::new(|s: &str| !s.is_empty(), "REQUIRED", "{} is required");
    /// assert!(rule.validate(&"value", "field").is_ok());
    /// let err = rule.validate(&"", "field").unwrap_err();
    /// assert!(err.message.contains("field"));
    /// ```
    fn validate(&self, value: &T, field_name: &str) -> ValidationResult<()> {
        if !(self.predicate)(value) {
            return Err(ValidationError::new(
                field_name,
                &self.error_code,
                &self.error_message.replace("{}", field_name),
            ));
        }
        Ok(())
    }
}

/// One-of validation for enums or allowed values
pub struct OneOf<T: Clone + PartialEq> {
    allowed_values: Vec<T>,
}

impl<T: Clone + PartialEq> OneOf<T> {
    /// Creates a `OneOf` validation rule that accepts only the provided allowed values.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::OneOf;
    ///
    /// let rule = OneOf::new(vec!["apple".to_string(), "banana".to_string()]);
    /// assert!(rule.validate(&"apple".to_string(), "fruit").is_ok());
    /// assert!(rule.validate(&"cherry".to_string(), "fruit").is_err());
    /// ```
    pub fn new(allowed_values: Vec<T>) -> Self {
        Self { allowed_values }
    }
}

impl<T: Clone + PartialEq> ValidationRule<T> for OneOf<T> {
    /// Validates that the provided value is contained in the rule's allowed values.
    ///
    /// Returns `Ok(())` if `value` is equal to one of the allowed values, `Err(ValidationError)` with code
    /// `"INVALID_VALUE"` and a message indicating the field otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::{OneOf, ValidationRule};
    ///
    /// let rule = OneOf::new(vec!["red".to_string(), "green".to_string()]);
    /// assert!(rule.validate(&"red".to_string(), "color").is_ok());
    /// assert!(rule.validate(&"blue".to_string(), "color").is_err());
    /// ```
    fn validate(&self, value: &T, field_name: &str) -> ValidationResult<()> {
        if !self.allowed_values.contains(value) {
            return Err(ValidationError::new(
                field_name,
                "INVALID_VALUE",
                &format!("{} must be one of the allowed values", field_name),
            ));
        }
        Ok(())
    }
}

/// URL format validation
pub struct Url;

impl ValidationRule<String> for Url {
    /// Validates that a string is a well-formed URL.
    ///
    /// On failure returns a `ValidationError` with code `"INVALID_URL"` and message
    /// `"<field_name> must be a valid URL"`.
    ///
    /// # Examples
    ///
    /// ```
    /// let rule = Url;
    /// let ok = rule.validate(&"https://example.com".to_string(), "website");
    /// assert!(ok.is_ok());
    ///
    /// let err = rule.validate(&"not-a-url".to_string(), "website");
    /// assert!(err.is_err());
    /// let e = err.unwrap_err();
    /// assert_eq!(e.code, "INVALID_URL");
    /// assert_eq!(e.message, "website must be a valid URL");
    /// ```
    fn validate(&self, value: &String, field_name: &str) -> ValidationResult<()> {
        if url::Url::parse(value).is_err() {
            return Err(ValidationError::new(
                field_name,
                "INVALID_URL",
                &format!("{} must be a valid URL", field_name),
            ));
        }
        Ok(())
    }
}

/// Boolean validation (must be true)
pub struct MustBeTrue;

impl ValidationRule<bool> for MustBeTrue {
    /// Ensures the boolean value is true.
    ///
    /// Returns `Ok(())` if `value` is `true`, `Err(ValidationError)` with code
    /// `"MUST_BE_TRUE"` and a message "<field_name> must be true" if `value` is `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// let rule = MustBeTrue;
    /// let ok = rule.validate(&true, "active");
    /// assert!(ok.is_ok());
    ///
    /// let err = rule.validate(&false, "active").unwrap_err();
    /// assert_eq!(err.code, "MUST_BE_TRUE");
    /// assert!(err.message.contains("active must be true"));
    /// ```
    fn validate(&self, value: &bool, field_name: &str) -> ValidationResult<()> {
        if !*value {
            return Err(ValidationError::new(
                field_name,
                "MUST_BE_TRUE",
                &format!("{} must be true", field_name),
            ));
        }
        Ok(())
    }
}

/// Creates a composite validation rule that requires every provided rule to succeed.
///
/// The returned rule applies all given rules to a value and fails if any single rule fails.
/// This is useful to combine multiple constraints using logical AND semantics.
///
/// # Examples
///
/// ```
/// let rule = all(vec![Required, Length { min: Some(2), max: Some(10) }]);
/// let ok = rule.validate(&"hello".to_string(), "name").is_ok();
/// assert!(ok);
/// let err = rule.validate(&"a".to_string(), "name").is_err();
/// assert!(err);
/// ```

/// Validator that succeeds only when all rules succeed
pub struct AllValidator<T, R: ValidationRule<T>> {
    rules: Vec<R>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, R: ValidationRule<T>> ValidationRule<T> for AllValidator<T, R> {
    /// Validates a value against every rule in the validator, returning on the first failure.
    ///
    /// Applies each contained rule in sequence; if any rule returns an error, that error is
    /// propagated immediately. If all rules succeed, validation succeeds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::validators::{all, Length, ValidationRule};
    /// let v = all(vec![Length { min: Some(3), max: Some(5) }]);
    /// let ok = v.validate(&"rust".to_string(), "username");
    /// assert!(ok.is_ok());
    /// let err = v.validate(&"hi".to_string(), "username");
    /// assert!(err.is_err());
    /// ```
    fn validate(&self, value: &T, field_name: &str) -> ValidationResult<()> {
        for rule in &self.rules {
            // Propagate the first error encountered
            rule.validate(value, field_name)?;
        }
        Ok(()) // All rules passed
    }
}

/// Constructs an AllValidator that applies every rule in `rules` in sequence.
///
/// The returned validator succeeds only if all contained rules succeed; it returns the first
/// encountered validation error otherwise.
///
/// # Examples
///
/// ```
/// // create an AllValidator for `i32` with no rules (always passes)
/// let _validator = crate::all::<i32, _>(vec![]);
/// ```
pub fn all<T, R: ValidationRule<T>>(rules: Vec<R>) -> AllValidator<T, R> {
    AllValidator {
        rules,
        _phantom: std::marker::PhantomData,
    }
}

/// Creates a composite validation rule that passes when at least one of the provided rules succeeds.
///
/// The returned rule validates the value against each rule in `rules` and succeeds if any rule returns `Ok(())`; if none pass it produces a `ValidationError` with code `"VALIDATION_FAILED"`.
///
/// # Examples
///
/// ```
/// let r1 = crate::Custom::new(|s: &String| s.contains('a'), "HAS_A", "must contain an 'a'");
/// let r2 = crate::Custom::new(|s: &String| s.contains('b'), "HAS_B", "must contain a 'b'");
/// let rule = crate::any(vec![r1, r2]);
///
/// assert!(rule.validate(&"apple".to_string(), "field").is_ok()); // contains 'a'
/// assert!(rule.validate(&"cherry".to_string(), "field").is_ok()); // contains 'b'
/// assert!(rule.validate(&"zzz".to_string(), "field").is_err());   // contains neither
/// ```

/// Validator that succeeds when at least one rule succeeds
pub struct AnyValidator<T, R: ValidationRule<T>> {
    rules: Vec<R>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, R: ValidationRule<T>> ValidationRule<T> for AnyValidator<T, R> {
    /// Validates a value against a set of rules and succeeds if any single rule passes.
    ///
    /// If no rules are provided, returns an error with code `VALIDATION_FAILED` and message
    /// "No validation rules provided". If all rules fail, returns an error with code
    /// `ANY_VALIDATION_FAILED` and a message that combines each rule's failure message.
    ///
    /// # Examples
    ///
    /// ```
    /// let non_empty = Custom::new(|s: &String| !s.is_empty(), "REQUIRED", "{} is required");
    /// let equals_ok = Custom::new(|s: &String| s == "ok", "MUST_BE_OK", "{} must be \"ok\"");
    /// let validator = any(vec![non_empty, equals_ok]);
    ///
    /// assert!(validator.validate(&"hello".to_string(), "field").is_ok());
    /// assert!(validator.validate(&"ok".to_string(), "field").is_ok());
    /// let err = validator.validate(&"".to_string(), "field").unwrap_err();
    /// assert_eq!(err.code, "ANY_VALIDATION_FAILED");
    /// ```
    fn validate(&self, value: &T, field_name: &str) -> ValidationResult<()> {
        let mut collected_errors = Vec::new();

        for rule in &self.rules {
            match rule.validate(value, field_name) {
                Ok(()) => return Ok(()), // Return immediately if any rule succeeds
                Err(error) => collected_errors.push(error),
            }
        }

        // All rules failed - return combined error
        if collected_errors.is_empty() {
            Err(ValidationError::new(
                field_name,
                "NO_RULES_PROVIDED",
                "No validation rules provided",
            ))
        } else {
            let combined_message = collected_errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            Err(ValidationError::new(
                field_name,
                "ANY_VALIDATION_FAILED",
                &format!("All validation rules failed: {}", combined_message),
            ))
        }
    }
}

/// Creates an AnyValidator that succeeds if at least one of the provided rules passes.
///
/// # Examples
///
/// ```
/// // Helper types for the example
/// struct AlwaysFail;
/// struct AlwaysPass;
///
/// impl crate::ValidationRule<i32> for AlwaysFail {
///     fn validate(&self, _value: &i32, _field_name: &str) -> crate::ValidationResult<()> {
///         Err(crate::ValidationError::new("x", "FAILED", "always fails"))
///     }
/// }
///
/// impl crate::ValidationRule<i32> for AlwaysPass {
///     fn validate(&self, _value: &i32, _field_name: &str) -> crate::ValidationResult<()> {
///         Ok(())
///     }
/// }
///
/// let v = crate::any(vec![AlwaysFail, AlwaysPass]);
/// assert!(v.validate(&42, "x").is_ok());
/// ```
pub fn any<T, R: ValidationRule<T>>(rules: Vec<R>) -> AnyValidator<T, R> {
    AnyValidator {
        rules,
        _phantom: std::marker::PhantomData,
    }
}

/// Creates a composite validation rule that succeeds only when the provided rule fails.
///
/// The returned rule validates a value by applying `rule` and interpreting a failure from `rule` as success; if `rule` succeeds the composite fails with error code `VALIDATION_FAILED` and message "Validation rule should have failed but passed".
///
/// # Examples
///
/// ```
/// # use your_crate::{Required, ValidationRule, ValidationResult};
/// let negated = not(Required);
/// // Required fails for the default String (empty), so negated succeeds
/// assert!(negated.validate(&String::new(), "name").is_ok());
/// // Required succeeds for non-empty, so negated fails
/// assert!(negated.validate(&"ok".to_string(), "name").is_err());
/// ```
/// Validator that succeeds when the inner rule fails
pub struct NotValidator<T, R: ValidationRule<T>> {
    rule: R,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, R: ValidationRule<T>> ValidationRule<T> for NotValidator<T, R> {
    fn validate(&self, value: &T, field_name: &str) -> ValidationResult<()> {
        match self.rule.validate(value, field_name) {
            Ok(()) => Err(ValidationError::new(
                field_name,
                "VALIDATION_FAILED",
                "Validation rule should have failed but passed",
            )),
            Err(_) => Ok(()),
        }
    }
}

pub fn not<T, R: ValidationRule<T>>(rule: R) -> NotValidator<T, R> {
    NotValidator {
        rule,
        _phantom: std::marker::PhantomData,
    }
}

/// Applies `rule` only when `condition` returns true for the value.
///
/// If the condition returns false the validation is skipped and treated as successful; if it
/// returns true the inner rule is applied and its result is returned. The produced value
/// implements `ValidationRule<T>`.
///
/// # Examples
///
/// ```
/// // Use a custom inner rule to avoid depending on other concrete rules in this example.
/// let inner = Custom::new(|v: &i32| *v >= 1 && *v <= 10, "OUT_OF_RANGE", "Value out of range");
/// let conditional = when(|v: &i32| *v != 0, inner);
///
/// assert!(conditional.validate(&5, "n").is_ok());   // condition true, inner rule passes
/// assert!(conditional.validate(&0, "n").is_ok());   // condition false, validation skipped
/// assert!(conditional.validate(&20, "n").is_err()); // condition true, inner rule fails
/// ```
/// Validator that applies a rule conditionally based on a predicate
pub struct WhenValidator<T, C, R>
where
    C: Fn(&T) -> bool,
    R: ValidationRule<T>,
{
    condition: C,
    rule: R,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, C, R> ValidationRule<T> for WhenValidator<T, C, R>
where
    C: Fn(&T) -> bool,
    R: ValidationRule<T>,
{
    fn validate(&self, value: &T, field_name: &str) -> ValidationResult<()> {
        if (self.condition)(value) {
            self.rule.validate(value, field_name)
        } else {
            Ok(()) // Skip validation if condition not met
        }
    }
}

pub fn when<T, C, R>(condition: C, rule: R) -> WhenValidator<T, C, R>
where
    C: Fn(&T) -> bool,
    R: ValidationRule<T>,
{
    WhenValidator {
        condition,
        rule,
        _phantom: std::marker::PhantomData,
    }
}

/// Validates that a collection has a minimum number of items
#[derive(Clone)]
pub struct MinItems {
    pub min: usize,
}

impl MinItems {
    pub fn new(min: usize) -> Self {
        Self { min }
    }
}

impl<T> ValidationRule<Vec<T>> for MinItems {
    fn validate(&self, value: &Vec<T>, field_name: &str) -> ValidationResult<()> {
        if value.len() < self.min {
            return Err(ValidationError::new(
                field_name,
                "TOO_FEW_ITEMS",
                &format!("{} must have at least {} items", field_name, self.min),
            ));
        }
        Ok(())
    }
}

/// Validates that a collection has a maximum number of items
#[derive(Clone)]
pub struct MaxItems {
    pub max: usize,
}

impl MaxItems {
    pub fn new(max: usize) -> Self {
        Self { max }
    }
}

impl<T> ValidationRule<Vec<T>> for MaxItems {
    fn validate(&self, value: &Vec<T>, field_name: &str) -> ValidationResult<()> {
        if value.len() > self.max {
            return Err(ValidationError::new(
                field_name,
                "TOO_MANY_ITEMS",
                &format!("{} must have at most {} items", field_name, self.max),
            ));
        }
        Ok(())
    }
}

/// Validates that all items in a collection are unique
pub struct UniqueItems;

impl<T: Eq + std::hash::Hash> ValidationRule<Vec<T>> for UniqueItems {
    fn validate(&self, value: &Vec<T>, field_name: &str) -> ValidationResult<()> {
        let mut seen = HashSet::new();
        for item in value {
            if !seen.insert(item) {
                return Err(ValidationError::new(
                    field_name,
                    "DUPLICATE_ITEMS",
                    &format!("{} contains duplicate items", field_name),
                ));
            }
        }
        Ok(())
    }
}

/// Validates that a collection contains only unique items (non-cloning implementation)
///
/// This function returns a `UniqueItems` validator, which is preferred over the deprecated
/// `Unique` struct because it does not require items to implement `Clone`. The older
/// `Unique` struct is deprecated due to its cloning behavior.
///
/// # Examples
///
/// ```
/// let validator = unique();
/// let ok = validator.validate(&vec![1, 2, 3], "numbers");
/// assert!(ok.is_ok());
///
/// let err = validator.validate(&vec![1, 2, 2], "numbers");
/// assert!(err.is_err());
/// ```
pub fn unique<T: Eq + std::hash::Hash>() -> UniqueItems {
    UniqueItems
}

/// Validates that a collection has items within a specified count range
#[derive(Clone)]
pub struct ItemsRange {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

impl ItemsRange {
    pub fn new(min: Option<usize>, max: Option<usize>) -> Self {
        Self { min, max }
    }
}

impl<T> ValidationRule<Vec<T>> for ItemsRange {
    fn validate(&self, value: &Vec<T>, field_name: &str) -> ValidationResult<()> {
        let len = value.len();

        if let Some(min) = self.min {
            if len < min {
                return Err(ValidationError::new(
                    field_name,
                    "TOO_FEW_ITEMS",
                    &format!("{} must have at least {} items", field_name, min),
                ));
            }
        }

        if let Some(max) = self.max {
            if len > max {
                return Err(ValidationError::new(
                    field_name,
                    "TOO_MANY_ITEMS",
                    &format!("{} must have at most {} items", field_name, max),
                ));
            }
        }

        Ok(())
    }
}

/// Validates that each item in a collection passes a given validation rule
pub struct ItemsValidator<R> {
    rule: R,
}

impl<R> ItemsValidator<R> {
    pub fn new(rule: R) -> Self {
        Self { rule }
    }
}

impl<T, R: ValidationRule<T>> ValidationRule<Vec<T>> for ItemsValidator<R> {
    fn validate(&self, value: &Vec<T>, field_name: &str) -> ValidationResult<()> {
        for (index, item) in value.iter().enumerate() {
            let item_field_name = format!("{}[{}]", field_name, index);
            self.rule.validate(item, &item_field_name)?;
        }
        Ok(())
    }
}

/// Creates a validator that applies a rule to each item in a collection
pub fn items<R>(rule: R) -> ItemsValidator<R> {
    ItemsValidator::new(rule)
}

/// Validates that a collection is not empty
pub struct NotEmpty;

impl<T> ValidationRule<Vec<T>> for NotEmpty {
    fn validate(&self, value: &Vec<T>, field_name: &str) -> ValidationResult<()> {
        if value.is_empty() {
            return Err(ValidationError::new(
                field_name,
                "EMPTY_COLLECTION",
                &format!("{} must not be empty", field_name),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct PassingRule;

    impl ValidationRule<i32> for PassingRule {
        fn validate(&self, _value: &i32, _field_name: &str) -> ValidationResult<()> {
            Ok(())
        }
    }

    struct FailingRule;

    impl ValidationRule<i32> for FailingRule {
        fn validate(&self, _value: &i32, field_name: &str) -> ValidationResult<()> {
            Err(ValidationError::new(
                field_name,
                "INNER_RULE_FAILED",
                &format!("{} failed validation", field_name),
            ))
        }
    }

    struct SpyRule {
        called: Rc<RefCell<bool>>,
        delegate: Box<dyn ValidationRule<i32>>,
    }

    impl SpyRule {
        fn new(delegate: Box<dyn ValidationRule<i32>>, called: Rc<RefCell<bool>>) -> Self {
            Self { called, delegate }
        }
    }

    impl ValidationRule<i32> for SpyRule {
        fn validate(&self, value: &i32, field_name: &str) -> ValidationResult<()> {
            *self.called.borrow_mut() = true;
            self.delegate.validate(value, field_name)
        }
    }

    #[test]
    fn test_required_string() {
        let rule = Required;
        assert!(rule.validate(&"test".to_string(), "name").is_ok());
        assert!(rule.validate(&"".to_string(), "name").is_err());
    }

    #[test]
    fn test_email_validation() {
        let rule = Email;
        assert!(rule
            .validate(&"test@example.com".to_string(), "email")
            .is_ok());
        assert!(rule
            .validate(&"invalid-email".to_string(), "email")
            .is_err());
    }

    #[test]
    fn test_length_validation() {
        let rule = Length {
            min: Some(2),
            max: Some(10),
        };
        assert!(rule.validate(&"test".to_string(), "name").is_ok());
        assert!(rule.validate(&"t".to_string(), "name").is_err());
        assert!(rule
            .validate(&"this_is_too_long".to_string(), "name")
            .is_err());
    }

    #[test]
    fn test_range_validation() {
        let rule = Range {
            min: Some(18),
            max: Some(65),
        };
        assert!(rule.validate(&25, "age").is_ok());
        assert!(rule.validate(&17, "age").is_err());
        assert!(rule.validate(&70, "age").is_err());
    }

    #[test]
    fn test_composition_all() {
        let rules: Vec<Length> = vec![
            Length {
                min: Some(2),
                max: None,
            },
            Length {
                min: None,
                max: Some(10),
            },
        ];
        let composed = all(rules);
        assert!(composed.validate(&"test".to_string(), "name").is_ok());
        assert!(composed.validate(&"t".to_string(), "name").is_err());
    }

    #[test]
    fn not_validator_fails_when_inner_rule_succeeds() {
        let inner = Custom::new(|value: &i32| *value == 5, "MATCH", "{} must equal five");
        let negated = not(inner);

        let error = negated
            .validate(&5, "number")
            .expect_err("negated rule should fail");

        assert_eq!(error.field, "number");
        assert_eq!(error.code, "VALIDATION_FAILED");
        assert_eq!(
            error.message,
            "Validation rule should have failed but passed"
        );
    }

    #[test]
    fn not_validator_succeeds_when_inner_rule_fails() {
        let inner = Custom::new(
            |value: &i32| *value > 10,
            "TOO_SMALL",
            "{} must be greater than 10",
        );
        let negated = not(inner);

        assert!(negated.validate(&5, "threshold").is_ok());
    }

    #[test]
    fn when_validator_invokes_rule_when_condition_true_and_passes() {
        let called = Rc::new(RefCell::new(false));
        let spy = SpyRule::new(Box::new(PassingRule), Rc::clone(&called));
        let validator = when(|value: &i32| *value > 0, spy);

        assert!(validator.validate(&5, "positive").is_ok());
        assert!(*called.borrow());
    }

    #[test]
    fn when_validator_returns_error_when_condition_true_and_rule_fails() {
        let called = Rc::new(RefCell::new(false));
        let spy = SpyRule::new(Box::new(FailingRule), Rc::clone(&called));
        let validator = when(|value: &i32| *value > 0, spy);

        let error = validator
            .validate(&5, "number")
            .expect_err("expected inner rule failure");

        assert!(*called.borrow());
        assert_eq!(error.field, "number");
        assert_eq!(error.code, "INNER_RULE_FAILED");
        assert_eq!(error.message, "number failed validation");
    }

    #[test]
    fn when_validator_skips_rule_when_condition_false() {
        let called = Rc::new(RefCell::new(false));
        let spy = SpyRule::new(Box::new(FailingRule), Rc::clone(&called));
        let validator = when(|value: &i32| *value > 10, spy);

        assert!(validator.validate(&5, "number").is_ok());
        assert!(!*called.borrow());
    }
}
