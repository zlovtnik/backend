//! # Functional Programming Performance Benchmarks
//!
//! This module provides comprehensive performance benchmarks for functional programming
//! operations, comparing functional vs imperative approaches and measuring performance
//! improvements in data processing speed, memory efficiency, and throughput.

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

criterion_group!(
    benches,
    benchmark_data_filtering,
    benchmark_data_transformation,
    benchmark_complex_pipeline,
    benchmark_parallel_processing,
    benchmark_memory_efficiency,
    benchmark_iterator_composition,
    benchmark_grouping_aggregation,
    benchmark_error_handling
);

criterion_main!(benches);
