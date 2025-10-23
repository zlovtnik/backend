//! # Functional Programming Tests Module
//!
//! This module provides test utilities and integration points for the comprehensive
//! functional programming test suite. It serves as a bridge between the test
//! infrastructure and the actual functional programming components.

#[cfg(test)]
// Re-export test utilities for external use
// Commented out to avoid import issues with integration tests
// pub use crate::tests::functional_tests::*;

// Re-export test utilities for external use
pub use crate::functional::{iterator_engine::*, pure_function_registry::*, validation_engine::*};

/// Test configuration constants
pub const DEFAULT_TEST_TENANT: &str = "test_tenant";
pub const TEST_BATCH_SIZE: usize = 100;
pub const PERFORMANCE_THRESHOLD_MS: u64 = 50;

/// Utility function for creating test data
pub fn create_test_person(id: u32) -> TestPerson {
    TestPerson {
        id,
        name: format!("Test Person {}", id),
        email: format!("test{}@example.com", id),
        age: 20 + (id % 50),
        active: id % 3 == 0,
    }
}

/// Test data structure for functional programming tests
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TestPerson {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub age: u32,
    pub active: bool,
}

/// Helper function for performance testing
pub fn measure_execution_time<F, R>(f: F) -> (R, std::time::Duration)
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_person() {
        let person = create_test_person(1);
        assert_eq!(person.id, 1);
        assert_eq!(person.name, "Test Person 1");
        assert_eq!(person.email, "test1@example.com");
        assert_eq!(person.age, 21);
        assert!(!person.active); // 1 % 3 != 0
    }

    #[test]
    fn test_measure_execution_time() {
        let (result, duration) = measure_execution_time(|| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);
        assert!(duration >= std::time::Duration::from_millis(10));
    }
}
