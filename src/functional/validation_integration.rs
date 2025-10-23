//! Integration example showing how to use the Iterator-Based Validation Engine
//! with existing Actix Web request processing and models.

#![allow(dead_code)]

use crate::functional::validation_engine::{ValidationEngine, ValidationOutcome};
use crate::functional::validation_rules::{Email, Length, Phone, Range, Required};
use crate::models::person::PersonDTO;

/// Validate a PersonDTO by applying field-specific rules and aggregating all validation errors.
///
/// Runs required and format/constraint validations for each field (name, email, age, address, phone)
/// and returns a single outcome containing all discovered errors.
///
/// Returns `ValidationOutcome::success(())` when no field errors are present, or
/// `ValidationOutcome::failure` with a vector of all collected `ValidationError`s otherwise.
///
/// # Examples
///
/// ```
/// # use crate::models::person::PersonDTO;
/// # use crate::functional::validation_integration::validate_person_dto;
/// let person = PersonDTO {
///     name: "Alice".into(),
///     gender: true,
///     age: 30,
///     address: "123 Main St".into(),
///     phone: "+1234567890".into(),
///     email: "alice@example.com".into(),
/// };
///
/// /// let outcome = validate_person_dto(&person);
/// assert!(outcome.is_valid);
/// ```
pub fn validate_person_dto(person: &PersonDTO) -> ValidationOutcome<()> {
    let engine = ValidationEngine::new();

    // Validate each field individually with appropriate types
    let name_result = engine.validate_field(&person.name, "name", vec![Required]);
    let name_length_result = engine.validate_field(
        &person.name,
        "name",
        vec![Length {
            min: Some(1),
            max: Some(100),
        }],
    );

    let email_result = engine.validate_field(&person.email, "email", vec![Required]);
    let email_format_result = engine.validate_field(&person.email, "email", vec![Email]);

    // Age validation requires a separate engine for i32 type
    let age_engine = ValidationEngine::<i32>::new();
    let age_result = age_engine.validate_field(
        &person.age,
        "age",
        vec![Range {
            min: Some(0),
            max: Some(150),
        }],
    );

    let address_result = engine.validate_field(&person.address, "address", vec![Required]);
    let address_length_result = engine.validate_field(
        &person.address,
        "address",
        vec![Length {
            min: Some(5),
            max: Some(200),
        }],
    );

    let phone_result = engine.validate_field(&person.phone, "phone", vec![Phone]);

    // Collect all errors
    let mut all_errors = Vec::new();
    all_errors.extend(name_result.errors);
    all_errors.extend(name_length_result.errors);
    all_errors.extend(email_result.errors);
    all_errors.extend(email_format_result.errors);
    all_errors.extend(age_result.errors);
    all_errors.extend(address_result.errors);
    all_errors.extend(address_length_result.errors);
    all_errors.extend(phone_result.errors);

    if all_errors.is_empty() {
        ValidationOutcome::success(())
    } else {
        ValidationOutcome::failure(all_errors)
    }
}

/// Validates a PersonDTO with composite rules: name presence/length and a required valid contact (email or phone).
///
/// Performs the following checks:
/// - `name` is present and its length is between 1 and 100 characters.
/// - At least one valid contact is provided: a non-empty, valid `email` or a non-empty, valid `phone`.
///
/// On failure, returns a `ValidationOutcome::failure` containing the collected validation errors; on success, returns `ValidationOutcome::success(())`.
///
/// # Examples
///
/// ```
/// # use crate::models::person::PersonDTO;
/// # use crate::functional::validation_integration::validate_person_with_complex_rules;
/// let person = PersonDTO {
///     name: "Alice".into(),
///     gender: false,
///     age: 30,
///     address: "123 Main St".into(),
///     phone: "".into(),
///     email: "alice@example.com".into(),
/// };
///
/// /// let outcome = validate_person_with_complex_rules(&person);
/// assert!(outcome.is_valid);
/// ```
pub fn validate_person_with_complex_rules(person: &PersonDTO) -> ValidationOutcome<()> {
    let engine = ValidationEngine::new();

    // Complex validation: name must be present AND either email OR phone must be valid
    let name_validation = engine.validate_field(&person.name, "name", vec![Required]);
    let name_length_validation = engine.validate_field(
        &person.name,
        "name",
        vec![Length {
            min: Some(1),
            max: Some(100),
        }],
    );

    if !name_validation.is_valid || !name_length_validation.is_valid {
        let mut errors = name_validation.errors;
        errors.extend(name_length_validation.errors);
        return ValidationOutcome::failure(errors);
    }

    // Either email or phone must be provided and valid
    let email_validation = engine.validate_field(&person.email, "email", vec![Email]);
    let phone_validation = engine.validate_field(&person.phone, "phone", vec![Phone]);

    let email_valid = !person.email.is_empty() && email_validation.is_valid;
    let phone_valid = !person.phone.is_empty() && phone_validation.is_valid;

    if !email_valid && !phone_valid {
        return ValidationOutcome::failure(vec![
            crate::functional::validation_rules::ValidationError::new(
                "contact",
                "MISSING_CONTACT",
                "Either a valid email or phone number must be provided",
            ),
        ]);
    }

    ValidationOutcome::success(())
}

/// Validates a collection of PersonDTO values eagerly and produces a per-item `ValidationOutcome`.
///
/// All validations are executed immediately when this function is called. Each returned entry corresponds to
/// the matching input person, yielding `ValidationOutcome::success(())` for valid inputs or
/// `ValidationOutcome::failure(errors)` with the collected validation errors otherwise.
///
/// # Returns
///
/// A `Vec<ValidationOutcome<()>>` where each entry is `success(())` for a valid person or `failure(errors)` for an invalid one.
///
/// # Examples
///
/// ```
/// # use crate::models::person::PersonDTO;
/// # use crate::functional::validation_integration::validate_person_batch;
/// let people = vec![PersonDTO {
///     name: "Alice".into(),
///     gender: true,
///     age: 30,
///     address: "123 Main St".into(),
///     phone: "+1234567890".into(),
///     email: "alice@example.com".into(),
/// }];
/// let results = validate_person_batch(people);
/// assert_eq!(results.len(), 1);
/// ```
pub fn validate_person_batch(people: Vec<PersonDTO>) -> Vec<ValidationOutcome<()>> {
    people
        .into_iter()
        .map(|person| validate_person_dto(&person))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::person::PersonDTO;

    #[test]
    fn test_valid_person_dto() {
        let person = PersonDTO {
            name: "John Doe".to_string(),
            gender: true,
            age: 30,
            address: "123 Main St, City, State".to_string(),
            phone: "+1-555-0123".to_string(),
            email: "john.doe@example.com".to_string(),
        };

        let result = validate_person_dto(&person);
        assert!(result.is_valid);
    }

    #[test]
    fn test_invalid_person_dto_missing_required_fields() {
        let person = PersonDTO {
            name: "".to_string(), // Invalid: empty name
            gender: true,
            age: 30,
            address: "".to_string(), // Invalid: empty address
            phone: "invalid-phone".to_string(),
            email: "invalid-email".to_string(), // Invalid: not an email
        };

        let result = validate_person_dto(&person);
        assert!(!result.is_valid);
        assert!(result.errors.len() >= 3); // Should have multiple errors
    }

    #[test]
    fn test_complex_validation_rules() {
        // Valid: has email
        let person1 = PersonDTO {
            name: "John Doe".to_string(),
            gender: true,
            age: 30,
            address: "123 Main St".to_string(),
            phone: "".to_string(),
            email: "john@example.com".to_string(),
        };

        // Valid: has phone
        let person2 = PersonDTO {
            name: "Jane Doe".to_string(),
            gender: false,
            age: 25,
            address: "456 Oak Ave".to_string(),
            phone: "+1-555-0123".to_string(),
            email: "".to_string(),
        };

        // Invalid: no email or phone
        let person3 = PersonDTO {
            name: "Bob Smith".to_string(),
            gender: true,
            age: 40,
            address: "789 Pine St".to_string(),
            phone: "".to_string(),
            email: "".to_string(),
        };

        assert!(validate_person_with_complex_rules(&person1).is_valid);
        assert!(validate_person_with_complex_rules(&person2).is_valid);
        assert!(!validate_person_with_complex_rules(&person3).is_valid);
    }

    /// Verifies that validate_person_batch returns a valid outcome for a valid PersonDTO and an invalid outcome for an invalid PersonDTO.
    ///
    /// # Examples
    ///
    /// ```
    /// let people = vec![
    ///     PersonDTO {
    ///         name: "Valid Person".to_string(),
    ///         gender: true,
    ///         age: 30,
    ///         address: "123 Valid St".to_string(),
    ///         phone: "+1-555-0123".to_string(),
    ///         email: "valid@example.com".to_string(),
    ///     },
    ///     PersonDTO {
    ///         name: "".to_string(), // Invalid
    ///         gender: true,
    ///         age: 30,
    ///         address: "123 Valid St".to_string(),
    ///         phone: "+1-555-0123".to_string(),
    ///         email: "valid@example.com".to_string(),
    ///     },
    /// ];
    ///
    /// let results = validate_person_batch(people);
    /// assert_eq!(results.len(), 2);
    /// assert!(results[0].is_valid);
    /// assert!(!results[1].is_valid);
    /// ```
    #[test]
    fn test_batch_validation() {
        let people = vec![
            PersonDTO {
                name: "Valid Person".to_string(),
                gender: true,
                age: 30,
                address: "123 Valid St".to_string(),
                phone: "+1-555-0123".to_string(),
                email: "valid@example.com".to_string(),
            },
            PersonDTO {
                name: "".to_string(), // Invalid
                gender: true,
                age: 30,
                address: "123 Valid St".to_string(),
                phone: "+1-555-0123".to_string(),
                email: "valid@example.com".to_string(),
            },
        ];

        let results = validate_person_batch(people);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_valid);
        assert!(!results[1].is_valid);
    }
}
