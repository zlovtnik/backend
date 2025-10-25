//! # Functional Programming Performance Benchmarks
//!
//! Comprehensive performance benchmarks for functional patterns used in the RCS service layer.
//! Compares functional vs imperative approaches to measure:
//! - Validation performance (old loops vs functional composition)
//! - QueryReader overhead (expected: zero-cost abstraction)
//! - Memoization cache efficiency and hit rates
//! - Iterator-based vs imperative data processing
//! - Error handling patterns
//! - Parallel processing capabilities

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use itertools::Itertools;
use rayon::prelude::*;
use std::time::Duration;

/// Test data structure for benchmarking
#[derive(Debug, Clone)]
pub struct BenchmarkPerson {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub age: u32,
    pub active: bool,
    pub score: f64,
}

impl BenchmarkPerson {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Person {}", id),
            email: format!("person{}@example.com", id),
            age: 20 + (id % 50),
            active: id % 3 == 0,
            score: (id as f64) * 1.5 + 10.0,
        }
    }
}

/// Generate test data for benchmarking
pub fn generate_test_data(size: usize) -> Vec<BenchmarkPerson> {
    (0..size).map(|i| BenchmarkPerson::new(i as u32)).collect()
}

/// Benchmark: Data filtering performance
pub fn benchmark_data_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_filtering");

    for size in [100, 1000, 10000].iter() {
        let data = generate_test_data(*size);

        // Functional approach
        group.bench_with_input(BenchmarkId::new("functional", size), &data, |b, data| {
            b.iter(|| {
                let result: Vec<_> = data
                    .iter()
                    .filter(|person| person.active && person.age > 25)
                    .collect();
                black_box(result)
            })
        });

        // Imperative approach
        group.bench_with_input(BenchmarkId::new("imperative", size), &data, |b, data| {
            b.iter(|| {
                let mut result = Vec::new();
                for person in data {
                    if person.active && person.age > 25 {
                        result.push(person);
                    }
                }
                black_box(result)
            })
        });
    }

    group.finish();
}

/// Benchmark: Data transformation performance
pub fn benchmark_data_transformation(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_transformation");

    for size in [100, 1000, 10000].iter() {
        let data = generate_test_data(*size);

        // Functional approach with iterator chains
        group.bench_with_input(
            BenchmarkId::new("functional_chains", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let result: Vec<_> = data
                        .iter()
                        .filter(|p| p.active)
                        .map(|p| (p.id, p.name.clone(), p.score * 2.0))
                        .collect();
                    black_box(result)
                })
            },
        );

        // Imperative approach
        group.bench_with_input(BenchmarkId::new("imperative", size), &data, |b, data| {
            b.iter(|| {
                let mut result = Vec::new();
                for person in data {
                    if person.active {
                        result.push((person.id, person.name.clone(), person.score * 2.0));
                    }
                }
                black_box(result)
            })
        });
    }

    group.finish();
}

/// Benchmark: Complex data processing pipeline
pub fn benchmark_complex_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_pipeline");

    for size in [1000, 5000, 10000].iter() {
        let data = generate_test_data(*size);

        // Functional approach with complex pipeline
        group.bench_with_input(
            BenchmarkId::new("functional_pipeline", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let result: Vec<_> = data
                        .iter()
                        .filter(|p| p.active && p.age >= 21)
                        .map(|p| (p.id, p.score))
                        .filter(|(_, score)| *score > 50.0)
                        .map(|(id, score)| (id, score * 1.1))
                        .sorted_by(|a, b| b.1.partial_cmp(&a.1).unwrap())
                        .take(100)
                        .collect();
                    black_box(result)
                })
            },
        );

        // Imperative approach
        group.bench_with_input(BenchmarkId::new("imperative", size), &data, |b, data| {
            b.iter(|| {
                let mut intermediate = Vec::new();

                // Filter and transform
                for person in data {
                    if person.active && person.age >= 21 && person.score > 50.0 {
                        intermediate.push((person.id, person.score * 1.1));
                    }
                }

                // Sort
                intermediate.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                // Take top 100
                let result: Vec<_> = intermediate.into_iter().take(100).collect();
                black_box(result)
            })
        });
    }

    group.finish();
}

/// Benchmark: Parallel processing performance
pub fn benchmark_parallel_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_processing");

    for size in [1000, 10000, 100000].iter() {
        let data = generate_test_data(*size);

        // Sequential functional approach
        group.bench_with_input(BenchmarkId::new("sequential", size), &data, |b, data| {
            b.iter(|| {
                let result: Vec<_> = data
                    .iter()
                    .filter(|p| p.active)
                    .map(|p| expensive_computation(p.score))
                    .collect();
                black_box(result)
            })
        });

        // Parallel functional approach with rayon
        group.bench_with_input(BenchmarkId::new("parallel", size), &data, |b, data| {
            b.iter(|| {
                let result: Vec<_> = data
                    .par_iter()
                    .filter(|p| p.active)
                    .map(|p| expensive_computation(p.score))
                    .collect();
                black_box(result)
            })
        });
    }

    group.finish();
}

/// Benchmark: Memory efficiency of functional vs imperative approaches
pub fn benchmark_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");
    group.measurement_time(Duration::from_secs(10));

    for size in [1000, 10000].iter() {
        let data = generate_test_data(*size);

        // Functional lazy evaluation
        group.bench_with_input(
            BenchmarkId::new("lazy_functional", size),
            &data,
            |b, data| {
                b.iter(|| {
                    // Using iterator adapters (lazy evaluation)
                    let result = data
                        .iter()
                        .filter(|p| p.active)
                        .map(|p| p.score * 2.0)
                        .take(50)
                        .collect::<Vec<_>>();
                    black_box(result)
                })
            },
        );

        // Imperative eager evaluation
        group.bench_with_input(
            BenchmarkId::new("eager_imperative", size),
            &data,
            |b, data| {
                b.iter(|| {
                    // Creating intermediate vectors
                    let mut filtered = Vec::new();
                    for person in data {
                        if person.active {
                            filtered.push(person);
                        }
                    }

                    let mut transformed = Vec::new();
                    for person in &filtered {
                        transformed.push(person.score * 2.0);
                    }

                    let result: Vec<_> = transformed.into_iter().take(50).collect();
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Iterator composition performance
pub fn benchmark_iterator_composition(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterator_composition");

    for size in [1000, 5000, 10000].iter() {
        let data: Vec<i32> = (0..*size).collect();

        // Chained iterator operations
        group.bench_with_input(
            BenchmarkId::new("chained_iterators", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let result: Vec<_> = data
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| i % 2 == 0)
                        .map(|(_, &x)| x * 2)
                        .filter(|&x| x > 100)
                        .take(100)
                        .collect();
                    black_box(result)
                })
            },
        );

        // Multiple separate loops
        group.bench_with_input(
            BenchmarkId::new("separate_loops", size),
            &data,
            |b, data| {
                b.iter(|| {
                    // Step 1: Filter by index
                    let mut step1 = Vec::new();
                    for (i, &x) in data.iter().enumerate() {
                        if i % 2 == 0 {
                            step1.push(x);
                        }
                    }

                    // Step 2: Transform
                    let mut step2 = Vec::new();
                    for &x in &step1 {
                        step2.push(x * 2);
                    }

                    // Step 3: Filter by value
                    let mut step3 = Vec::new();
                    for &x in &step2 {
                        if x > 100 {
                            step3.push(x);
                        }
                    }

                    // Step 4: Take first 100
                    let result: Vec<_> = step3.into_iter().take(100).collect();
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Grouping and aggregation operations
pub fn benchmark_grouping_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("grouping_aggregation");

    for size in [1000, 5000, 10000].iter() {
        let data = generate_test_data(*size);

        // Functional approach with itertools
        group.bench_with_input(
            BenchmarkId::new("functional_itertools", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let result: Vec<_> = data
                        .iter()
                        .filter(|p| p.active)
                        .into_group_map_by(|p| p.age / 10) // Group by decade
                        .into_iter()
                        .map(|(decade, people)| {
                            let avg_score =
                                people.iter().map(|p| p.score).sum::<f64>() / people.len() as f64;
                            (decade, people.len(), avg_score)
                        })
                        .collect();
                    black_box(result)
                })
            },
        );

        // Imperative approach
        group.bench_with_input(BenchmarkId::new("imperative", size), &data, |b, data| {
            b.iter(|| {
                use std::collections::HashMap;

                let mut groups: HashMap<u32, Vec<&BenchmarkPerson>> = HashMap::new();

                // Group by decade
                for person in data {
                    if person.active {
                        let decade = person.age / 10;
                        groups.entry(decade).or_insert_with(Vec::new).push(person);
                    }
                }

                // Calculate aggregations
                let mut result = Vec::new();
                for (decade, people) in groups {
                    let count = people.len();
                    let avg_score = people.iter().map(|p| p.score).sum::<f64>() / count as f64;
                    result.push((decade, count, avg_score));
                }

                black_box(result)
            })
        });
    }

    group.finish();
}

/// Simulate an expensive computation for parallel processing benchmarks
fn expensive_computation(score: f64) -> f64 {
    // Simulate some CPU-intensive work
    let mut result = score;
    for _ in 0..100 {
        result = (result * 1.01).sin().abs();
    }
    result
}

/// Benchmark: Error handling in functional pipelines
pub fn benchmark_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");

    for size in [1000, 5000].iter() {
        let data: Vec<i32> = (0..*size).collect();

        // Functional approach with Result handling
        group.bench_with_input(
            BenchmarkId::new("functional_result", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let result: Result<Vec<_>, &str> = data
                        .iter()
                        .map(|&x| {
                            if x % 7 == 0 && x != 0 {
                                Err("Divisible by 7")
                            } else {
                                Ok(x * 2)
                            }
                        })
                        .collect();
                    black_box(result)
                })
            },
        );

        // Imperative approach with explicit error checking
        group.bench_with_input(
            BenchmarkId::new("imperative_errors", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut result = Vec::new();
                    for &x in data {
                        if x % 7 == 0 && x != 0 {
                            let error_result: Result<Vec<i32>, &str> = Err("Divisible by 7");
                            return black_box(error_result);
                        }
                        result.push(x * 2);
                    }
                    let success_result: Result<Vec<i32>, &str> = Ok(result);
                    black_box(success_result)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Validation performance - Functional composition vs imperative loops
///
/// This benchmark compares the performance of:
/// - Functional validator pattern: Composable validation rules
/// - Imperative loops: Traditional nested validation loops
pub fn benchmark_validation_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");
    group.measurement_time(Duration::from_secs(5));

    // Test DTO structure for validation
    #[derive(Clone)]
    struct PersonDTO {
        name: String,
        email: String,
        age: u32,
    }

    let test_cases = vec![
        PersonDTO {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        },
        PersonDTO {
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            age: 25,
        },
        PersonDTO {
            name: "Bob Johnson".to_string(),
            email: "bob@example.com".to_string(),
            age: 35,
        },
    ];

    for size in [10, 100, 1000].iter() {
        let test_data: Vec<PersonDTO> = (0..*size)
            .map(|i| test_cases[i % test_cases.len()].clone())
            .collect();

        // Functional approach: Validator combinator pattern
        group.bench_with_input(
            BenchmarkId::new("functional_validator", size),
            &test_data,
            |b, data| {
                b.iter(|| {
                    let validated_count = data
                        .iter()
                        .filter(|dto| {
                            // Functional validation rules chained
                            !dto.name.trim().is_empty()
                                && dto.name.len() <= 100
                                && !dto.email.is_empty()
                                && dto.email.contains('@')
                                && dto.email.len() <= 255
                                && dto.age >= 18
                                && dto.age <= 120
                        })
                        .count();
                    black_box(validated_count)
                })
            },
        );

        // Imperative approach: Nested validation loops
        group.bench_with_input(
            BenchmarkId::new("imperative_loops", size),
            &test_data,
            |b, data| {
                b.iter(|| {
                    let mut validated_count = 0;

                    for dto in data.iter() {
                        // Name validation
                        if dto.name.trim().is_empty() || dto.name.len() > 100 {
                            continue;
                        }

                        // Email validation
                        if dto.email.is_empty() || !dto.email.contains('@') || dto.email.len() > 255
                        {
                            continue;
                        }

                        // Age validation
                        if dto.age < 18 || dto.age > 120 {
                            continue;
                        }

                        validated_count += 1;
                    }

                    black_box(validated_count)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: QueryReader monad overhead - Testing zero-cost abstraction claim
///
/// This benchmark measures the overhead of the QueryReader pattern by comparing:
/// - Direct function calls
/// - QueryReader wrapped calls (should be zero-cost due to inlining)
pub fn benchmark_query_reader_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_reader_overhead");

    // Simulate a simple query operation
    fn simulate_query(id: u32) -> u32 {
        id * 2 + 10
    }

    for size in [1000, 10000, 100000].iter() {
        // Direct function calls - baseline
        group.bench_with_input(BenchmarkId::new("direct_calls", size), size, |b, &size| {
            b.iter(|| {
                let results: Vec<u32> = (0..size).map(simulate_query).collect();
                black_box(results)
            })
        });

        // Simulated QueryReader pattern with closure wrapping
        group.bench_with_input(
            BenchmarkId::new("query_reader_wrapped", size),
            size,
            |b, &size| {
                b.iter(|| {
                    // Simulate QueryReader-like behavior
                    let query = Box::new(|id: u32| simulate_query(id));
                    let results: Vec<u32> = (0..size).map(|i| query(i)).collect();
                    black_box(results)
                })
            },
        );

        // Chained QueryReader operations
        group.bench_with_input(
            BenchmarkId::new("query_reader_chained", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let results: Vec<u32> = (0..size)
                        .map(|id| {
                            // Simulating: QueryReader::new(|conn| query(id))
                            //     .map(|result| result * 2)
                            //     .run(conn)
                            let step1 = simulate_query(id);
                            let step2 = step1 * 2;
                            step2
                        })
                        .collect();
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Memoization cache efficiency
///
/// This benchmark tests the performance benefits of memoization by measuring:
/// - Expensive computation without cache
/// - With memoization (high hit rate)
/// - Cache lookup overhead
pub fn benchmark_memoization_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memoization");
    group.measurement_time(Duration::from_secs(5));

    // Simulate an expensive computation
    fn expensive_computation(n: u32) -> u64 {
        let mut result: u64 = 1;
        for i in 1..=n as u64 {
            result = result.wrapping_mul(i);
        }
        result
    }

    for cache_hit_rate in [0.0, 0.5, 0.9].iter() {
        // Generate test sequence with specified hit rate
        let test_size = 1000;
        let cache_size = ((test_size as f64) * cache_hit_rate) as usize;
        let mut sequence: Vec<u32> = Vec::new();

        // Fill with repeated values to create cache hits
        for i in 0..cache_size {
            sequence.push((i % 20) as u32 + 1);
        }
        // Fill remaining with new values
        for i in cache_size..test_size {
            sequence.push(((i - cache_size) as u32) + 20);
        }

        // Without memoization - pure computation
        group.bench_with_input(
            BenchmarkId::new("no_cache", format!("{:.0}%_hits", cache_hit_rate * 100.0)),
            &sequence,
            |b, sequence| {
                b.iter(|| {
                    let results: Vec<u64> =
                        sequence.iter().map(|&n| expensive_computation(n)).collect();
                    black_box(results)
                })
            },
        );

        // With memoization using simple HashMap
        group.bench_with_input(
            BenchmarkId::new("with_cache", format!("{:.0}%_hits", cache_hit_rate * 100.0)),
            &sequence,
            |b, sequence| {
                b.iter(|| {
                    use std::collections::HashMap;

                    let mut cache: HashMap<u32, u64> = HashMap::new();
                    let results: Vec<u64> = sequence
                        .iter()
                        .map(|&n| *cache.entry(n).or_insert_with(|| expensive_computation(n)))
                        .collect();
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Validation rule count impact
///
/// Tests how validation performance scales with the number of rules
pub fn benchmark_validation_rule_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation_rule_scaling");

    #[derive(Clone)]
    struct TestData {
        field1: String,
        field2: String,
        field3: u32,
        field4: String,
        field5: String,
        field6: u32,
    }

    let test_data = TestData {
        field1: "test".to_string(),
        field2: "test@example.com".to_string(),
        field3: 25,
        field4: "valid".to_string(),
        field5: "data".to_string(),
        field6: 100,
    };

    for rule_count in [1, 3, 5, 10].iter() {
        let data_vec = vec![test_data.clone(); 100];

        // Functional: Direct predicate evaluation
        group.bench_with_input(
            BenchmarkId::new("functional_rules", rule_count),
            &data_vec,
            |b, data| {
                b.iter(|| {
                    let validated = data
                        .iter()
                        .filter(|d| {
                            // Simulate rule_count validation rules
                            let mut valid = true;
                            for _ in 0..*rule_count {
                                valid = valid && !d.field1.is_empty() && d.field3 > 0;
                            }
                            valid
                        })
                        .count();
                    black_box(validated)
                })
            },
        );

        // Imperative: Explicit loop with multiple checks
        group.bench_with_input(
            BenchmarkId::new("imperative_rules", rule_count),
            &data_vec,
            |b, data| {
                b.iter(|| {
                    let mut validated = 0;
                    for d in data.iter() {
                        let mut valid = true;
                        for _ in 0..*rule_count {
                            if d.field1.is_empty() || d.field3 == 0 {
                                valid = false;
                                break;
                            }
                        }
                        if valid {
                            validated += 1;
                        }
                    }
                    black_box(validated)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Error propagation performance
///
/// Compares error handling patterns through functional chains vs imperative error checks
pub fn benchmark_error_propagation(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_propagation");

    for size in [1000, 5000, 10000].iter() {
        let data: Vec<i32> = (0..*size).collect();

        // Functional: Result chaining with ?
        group.bench_with_input(
            BenchmarkId::new("functional_result_chain", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let result: Result<Vec<i32>, &str> = (|| {
                        let step1: Vec<i32> = data.iter().map(|x| x * 2).collect();
                        let step2: Vec<i32> = step1.iter().filter(|&&x| x > 100).copied().collect();
                        let step3: Vec<i32> = step2.iter().map(|x| x + 50).collect();

                        if step3.is_empty() {
                            Err("Empty result set")
                        } else {
                            Ok(step3)
                        }
                    })();
                    black_box(result)
                })
            },
        );

        // Imperative: Explicit error checks
        group.bench_with_input(
            BenchmarkId::new("imperative_error_checks", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let result: Result<Vec<i32>, &str> = {
                        let mut step1 = Vec::new();
                        for x in data {
                            step1.push(x * 2);
                        }

                        let mut step2 = Vec::new();
                        for x in &step1 {
                            if *x > 100 {
                                step2.push(*x);
                            }
                        }

                        let mut step3 = Vec::new();
                        for x in &step2 {
                            step3.push(x + 50);
                        }

                        if step3.is_empty() {
                            Err("Empty result set")
                        } else {
                            Ok(step3)
                        }
                    };
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_data_filtering,
    benchmark_data_transformation,
    benchmark_complex_pipeline,
    benchmark_parallel_processing,
    benchmark_memory_efficiency,
    benchmark_iterator_composition,
    benchmark_grouping_aggregation,
    benchmark_error_handling,
    benchmark_validation_performance,
    benchmark_query_reader_overhead,
    benchmark_memoization_efficiency,
    benchmark_validation_rule_scaling,
    benchmark_error_propagation
);

criterion_main!(benches);
