//! Prelude for functional programming utilities
//!
//! This module re-exports commonly used types and functions from the functional
//! programming infrastructure to make them easily accessible.

pub use crate::function_traits::{FunctionCategory, FunctionWrapper};
pub use crate::math_functions::{register_math_functions, register_string_functions};
pub use crate::pure_function_registry::{PureFunctionRegistry, RegistryError, SharedRegistry};

/// Creates a shared PureFunctionRegistry populated with common pure functions
/// including mathematical, string, and transformation functions.
///
/// On success returns a SharedRegistry containing the pre-registered functions:
/// - Mathematical functions (add, subtract, multiply, etc.)
/// - String functions (length, uppercase, lowercase, etc.)
/// - Transformation functions (identity, double, etc.)
///
/// # Errors
///
/// Returns a RegistryError if any registration fails (for example, due to lock poisoning or a duplicate signature).
///
/// # Examples
///
///
pub fn create_enhanced_registry() -> Result<SharedRegistry, RegistryError> {
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

    // Register mathematical functions
    register_math_functions(&registry)?;

    // Register string functions
    register_string_functions(&registry)?;

    Ok(registry)
}
