# Quick Reference: Concurrent Processing & Response Transformers

## Parallel Iterator Patterns

### par_flat_map
```rust
// Flatten nested structures in parallel
let nested = vec![vec![1, 2], vec![3, 4]];
let flat = nested.into_iter()
    .par_flat_map(&config, |v| v.into_iter())
    .data; // [1, 2, 3, 4]
```

### par_partition
```rust
// Split into two groups
let numbers = vec![1, 2, 3, 4, 5, 6];
let (evens, odds) = numbers.into_iter()
    .par_partition(&config, |&x| x % 2 == 0)
    .data; // evens: [2,4,6], odds: [1,3,5]
```

### par_find
```rust
// Early termination search
let data = vec![1, 2, 3, 4, 5];
let found = data.into_iter()
    .par_find(&config, |&x| x > 3)
    .data; // Some(4) or Some(5)
```

## Dynamic Load Balancing

### Basic Usage
```rust
let balancer = DynamicLoadBalancer::new(0.8); // 80% target efficiency
let chunk_size = balancer.calculate_chunk_size(10000, 8);
```

### With Learning
```rust
// Record performance
balancer.record_sample(chunk_size, efficiency, thread_utilization);

// Get statistics
let stats = balancer.get_stats();
println!("Efficiency: {:.2}", stats.avg_efficiency);
```

## Response Formats

### XML
```rust
ResponseTransformer::new(data)
    .allow_format(ResponseFormat::Xml)
    .respond_to(&req);
// Content-Type: application/xml
```

### CSV
```rust
ResponseTransformer::new(data)
    .allow_format(ResponseFormat::Csv)
    .respond_to(&req);
// Content-Type: text/csv
```

### MessagePack
```rust
ResponseTransformer::new(data)
    .allow_format(ResponseFormat::MessagePack)
    .respond_to(&req);
// Content-Type: application/msgpack
```

## Content Negotiation

### Quality Values
```http
Accept: application/json;q=0.8, text/csv;q=0.9
```
```rust
// Automatically selects CSV (higher q-value)
ResponseTransformer::new(data)
    .allow_format(ResponseFormat::Json)
    .allow_format(ResponseFormat::Csv)
    .respond_to(&req);
```

### Wildcards
```http
Accept: text/*
```
```rust
// Prefers text/plain over text/csv
ResponseTransformer::new(data)
    .allow_format(ResponseFormat::Text)
    .allow_format(ResponseFormat::Csv)
    .respond_to(&req);
```

## ConcurrentProcessor Methods

```rust
let processor = ConcurrentProcessor::try_default()?;

// Flat map
processor.flat_map(data, |x| expand(x));

// Partition
processor.partition(data, |x| x.is_valid());

// Find
processor.find(data, |x| x.matches());
```

## Common Patterns

### Process & Split
```rust
let (valid, invalid) = processor.partition(records, |r| r.validate().is_ok());
```

### Expand & Flatten
```rust
let expanded = processor.flat_map(items, |item| item.children());
```

### Search Large Dataset
```rust
let result = processor.find(huge_dataset, |item| item.id == target_id);
```

### Multi-Format API
```rust
ResponseTransformer::new(data)
    .allow_format(ResponseFormat::Json)
    .allow_format(ResponseFormat::Xml)
    .allow_format(ResponseFormat::Csv)
    .respond_to(&req);
```

## Performance Tips

1. **Use `par_partition`** instead of two `par_filter` calls
2. **Use `par_find`** for existence checks (early termination)
3. **Enable adaptive chunk sizing** for variable workloads
4. **Specify quality values** in Accept headers for preferences
5. **Use `DynamicLoadBalancer`** for long-running operations

## Testing

```bash
# Run all tests
cargo test --features functional

# Run specific pattern tests
cargo test --features functional test_par_flat_map
cargo test --features functional test_par_partition
cargo test --features functional test_par_find

# Run content negotiation tests
cargo test --features functional negotiate_format_with_quality_values
```
