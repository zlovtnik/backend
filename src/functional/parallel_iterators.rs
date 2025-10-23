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

use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::time::{Duration, Instant};

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
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            thread_pool_size: 0, // Automatic
            min_parallel_size: 1024,
            enable_work_stealing: true,
            chunk_size: 1024,
        }
    }
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
            };
            return ParallelResult {
                data: result,
                metrics,
            };
        }

        // Parallel processing for large datasets
        let chunk_size = config.chunk_size.max(1);
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
        };

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
                throughput: (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64,
                memory_usage: (data_len * std::mem::size_of::<T>()) as u64,
                efficiency: 1.0,
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
        };

        ParallelResult {
            data: result,
            metrics,
        }
    }

    /// Group items by a computed key and record parallel execution metrics.
    ///
    /// The provided `key_fn` is applied to each item to compute its grouping key; items are moved
    /// into vectors stored under their respective keys. The relative order of items within each
    /// group is not guaranteed.
    ///
    /// # Parameters
    ///
    /// - `key_fn`: Function applied to each item to produce its grouping key.
    ///
    /// # Returns
    ///
    /// A `ParallelResult` containing a `HashMap` that maps each key to a `Vec` of items grouped
    /// under that key, along with `ParallelMetrics` describing the operation.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = ParallelConfig::default();
    /// let items = vec![1, 2, 3, 4, 5, 6];
    /// let result = items.into_iter().par_group_by(&config, |&x| x % 2);
    /// let groups = result.into_inner();
    /// let mut evens = groups.get(&0).cloned().unwrap_or_default();
    /// let mut odds = groups.get(&1).cloned().unwrap_or_default();
    /// evens.sort();
    /// odds.sort();
    /// assert_eq!(evens, vec![2, 4, 6]);
    /// assert_eq!(odds, vec![1, 3, 5]);
    /// ```
    fn par_group_by<K, F>(
        self,
        config: &ParallelConfig,
        key_fn: F,
    ) -> ParallelResult<HashMap<K, Vec<T>>>
    where
        K: Hash + Eq + Send + Clone + Sync,
        F: Fn(&T) -> K + Send + Sync,
        T: Send + Clone,
        Self: Sized,
    {
        let _start_time = Instant::now();

        // First reduce to parallel fold operation
        let groups = self.par_fold(
            config,
            HashMap::new(),
            |mut groups: HashMap<K, Vec<T>>, item| {
                let key = key_fn(&item);
                groups.entry(key).or_insert_with(Vec::new).push(item);
                groups
            },
            |mut left: HashMap<K, Vec<T>>, mut right: HashMap<K, Vec<T>>| {
                for (key, mut values) in right.drain() {
                    left.entry(key).or_insert_with(Vec::new).append(&mut values);
                }
                left
            },
        );

        groups
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
    }
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
}
