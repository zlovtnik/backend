# Using Functional Patterns in Services - Quick Reference

## Overview
This guide shows how to use the new functional programming patterns in the service layer.

## Validation with Validator Combinator

### Basic Validation
```rust
use crate::services::functional_patterns::Validator;

fn create_my_validator() -> Validator<MyDTO> {
    Validator::new()
        .rule(|dto: &MyDTO| {
            if dto.field.is_empty() {
                Err(ServiceError::bad_request("Field cannot be empty"))
            } else {
                Ok(())
            }
        })
}

// Use it
let validator = create_my_validator();
validator.validate(&my_dto)?;
```

### Reusable Validation Rules
```rust
// Define reusable rules
fn not_empty(field: &str, name: &str) -> Result<(), ServiceError> {
    if field.trim().is_empty() {
        Err(ServiceError::bad_request(format!("{} cannot be empty", name)))
    } else {
        Ok(())
    }
}

fn length_check(field: &str, min: usize, max: usize, name: &str) -> Result<(), ServiceError> {
    if field.len() < min || field.len() > max {
        Err(ServiceError::bad_request(format!("{} must be {}-{} chars", name, min, max)))
    } else {
        Ok(())
    }
}

// Compose them
fn create_user_validator() -> Validator<UserDTO> {
    Validator::new()
        .rule(|dto: &UserDTO| not_empty(&dto.username, "Username"))
        .rule(|dto: &UserDTO| length_check(&dto.username, 3, 50, "Username"))
        .rule(|dto: &UserDTO| not_empty(&dto.email, "Email"))
}
```

## Database Operations with QueryReader

### Simple Query
```rust
use crate::services::functional_patterns::{QueryReader, run_query};

let query = QueryReader::new(|conn| {
    Person::find_by_id(id, conn)
        .map_err(|_| ServiceError::not_found("Person not found"))
});

let person = run_query(query, pool)?;
```

### Chained Queries
```rust
let query = QueryReader::new(|conn| {
    User::find_by_username(&username, conn)
        .map_err(|_| ServiceError::not_found("User not found"))
})
.and_then(|user| QueryReader::new(move |conn| {
    // Query depends on user from first query
    LoginHistory::find_by_user_id(user.id, conn)
        .map_err(|_| ServiceError::internal_server_error("Failed to get history"))
}));

let history = run_query(query, pool)?;
```

### Query with Validation
```rust
let query = QueryReader::new(|conn| {
    Person::find_by_id(id, conn)
        .map_err(|_| ServiceError::not_found("Person not found"))
})
.validate(|person| {
    if person.is_deleted {
        Err(ServiceError::bad_request("Person is deleted"))
    } else {
        Ok(())
    }
});

let person = run_query(query, pool)?;
```

## Data Transformation with Pipeline

### Simple Pipeline
```rust
use crate::services::functional_patterns::Pipeline;

let pipeline = Pipeline::new()
    .then(|dto| {
        // Transformation 1: normalize email
        let mut dto = dto;
        dto.email = dto.email.to_lowercase();
        Ok(dto)
    })
    .then(|dto| {
        // Transformation 2: trim whitespace
        let mut dto = dto;
        dto.name = dto.name.trim().to_string();
        Ok(dto)
    });

let normalized_dto = pipeline.execute(raw_dto)?;
```

### Reusable Pipeline
```rust
fn create_normalization_pipeline<T>() -> Pipeline<T> 
where 
    T: NormalizableDTO
{
    Pipeline::new()
        .then(|dto| dto.normalize_email())
        .then(|dto| dto.trim_fields())
        .then(|dto| dto.validate_format())
}

// Use it
let pipeline = create_normalization_pipeline();
let normalized = pipeline.execute(dto)?;
```

## Error Handling with Either

### Dual-Path Processing
```rust
use crate::services::functional_patterns::Either;

fn process_user(user_id: i32) -> Either<String, User> {
    match User::find_by_id(user_id) {
        Ok(user) if user.is_active => Either::Right(user),
        Ok(user) => Either::Left(format!("User {} is inactive", user_id)),
        Err(_) => Either::Left(format!("User {} not found", user_id)),
    }
}

// Use it
match process_user(123) {
    Either::Right(user) => {
        // Process active user
    }
    Either::Left(error) => {
        // Handle error
    }
}
```

## Caching with Memoization

### Expensive Computation Cache
```rust
use crate::services::functional_patterns::Memoized;

// Create memoized function
let user_stats = Memoized::new(|user_id: &i32| {
    // Expensive database aggregation
    compute_user_statistics(*user_id, pool)
});

// First call computes and caches
let stats1 = user_stats.get(&123)?;

// Second call returns cached result
let stats2 = user_stats.get(&123)?; // Instant!

// Clear cache when needed
user_stats.clear();
```

## Retry Pattern for Resilience

### Simple Retry
```rust
use crate::services::functional_patterns::Retry;

let result = Retry::new(|| {
    // Operation that might fail transiently
    external_api_call(params)
})
.max_attempts(3)
.delay(100) // 100ms between retries
.execute()?;
```

## Complete Example: User Registration

```rust
use crate::services::functional_patterns::{Validator, Pipeline, Retry};

// 1. Create validator
fn create_signup_validator() -> Validator<UserDTO> {
    Validator::new()
        .rule(|dto: &UserDTO| {
            if dto.username.len() < 3 {
                Err(ServiceError::bad_request("Username too short"))
            } else {
                Ok(())
            }
        })
        .rule(|dto: &UserDTO| {
            if !dto.email.contains('@') {
                Err(ServiceError::bad_request("Invalid email"))
            } else {
                Ok(())
            }
        })
}

// 2. Create normalization pipeline
fn create_user_pipeline() -> Pipeline<UserDTO> {
    Pipeline::new()
        .then(|mut dto| {
            dto.email = dto.email.to_lowercase();
            Ok(dto)
        })
        .then(|mut dto| {
            dto.username = dto.username.trim().to_string();
            Ok(dto)
        })
}

// 3. Complete signup function
pub fn signup(user: UserDTO, pool: &Pool) -> Result<String, ServiceError> {
    // Validate
    let validator = create_signup_validator();
    validator.validate(&user)?;
    
    // Normalize
    let pipeline = create_user_pipeline();
    let normalized_user = pipeline.execute(user)?;
    
    // Save with retry (for transient failures)
    let result = Retry::new(|| {
        ServicePipeline::new(pool.clone())
            .with_data(normalized_user.clone())
            .execute(|user, conn| {
                User::insert(user, conn)
                    .map_err(|e| ServiceError::internal_server_error(e))
            })
    })
    .max_attempts(3)
    .execute()?;
    
    Ok("User created successfully".to_string())
}
```

## Best Practices

### 1. Keep Validators Pure
```rust
// ✅ Good: Pure function
.rule(|dto: &UserDTO| {
    if dto.username.len() < 3 {
        Err(ServiceError::bad_request("Too short"))
    } else {
        Ok(())
    }
})

// ❌ Bad: Side effects
.rule(|dto: &UserDTO| {
    log::info!("Validating user"); // Side effect!
    // ...
})
```

### 2. Compose Small Functions
```rust
// ✅ Good: Composable
fn validate_username(username: &str) -> Result<(), ServiceError> { /* ... */ }
fn validate_email(email: &str) -> Result<(), ServiceError> { /* ... */ }

let validator = Validator::new()
    .rule(|dto: &UserDTO| validate_username(&dto.username))
    .rule(|dto: &UserDTO| validate_email(&dto.email));

// ❌ Bad: Monolithic
.rule(|dto: &UserDTO| {
    // 100 lines of validation logic
})
```

### 3. Use Type Aliases for Clarity
```rust
type UserValidator = Validator<UserDTO>;
type PersonValidator = Validator<PersonDTO>;

fn create_user_validator() -> UserValidator {
    // ...
}
```

### 4. Document Your Validators
```rust
/// Validates user registration data.
///
/// Checks:
/// - Username: 3-50 characters, alphanumeric
/// - Email: Valid format with @
/// - Password: Minimum 6 characters
fn create_signup_validator() -> Validator<UserDTO> {
    // ...
}
```

## Migration from Imperative Code

### Before (Imperative)
```rust
pub fn create_person(dto: PersonDTO, pool: &Pool) -> Result<(), ServiceError> {
    // Manual validation
    if dto.name.trim().is_empty() {
        return Err(ServiceError::bad_request("Name cannot be empty"));
    }
    if dto.name.len() > 100 {
        return Err(ServiceError::bad_request("Name too long"));
    }
    if !dto.email.contains('@') {
        return Err(ServiceError::bad_request("Invalid email"));
    }
    
    // Get connection
    let mut conn = pool.get()
        .map_err(|_| ServiceError::internal_server_error("DB error"))?;
    
    // Insert
    Person::insert(dto, &mut conn)
        .map_err(|_| ServiceError::internal_server_error("Insert failed"))?;
    
    Ok(())
}
```

### After (Functional)
```rust
fn create_person_validator() -> Validator<PersonDTO> {
    Validator::new()
        .rule(|dto: &PersonDTO| {
            if dto.name.trim().is_empty() || dto.name.len() > 100 {
                Err(ServiceError::bad_request("Invalid name"))
            } else {
                Ok(())
            }
        })
        .rule(|dto: &PersonDTO| {
            if !dto.email.contains('@') {
                Err(ServiceError::bad_request("Invalid email"))
            } else {
                Ok(())
            }
        })
}

pub fn create_person(dto: PersonDTO, pool: &Pool) -> Result<(), ServiceError> {
    create_person_validator().validate(&dto)?;
    
    ServicePipeline::new(pool.clone())
        .with_data(dto)
        .execute(|person, conn| {
            Person::insert(person, conn)
                .map_err(|_| ServiceError::internal_server_error("Insert failed"))
        })
}
```

## Performance Tips

1. **Validators are cheap**: Create them inline, they're lightweight
2. **Memoization for expensive ops**: Use for database aggregations, not simple queries
3. **Pipeline early returns**: Transformations stop on first error
4. **QueryReader is zero-cost**: Compiles to same code as manual management

## Common Patterns

### Validate → Transform → Save
```rust
pub fn create_entity(dto: DTO, pool: &Pool) -> Result<(), ServiceError> {
    validator.validate(&dto)?;
    let transformed = pipeline.execute(dto)?;
    save_to_db(transformed, pool)
}
```

### Query → Validate → Transform
```rust
pub fn get_entity(id: i32, pool: &Pool) -> Result<DTO, ServiceError> {
    let entity = fetch_from_db(id, pool)?;
    validator.validate(&entity)?;
    transform_to_dto(entity)
}
```

### Try First → Fallback → Retry
```rust
let result = try_primary_source()
    .or_else(|_| try_backup_source())
    .or_else(|_| Retry::new(|| try_primary_source()).execute());
```
