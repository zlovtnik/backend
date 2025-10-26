# Functional Programming Patterns Implementation Guide

## Overview

This document provides a comprehensive guide to the functional programming patterns implemented in the Actix-web REST API, based on the Functional Programming Codemap analysis. The implementation demonstrates advanced functional programming techniques in Rust for building composable, testable, and maintainable service layers.

## Core Patterns Implemented

### 1. QueryReader Monad Pattern

The QueryReader monad encapsulates database operations, allowing composition of database queries without explicitly passing connection parameters.

#### Implementation Location: `src/services/functional_patterns.rs`

```rust
pub struct QueryReader<T> {
    run: Box<dyn Fn(&mut PgConnection) -> ServiceResult<T> + Send + Sync>,
}
```

#### Key Methods:
- `new()`: Create a QueryReader from a database operation function
- `map()`: Transform the result of a query
- `and_then()`: Chain queries that depend on previous results
- `validate()`: Add validation logic before execution
- `transaction()`: Execute within a database transaction

#### Usage Examples:

**User Signup Flow** (`account_service.rs`):
```rust
pub fn signup_reader(user: UserDTO) -> Result<QueryReader<String>, ServiceError> {
    let sanitized_user = build_user_signup_pipeline().execute(user)?;
    Ok(QueryReader::new(move |conn| {
        user_ops::signup_user(sanitized_user.clone(), conn)
    }))
}
```

**Login Flow with Chaining** (`account_service.rs`):
```rust
let login_flow = QueryReader::new(move |conn| {
    user_ops::login_user(sanitized_login.clone(), conn)
    .ok_or_else(|| ServiceError::unauthorized(constants::MESSAGE_LOGIN_FAILED.to_string()))
})
.and_then(move |login_info| {
    // Additional operations...
});
```

**System Statistics Aggregation** (`tenant_service.rs`):
```rust
QueryReader::new(|conn| {
    // Get total tenant count
    let total_tenants = Tenant::count_all(conn).map_err(|e| {
        // error handling
    })?;
    // Aggregate additional statistics...
    Ok(stats)
})
```

### 2. Validator Pattern

Composable validation rules that can be chained together for complex validation logic.

#### Implementation Location: `src/services/functional_patterns.rs`

```rust
pub struct Validator<T> {
    rules: Vec<Box<dyn Fn(&T) -> ServiceResult<()> + Send + Sync>>,
}
```

#### Key Methods:
- `rule()`: Add a validation rule
- `validate()`: Execute all rules against input
- `and()` / `or()`: Combine validators logically
- `when()`: Conditional validation
- `not()`: Negate validation

#### Usage Examples:

**User Validation Chain** (`account_service.rs`):
```rust
fn validate_user_update_dto(user_update: &UserUpdateDTO) -> Result<(), ServiceError> {
    Validator::new()
        .rule(|dto: &UserUpdateDTO| validation_rules::required("username")(&dto.username))
        .rule(|dto: &UserUpdateDTO| validation_rules::min_length("username", 3)(&dto.username))
        .rule(|dto: &UserUpdateDTO| validation_rules::max_length("username", 50)(&dto.username))
        .rule(|dto: &UserUpdateDTO| validation_rules::required("email")(&dto.email))
        .rule(|dto: &UserUpdateDTO| validation_rules::email("email")(&dto.email))
        .validate(user_update)
}
```

**Model-Level Validators** (`models/user/validators.rs`):
```rust
pub fn user_validator() -> Validator<UserDTO> {
    Validator::new()
        .rule(|dto: &UserDTO| validation_rules::required("username")(&dto.username))
        .rule(|dto: &UserDTO| validation_rules::min_length("username", 3)(&dto.username))
        .rule(|dto: &UserDTO| validation_rules::max_length("username", 50)(&dto.username))
        .rule(|dto: &UserDTO| validate_password(&dto.password))
        .rule(|dto: &UserDTO| validation_rules::required("email")(&dto.email))
        .rule(|dto: &UserDTO| validation_rules::email("email")(&dto.email))
        .rule(|dto: &UserDTO| validation_rules::max_length("email", 255)(&dto.email))
}
```

### 3. Either Error Handling Pattern

Functional error handling using Either types for explicit dual-path error propagation.

#### Implementation Location: `src/services/functional_patterns.rs`

```rust
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
```

#### Key Methods:
- `map_right()` / `map_left()`: Transform values
- `flat_map()` / `and_then()`: Monadic composition
- `into_result()`: Convert to standard Result type

#### Usage Examples:

**Signup with Either** (`account_service.rs`):
```rust
pub fn signup_either(user: UserDTO, pool: &Pool) -> Either<ServiceError, String> {
    match signup(user, pool) {
        Ok(message) => Either::Right(message),
        Err(error) => Either::Left(error),
    }
}
```

### 4. Pipeline Pattern

Composable data transformation chains for processing input data through multiple stages.

#### Implementation Location: `src/services/functional_patterns.rs`

```rust
pub struct Pipeline<T> {
    transformations: Vec<Box<dyn Fn(T) -> ServiceResult<T> + Send + Sync>>,
}
```

#### Key Methods:
- `then()`: Add a transformation step
- `execute()`: Run the entire pipeline
- `and_then()`: Chain with another pipeline

#### Usage Examples:

**User Signup Pipeline** (`account_service.rs`):
```rust
pub fn build_user_signup_pipeline() -> Pipeline<UserDTO> {
    Pipeline::new()
        .then(|user| {
            // Sanitization logic
            Ok(user)
        })
        .then(|user| {
            // Additional transformations
            Ok(user)
        })
}
```

### 5. Retry Pattern

Resilient operation handling with configurable retry logic and backoff strategies.

#### Implementation Location: `src/services/functional_patterns.rs`

```rust
pub struct Retry<T> {
    operation: Box<dyn Fn() -> ServiceResult<T> + Send + Sync>,
    max_attempts: usize,
    delay_ms: u64,
}
```

#### Usage Examples:

**Token Verification with Retry** (`account_service.rs`):
```rust
let shared_data = Arc::new(token_data);
let pool = pool.clone();

Retry::new(move || {
    token_utils::verify_token(shared_data.as_ref(), &pool)
        .map_err(|err| ServiceError::unauthorized(err))
})
.max_attempts(3)
.delay(150)
.execute()
```

### 6. Memoization Pattern

Performance optimization through caching of pure function results.

#### Implementation Location: `src/services/functional_patterns.rs`

```rust
pub struct Memoized<K, V> {
    cache: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<K, CacheEntry<V>>>>,
    compute: Box<dyn Fn(&K) -> ServiceResult<V> + Send + Sync>,
    config: MemoizationConfig,
}
```

## Service Integration Patterns

### Functional Service Base

A base service providing common functional patterns for all services.

#### Location: `src/services/functional_service_base.rs`

Key features:
- Query execution with error logging
- Transaction management
- Common service operations

### Controller Integration

Controllers use QueryReaders for orchestration and functional composition.

#### Example from Account Controller:
```rust
pub async fn signup(
    user_dto: web::Json<UserDTO>,
    context: web::Data<ControllerContext>,
) -> Result<HttpResponse, Error> {
    let signup_flow = account_service::signup_reader(user_dto.into_inner())?;
    let signup_message = measure_operation!(operation, {
        context.run_query(signup_flow)
    })?;
    Ok(HttpResponse::Created().json(json!({ "message": signup_message })))
}
```

## Benefits Achieved

1. **Composability**: Patterns can be combined in flexible ways
2. **Testability**: Pure functions and separated concerns
3. **Type Safety**: Compile-time guarantees
4. **Error Handling**: Explicit and functional error propagation
5. **Performance**: Zero-cost abstractions
6. **Maintainability**: Clear separation of concerns

## Testing

Unit test examples covering the core patterns live in `functional_patterns.rs`, such as:

```rust
#[test]
fn test_validator() {
    let validator = Validator::<i32>::new()
        .rule(|&x| {
            if x > 0 {
                Ok(())
            } else {
                Err(ServiceError::bad_request("Must be positive"))
            }
        });

    assert!(validator.validate(&5).is_ok());
    assert!(validator.validate(&-1).is_err());
}
```

## Future Extensions

Based on the roadmap, remaining work includes:

- Extending functional validation to all DTOs
- Implementing Either error handling across all services
- Adding functional middleware composition in controllers
- Performance monitoring for functional pipelines

## References

- [ADR-001: Functional Programming Patterns](./ADR-001-FUNCTIONAL-PATTERNS.md)
- [Railway Oriented Programming](https://fsharpforfunandprofit.com/rop/)
- [Functional Programming in Rust](https://doc.rust-lang.org/book/ch13-00-functional-features.html)
