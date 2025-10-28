//! Mathematical functions for the Pure Function Registry
//!
//! This module provides common mathematical operations that can be registered
//! in the Pure Function Registry for functional composition.

use super::function_traits::{FunctionCategory, FunctionWrapper};
use super::pure_function_registry::{PureFunctionRegistry, RegistryError};

/// Creates a set of common mathematical functions and registers them
/// with the provided registry.
///
/// # Examples
///
/// ```
/// use crate::functional::pure_function_registry::PureFunctionRegistry;
/// use crate::functional::math_functions::register_math_functions;
///
/// let registry = PureFunctionRegistry::new();
/// register_math_functions(&registry).unwrap();
/// ```
pub fn register_math_functions(registry: &PureFunctionRegistry) -> Result<(), RegistryError> {
    // Addition function
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| a + b,
        "add_i32",
        FunctionCategory::Mathematical,
    ))?;

    // Subtraction function
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| a - b,
        "subtract_i32",
        FunctionCategory::Mathematical,
    ))?;

    // Multiplication function
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| a * b,
        "multiply_i32",
        FunctionCategory::Mathematical,
    ))?;

    // Division function (unsafe: panics on b == 0; also overflows on i32::MIN / -1)
    // In debug mode: panic on i32::MIN / -1 (overflow)
    // In release mode: wraps to i32::MIN (undefined behavior in C, but defined in Rust)
    // Recommendation: Use checked_div() or validate (a == i32::MIN && b == -1) before dividing
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| a / b,
        "divide_i32",
        FunctionCategory::Mathematical,
    ))?;

    // Safe division: returns None on b == 0 or overflow
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| a.checked_div(b),
        "safe_divide_i32",
        FunctionCategory::Mathematical,
    ))?;

    // Power function (unsafe for negative b and may overflow)
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| a.pow(b as u32),
        "power_i32",
        FunctionCategory::Mathematical,
    ))?;

    // Checked power: negative exponents => None; overflow => None
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| {
            if b < 0 { None } else { i32::checked_pow(a, b as u32) }
        },
        "power_i32_checked",
        FunctionCategory::Mathematical,
    ))?;

    // Square root function
    registry.register(FunctionWrapper::new(
        |a: f64| a.sqrt(),
        "sqrt_f64",
        FunctionCategory::Mathematical,
    ))?;

    // Absolute value function
    registry.register(FunctionWrapper::new(
        |a: i32| a.abs(),
        "abs_i32",
        FunctionCategory::Mathematical,
    ))?;

    // Modulo function (unsafe: panics on b == 0 or on overflow (i32::MIN % -1))
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| a % b,
        "modulo_i32",
        FunctionCategory::Mathematical,
    ))?;

    // Safe modulo: returns None on b == 0 or overflow (e.g., i32::MIN % -1)
    registry.register(FunctionWrapper::new(
        |(a, b): (i32, i32)| a.checked_rem(b),
        "safe_modulo_i32",
        FunctionCategory::Mathematical,
    ))?;

    Ok(())
}

/// Creates a set of common string processing functions and registers them
/// with the provided registry.
pub fn register_string_functions(registry: &PureFunctionRegistry) -> Result<(), RegistryError> {
    // String length function
    registry.register(FunctionWrapper::new(
        |s: String| s.len(),
        "string_length",
        FunctionCategory::StringProcessing,
    ))?;

    // String to uppercase
    registry.register(FunctionWrapper::new(
        |s: String| s.to_uppercase(),
        "to_uppercase",
        FunctionCategory::StringProcessing,
    ))?;

    // String to lowercase
    registry.register(FunctionWrapper::new(
        |s: String| s.to_lowercase(),
        "to_lowercase",
        FunctionCategory::StringProcessing,
    ))?;

    // String trim
    registry.register(FunctionWrapper::new(
        |s: String| s.trim().to_string(),
        "trim",
        FunctionCategory::StringProcessing,
    ))?;

    // String contains
    registry.register(FunctionWrapper::new(
        |(s, pattern): (String, String)| s.contains(&pattern),
        "contains",
        FunctionCategory::StringProcessing,
    ))?;

    Ok(())
}

/// Creates a set of common date/time processing functions and registers them
/// with the provided registry.
#[cfg(feature = "datetime")]
pub fn register_datetime_functions(registry: &PureFunctionRegistry) -> Result<(), RegistryError> {
    use chrono::{DateTime, Duration, Utc};

    // Current timestamp
    registry.register(FunctionWrapper::new(
        |_: ()| Utc::now(),
        "current_timestamp",
        FunctionCategory::DateTimeProcessing,
    ))?;

    // Add days to timestamp
    registry.register(FunctionWrapper::new(
        |(dt, days): (DateTime<Utc>, i64)| dt + Duration::days(days),
        "add_days",
        FunctionCategory::DateTimeProcessing,
    ))?;

    // Format timestamp
    registry.register(FunctionWrapper::new(
        |dt: DateTime<Utc>| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        "format_timestamp",
        FunctionCategory::DateTimeProcessing,
    ))?;

    Ok(())
}
