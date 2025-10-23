# Best Practices: Functional Programming Patterns

## Overview

This document outlines best practices, common patterns, and anti-patterns when using functional programming patterns in our service layer.

## Core Principles

### 1. Pure Functions First

**Do**: Write pure functions whenever possible
```rust
// Good: Pure function
fn calculate_tax_amount(price: f64, tax_rate: f64) -> f64 {
    price * tax_rate
}

// Good: Pure validation rule
fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}
```

**Don't**: Add side effects to validation rules
```rust
// Bad: Side effects in validation
fn validate_email(email: &str) -> Result<(), ServiceError> {
    log::info!("Validating email: {}", email); // Side effect!
    audit_log::record_validation(email);       // Side effect!
    
    if email.contains('@') {
        Ok(())
    } else {
        Err(ServiceError::bad_request("Invalid email"))
    }
}
```

### 2. Composability Over Complexity

**Do**: Build complex behavior from simple, composable pieces
```rust
// Good: Composable validators
fn create_user_validator() -> Validator<UserDTO> {
    Validator::new()
        .rule(username_length_rule)
        .rule(username_format_rule)
        .rule(email_format_rule)
        .rule(password_strength_rule)
}

fn username_length_rule(dto: &UserDTO) -> Result<(), ServiceError> {
    if dto.username.len() >= 3 && dto.username.len() <= 50 {
        Ok(())
    } else {
        Err(ServiceError::bad_request("Username must be 3-50 characters"))
    }
}
```

**Don't**: Create monolithic validation functions
```rust
// Bad: Monolithic validator
fn validate_user(dto: &UserDTO) -> Result<(), ServiceError> {
    // 50+ lines of mixed validation logic
    if dto.username.len() < 3 { /* ... */ }
    if !dto.email.contains('@') { /* ... */ }
    if dto.password.len() < 8 { /* ... */ }
    // ... many more conditions
}
```

### 3. Explicit Error Context

**Do**: Provide specific, actionable error messages
```rust
// Good: Specific error context
.rule(|dto| {
    if dto.price < 0.0 {
        Err(ServiceError::bad_request("Price cannot be negative"))
    } else if dto.price > 1_000_000.0 {
        Err(ServiceError::bad_request("Price cannot exceed $1,000,000"))
    } else {
        Ok(())
    }
})
```

**Don't**: Use generic error messages
```rust
// Bad: Generic error
.rule(|dto| {
    if dto.price < 0.0 || dto.price > 1_000_000.0 {
        Err(ServiceError::bad_request("Invalid price"))
    } else {
        Ok(())
    }
})
```

## Pattern-Specific Best Practices

### Validator Pattern

#### ✅ Good Practices

**1. Single Responsibility Rules**
```rust
// Each rule tests one thing
.rule(|user| validate_username_length(&user.username))
.rule(|user| validate_username_format(&user.username))
.rule(|user| validate_email_format(&user.email))
```

**2. Reusable Rule Functions**
```rust
fn create_length_rule(min: usize, max: usize, field: &str) -> impl Fn(&String) -> Result<(), ServiceError> {
    let field_name = field.to_string();
    move |value: &String| {
        let len = value.len();
        if len >= min && len <= max {
            Ok(())
        } else {
            Err(ServiceError::bad_request(
                format!("{} must be {}-{} characters", field_name, min, max)
            ))
        }
    }
}

// Usage
.rule(|dto| create_length_rule(3, 50, "username")(&dto.username))
```

**3. Conditional Validation**
```rust
.rule(|dto| {
    if dto.account_type == AccountType::Business {
        validate_business_fields(dto)
    } else {
        Ok(()) // Skip for non-business accounts
    }
})
```

#### ❌ Anti-patterns

**1. Rules with Multiple Responsibilities**
```rust
// Bad: Testing multiple unrelated things
.rule(|dto| {
    if dto.username.is_empty() {
        Err(ServiceError::bad_request("Username required"))
    } else if dto.email.is_empty() {
        Err(ServiceError::bad_request("Email required"))
    } else if dto.phone.is_empty() {
        Err(ServiceError::bad_request("Phone required"))
    } else {
        Ok(())
    }
})
```

**2. Stateful Validation Rules**
```rust
// Bad: Rule depends on external state
.rule(|dto| {
    let count = GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst); // Bad!
    if count > 100 {
        Err(ServiceError::bad_request("Too many validations"))
    } else {
        Ok(())
    }
})
```

### Pipeline Pattern

#### ✅ Good Practices

**1. Clear Transformation Steps**
```rust
// Good: Each step has a clear purpose
Pipeline::new(raw_input)
    .then(|input| sanitize_user_input(input))     // Security
    .then(|input| validate_business_rules(input)) // Validation
    .then(|input| enrich_with_defaults(input))    // Enhancement
    .then(|input| convert_to_domain_model(input)) // Conversion
    .execute()
```

**2. Small, Focused Transformations**
```rust
fn sanitize_user_input(input: UserInput) -> Result<UserInput, ServiceError> {
    Ok(UserInput {
        username: input.username.trim().to_lowercase(),
        email: input.email.trim().to_lowercase(),
        phone: input.phone.chars().filter(|c| c.is_numeric()).collect(),
        ..input
    })
}
```

**3. Type-Safe Transformations**
```rust
// Transform through different types
Pipeline::new(raw_json)               // String
    .then(|json| parse_json(json))    // -> UserDTO
    .then(|dto| validate_dto(dto))    // -> UserDTO (validated)
    .then(|dto| convert_to_user(dto)) // -> User
    .execute()
```

#### ❌ Anti-patterns

**1. Complex Single-Step Transformations**
```rust
// Bad: Doing too much in one step
.then(|input| {
    // 50+ lines of complex transformation logic
    let sanitized = sanitize(input);
    let validated = validate(sanitized)?;
    let enriched = enrich(validated)?;
    let converted = convert(enriched)?;
    Ok(converted)
})
```

**2. Side Effects in Transformations**
```rust
// Bad: Side effects in pipeline steps
.then(|input| {
    log::info!("Processing input: {:?}", input); // Side effect
    send_analytics_event("user_processing");     // Side effect
    transform_input(input)
})
```

### QueryReader Pattern

#### ✅ Good Practices

**1. Database Operation Composition**
```rust
// Good: Compose related database operations
fn get_user_profile(user_id: i32) -> QueryReader<UserProfile> {
    QueryReader::new(move |conn| find_user(user_id, conn))
        .and_then(|user| {
            QueryReader::new(move |conn| find_user_settings(user.id, conn))
                .map(move |settings| UserProfile { user, settings })
        })
}
```

**2. Error Handling in Queries**
```rust
fn find_user_safe(user_id: i32) -> QueryReader<Option<User>> {
    QueryReader::new(move |conn| {
        match users::table.find(user_id).first::<User>(conn) {
            Ok(user) => Ok(Some(user)),
            Err(diesel::NotFound) => Ok(None),
            Err(e) => Err(ServiceError::database_error(e.to_string())),
        }
    })
}
```

**3. Query Result Transformations**
```rust
// Transform query results functionally
QueryReader::new(|conn| load_all_users(conn))
    .map(|users| {
        users.into_iter()
            .filter(|u| u.is_active)
            .map(|u| UserSummary::from(u))
            .collect()
    })
```

#### ❌ Anti-patterns

**1. Non-Database Operations in QueryReader**
```rust
// Bad: HTTP calls inside QueryReader
QueryReader::new(|conn| {
    let user = find_user(123, conn)?;
    let profile = http_client.get_user_profile(user.id)?; // Bad!
    Ok((user, profile))
})
```

**2. Long-Running Operations**
```rust
// Bad: Expensive computation in QueryReader
QueryReader::new(|conn| {
    let data = load_large_dataset(conn)?;
    let result = expensive_calculation(data); // Bad! Blocks DB connection
    Ok(result)
})
```

### Either Pattern

#### ✅ Good Practices

**1. Clear Success/Failure Paths**
```rust
fn parse_configuration(input: &str) -> Either<ConfigError, Config> {
    match serde_json::from_str::<Config>(input) {
        Ok(config) => Either::Right(config),
        Err(e) => Either::Left(ConfigError::ParseError(e.to_string())),
    }
}
```

**2. Transformation of Either Values**
```rust
fn process_result(result: Either<String, i32>) -> String {
    result
        .map_left(|err| format!("Error: {}", err))
        .map_right(|val| format!("Success: {}", val))
        .into_result()
        .unwrap_or_else(|left| left)
}
```

#### ❌ Anti-patterns

**1. Overusing Either Instead of Result**
```rust
// Bad: Result is more appropriate for errors
fn divide(a: f64, b: f64) -> Either<String, f64> {
    if b == 0.0 {
        Either::Left("Division by zero".to_string())
    } else {
        Either::Right(a / b)
    }
}

// Good: Use Result for error handling
fn divide(a: f64, b: f64) -> Result<f64, MathError> {
    if b == 0.0 {
        Err(MathError::DivisionByZero)
    } else {
        Ok(a / b)
    }
}
```

### Memoization Pattern

#### ✅ Good Practices

**1. Pure, Expensive Functions Only**
```rust
// Good: Expensive pure computation
let fibonacci = Memoized::new(|n: u32| {
    if n <= 1 {
        n as u64
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
});
```

**2. Bounded Cache Size**
```rust
// Good: Limited cache to prevent memory leaks
let cache = Memoized::with_capacity(1000, |key: String| {
    expensive_computation(key)
});
```

#### ❌ Anti-patterns

**1. Memoizing Functions with Side Effects**
```rust
// Bad: Function has side effects
let bad_memoized = Memoized::new(|input: String| {
    log::info!("Processing: {}", input); // Side effect!
    expensive_computation(input)
});
```

**2. Unbounded Memoization**
```rust
// Bad: Potential memory leak
let unbounded = Memoized::new(|input: String| {
    // Could cache unlimited data
    process_user_data(input)
});
```

## Performance Best Practices

### 1. Zero-Cost Abstractions

**Do**: Prefer patterns that compile to efficient code
```rust
// Good: Pipeline compiles to simple function calls
Pipeline::new(data)
    .then(step1)  // Inlined by compiler
    .then(step2)  // Inlined by compiler
    .execute()
```

**Don't**: Create unnecessary allocations
```rust
// Bad: Unnecessary vector allocation
.rule(|dto| {
    let errors = vec![]; // Unnecessary allocation
    if dto.name.is_empty() {
        errors.push("Name required");
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
})
```

### 2. Lazy Evaluation

**Do**: Use lazy patterns for expensive operations
```rust
// Good: Only evaluate when needed
let lazy_validator = || {
    Validator::new()
        .rule(expensive_validation_rule)
        .rule(another_expensive_rule)
};

// Only create validator when actually needed
if needs_validation {
    lazy_validator().validate(&dto)?;
}
```

### 3. Efficient Error Handling

**Do**: Fail fast with specific errors
```rust
// Good: Early return on first failure
.rule(|dto| {
    if dto.email.is_empty() {
        return Err(ServiceError::bad_request("Email required"));
    }
    expensive_email_validation(&dto.email)
})
```

## Testing Best Practices

### 1. Test Individual Rules

```rust
#[test]
fn test_username_length_rule() {
    let rule = username_length_rule;
    
    // Test boundary conditions
    let short = UserDTO { username: "ab".to_string(), ..Default::default() };
    assert!(rule(&short).is_err());
    
    let min_valid = UserDTO { username: "abc".to_string(), ..Default::default() };
    assert!(rule(&min_valid).is_ok());
    
    let max_valid = UserDTO { username: "a".repeat(50), ..Default::default() };
    assert!(rule(&max_valid).is_ok());
    
    let too_long = UserDTO { username: "a".repeat(51), ..Default::default() };
    assert!(rule(&too_long).is_err());
}
```

### 2. Test Validator Composition

```rust
#[test]
fn test_user_validator_composition() {
    let validator = create_user_validator();
    
    // Test that all rules are applied
    let invalid_user = UserDTO {
        username: "ab".to_string(),      // Fails username rule
        email: "invalid".to_string(),    // Fails email rule
        password: "weak".to_string(),    // Fails password rule
    };
    
    // Should fail on first rule (username)
    let result = validator.validate(&invalid_user);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("username"));
}
```

### 3. Test Pipeline Transformations

```rust
#[test]
fn test_data_pipeline() {
    let input = RawUserData {
        name: "  JOHN DOE  ",
        email: "  JOHN@EXAMPLE.COM  ",
    };
    
    let result = Pipeline::new(input)
        .then(sanitize_input)
        .then(validate_input)
        .then(convert_to_user)
        .execute();
    
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.name, "john doe");
    assert_eq!(user.email, "john@example.com");
}
```

## Error Handling Best Practices

### 1. Preserve Error Context

```rust
// Good: Chain errors to preserve context
.rule(|dto| {
    complex_validation(&dto.data)
        .map_err(|e| ServiceError::bad_request(
            format!("Data validation failed: {}", e)
        ))
})
```

### 2. Use Appropriate Error Types

```rust
// Good: Specific error types for different scenarios
#[derive(Debug)]
pub enum ValidationError {
    Required(String),
    InvalidFormat(String),
    BusinessRule(String),
}

impl From<ValidationError> for ServiceError {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::Required(field) => 
                ServiceError::bad_request(format!("{} is required", field)),
            ValidationError::InvalidFormat(field) => 
                ServiceError::bad_request(format!("{} has invalid format", field)),
            ValidationError::BusinessRule(msg) => 
                ServiceError::conflict(msg),
        }
    }
}
```

## Documentation Best Practices

### 1. Document Complex Rules

```rust
/// Validates that a password meets security requirements:
/// - At least 8 characters long
/// - Contains at least one uppercase letter
/// - Contains at least one lowercase letter  
/// - Contains at least one digit
/// - Contains at least one special character (!@#$%^&*)
fn password_strength_rule(dto: &UserDTO) -> Result<(), ServiceError> {
    let password = &dto.password;
    
    if password.len() < 8 {
        return Err(ServiceError::bad_request("Password must be at least 8 characters"));
    }
    
    // ... rest of validation
}
```

### 2. Provide Usage Examples

```rust
/// Creates a validator for user registration data.
/// 
/// # Example
/// 
/// ```rust
/// let validator = create_user_validator();
/// let user_dto = UserDTO { /* ... */ };
/// 
/// match validator.validate(&user_dto) {
///     Ok(()) => println!("User data is valid"),
///     Err(e) => println!("Validation failed: {}", e),
/// }
/// ```
pub fn create_user_validator() -> Validator<UserDTO> {
    // Implementation...
}
```

## Resources

- [Developer Guide](./DEVELOPER-GUIDE-FUNCTIONAL-PATTERNS.md) - Comprehensive usage guide
- [Migration Guide](./MIGRATION-GUIDE-FUNCTIONAL-PATTERNS.md) - How to migrate existing code
- [ADR-001](./ADR-001-FUNCTIONAL-PATTERNS.md) - Architecture decision rationale