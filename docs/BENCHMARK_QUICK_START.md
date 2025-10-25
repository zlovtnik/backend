# Functional Patterns Performance Benchmarks - Quick Start Guide

## Quick Start

```bash
# Build benchmarks
cargo build --benches

# Run all benchmarks
cargo bench --bench functional_benchmarks

# Run with smaller sample size (faster, less precise)
cargo bench --bench functional_benchmarks -- --sample-size 10

# Run specific benchmark
cargo bench --bench functional_benchmarks -- validation
cargo bench --bench functional_benchmarks -- query_reader_overhead
cargo bench --bench functional_benchmarks -- memoization
cargo bench --bench functional_benchmarks -- error_propagation
```

## Benchmark Groups

The comprehensive benchmark suite includes:

### Validation Performance
**Command**: `cargo bench --bench functional_benchmarks -- validation`

Compares:
- Functional validator combinator pattern
- Imperative validation loops
- Tests with 10, 100, 1000 DTOs

### QueryReader Overhead
**Command**: `cargo bench --bench functional_benchmarks -- query_reader_overhead`

Measures:
- Direct function calls (baseline)
- QueryReader wrapped calls
- Chained operations
- Tests 1K, 10K, 100K operations

### Memoization Efficiency
**Command**: `cargo bench --bench functional_benchmarks -- memoization`

Profiles:
- No cache vs with cache
- Cache hit rates: 0%, 50%, 90%
- Expensive computation caching

### Error Propagation
**Command**: `cargo bench --bench functional_benchmarks -- error_propagation`

Compares:
- Functional Result chaining
- Imperative error checks
- 1K, 5K, 10K data items

## Existing Benchmarks

The suite also includes:

- `data_filtering` - Filter chains vs loops
- `data_transformation` - Iterator composition vs imperative
- `complex_pipeline` - Multi-step transformations
- `parallel_processing` - Sequential vs rayon parallel
- `memory_efficiency` - Lazy vs eager evaluation
- `iterator_composition` - Chained operations
- `grouping_aggregation` - Grouping operations
- `error_handling` - Error handling patterns
- `validation_rule_scaling` - Rule count impact
- `validation_performance` - Core validation comparison

## Generating Reports

```bash
# HTML report generation (auto-generated in target/criterion/)
cargo bench --bench functional_benchmarks

# View reports
open target/criterion/report/index.html  # macOS
xdg-open target/criterion/report/index.html  # Linux
```

## Advanced Options

```bash
# Custom measurement time (seconds)
cargo bench --bench functional_benchmarks -- --measurement-time 10

# Verbose output
cargo bench --bench functional_benchmarks -- --verbose

# Run with profiling (requires cargo-flamegraph)
cargo install flamegraph
cargo flamegraph --bench functional_benchmarks

# Custom warm-up time
cargo bench --bench functional_benchmarks -- --warm-up-time 3
```

## Performance Baseline

Expected overhead/improvements for functional patterns:

| Pattern | Overhead | Status |
|---|---|---|
| Validation Combinator | 0-5% | ✅ Minimal |
| QueryReader | 0% | ✅ Zero-cost |
| Memoization (90% hits) | -80% | ✅ Major improvement |
| Error Propagation | 0-10% | ✅ Comparable |

## Notes

- Benchmarks use Criterion for statistically significant results
- HTML reports provide detailed comparisons and charts
- Run on your target hardware for meaningful results
- Warm-up runs ensure consistent performance
- Statistical analysis included automatically

## Troubleshooting

### Benchmarks hang
- Use `--sample-size 10` for faster runs
- Press Ctrl+C to interrupt

### Out of memory
- Reduce test data sizes in benchmark functions
- Run individual benchmark groups

### Inaccurate results
- Close other applications
- Run multiple times
- Use default sample size (100)

## Documentation

Full benchmark specification: [`docs/PERFORMANCE_BENCHMARKS.md`](./PERFORMANCE_BENCHMARKS.md)

Implementation: [`benches/functional_benchmarks.rs`](../benches/functional_benchmarks.rs)

Functional Patterns: [`src/services/functional_patterns.rs`](../src/services/functional_patterns.rs)
