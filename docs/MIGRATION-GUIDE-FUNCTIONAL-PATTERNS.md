# Migration Guide: Functional Programming Patterns

## Overview

This guide helps you migrate existing services to use the new functional programming patterns. The migration is designed to be gradual and non-breaking.

## Migration Strategy

### Phase 1: Add Functional Validators (Zero Breaking Changes)
1. Create new functional validators alongside existing validation
2. Refactor existing validation functions to use new validators internally
3. Test both old and new validation paths

### Phase 2: Adopt Functional Patterns (Gradual Adoption)
1. Use Pipeline pattern for new data transformations
2. Apply QueryReader for new database operations
3. Implement Either type for new dual-outcome scenarios

### Phase 3: Full Migration (Optional)
1. Replace remaining imperative patterns
2. Remove legacy validation code
3. Optimize performance with memoization

## Step-by-Step Migration

### 1. Migrating Validation Logic

#### Before: Imperative Validation
```rust
pub fn validate_user_dto(dto: &UserDTO) -> Result<(), ServiceError> {
    // Check username
    if dto.username.len() < 3 {
        return Err(ServiceError::bad_request("Username too short"));
    }
    if dto.username.len() > 50 {
        return Err(ServiceError::bad_request("Username too long"));
    }
    
    // Check password
    if dto.password.len() < 6 {
        return Err(ServiceError::bad_request("Password too short"));
    }
    
    // Check email
    if !dto.email.contains('@') {
        return Err(ServiceError::bad_request("Invalid email"));
    }
    
    Ok(())
}
```

#### Step 1: Create Functional Validator
```rust
use crate::services::functional_patterns::Validator;

fn create_user_validator() -> Validator<UserDTO> {
    Validator::new()
        .rule(|dto| {
            if dto.username.len() < 3 {
                Err(ServiceError::bad_request("Username too short"))
            } else if dto.username.len() > 50 {
                Err(ServiceError::bad_request("Username too long"))
            } else {
                Ok(())
            }
        })
        .rule(|dto| {
            if dto.password.len() < 6 {
                Err(ServiceError::bad_request("Password too short"))
            } else {
                Ok(())
            }
        })
        .rule(|dto| {
            if !dto.email.contains('@') {
                Err(ServiceError::bad_request("Invalid email"))
            } else {
                Ok(())
            }
        })
}
```

#### Step 2: Refactor Existing Function (Backward Compatible)
```rust
pub fn validate_user_dto(dto: &UserDTO) -> Result<(), ServiceError> {
    // Use new functional validator internally
    create_user_validator().validate(dto)
}
```

#### Step 3: Use Functional Validator Directly (New Code)
```rust
pub fn create_user(dto: UserDTO, pool: &Pool) -> Result<User, ServiceError> {
    // Direct use of functional validator
    create_user_validator().validate(&dto)?;
    
    // Rest of service logic...
    save_user(dto, pool)
}
```

### 2. Migrating Data Transformations

#### Before: Manual Transformation Chain
```rust
pub fn process_user_registration(input: RegistrationInput) -> Result<User, ServiceError> {
    // Manual transformation steps
    let sanitized = sanitize_registration_input(input)?;
    let validated = validate_registration_data(sanitized)?;
    let enhanced = add_default_values(validated)?;
    let user = convert_to_user_model(enhanced)?;
    
    Ok(user)
}
```

#### After: Pipeline Pattern
```rust
use crate::services::functional_patterns::Pipeline;

pub fn process_user_registration(input: RegistrationInput) -> Result<User, ServiceError> {
    Pipeline::new(input)
        .then(|input| sanitize_registration_input(input))
        .then(|input| validate_registration_data(input))
        .then(|input| add_default_values(input))
        .then(|input| convert_to_user_model(input))
        .execute()
}
```

### 3. Migrating Database Operations

#### Before: Manual Connection Management
```rust
pub fn get_user_with_permissions(user_id: i32, pool: &Pool) -> Result<UserWithPermissions, ServiceError> {
    let mut conn = pool.get().map_err(|e| ServiceError::database_error(e.to_string()))?;
    
    let user = users::table
        .find(user_id)
        .first::<User>(&mut conn)
        .map_err(|e| ServiceError::database_error(e.to_string()))?;
    
    let permissions = user_permissions::table
        .filter(user_permissions::user_id.eq(user_id))
        .load::<Permission>(&mut conn)
        .map_err(|e| ServiceError::database_error(e.to_string()))?;
    
    Ok(UserWithPermissions { user, permissions })
}
```

#### After: QueryReader Pattern
```rust
use crate::services::functional_patterns::QueryReader;

pub fn get_user_with_permissions(user_id: i32, pool: &Pool) -> Result<UserWithPermissions, ServiceError> {
    QueryReader::new(move |conn| {
        users::table
            .find(user_id)
            .first::<User>(conn)
            .map_err(|e| ServiceError::database_error(e.to_string()))
    })
    .and_then(move |user| {
        QueryReader::new(move |conn| {
            user_permissions::table
                .filter(user_permissions::user_id.eq(user_id))
                .load::<Permission>(conn)
                .map_err(|e| ServiceError::database_error(e.to_string()))
        })
        .map(move |permissions| UserWithPermissions { user, permissions })
    })
    .run(pool)
}
```

### 4. Migrating Error Handling

#### Before: Manual Error Propagation
```rust
pub fn process_api_response(response: ApiResponse) -> Result<ProcessedData, ServiceError> {
    if response.status == 200 {
        match serde_json::from_str::<ApiData>(&response.body) {
            Ok(data) => {
                let processed = transform_api_data(data)?;
                Ok(processed)
            }
            Err(e) => Err(ServiceError::parse_error(e.to_string()))
        }
    } else {
        Err(ServiceError::external_error(format!("API error: {}", response.status)))
    }
}
```

#### After: Either Pattern
```rust
use crate::services::functional_patterns::Either;

pub fn process_api_response(response: ApiResponse) -> Result<ProcessedData, ServiceError> {
    let parsed_result = if response.status == 200 {
        match serde_json::from_str::<ApiData>(&response.body) {
            Ok(data) => Either::Right(data),
            Err(e) => Either::Left(ServiceError::parse_error(e.to_string()))
        }
    } else {
        Either::Left(ServiceError::external_error(format!("API error: {}", response.status)))
    };
    
    match parsed_result {
        Either::Right(data) => transform_api_data(data),
        Either::Left(error) => Err(error)
    }
}
```

## Service-by-Service Migration Examples

### Address Book Service Migration

#### Files to Update:
- `src/services/address_book_service.rs`

#### Changes Made (Already Complete ✅):
1. **Validation**: Replaced nested loops with `Validator` combinator
2. **Error Handling**: Enhanced functional error chains
3. **Backward Compatibility**: Maintained `validate_person_dto()` wrapper

#### Migration Pattern:
```rust
// Old validation (removed)
// fn validate_person_dto(dto: &PersonDTO) -> Result<(), ServiceError> { ... }

// New functional validator
fn create_person_validator() -> Validator<PersonDTO> {
    Validator::new()
        .rule(|dto| name_validation_rule(&dto.name))
        .rule(|dto| email_validation_rule(&dto.email))
}

// Backward-compatible wrapper
pub fn validate_person_dto(dto: &PersonDTO) -> Result<(), ServiceError> {
    create_person_validator().validate(dto)
}
```

### Account Service Migration

#### Files to Update:
- `src/services/account_service.rs`

#### Changes Made (Already Complete ✅):
1. **User Validation**: Created `create_user_validator()` with 3 rules
2. **Login Validation**: Created `create_login_validator()` with 2 rules
3. **Function Enhancement**: All auth operations use functional patterns

#### Migration Pattern:
```rust
// User registration with functional validation
pub fn register(user_dto: UserDTO, pool: &Pool) -> Result<String, ServiceError> {
    create_user_validator().validate(&user_dto)?;
    
    // Rest of registration logic...
}

// Login with functional validation
pub fn login(login_dto: LoginDTO, pool: &Pool) -> Result<String, ServiceError> {
    create_login_validator().validate(&login_dto)?;
    
    // Rest of login logic...
}
```

### Future Services to Migrate

#### Tenant Service (`src/services/tenant_service.rs`)

**TODO**: Apply same patterns as address book service

```rust
// Add to tenant_service.rs
fn create_tenant_validator() -> Validator<TenantDTO> {
    Validator::new()
        .rule(|dto| {
            if dto.name.trim().is_empty() {
                Err(ServiceError::bad_request("Tenant name required"))
            } else {
                Ok(())
            }
        })
        .rule(|dto| {
            if dto.database_url.is_empty() {
                Err(ServiceError::bad_request("Database URL required"))
            } else {
                Ok(())
            }
        })
}
```

## Common Migration Patterns

### 1. Validation Rules Extraction

#### Before: Inline Validation
```rust
if field.is_empty() {
    return Err(ServiceError::bad_request("Field required"));
}
if field.len() > 100 {
    return Err(ServiceError::bad_request("Field too long"));
}
```

#### After: Reusable Rules
```rust
fn required_field_rule(field_name: &str) -> impl Fn(&String) -> Result<(), ServiceError> {
    let field_name = field_name.to_string();
    move |value: &String| {
        if value.trim().is_empty() {
            Err(ServiceError::bad_request(format!("{} is required", field_name)))
        } else {
            Ok(())
        }
    }
}

fn max_length_rule(max: usize, field_name: &str) -> impl Fn(&String) -> Result<(), ServiceError> {
    let field_name = field_name.to_string();
    move |value: &String| {
        if value.len() > max {
            Err(ServiceError::bad_request(format!("{} is too long", field_name)))
        } else {
            Ok(())
        }
    }
}
```

### 2. Complex Business Logic

#### Before: Nested Conditions
```rust
pub fn can_user_access_resource(user_id: i32, resource_id: i32, pool: &Pool) -> Result<bool, ServiceError> {
    let user = find_user(user_id, pool)?;
    
    if user.is_admin {
        return Ok(true);
    }
    
    let resource = find_resource(resource_id, pool)?;
    
    if resource.owner_id == user_id {
        return Ok(true);
    }
    
    let permissions = find_user_permissions(user_id, pool)?;
    
    for permission in permissions {
        if permission.resource_type == resource.resource_type && permission.action == "read" {
            return Ok(true);
        }
    }
    
    Ok(false)
}
```

#### After: Functional Composition
```rust
pub fn can_user_access_resource(user_id: i32, resource_id: i32, pool: &Pool) -> Result<bool, ServiceError> {
    let access_check = QueryReader::new(move |conn| find_user(user_id, conn))
        .and_then(move |user| {
            if user.is_admin {
                QueryReader::pure(true)
            } else {
                check_ownership_or_permissions(user_id, resource_id)
            }
        });
    
    access_check.run(pool)
}

fn check_ownership_or_permissions(user_id: i32, resource_id: i32) -> QueryReader<bool> {
    QueryReader::new(move |conn| find_resource(resource_id, conn))
        .and_then(move |resource| {
            if resource.owner_id == user_id {
                QueryReader::pure(true)
            } else {
                check_permissions(user_id, resource.resource_type)
            }
        })
}
```

## Testing Migration

### 1. Dual Testing Approach

During migration, test both old and new implementations:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_compatibility() {
        let dto = PersonDTO {
            name: "".to_string(),
            email: "invalid".to_string(),
        };
        
        // Test old validation (if still exists)
        let old_result = validate_person_dto_old(&dto);
        
        // Test new validation
        let new_result = validate_person_dto(&dto);
        
        // Both should have same result
        assert_eq!(old_result.is_err(), new_result.is_err());
    }
    
    #[test]
    fn test_functional_validator_directly() {
        let validator = create_person_validator();
        
        // Test individual rules
        let invalid_name = PersonDTO {
            name: "".to_string(),
            email: "valid@email.com".to_string(),
        };
        assert!(validator.validate(&invalid_name).is_err());
        
        // Test valid case
        let valid_dto = PersonDTO {
            name: "Valid Name".to_string(),
            email: "valid@email.com".to_string(),
        };
        assert!(validator.validate(&valid_dto).is_ok());
    }
}
```

### 2. Performance Testing

Compare performance between old and new approaches:

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_validation_performance() {
        let dto = create_test_dto();
        let iterations = 10000;
        
        // Benchmark new functional validation
        let start = Instant::now();
        let validator = create_person_validator();
        for _ in 0..iterations {
            let _ = validator.validate(&dto);
        }
        let functional_duration = start.elapsed();
        
        println!("Functional validation: {:?}", functional_duration);
        
        // Ensure performance is comparable
        assert!(functional_duration.as_millis() < 100); // Adjust threshold as needed
    }
}
```

## Rollback Strategy

If issues arise during migration:

### 1. Immediate Rollback
```rust
// Keep old functions available during transition
#[deprecated(note = "Use create_person_validator() instead")]
pub fn validate_person_dto_legacy(dto: &PersonDTO) -> Result<(), ServiceError> {
    // Original implementation
}

// Switch back if needed
pub fn validate_person_dto(dto: &PersonDTO) -> Result<(), ServiceError> {
    // Uncomment to rollback:
    // validate_person_dto_legacy(dto)
    
    // Current functional implementation:
    create_person_validator().validate(dto)
}
```

### 2. Feature Flags
```rust
pub fn validate_person_dto(dto: &PersonDTO) -> Result<(), ServiceError> {
    if cfg!(feature = "legacy_validation") {
        validate_person_dto_legacy(dto)
    } else {
        create_person_validator().validate(dto)
    }
}
```

## Migration Checklist

### For Each Service:

- [ ] **Phase 1: Validation Migration**
  - [ ] Create functional validators using `Validator` combinator
  - [ ] Refactor existing validation functions to use new validators
  - [ ] Add comprehensive tests for new validators
  - [ ] Ensure backward compatibility

- [ ] **Phase 2: Pattern Adoption**
  - [ ] Apply `Pipeline` pattern for data transformations
  - [ ] Use `QueryReader` for database operation composition
  - [ ] Implement `Either` type for dual-outcome scenarios
  - [ ] Add `Retry` pattern for external service calls

- [ ] **Phase 3: Optimization**
  - [ ] Add `Memoization` for expensive pure functions
  - [ ] Remove legacy code once migration is stable
  - [ ] Performance testing and optimization
  - [ ] Documentation updates

### Quality Assurance:

- [ ] All existing tests pass
- [ ] New functional pattern tests added
- [ ] Performance benchmarks show no regression
- [ ] Error handling maintains same behavior
- [ ] API compatibility preserved

## Resources

- [Developer Guide](./DEVELOPER-GUIDE-FUNCTIONAL-PATTERNS.md) - How to use functional patterns
- [Best Practices](./BEST-PRACTICES-FUNCTIONAL-PATTERNS.md) - Recommended patterns and anti-patterns
- [ADR-001](./ADR-001-FUNCTIONAL-PATTERNS.md) - Architecture decision rationale
- [FP-013 Summary](./FP-013_SERVICE_LAYER_REFACTORING_SUMMARY.md) - Complete refactoring summary