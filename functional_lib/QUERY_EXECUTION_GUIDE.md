# Query Execution with FunctionalQueryComposer

This guide explains how to use the `execute_chunk_query` method in the `FunctionalQueryComposer` to execute database queries with lazy evaluation.

## Overview

The `execute_chunk_query` method enables chunked query execution for lazy loading of large datasets. It integrates with the `TypeSafeQueryBuilder` to provide:

- Automatic pagination with LIMIT and OFFSET
- Type-safe query building
- Lazy evaluation for memory efficiency
- Custom query execution strategies

## Architecture

The implementation uses a query executor pattern to decouple generic query composition from table-specific query execution:

```
FunctionalQueryComposer
  ├─ TypeSafeQueryBuilder (generic, type-safe)
  ├─ Pool (database connection pool)
  └─ QueryExecutor (table-specific execution logic)
```

## Usage

### Step 1: Define a Query Executor

A query executor is a function that knows how to build and execute queries for a specific table type:

```rust
use crate::query_builders::TenantQueryBuilder;
use crate::query_composition::Pool;
use diesel::prelude::*;

// Example executor for TenantQueryBuilder
let tenant_executor = |builder: &TenantQueryBuilder, pool: &Pool| -> Result<Vec<Tenant>, String> {
    // Get a connection from the pool
    let mut conn = pool.get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    
    // Build the query (note: we need to reconstruct a TenantQueryBuilder from the generic builder)
    // In practice, you would cast or use the builder directly if it's already the right type
    let query = builder.clone().build_tenant_query()
        .map_err(|e| format!("Failed to build query: {}", e))?;
    
    // Execute the query
    query.load(&mut conn)
        .map_err(|e| format!("Query execution failed: {}", e))
};
```

### Step 2: Create a FunctionalQueryComposer

Configure the composer with the pool and executor:

```rust
use crate::query_composition::FunctionalQueryComposer;

let composer: FunctionalQueryComposer<tenants::table, Tenant> = 
    FunctionalQueryComposer::new()
        .with_pool(pool.clone())
        .with_query_executor(tenant_executor);
```

### Step 3: Execute Chunked Queries

Use the composer to execute queries with automatic pagination:

```rust
// Load the first 100 records
let chunk = composer.execute_chunk_query(0, 100)?;
println!("Loaded {} records", chunk.len());

// Load the next 100 records
let next_chunk = composer.execute_chunk_query(100, 100)?;
```

### Step 4: Use with LazyQueryIterator

The real power comes when using with `LazyQueryIterator` for automatic chunking:

```rust
use crate::query_composition::LazyQueryIterator;
use std::sync::Arc;
use tokio::sync::Semaphore;

let composer_arc = Arc::new(composer);
let semaphore = Arc::new(Semaphore::new(4));
let metrics = QueryPerformanceMetrics::default();

let mut iterator = LazyQueryIterator::new(composer_arc, semaphore, metrics);

// Process records lazily
for result in iterator {
    match result {
        Ok(record) => {
            // Process record
            println!("Processing: {:?}", record);
        }
        Err(e) => {
            eprintln!("Error loading chunk: {}", e);
            break;
        }
    }
}
```

## Complete Example

Here's a complete example showing how to implement a query executor for a custom table:

```rust
use diesel::prelude::*;
use crate::query_composition::{FunctionalQueryComposer, Pool, LazyEvaluationConfig};
use crate::query_builder::{TypeSafeQueryBuilder, QueryFilter};
use crate::schema::users;

#[derive(Queryable, Clone, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}

// Define a type alias for the specific query builder
pub type UserQueryBuilder = TypeSafeQueryBuilder<users::table, User>;

// Implement a builder method if needed (similar to TenantQueryBuilder)
impl UserQueryBuilder {
    pub fn build_user_query(self) -> Result<users::BoxedQuery<'static, Pg>, String> {
        let mut query = users::table.into_boxed();
        
        // Apply filters, ordering, limit, offset
        // ... (implementation details)
        
        if let Some(limit) = self.limit_value() {
            query = query.limit(limit);
        }
        if let Some(offset) = self.offset_value() {
            query = query.offset(offset);
        }
        
        Ok(query)
    }
}

// Create the executor
fn create_user_executor() -> impl Fn(&UserQueryBuilder, &Pool) -> Result<Vec<User>, String> + Send + Sync + 'static {
    move |builder: &UserQueryBuilder, pool: &Pool| {
        let mut conn = pool.get()
            .map_err(|e| format!("Connection error: {}", e))?;
        
        let query = builder.clone().build_user_query()?;
        
        query.load(&mut conn)
            .map_err(|e| format!("Query error: {}", e))
    }
}

// Usage
fn process_users(pool: Pool) -> Result<(), String> {
    let executor = create_user_executor();
    
    let composer: FunctionalQueryComposer<users::table, User> = 
        FunctionalQueryComposer::new()
            .with_pool(pool)
            .with_query_executor(executor);
    
    // Process users in chunks of 1000
    let mut offset = 0;
    let chunk_size = 1000;
    
    loop {
        let chunk = composer.execute_chunk_query(offset, chunk_size)?;
        
        if chunk.is_empty() {
            break;
        }
        
        // Process the chunk
        for user in chunk {
            println!("Processing user: {}", user.name);
        }
        
        offset += chunk_size;
    }
    
    Ok(())
}
```

## Error Handling

The `execute_chunk_query` method returns descriptive errors:

1. **Missing Pool**: `"Database pool not configured. Use with_pool() to set it."`
2. **Missing Executor**: `"Query executor not configured. Use with_query_executor() to provide..."`
3. **Query Execution Errors**: Errors from the executor function

## Best Practices

1. **Reuse Executors**: Create executor functions once and reuse them across composers
2. **Error Mapping**: Convert Diesel errors to strings for consistent error handling
3. **Connection Pooling**: Always use a connection pool for efficient resource management
4. **Chunk Size**: Choose chunk sizes based on your data and memory constraints (typically 100-1000 records)
5. **Testing**: Use mock executors for testing without requiring a database

## Testing

Example of testing with a mock executor:

```rust
#[test]
fn test_user_query_execution() {
    let mock_executor = |builder: &UserQueryBuilder, _pool: &Pool| {
        // Verify pagination was applied
        assert_eq!(builder.limit_value(), Some(10));
        assert_eq!(builder.offset_value(), Some(0));
        
        // Return mock data
        Ok(vec![
            User { 
                id: "1".to_string(), 
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
        ])
    };
    
    let composer = FunctionalQueryComposer::new()
        .with_pool(test_pool())
        .with_query_executor(mock_executor);
    
    let result = composer.execute_chunk_query(0, 10);
    assert!(result.is_ok());
}
```

## Migration from TODO

The original TODO was:

```rust
pub fn execute_chunk_query(&self, _offset: usize, _limit: usize) -> Result<Vec<U>, String> {
    // TODO: Implement actual query execution with TypeSafeQueryBuilder
    Err("execute_chunk_query not yet implemented".to_string())
}
```

This has been replaced with a full implementation that:

1. Validates the pool and executor are configured
2. Reconstructs the query builder with pagination parameters
3. Delegates to the table-specific executor for actual query execution
4. Returns results or descriptive errors

## See Also

- `query_builder.rs` - Type-safe query building
- `query_builders.rs` - Table-specific query builders (e.g., TenantQueryBuilder)
- `query_composition.rs` - Functional query composition and lazy evaluation
