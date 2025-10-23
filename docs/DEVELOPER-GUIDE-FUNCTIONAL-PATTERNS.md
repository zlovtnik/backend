# Developer Guide: Functional Programming Patterns

## Table of Contents

1. [Overview](#overview)
2. [Core Patterns](#core-patterns)
3. [Quick Start](#quick-start)
4. [Pattern Reference](#pattern-reference)
5. [Best Practices](#best-practices)
6. [Common Patterns](#common-patterns)
7. [Troubleshooting](#troubleshooting)

## Overview

This guide helps developers understand and use the functional programming patterns implemented in our service layer. All patterns are located in `src/services/functional_patterns.rs`.

### Why Functional Patterns?

- **Composability**: Build complex logic from simple, reusable pieces
- **Testability**: Each piece is a pure function that's easy to test
- **Type Safety**: Compiler enforces correct usage
- **Maintainability**: Clear separation of concerns

## Core Patterns

### 1. Validator Combinator

**Use Case**: Build complex validation from simple rules

```rust
use crate::services::functional_patterns::Validator;

// Create a validator with multiple rules
fn create_person_validator() -> Validator<PersonDTO> {
    Validator::new()
        .rule(|dto| {
            if dto.name.trim().is_empty() {
                Err(ServiceError::bad_request("Name is required"))
            } else {
                Ok(())
            }
        })
        .rule(|dto| {
            if dto.email.contains('@') {
                Ok(())
            } else {
                Err(ServiceError::bad_request("Invalid email"))
            }
        })
}

// Use the validator
let validator = create_person_validator();
validator.validate(&person_dto)?;
```

### 2. Pipeline Pattern

**Use Case**: Chain data transformations

```rust
use crate::services::functional_patterns::Pipeline;

// Create a transformation pipeline
let result = Pipeline::new(raw_data)
    .then(|data| normalize_data(data))
    .then(|data| validate_data(data))
    .then(|data| enrich_data(data))
    .execute()?;
```

### 3. Either Type

**Use Case**: Handle operations with two possible outcomes

```rust
use crate::services::functional_patterns::Either;

// Function that might return different types
fn process_data(input: &str) -> Either<String, i32> {
    if input.parse::<i32>().is_ok() {
        Either::Right(input.parse().unwrap())
    } else {
        Either::Left(format!("Invalid number: {}", input))
    }
}

// Handle both cases
match process_data("42") {
    Either::Left(error) => println!("Error: {}", error),
    Either::Right(number) => println!("Number: {}", number),
}
```

### 4. QueryReader Monad

**Use Case**: Compose database operations

```rust
use crate::services::functional_patterns::QueryReader;

// Chain queries without explicit connection passing
let result = QueryReader::new(|pool| {
    find_user_by_id(pool, user_id)
})
.and_then(|user| QueryReader::new(move |pool| {
    find_user_permissions(pool, user.id)
}))
.map(|permissions| {
    permissions.into_iter().map(|p| p.name).collect()
})
.run(&pool)?;
```

### 5. Memoization

**Use Case**: Cache expensive function results

```rust
use crate::services::functional_patterns::Memoized;

// Create a memoized function
let expensive_calc = Memoized::new(|x: i32| {
    // Expensive computation
    std::thread::sleep(std::time::Duration::from_secs(1));
    x * x
});

// First call: computed and cached
let result1 = expensive_calc.call(5); // Takes 1 second

// Second call: returned from cache
let result2 = expensive_calc.call(5); // Instant
```

### 6. Retry Pattern

**Use Case**: Handle transient failures

```rust
use crate::services::functional_patterns::Retry;

// Retry with exponential backoff
let result = Retry::new()
    .max_attempts(3)
    .delay(std::time::Duration::from_millis(100))
    .execute(|| {
        // Operation that might fail
        call_external_api()
    })?;
```

## Quick Start

### 1. Adding Validation to a Service

```rust
// In your service file (e.g., src/services/my_service.rs)
use crate::services::functional_patterns::Validator;

fn create_my_dto_validator() -> Validator<MyDTO> {
    Validator::new()
        .rule(|dto| {
            if dto.required_field.is_empty() {
                Err(ServiceError::bad_request("Required field missing"))
            } else {
                Ok(())
            }
        })
        .rule(|dto| {
            if dto.numeric_field < 0 {
                Err(ServiceError::bad_request("Must be positive"))
            } else {
                Ok(())
            }
        })
}

pub fn create_item(dto: MyDTO, pool: &Pool) -> Result<MyModel, ServiceError> {
    // Validate using functional validator
    create_my_dto_validator().validate(&dto)?;
    
    // Rest of your service logic...
    Ok(saved_item)
}
```

### 2. Adding Data Transformation

```rust
use crate::services::functional_patterns::Pipeline;

pub fn process_user_data(raw_data: UserInput) -> Result<User, ServiceError> {
    Pipeline::new(raw_data)
        .then(|input| sanitize_input(input))
        .then(|input| validate_business_rules(input))
        .then(|input| convert_to_model(input))
        .execute()
}

fn sanitize_input(input: UserInput) -> Result<UserInput, ServiceError> {
    // Sanitization logic
    Ok(input)
}

fn validate_business_rules(input: UserInput) -> Result<UserInput, ServiceError> {
    // Business validation
    Ok(input)
}

fn convert_to_model(input: UserInput) -> Result<User, ServiceError> {
    // Conversion logic
    Ok(User::new(input))
}
```

## Pattern Reference

### Validator<T>

```rust
impl<T> Validator<T> {
    pub fn new() -> Self
    pub fn rule<F>(self, rule: F) -> Self 
    where F: Fn(&T) -> Result<(), ServiceError> + Send + Sync + 'static
    pub fn validate(&self, input: &T) -> Result<(), ServiceError>
}
```

### Pipeline<T>

```rust
impl<T> Pipeline<T> {
    pub fn new(initial: T) -> Self
    pub fn then<U, F>(self, step: F) -> Pipeline<U>
    where F: FnOnce(T) -> Result<U, ServiceError>
    pub fn execute(self) -> Result<T, ServiceError>
    pub fn into_fn(self) -> Box<dyn Fn() -> Result<T, ServiceError>>
}
```

### Either<L, R>

```rust
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    pub fn map_left<F, T>(self, f: F) -> Either<T, R>
    pub fn map_right<F, T>(self, f: F) -> Either<L, T>
    pub fn into_result(self) -> Result<R, L>
}
```

## Best Practices

### 1. Keep Rules Simple

```rust
// Good: Single responsibility
.rule(|dto| {
    if dto.name.is_empty() {
        Err(ServiceError::bad_request("Name required"))
    } else {
        Ok(())
    }
})

// Bad: Multiple responsibilities
.rule(|dto| {
    if dto.name.is_empty() {
        Err(ServiceError::bad_request("Name required"))
    } else if dto.email.is_empty() {
        Err(ServiceError::bad_request("Email required"))
    } else if !dto.email.contains('@') {
        Err(ServiceError::bad_request("Invalid email"))
    } else {
        Ok(())
    }
})
```

### 2. Create Reusable Validators

```rust
// Create common validators
fn required_string_validator(field_name: &str) -> impl Fn(&String) -> Result<(), ServiceError> {
    let field_name = field_name.to_string();
    move |value: &String| {
        if value.trim().is_empty() {
            Err(ServiceError::bad_request(format!("{} is required", field_name)))
        } else {
            Ok(())
        }
    }
}

// Use in multiple places
fn create_person_validator() -> Validator<PersonDTO> {
    Validator::new()
        .rule(|dto| required_string_validator("name")(&dto.name))
        .rule(|dto| required_string_validator("email")(&dto.email))
}
```

### 3. Error Context

```rust
// Good: Specific error messages
.rule(|dto| {
    if dto.age < 0 {
        Err(ServiceError::bad_request("Age must be positive"))
    } else if dto.age > 150 {
        Err(ServiceError::bad_request("Age must be realistic"))
    } else {
        Ok(())
    }
})

// Bad: Generic error
.rule(|dto| {
    if dto.age < 0 || dto.age > 150 {
        Err(ServiceError::bad_request("Invalid age"))
    } else {
        Ok(())
    }
})
```

### 4. Pure Functions

```rust
// Good: Pure function
fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

// Bad: Side effects
fn normalize_email(email: &str) -> String {
    log::info!("Normalizing email: {}", email); // Side effect!
    email.trim().to_lowercase()
}
```

## Common Patterns

### User Registration Flow

```rust
pub fn register_user(dto: CreateUserDTO, pool: &Pool) -> Result<User, ServiceError> {
    // Validate input
    create_user_validator().validate(&dto)?;
    
    // Transform and save
    Pipeline::new(dto)
        .then(|dto| hash_password(dto))
        .then(|dto| check_email_uniqueness(dto, pool))
        .then(|dto| save_user_to_db(dto, pool))
        .execute()
}

fn hash_password(mut dto: CreateUserDTO) -> Result<CreateUserDTO, ServiceError> {
    dto.password = bcrypt::hash(dto.password, 12)
        .map_err(|_| ServiceError::internal_error("Password hashing failed"))?;
    Ok(dto)
}
```

### API Response Processing

```rust
pub fn process_api_response<T>(response: ApiResponse) -> Either<ApiError, T> 
where T: serde::de::DeserializeOwned 
{
    if response.status().is_success() {
        match response.json::<T>() {
            Ok(data) => Either::Right(data),
            Err(e) => Either::Left(ApiError::ParseError(e.to_string())),
        }
    } else {
        Either::Left(ApiError::HttpError(response.status().as_u16()))
    }
}
```

### Retry with Circuit Breaker

```rust
pub fn call_external_service() -> Result<ApiResponse, ServiceError> {
    Retry::new()
        .max_attempts(3)
        .delay(Duration::from_millis(200))
        .execute(|| {
            let result = external_api_client.call();
            match result {
                Ok(response) if response.status().is_server_error() => {
                    Err(ServiceError::temporary("Server error, retrying"))
                }
                other => other.map_err(|e| ServiceError::external_error(e.to_string()))
            }
        })
}
```

## Troubleshooting

### Common Issues

#### 1. Closure Capture Errors

```rust
// Problem: Trying to capture by reference
let field_name = "email";
.rule(|dto| {
    if dto.email.is_empty() {
        Err(ServiceError::bad_request(format!("{} required", field_name))) // Error!
    } else {
        Ok(())
    }
})

// Solution: Capture by value
let field_name = "email".to_string();
.rule(move |dto| {
    if dto.email.is_empty() {
        Err(ServiceError::bad_request(format!("{} required", field_name)))
    } else {
        Ok(())
    }
})
```

#### 2. Type Inference Issues

```rust
// Problem: Compiler can't infer type
let pipeline = Pipeline::new(data)
    .then(|x| process_data(x)); // What type is this?

// Solution: Explicit type annotations
let pipeline: Pipeline<ProcessedData> = Pipeline::new(data)
    .then(|x| process_data(x));
```

#### 3. Lifetime Issues

```rust
// Problem: Borrowed value doesn't live long enough
fn create_validator(field: &str) -> Validator<DTO> {
    Validator::new()
        .rule(|dto| validate_field(dto, field)) // Error: field borrowed
}

// Solution: Own the data
fn create_validator(field: String) -> Validator<DTO> {
    Validator::new()
        .rule(move |dto| validate_field(dto, &field))
}
```

### Performance Tips

1. **Memoization**: Use for expensive pure functions only
2. **Pipeline**: Prefer for data transformations over nested function calls
3. **Validator**: Create once, reuse multiple times
4. **QueryReader**: Use for database operation composition

### Testing Tips

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_individual_rules() {
        let validator = create_person_validator();
        
        // Test each rule individually
        let invalid_name = PersonDTO {
            name: "".to_string(),
            email: "valid@email.com".to_string(),
        };
        assert!(validator.validate(&invalid_name).is_err());
        
        let invalid_email = PersonDTO {
            name: "Valid Name".to_string(),
            email: "invalid-email".to_string(),
        };
        assert!(validator.validate(&invalid_email).is_err());
        
        let valid_dto = PersonDTO {
            name: "Valid Name".to_string(),
            email: "valid@email.com".to_string(),
        };
        assert!(validator.validate(&valid_dto).is_ok());
    }
}
```

## Further Reading

- [ADR-001: Functional Programming Patterns](./ADR-001-FUNCTIONAL-PATTERNS.md)
- [Migration Guide](./MIGRATION-GUIDE-FUNCTIONAL-PATTERNS.md)
- [Best Practices](./BEST-PRACTICES-FUNCTIONAL-PATTERNS.md)
- [Rust Functional Programming](https://doc.rust-lang.org/book/ch13-00-functional-features.html)