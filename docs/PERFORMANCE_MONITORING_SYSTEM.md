# Performance Monitoring and Metrics Collection System

## Overview

This document provides a comprehensive guide to the performance monitoring and metrics collection system implemented in the Actix-web REST API. The system provides real-time performance tracking, adaptive optimization, and comprehensive benchmarking for functional programming patterns.

## Core Components

### 1. Parallel Pipeline Metrics Accumulation

The `ParallelPipeline` accumulates performance metrics across chained parallel operations, providing detailed insights into pipeline efficiency.

#### Implementation Location: `src/functional/parallel_iterators.rs`

```rust
pub struct ParallelPipeline<T> {
    data: Vec<T>,
    config: ParallelConfig,
    metrics_history: Vec<ParallelMetrics>,
}
```

#### Key Features:
- **Metrics Accumulation**: Each operation (map, filter, sort) appends its metrics to the history
- **Non-Consuming Access**: `with_metrics()` provides read-only access to metrics without consuming the pipeline
- **Aggregated Summary**: `metrics_summary()` combines all metrics into a comprehensive overview
- **Flexible Consumption**: `into_metrics()` consumes the pipeline and returns the complete metrics history

#### Usage Examples:

**Basic Pipeline with Metrics**:
```rust
use rcs::functional::parallel_iterators::{ParallelConfig, ParallelPipeline};

let data: Vec<i32> = (1..=100).collect();
let config = ParallelConfig::default();

// Create pipeline and chain operations
let pipeline = ParallelPipeline::new(data, config)
    .map(|x| x * 2)
    .filter(|&x| x % 4 == 0)
    .map(|x| x / 2);

// Access individual operation metrics
let metrics_history = pipeline.with_metrics();
println!("Operations recorded: {}", metrics_history.len());
for (i, metrics) in metrics_history.iter().enumerate() {
    println!("Operation {}: {}", i + 1, metrics);
}

// Get aggregated summary
let summary = pipeline.metrics_summary();
println!("Total time: {:?}", summary.total_time);
println!("Average efficiency: {:.2}", summary.efficiency);

// Execute pipeline
let result = pipeline.execute();
```

**Consuming Pipeline with Metrics Extraction**:
```rust
let pipeline = ParallelPipeline::new(data, config)
    .map(|x| x * 3)
    .sort();

// Extract metrics and data separately
let metrics = pipeline.into_metrics(); // Consumes pipeline
// Data is implicitly executed and discarded
println!("Extracted {} operation metrics", metrics.len());
```

### 2. Adaptive Performance Optimization

The system learns from performance history to dynamically optimize chunk sizing for parallel operations.

#### Implementation Location: `src/functional/parallel_iterators.rs`

```rust
fn calculate_adaptive_chunk_size(
    operation_key: &str,
    data_size: usize,
    thread_count: usize,
    base_chunk_size: usize,
    max_chunk_size: usize,
) -> usize
```

#### Key Features:
- **Historical Learning**: Analyzes past performance data for similar operations
- **Weighted Efficiency**: Uses efficiency-weighted averages for optimal chunk size selection
- **Size Range Filtering**: Considers only operations with similar data sizes (±20%)
- **Bounds Enforcement**: Ensures chunk sizes stay within reasonable limits

#### How It Works:

1. **Operation Key Generation**: Creates unique identifiers for operation types
2. **Historical Data Retrieval**: Fetches performance history from global storage
3. **Similarity Filtering**: Finds operations with comparable data sizes
4. **Weighted Calculation**: Computes optimal chunk size based on past efficiency
5. **Performance Recording**: Stores new performance data for future optimization

#### Usage in Parallel Operations:
```rust
let chunk_size = if config.adaptive_chunk_sizing {
    calculate_adaptive_chunk_size(
        &operation_key,
        data_len,
        rayon::current_num_threads(),
        base_chunk_size,
        config.max_chunk_size,
    )
} else {
    base_chunk_size
};
```

### 3. Real-Time Performance Monitoring

Global performance monitoring with threshold-based alerting and health check integration.

#### Implementation Location: `src/functional/performance_monitoring.rs`

```rust
pub struct PerformanceMonitor {
    metrics: RwLock<HashMap<OperationType, PerformanceMetrics>>,
    config: PerformanceConfig,
    thresholds: RwLock<HashMap<OperationType, PerformanceThreshold>>,
}
```

#### Key Features:
- **Operation Type Tracking**: Monitors different types of functional operations
- **Rolling Averages**: Maintains running statistics for execution time and memory usage
- **Threshold Alerts**: Configurable limits with automatic logging of violations
- **Health Summary**: Integration with health check endpoints
- **Sampling Support**: Configurable sampling rates for performance optimization

#### Operation Types Monitored:
```rust
pub enum OperationType {
    IteratorChain,
    PureFunctionCall,
    StateTransition,
    QueryComposition,
    ValidationPipeline,
    LazyPipeline,
    ConcurrentProcessing,
    ResponseTransformation,
    Custom(String),
}
```

#### Usage Examples:

**Measuring Operations with Macro**:
```rust
use rcs::measure_operation;

let result = measure_operation!(OperationType::IteratorChain, {
    // Your functional operation here
    data.into_iter()
        .map(|x| x * 2)
        .filter(|&x| x > 10)
        .collect::<Vec<_>>()
});

// Result handling with automatic performance measurement
match result {
    Ok(data) => println!("Processed {} items", data.len()),
    Err(e) => println!("Error: {}", e),
}
```

**Manual Performance Measurement**:
```rust
let monitor = get_performance_monitor();
let measurement = monitor.start_measurement(OperationType::QueryComposition);

// Perform operation
let result = expensive_query_operation();

// Complete measurement
if let Some(m) = measurement {
    m.complete(); // or m.complete_with_error() on failure
}
```

**Setting Thresholds**:
```rust
let monitor = get_performance_monitor();
let threshold = PerformanceThreshold {
    max_execution_time: Duration::from_millis(100),
    max_memory_per_operation: 1024 * 1024, // 1MB
    max_error_rate: 0.05, // 5%
};

monitor.set_threshold(OperationType::ValidationPipeline, threshold);
```

### 4. Functional vs Imperative Performance Validation

Comprehensive benchmarking suite comparing functional programming patterns against imperative alternatives.

#### Implementation Location: `benches/functional_benchmarks.rs`

#### Benchmark Categories:

**Validation Performance**:
- Functional validator combinators vs imperative nested loops
- Rule scaling impact (1, 3, 5, 10 rules)
- Memory efficiency comparisons

**Data Processing**:
- Iterator chain composition
- Parallel processing efficiency
- Memory allocation patterns

**Error Handling**:
- Result chaining (`?` operator) vs explicit error checks
- Exception propagation overhead

**Memoization Benefits**:
- Cache hit rate impact (0%, 50%, 90%)
- Computation avoidance savings

#### Running Benchmarks:
```bash
# Run all functional benchmarks
cargo bench --bench functional_benchmarks

# Run specific benchmark group
cargo bench --bench functional_benchmarks validation_performance

# Run with detailed output
cargo bench --bench functional_benchmarks -- --verbose
```

#### Key Findings from Benchmarks:

1. **Functional patterns are competitive**: Zero-cost abstractions in most cases
2. **Validation scaling**: Functional combinators maintain performance as rule count increases
3. **Memory efficiency**: Functional pipelines often use less memory due to iterator laziness
4. **Error handling**: Result chaining has minimal overhead compared to imperative checks

## Integration Examples

### Controller-Level Performance Monitoring

```rust
use rcs::measure_operation;
use rcs::functional::performance_monitoring::OperationType;

#[get("/users")]
async fn list_users(context: web::Data<AppContext>) -> Result<HttpResponse, Error> {
    let result = measure_operation!(OperationType::QueryComposition, {
        let users = user_service::find_all_users(limit, offset, &pool)?;
        Ok(HttpResponse::Ok().json(users))
    });

    result
}
```

### Service-Level Metrics Collection

```rust
use rcs::functional::parallel_iterators::{ParallelConfig, ParallelPipeline};

pub fn process_user_data(users: Vec<UserDTO>, config: &ParallelConfig)
    -> Result<Vec<ProcessedUser>, ServiceError>
{
    let pipeline = ParallelPipeline::new(users, config.clone())
        .map(|user| validate_user(user))
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap())
        .map(|user| process_user(user));

    // Get performance insights
    let metrics_summary = pipeline.metrics_summary();
    log::info!("User processing efficiency: {:.2}", metrics_summary.efficiency);

    Ok(pipeline.execute())
}
```

### Health Check Integration

```rust
use rcs::functional::performance_monitoring::get_performance_monitor;

#[get("/health")]
async fn health_check() -> impl Responder {
    let monitor = get_performance_monitor();
    let summary = monitor.get_health_summary();

    HttpResponse::Ok().json(json!({
        "status": "healthy",
        "performance": {
            "total_operations": summary.total_operations,
            "error_rate": summary.error_rate,
            "slowest_operation_ms": summary.slowest_operation.as_millis(),
            "highest_memory_usage": summary.highest_memory_usage,
            "operation_types_tracked": summary.operation_types_tracked
        }
    }))
}
```

## Configuration

### Performance Monitor Configuration

```rust
use rcs::functional::performance_monitoring::{init_performance_monitor, PerformanceConfig};

let config = PerformanceConfig {
    enabled: true,
    max_operation_types: 100,
    memory_tracking_enabled: true,
    sampling_rate: 1.0, // Track all operations
};

init_performance_monitor(config);
```

### Parallel Processing Configuration

```rust
use rcs::functional::parallel_iterators::ParallelConfig;

let config = ParallelConfig {
    thread_pool_size: 0, // Automatic
    min_parallel_size: 1024,
    enable_work_stealing: true,
    chunk_size: 1024,
    adaptive_chunk_sizing: true,
    max_chunk_size: 8192,
};
```

## Performance Characteristics

### Memory Usage
- **Functional Pipelines**: Lower memory footprint due to lazy evaluation
- **Parallel Operations**: Efficient memory usage with configurable chunking
- **Metrics Storage**: Minimal overhead with configurable retention

### CPU Utilization
- **Adaptive Chunking**: Optimizes thread utilization based on historical data
- **Work Stealing**: Balances load across available CPU cores
- **Sampling**: Reduces monitoring overhead for high-frequency operations

### Scalability
- **Horizontal Scaling**: Performance monitoring scales with operation volume
- **Memory Bounded**: Configurable limits prevent unbounded growth
- **Concurrent Access**: Thread-safe metrics collection and retrieval

## Monitoring and Alerting

### Automatic Threshold Alerts
The system automatically logs warnings when operations exceed configured thresholds:

```
Performance alert: validation_pipeline operation exceeds time threshold. Average: 150ms, Threshold: 100ms
Performance alert: query_composition operation exceeds memory threshold. Average: 2MB, Threshold: 1MB
```

### Health Check Endpoints
Integrates with existing health check systems to provide performance insights:

```json
{
  "status": "healthy",
  "performance": {
    "total_operations": 15420,
    "error_rate": 0.023,
    "slowest_operation_ms": 450,
    "highest_memory_usage": 2097152,
    "operation_types_tracked": 8
  }
}
```

## Future Enhancements

Based on the roadmap, planned improvements include:

1. **Distributed Tracing**: Cross-service operation correlation
2. **Metrics Dashboard**: Web-based visualization of performance data
3. **Predictive Optimization**: ML-based performance prediction and optimization
4. **Resource Pool Monitoring**: Database connection and thread pool metrics
5. **Custom Metrics**: Domain-specific performance indicators

## References

- [Functional Programming Patterns Implementation](./FUNCTIONAL_PATTERNS_IMPLEMENTATION_GUIDE.md)
- [ADR-001: Functional Programming Patterns](./ADR-001-FUNCTIONAL-PATTERNS.md)
- [Benchmark Results](./benchmark_results/) (generated by `cargo bench`)
