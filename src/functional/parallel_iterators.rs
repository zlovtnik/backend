//! Parallel Iterator Patterns
//!
//! This module provides parallel iterator patterns for CPU-intensive operations
//! leveraging rayon's work-stealing algorithm. It enables safe concurrent processing
//! while maintaining immutability and functional programming principles.
//!
//! Key features:
//! - Parallel map operations on large datasets
//! - Concurrent folding and reduction operations
//! - Work-stealing thread pool integration
//! - Thread-safe immutable data handling
//! - Iterator-based parallel processing with itertools compatibility

#![allow(dead_code)]
#![allow(unused_variables)]

use log;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Performance history entry for adaptive chunk sizing
#[derive(Debug, Clone)]
struct PerformanceEntry {
    chunk_size: usize,
    data_size: usize,
    thread_count: usize,
    efficiency: f64,
    throughput: u64,
    timestamp: Instant,
}

/// Global performance history for adaptive chunk sizing
static PERFORMANCE_HISTORY: std::sync::OnceLock<
    Arc<RwLock<HashMap<String, Vec<PerformanceEntry>>>>,
> = std::sync::OnceLock::new();

/// Get or initialize the performance history store
fn get_performance_history() -> Arc<RwLock<HashMap<String, Vec<PerformanceEntry>>>> {
    PERFORMANCE_HISTORY
        .get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
        .clone()
}

/// Record a performance entry for adaptive learning
fn record_performance(operation_key: String, entry: PerformanceEntry) {
    let history = get_performance_history();
    let mut map = match history.write() {
        Ok(guard) => guard,
        Err(_) => {
            log::warn!("Performance history lock was poisoned, skipping recording");
            return;
        }
    };

    let entries = map.entry(operation_key).or_insert_with(Vec::new);
    entries.push(entry);

    // Keep only recent entries (last 100 per operation type) - use split_off for efficient removal of old entries
    if entries.len() > 100 {
        let keep = entries.len() - 100;
        *entries = entries.split_off(keep);
    }

    // Remove entries older than 1 hour - use checked_sub to prevent panic
    let now = Instant::now();
    let one_hour_ago = now
        .checked_sub(Duration::from_secs(3600))
        .unwrap_or(now);
    entries.retain(|entry| entry.timestamp > one_hour_ago);
}

/// Calculate optimal chunk size based on performance history
fn calculate_adaptive_chunk_size(
    operation_key: &str,
    data_size: usize,
    thread_count: usize,
    base_chunk_size: usize,
    max_chunk_size: usize,
) -> usize {
    if data_size == 0 {
        return base_chunk_size;
    }

    let history = get_performance_history();
    let map = match history.read() {
        Ok(map) => map,
        Err(poison) => {
            log::warn!("Performance history lock was poisoned, using default chunk size");
            return base_chunk_size;
        }
    };

    if let Some(entries) = map.get(operation_key) {
        // Find entries with similar data size range (±20%)
        let similar_entries: Vec<_> = entries
            .iter()
            .filter(|entry| {
                let size_ratio = entry.data_size as f64 / data_size as f64;
                size_ratio >= 0.8 && size_ratio <= 1.2
            })
            .collect();

        if similar_entries.len() >= 3 {
            // Calculate weighted average of chunk sizes based on efficiency
            let total_weight: f64 = similar_entries.iter().map(|e| e.efficiency.max(0.1)).sum();
            let weighted_sum: f64 = similar_entries
                .iter()
                .map(|e| e.chunk_size as f64 * e.efficiency.max(0.1))
                .sum();

            let optimal_chunk = (weighted_sum / total_weight) as usize;

            // Constrain to reasonable bounds
            optimal_chunk.max(64).min(max_chunk_size)
        } else {
            // Not enough data, use base calculation
            base_chunk_size
        }
    } else {
        base_chunk_size
    }
}

/// Parallel processing configuration for performance tuning
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Thread pool size hint (0 = automatic)
    pub thread_pool_size: usize,
    /// Minimum dataset size for parallel processing
    pub min_parallel_size: usize,
    /// Enable work-stealing optimization
    pub enable_work_stealing: bool,
    /// Memory buffer size for chunked operations
    pub chunk_size: usize,
    /// Enable adaptive chunk sizing based on performance history
    pub adaptive_chunk_sizing: bool,
    /// Maximum chunk size for adaptive sizing
    pub max_chunk_size: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            thread_pool_size: 0, // Automatic
            min_parallel_size: 1024,
            enable_work_stealing: true,
            chunk_size: 1024,
            adaptive_chunk_sizing: true,
            max_chunk_size: 8192,
        }
    }
}

/// Note: These metrics are not yet collected and will contain default values.
/// Work-stealing performance metrics
#[derive(Debug, Clone, Default)]
pub struct WorkStealingMetrics {
    /// Number of tasks stolen by threads
    pub tasks_stolen: u64,
    /// Number of tasks executed locally
    pub tasks_local: u64,
    /// Work-stealing efficiency ratio (0.0 - 1.0)
    pub stealing_efficiency: f64,
    /// Load imbalance factor
    pub load_imbalance: f64,
}

/// Note: These metrics are not yet collected and will contain default values.
/// Load balancing metrics for detailed performance analysis
#[derive(Debug, Clone, Default)]
pub struct LoadBalancingMetrics {
    /// Average work per thread
    pub avg_work_per_thread: f64,
    /// Standard deviation of work distribution
    pub work_distribution_std_dev: f64,
    /// Maximum work assigned to any thread
    pub max_thread_work: u64,
    /// Minimum work assigned to any thread
    pub min_thread_work: u64,
    /// Load balancing efficiency (0.0 - 1.0)
    pub balancing_efficiency: f64,
}

/// Performance metrics for parallel operations
#[derive(Debug, Clone, Default)]
pub struct ParallelMetrics {
    /// Total processing time
    pub total_time: Duration,
    /// Thread count used
    pub thread_count: usize,
    /// Items processed per second
    pub throughput: u64,
    /// Memory usage estimate
    pub memory_usage: u64,
    /// Parallel efficiency (0.0 - 1.0)
    pub efficiency: f64,
    /// Work-stealing efficiency metrics
    pub work_stealing_metrics: WorkStealingMetrics,
    /// Detailed load balancing metrics
    pub load_balancing_metrics: LoadBalancingMetrics,
}

/// Parallel iterator extension trait for functional programming
pub trait ParallelIteratorExt<T: Send + Sync>: Iterator<Item = T> + Send + Sync {
    /// Maps each item of the iterator through `f`, running the operation in parallel when the input
    /// size exceeds the configured threshold, and returns the mapped results together with runtime
    /// metrics.
    ///
    /// The method collects the iterator into a vector, chooses between a sequential or Rayon-backed
    /// parallel execution based on `config.min_parallel_size`, and records timing, thread usage,
    /// throughput, memory estimate, and a simple efficiency heuristic in the returned `ParallelResult`.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = ParallelConfig::default();
    /// let result = (0..8).into_iter().par_map(&cfg, |x| x * 2);
    /// assert_eq!(result.into_inner(), vec![0, 2, 4, 6, 8, 10, 12, 14]);
    /// ```
    fn par_map<F, U>(self, config: &ParallelConfig, f: F) -> ParallelResult<Vec<U>>
    where
        F: Fn(T) -> U + Send + Sync,
        U: Send,
        Self: Sized,
    {
        let start_time = Instant::now();

        // Convert to vector for parallel processing
        let data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Use sequential processing for small datasets
            let result = data.into_iter().map(f).collect();
            let elapsed = start_time.elapsed();
            let metrics = ParallelMetrics {
                total_time: elapsed,
                thread_count: 1,
                throughput: (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64,
                memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
                efficiency: 1.0,
                work_stealing_metrics: WorkStealingMetrics::default(),
                load_balancing_metrics: LoadBalancingMetrics::default(),
            };
            return ParallelResult {
                data: result,
                metrics,
            };
        }

        // Parallel processing for large datasets
        let base_chunk_size = config.chunk_size.max(1);
        let chunk_size = if config.adaptive_chunk_sizing {
            let operation_key = format!("{}:{}", "par_map", std::any::type_name::<T>());
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

        let result: Vec<U> = data
            .into_par_iter()
            .with_min_len(chunk_size)
            .with_max_len(chunk_size * 4)
            .map(f)
            .collect();

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;

        // Estimate parallel efficiency (simplified heuristic)
        let efficiency = if data_len < config.min_parallel_size {
            0.9 // Sequential baseline efficiency
        } else {
            let data_len_f64 = data_len as f64;
            let elapsed_secs = elapsed.as_secs_f64();
            (throughput as f64 / (data_len_f64 / elapsed_secs)).min(1.0)
        };

        let metrics = ParallelMetrics {
            total_time: elapsed,
            thread_count,
            throughput,
            memory_usage: ((data_len * std::mem::size_of::<T>())
                + (result.len() * std::mem::size_of::<U>())) as u64,
            efficiency,
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };

        // Record performance for adaptive chunk sizing
        if config.adaptive_chunk_sizing {
            let operation_key = format!("{}:{}", "par_map", std::any::type_name::<T>());
            let entry = PerformanceEntry {
                chunk_size,
                data_size: data_len,
                thread_count,
                efficiency,
                throughput,
                timestamp: Instant::now(),
            };
            record_performance(operation_key, entry);
        }

        ParallelResult {
            data: result,
            metrics,
        }
    }

    /// Performs a fold (reduction) over the iterator, using `fold` per item and `combine` to merge partial results.
    ///
    /// Chooses a sequential fold when the collected input length is less than `config.min_parallel_size`; otherwise it performs a parallel fold and reduction using Rayon. The returned `ParallelResult` includes the folded value and `ParallelMetrics` (total time, thread count, throughput, memory usage, and an efficiency heuristic).
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::{ParallelConfig, ParallelIteratorExt};
    ///
    /// let config = ParallelConfig::default();
    /// let result = (0usize..100usize)
    ///     .par_fold(&config, 0usize, |acc, x| acc + x, |a, b| a + b);
    ///
    /// assert_eq!(result.data, (0usize..100usize).sum());
    /// assert!(result.metrics.throughput > 0);
    /// ```
    fn par_fold<F, B, C>(
        self,
        config: &ParallelConfig,
        init: B,
        fold: F,
        combine: C,
    ) -> ParallelResult<B>
    where
        F: Fn(B, T) -> B + Send + Sync,
        C: Fn(B, B) -> B + Send + Sync,
        B: Send + Clone + Sync,
        Self: Sized,
    {
        let start_time = Instant::now();
        let data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Sequential fold for small datasets
            let result = data.into_iter().fold(init, fold);
            let elapsed = start_time.elapsed();
            let metrics = ParallelMetrics {
                total_time: elapsed,
                thread_count: 1,
                throughput: (data_len as u64 * 1_000_000)
                    / (start_time.elapsed().as_micros() as u64).max(1),
                memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
                efficiency: 1.0,
                work_stealing_metrics: WorkStealingMetrics::default(),
                load_balancing_metrics: LoadBalancingMetrics::default(),
            };
            return ParallelResult {
                data: result,
                metrics,
            };
        }

        // Parallel fold with combiner
        let result = data
            .into_par_iter()
            .fold(|| init.clone(), fold)
            .reduce(|| init.clone(), combine);

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = (data_len as u64 * 1_000_000) / (elapsed.as_micros() as u64).max(1);

        // Estimate parallel efficiency (simplified heuristic)
        let efficiency = (throughput as f64 / (data_len as f64 / elapsed.as_secs_f64())).min(1.0);

        let metrics = ParallelMetrics {
            total_time: elapsed,
            thread_count,
            throughput,
            memory_usage: (data_len * std::mem::size_of::<B>()) as u64,
            efficiency,
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };

        ParallelResult {
            data: result,
            metrics,
        }
    }

    /// Filters elements using `predicate`, processing in parallel when the dataset exceeds the configured threshold, and preserves the original input order.
    ///
    /// If the collected input length is less than `config.min_parallel_size`, the function performs a sequential filter; otherwise it performs a parallel filter. The predicate is applied to references to elements (`&T`). The returned `ParallelResult` includes both the filtered `Vec<T>` and operation metrics (timing, thread count, throughput, memory usage, efficiency).
    ///
    /// # Examples
    ///
    /// ```
    /// let config = ParallelConfig::default();
    /// let data = vec![1, 2, 3, 4, 5];
    /// let res = data.into_iter().par_filter(&config, |&x| x % 2 == 0);
    /// assert_eq!(res.data, vec![2, 4]);
    /// assert!(res.metrics.throughput > 0);
    /// ```
    fn par_filter<F>(self, config: &ParallelConfig, predicate: F) -> ParallelResult<Vec<T>>
    where
        F: Fn(&T) -> bool + Send + Sync,
        T: Clone + Send + Sync,
        Self: Sized,
    {
        let start_time = Instant::now();
        let data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Sequential filter for small datasets
            let result = data.into_iter().filter(predicate).collect();
            let metrics = ParallelMetrics {
                total_time: start_time.elapsed(),
                thread_count: 1,
                throughput: (data_len as u64 * 1_000_000)
                    / (start_time.elapsed().as_micros() as u64).max(1),
                memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
                efficiency: 1.0,
                work_stealing_metrics: WorkStealingMetrics::default(),
                load_balancing_metrics: LoadBalancingMetrics::default(),
            };
            return ParallelResult {
                data: result,
                metrics,
            };
        }

        // Parallel filter with temporary indices to preserve order
        let indexed: Vec<(usize, T)> = data.into_iter().enumerate().collect();
        let mut filtered: Vec<(usize, T)> = indexed
            .into_par_iter()
            .filter(|(_, item)| predicate(item))
            .collect();

        // Sort by original index to restore input order
        filtered.sort_unstable_by_key(|(idx, _)| *idx);

        // Extract values in sorted order
        let result: Vec<T> = filtered.into_iter().map(|(_, item)| item).collect();

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;

        let metrics = ParallelMetrics {
            total_time: elapsed,
            thread_count,
            throughput,
            memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
            efficiency: (throughput as f64 / (data_len as f64 / elapsed.as_secs_f64())).min(1.0),
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };

        ParallelResult {
            data: result,
            metrics,
        }
    }

    /// Reduces the iterator to a single value using parallel reduction.
    ///
    /// Unlike `par_fold`, this method requires the reduction operation to be associative
    /// and commutative, allowing for more efficient parallel execution. The `reduce`
    /// closure combines two values of the same type into one.
    ///
    /// # Returns
    ///
    /// `ParallelResult<Option<T>>` containing the reduced value (if any) and performance metrics.
    /// Returns `None` if the iterator is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::parallel_iterators::{ParallelConfig, ParallelIteratorExt};
    ///
    /// let config = ParallelConfig::default();
    /// let sum = (0..100).into_iter().par_reduce(&config, |a, b| a + b);
    /// assert_eq!(sum.data, Some(4950)); // sum of 0..100 = 4950
    /// ```
    fn par_reduce<F>(self, config: &ParallelConfig, reduce: F) -> ParallelResult<Option<T>>
    where
        F: Fn(T, T) -> T + Send + Sync,
        T: Send + Clone,
        Self: Sized,
    {
        let start_time = Instant::now();
        let data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Sequential reduction for small datasets
            let result = data.into_iter().reduce(reduce);
            let elapsed = start_time.elapsed();
            let throughput = if elapsed.as_micros() > 0 {
                (data_len as u64 * 1_000_000) / elapsed.as_micros() as u64
            } else {
                0
            };
            let metrics = ParallelMetrics {
                total_time: elapsed,
                thread_count: 1,
                throughput,
                memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
                efficiency: 1.0,
                work_stealing_metrics: WorkStealingMetrics::default(),
                load_balancing_metrics: LoadBalancingMetrics::default(),
            };
            return ParallelResult {
                data: result,
                metrics,
            };
        }

        // Parallel reduction for large datasets
        let result = data.into_par_iter().reduce_with(reduce);

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = if elapsed.as_micros() > 0 {
            (data_len as u64 * 1_000_000) / elapsed.as_micros() as u64
        } else {
            0
        };

        // Estimate parallel efficiency
        let efficiency = if elapsed.as_secs_f64() > 0.0 && data_len > 0 {
            (throughput as f64 / (data_len as f64 / elapsed.as_secs_f64())).min(1.0)
        } else {
            1.0
        };

        let metrics = ParallelMetrics {
            total_time: elapsed,
            thread_count,
            throughput,
            memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
            efficiency,
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };

        ParallelResult {
            data: result,
            metrics,
        }
    }

    /// Groups items by a key produced from each element using the provided key function.
    ///
    /// Returns a `HashMap` that maps each distinct key to a `Vec<T>` containing the items that produced that key.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = ParallelConfig::default();
    /// let data = vec![1, 2, 3, 4, 5, 6];
    /// let result = data.into_iter().par_group_by(&config, |&x| x % 2);
    /// assert_eq!(result.data.get(&0).unwrap().len(), 3); // 2, 4, 6
    /// assert_eq!(result.data.get(&1).unwrap().len(), 3); // 1, 3, 5
    /// ```
    fn par_group_by<K, KeyFn>(
        self,
        config: &ParallelConfig,
        key_fn: KeyFn,
    ) -> ParallelResult<HashMap<K, Vec<T>>>
    where
        K: std::hash::Hash + Eq + Clone + Send + Sync,
        KeyFn: Fn(&T) -> K + Send + Sync,
        T: Clone + Send + Sync,
        Self: Sized,
    {
        let start_time = Instant::now();
        let data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Sequential grouping for small datasets
            let mut groups = HashMap::new();
            for item in data {
                let key = key_fn(&item);
                groups.entry(key).or_insert_with(Vec::new).push(item);
            }
            let elapsed = start_time.elapsed();
            let metrics = ParallelMetrics {
                total_time: elapsed,
                thread_count: 1,
                throughput: (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64,
                memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
                efficiency: 1.0,
                work_stealing_metrics: WorkStealingMetrics::default(),
                load_balancing_metrics: LoadBalancingMetrics::default(),
            };
            return ParallelResult {
                data: groups,
                metrics,
            };
        }

        // Parallel grouping using fold and combine
        let result = data
            .into_par_iter()
            .fold(
                || HashMap::new(),
                |mut groups: HashMap<K, Vec<T>>, item| {
                    let key = key_fn(&item);
                    groups.entry(key).or_insert_with(Vec::new).push(item);
                    groups
                },
            )
            .reduce(
                || HashMap::new(),
                |mut acc: HashMap<K, Vec<T>>, map: HashMap<K, Vec<T>>| {
                    for (key, mut values) in map {
                        acc.entry(key).or_insert_with(Vec::new).append(&mut values);
                    }
                    acc
                },
            );

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;

        // Estimate parallel efficiency
        let efficiency = (throughput as f64 / (data_len as f64 / elapsed.as_secs_f64())).min(1.0);

        let metrics = ParallelMetrics {
            total_time: elapsed,
            thread_count,
            throughput,
            memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
            efficiency,
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };

        ParallelResult {
            data: result,
            metrics,
        }
    }

    /// Sorts the elements of the iterator in parallel.
    ///
    /// The elements must implement `Ord` for comparison. This method collects the iterator
    /// into a vector and sorts it using Rayon's parallel sort when the dataset size exceeds
    /// the configured threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = ParallelConfig::default();
    /// let data = vec![3, 1, 4, 1, 5];
    /// let result = data.into_iter().par_sort(&config);
    /// assert_eq!(result.data, vec![1, 1, 3, 4, 5]);
    /// ```
    fn par_sort(self, config: &ParallelConfig) -> ParallelResult<Vec<T>>
    where
        T: Ord + Send + Clone,
        Self: Sized,
    {
        let start_time = Instant::now();
        let mut data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Sequential sort for small datasets
            data.sort();
            let elapsed = start_time.elapsed();
            let metrics = ParallelMetrics {
                total_time: elapsed,
                thread_count: 1,
                throughput: (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64,
                memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
                efficiency: 1.0,
                work_stealing_metrics: WorkStealingMetrics::default(),
                load_balancing_metrics: LoadBalancingMetrics::default(),
            };
            return ParallelResult {
                data,
                metrics,
            };
        }

        // Parallel sort for large datasets
        data.par_sort();

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;

        // Estimate parallel efficiency
        let efficiency = (throughput as f64 / (data_len as f64 / elapsed.as_secs_f64())).min(1.0);

        let metrics = ParallelMetrics {
            total_time: elapsed,
            thread_count,
            throughput,
            memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
            efficiency,
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };

        ParallelResult {
            data,
            metrics,
        }
    }
}

/// Result wrapper for parallel operations with performance metrics
#[derive(Debug)]
pub struct ParallelResult<T> {
    pub data: T,
    pub metrics: ParallelMetrics,
}

impl<T> ParallelResult<T> {
    /// Get the result data
    pub fn into_inner(self) -> T {
        self.data
    }

    /// Get performance metrics
    pub fn metrics(&self) -> &ParallelMetrics {
        &self.metrics
    }

    /// Check if operation was efficient (parallel processing beneficial)
    pub fn is_efficient(&self) -> bool {
        self.metrics.efficiency > 0.7
    }
}

impl fmt::Display for ParallelMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parallel Metrics: time={:?}, threads={}, throughput={} ops/s, efficiency={:.2}",
            self.total_time, self.thread_count, self.throughput, self.efficiency
        )
    }
}

// Implement the trait for all compatible iterators
impl<T: Send + Sync, I: Iterator<Item = T> + Send + Sync> ParallelIteratorExt<T> for I {}

// Standalone parallel processing functions for common patterns

/// Applies a CPU-bound transformation to each element of `data`, using parallel execution when `config` permits.
///
/// Returns a `ParallelResult` containing the transformed `Vec<U>` and associated `ParallelMetrics`.
///
/// # Examples
///
/// ```
/// use crate::{parallel_transform, ParallelConfig};
///
/// let cfg = ParallelConfig::default();
/// let input = vec![1, 2, 3, 4];
/// let result = parallel_transform(input, |n| n * 2, &cfg);
/// assert_eq!(result.into_inner(), vec![2, 4, 6, 8]);
/// ```
pub fn parallel_transform<T, U, F>(
    data: Vec<T>,
    transform: F,
    config: &ParallelConfig,
) -> ParallelResult<Vec<U>>
where
    T: Send + Sync,
    U: Send,
    F: Fn(T) -> U + Send + Sync,
{
    data.into_iter().par_map(config, transform)
}

/// Aggregates the elements of `data` into a single accumulator, using a parallel fold when the input size meets the configured threshold.
///
/// Uses `aggregate` to incorporate each item into a per-thread accumulator and `combine` to merge those accumulators into the final result. If `data.len() < config.min_parallel_size`, a sequential fold is performed. The returned `ParallelResult` contains the aggregated value and measured execution metrics.
///
/// # Returns
///
/// A `ParallelResult` whose `data` is the final accumulator and whose `metrics` describe execution time, thread usage, throughput, memory usage, and a simple efficiency estimate.
///
/// # Examples
///
/// ```
/// use crate::{parallel_aggregate, ParallelConfig};
///
/// let data: Vec<u32> = (1..=100u32).collect();
/// let config = ParallelConfig::default();
///
/// let result = parallel_aggregate(
///     data,
///     0u32,
///     |acc, x| acc + x,
///     |a, b| a + b,
///     &config,
/// );
///
/// assert_eq!(result.into_inner(), 5050);
/// assert!(result.metrics().throughput > 0);
/// ```
pub fn parallel_aggregate<T, B, F, C>(
    data: Vec<T>,
    init: B,
    aggregate: F,
    combine: C,
    config: &ParallelConfig,
) -> ParallelResult<B>
where
    T: Send + Sync,
    B: Send + Clone + Sync,
    F: Fn(B, T) -> B + Send + Sync,
    C: Fn(B, B) -> B + Send + Sync,
{
    // Manually implement the parallel fold logic directly on Vec
    let start_time = Instant::now();
    let data_len = data.len();

    if data_len < config.min_parallel_size {
        // Sequential fold for small datasets
        let result = data.into_iter().fold(init.clone(), &aggregate);
        let metrics = ParallelMetrics {
            total_time: start_time.elapsed(),
            thread_count: 1,
            throughput: (data_len as u64 * 1_000_000)
                / (start_time.elapsed().as_micros() as u64).max(1),
            memory_usage: (data_len * std::mem::size_of::<B>()) as u64,
            efficiency: 1.0,
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };
        return ParallelResult {
            data: result,
            metrics,
        };
    }

    // Parallel fold with combiner
    let result = data
        .into_par_iter()
        .fold(|| init.clone(), aggregate)
        .reduce(|| init.clone(), combine);

    let elapsed = start_time.elapsed();
    let thread_count = rayon::current_num_threads();
    let throughput = (data_len as u64 * 1_000_000) / (elapsed.as_micros() as u64).max(1);

    // Estimate parallel efficiency (simplified heuristic)
    let efficiency = (throughput as f64 / (data_len as f64 / elapsed.as_secs_f64())).min(1.0);

    let metrics = ParallelMetrics {
        total_time: elapsed,
        thread_count,
        throughput,
        memory_usage: (data_len * std::mem::size_of::<B>()) as u64,
        efficiency,
        work_stealing_metrics: WorkStealingMetrics::default(),
        load_balancing_metrics: LoadBalancingMetrics::default(),
    };

    ParallelResult {
        data: result,
        metrics,
    }
}

/// Parallel filtering with configurable predicate
#[allow(dead_code)]
pub fn parallel_filter<T, F>(
    data: Vec<T>,
    predicate: F,
    config: &ParallelConfig,
) -> ParallelResult<Vec<T>>
where
    T: Send + Sync + Clone,
    F: Fn(&T) -> bool + Send + Sync,
{
    data.into_iter().par_filter(config, predicate)
}

/// In-place parallel transformation to reduce memory allocations
///
/// This function modifies the input vector in-place, applying the transformation
/// in parallel without creating intermediate allocations for the result.
#[allow(dead_code)]
pub fn parallel_transform_inplace<T, F>(
    data: &mut [T],
    config: &ParallelConfig,
    transform: F,
) -> ParallelMetrics
where
    T: Send + Sync,
    F: Fn(&mut T) + Send + Sync,
{
    let start_time = Instant::now();
    let data_len = data.len();

    if data_len < config.min_parallel_size {
        // Sequential transformation
        data.iter_mut().for_each(transform);
        let elapsed = start_time.elapsed();
        return ParallelMetrics {
            total_time: elapsed,
            thread_count: 1,
            throughput: (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64,
            memory_usage: 0, // In-place, no additional allocation
            efficiency: 1.0,
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };
    }

    // Parallel in-place transformation
    data.par_iter_mut().for_each(transform);

    let elapsed = start_time.elapsed();
    let thread_count = rayon::current_num_threads();
    let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;

    // Estimate parallel efficiency
    let efficiency = (throughput as f64 / (data_len as f64 / elapsed.as_secs_f64())).min(1.0);

    ParallelMetrics {
        total_time: elapsed,
        thread_count,
        throughput,
        memory_usage: 0, // In-place, no additional allocation
        efficiency,
        work_stealing_metrics: WorkStealingMetrics::default(),
        load_balancing_metrics: LoadBalancingMetrics::default(),
    }
}

/// Memory-efficient parallel chunk processing
///
/// Processes data in chunks to minimize memory usage for large datasets
#[allow(dead_code)]
pub fn parallel_process_chunks<T, U, F>(
    data: Vec<T>,
    chunk_size: usize,
    config: &ParallelConfig,
    processor: F,
) -> ParallelResult<Vec<U>>
where
    T: Send + Sync + Clone,
    U: Send,
    F: Fn(Vec<T>) -> Vec<U> + Send + Sync,
{
    let start_time = Instant::now();
    let data_len = data.len();

    if data_len < config.min_parallel_size {
        // Sequential processing
        let result = processor(data);
        let elapsed = start_time.elapsed();
        let metrics = ParallelMetrics {
            total_time: elapsed,
            thread_count: 1,
            throughput: (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64,
            memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
            efficiency: 1.0,
            work_stealing_metrics: WorkStealingMetrics::default(),
            load_balancing_metrics: LoadBalancingMetrics::default(),
        };
        return ParallelResult {
            data: result,
            metrics,
        };
    }

    // Process chunks in parallel without copying
    let results: Vec<Vec<U>> = (0..data.len())
        .step_by(chunk_size)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|start| {
            let end = (start + chunk_size).min(data.len());
            let chunk = data[start..end].to_vec();
            processor(chunk)
        })
        .collect();

    // Flatten results
    let result: Vec<U> = results.into_iter().flatten().collect();

    let elapsed = start_time.elapsed();
    let thread_count = rayon::current_num_threads();
    let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;

    // Estimate parallel efficiency
    let efficiency = (throughput as f64 / (data_len as f64 / elapsed.as_secs_f64())).min(1.0);

    let metrics = ParallelMetrics {
        total_time: elapsed,
        thread_count,
        throughput,
        memory_usage: (data_len * std::mem::size_of::<T>()
            + result.len() * std::mem::size_of::<U>()) as u64,
        efficiency,
        work_stealing_metrics: WorkStealingMetrics::default(),
        load_balancing_metrics: LoadBalancingMetrics::default(),
    };

    ParallelResult {
        data: result,
        metrics,
    }
}

/// Estimates a suggested number of worker threads based on the input dataset size.
///
/// The function returns `1` for very small datasets (< 1,000), `2` for small datasets
/// (1,000..10,000), `4` for medium datasets (10,000..100,000), and `0` to indicate
/// that the caller should delegate thread-count selection to Rayon for large datasets
/// (>= 100,000).
///
/// # Examples
///
/// ```
/// assert_eq!(estimate_thread_count(500), 1);
/// assert_eq!(estimate_thread_count(5_000), 2);
/// assert_eq!(estimate_thread_count(50_000), 4);
/// assert_eq!(estimate_thread_count(200_000), 0);
/// ```
#[allow(dead_code)]
pub fn estimate_thread_count(data_size: usize) -> usize {
    if data_size < 1000 {
        1
    } else if data_size < 10000 {
        2
    } else if data_size < 100000 {
        4
    } else {
        // Let rayon determine optimal count for large datasets
        0
    }
}

/// Create a ParallelConfig tuned to the given dataset size.
///
/// For inputs smaller than 1000, `min_parallel_size` is set to `usize::MAX` to
/// force sequential execution; otherwise `min_parallel_size` is `data_size / 10`.
/// `thread_pool_size` is estimated by `estimate_thread_count(data_size)`,
/// `enable_work_stealing` is `true`, and `chunk_size` is `max(data_size / max(thread_count, 1), 100)`.
///
/// # Examples
///
/// ```
/// let cfg = optimized_config(10_000);
/// assert!(cfg.min_parallel_size <= 1_000);
/// assert!(cfg.chunk_size >= 100);
/// ```
#[allow(dead_code)]
pub fn optimized_config(data_size: usize) -> ParallelConfig {
    let thread_count = estimate_thread_count(data_size);
    let min_parallel_size = if data_size < 1000 {
        usize::MAX
    } else {
        data_size / 10
    };

    ParallelConfig {
        thread_pool_size: thread_count,
        min_parallel_size,
        enable_work_stealing: true,
        chunk_size: (data_size / thread_count.max(1)).max(100),
        adaptive_chunk_sizing: true,
        max_chunk_size: (data_size / 4).max(4096).min(16384),
    }
}

/// Concurrent pipeline for chaining multiple parallel operations
#[derive(Debug)]
pub struct ParallelPipeline<T> {
    data: Vec<T>,
    config: ParallelConfig,
}

impl<T: Send + Sync + Clone + 'static> ParallelPipeline<T> {
    /// Create a new pipeline with initial data
    pub fn new(data: Vec<T>, config: ParallelConfig) -> Self {
        Self { data, config }
    }

    /// Apply a mapping operation in the pipeline
    pub fn map<U, F>(self, transform: F) -> ParallelPipeline<U>
    where
        U: Send + Sync + Clone + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let result = self.data.into_iter().par_map(&self.config, transform);
        ParallelPipeline {
            data: result.data,
            config: self.config,
        }
    }

    /// Apply a filtering operation in the pipeline
    pub fn filter<F>(self, predicate: F) -> ParallelPipeline<T>
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        let result = self.data.into_iter().par_filter(&self.config, predicate);
        ParallelPipeline {
            data: result.data,
            config: self.config,
        }
    }

    /// Apply a folding operation to reduce to a single value
    pub fn fold<B, F, C>(self, init: B, fold: F, combine: C) -> ParallelResult<B>
    where
        B: Send + Clone + Sync + 'static,
        F: Fn(B, T) -> B + Send + Sync + 'static,
        C: Fn(B, B) -> B + Send + Sync + 'static,
    {
        self.data
            .into_iter()
            .par_fold(&self.config, init, fold, combine)
    }

    /// Apply a reduction operation to reduce to a single value
    pub fn reduce<F>(self, reduce: F) -> ParallelResult<Option<T>>
    where
        F: Fn(T, T) -> T + Send + Sync + 'static,
    {
        self.data.into_iter().par_reduce(&self.config, reduce)
    }

    /// Sort the data in the pipeline
    pub fn sort(self) -> ParallelPipeline<T>
    where
        T: Ord,
    {
        let result = self.data.into_iter().par_sort(&self.config);
        ParallelPipeline {
            data: result.data,
            config: self.config,
        }
    }

    /// Execute the pipeline and return the final result
    pub fn execute(self) -> Vec<T> {
        self.data
    }

    /// Get the current data without consuming the pipeline
    pub fn get_data(&self) -> &[T] {
        &self.data
    }
}

/// Create a parallel pipeline from an iterator
pub fn pipeline<T, I>(iter: I, config: ParallelConfig) -> ParallelPipeline<T>
where
    T: Send + Sync + Clone + 'static,
    I: IntoIterator<Item = T>,
{
    ParallelPipeline::new(iter.into_iter().collect(), config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_parallel_map_basic() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_map(&config, |x| x * 2);

        assert_eq!(result.data, vec![2, 4, 6, 8, 10]);
        assert!(result.is_efficient());
    }

    #[test]
    fn test_parallel_filter() {
        let data = vec![1, 2, 3, 4, 5, 6];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_filter(&config, |&x| x % 2 == 0);

        assert_eq!(result.data, vec![2, 4, 6]);
    }

    #[test]
    fn test_parallel_fold() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();

        let result = data
            .into_iter()
            .par_fold(&config, 0, |sum, x| sum + x, |a, b| a + b);

        assert_eq!(result.data, 15);
    }

    #[test]
    fn test_parallel_group_by() {
        let data = vec![1, 2, 3, 4, 5, 6];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_group_by(&config, |&x| x % 2);

        let mut even = result.data.get(&0).cloned().unwrap_or_default();
        let mut odd = result.data.get(&1).cloned().unwrap_or_default();

        even.sort();
        odd.sort();

        assert_eq!(even, vec![2, 4, 6]);
        assert_eq!(odd, vec![1, 3, 5]);
    }

    #[test]
    fn test_sequential_fallback() {
        let data = vec![1, 2, 3]; // Small dataset
        let mut config = ParallelConfig::default();
        config.min_parallel_size = 10; // Force sequential

        let result = data.into_iter().par_map(&config, |x| x * 2);

        assert_eq!(result.data, vec![2, 4, 6]);
        assert_eq!(result.metrics.thread_count, 1);
    }

    #[test]
    fn test_optimized_config() {
        let config = optimized_config(50000);
        assert!(config.min_parallel_size > 0);
        assert!(config.chunk_size > 0);
    }

    #[test]
    fn test_performance_metrics() {
        let data = (0..1000).collect::<Vec<_>>();
        let config = ParallelConfig::default();

        let start = Instant::now();
        let result = data.into_iter().par_map(&config, |x| x * x);
        let duration = start.elapsed();

        assert!(result.metrics.total_time <= duration);
        assert!(result.metrics.throughput > 0);
        assert!(result.metrics.efficiency > 0.0);
        assert!(result.metrics.efficiency <= 1.0);
    }

    #[test]
    fn test_par_reduce_empty() {
        let data: Vec<u32> = vec![];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_reduce(&config, |a, b| a + b);
        assert_eq!(result.data, None);
        assert!(result.metrics.throughput >= 0);
    }

    #[test]
    fn test_par_reduce_single_element() {
        let data = vec![42];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_reduce(&config, |a, b| a + b);
        assert_eq!(result.data, Some(42));
    }

    #[test]
    fn test_par_reduce_multiple_elements() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_reduce(&config, |a, b| a + b);
        assert_eq!(result.data, Some(15));
    }
}
