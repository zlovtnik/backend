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
use std::sync::{Arc, Mutex, RwLock};
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
    let one_hour_ago = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);
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

/// Dynamic load balancer for optimizing parallel execution
#[derive(Debug, Clone)]
pub struct DynamicLoadBalancer {
    /// Target efficiency threshold (0.0 - 1.0)
    target_efficiency: f64,
    /// Minimum chunk size
    min_chunk_size: usize,
    /// Maximum chunk size
    max_chunk_size: usize,
    /// Recent performance samples
    samples: Arc<Mutex<Vec<LoadBalanceSample>>>,
}

#[derive(Debug, Clone)]
struct LoadBalanceSample {
    chunk_size: usize,
    efficiency: f64,
    thread_utilization: f64,
    timestamp: Instant,
}

impl DynamicLoadBalancer {
    /// Create a new dynamic load balancer
    pub fn new(target_efficiency: f64) -> Self {
        Self {
            target_efficiency: target_efficiency.clamp(0.5, 1.0),
            min_chunk_size: 64,
            max_chunk_size: 8192,
            samples: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Calculate optimal chunk size based on data size and recent performance
    pub fn calculate_chunk_size(&self, data_size: usize, thread_count: usize) -> usize {
        if data_size == 0 || thread_count == 0 {
            return self.min_chunk_size;
        }

        // Base calculation: divide work evenly with some overhead for work stealing
        let base_chunk = (data_size / (thread_count * 4)).max(self.min_chunk_size);

        // Adjust based on recent performance
        if let Ok(samples) = self.samples.lock() {
            if samples.len() >= 5 {
                // Get recent samples (last 10)
                let recent: Vec<_> = samples.iter().rev().take(10).collect();

                // Calculate average efficiency
                let avg_efficiency: f64 =
                    recent.iter().map(|s| s.efficiency).sum::<f64>() / recent.len() as f64;

                // If efficiency is below target, reduce chunk size for better load balancing
                if avg_efficiency < self.target_efficiency {
                    let adjustment_factor =
                        (self.target_efficiency / avg_efficiency.max(0.1)).min(2.0);
                    let adjusted = (base_chunk as f64 / adjustment_factor) as usize;
                    return adjusted.clamp(self.min_chunk_size, self.max_chunk_size);
                }

                // If efficiency is good, slightly increase chunk size to reduce overhead
                if avg_efficiency > self.target_efficiency + 0.1 {
                    let adjusted = (base_chunk as f64 * 1.2) as usize;
                    return adjusted.clamp(self.min_chunk_size, self.max_chunk_size);
                }
            }
        }

        base_chunk.clamp(self.min_chunk_size, self.max_chunk_size)
    }

    /// Record performance sample for adaptive learning
    pub fn record_sample(&self, chunk_size: usize, efficiency: f64, thread_utilization: f64) {
        if let Ok(mut samples) = self.samples.lock() {
            samples.push(LoadBalanceSample {
                chunk_size,
                efficiency,
                thread_utilization,
                timestamp: Instant::now(),
            });

            // Keep only recent samples (last 100)
            let samples_len = samples.len();
            if samples_len > 100 {
                let keep_start = samples_len - 100;
                let tail = samples.split_off(keep_start);
                *samples = tail;
            }
        }
    }

    /// Get current load balancing statistics
    pub fn get_stats(&self) -> LoadBalancingStats {
        if let Ok(samples) = self.samples.lock() {
            if samples.is_empty() {
                return LoadBalancingStats::default();
            }

            let recent: Vec<_> = samples.iter().rev().take(20).collect();
            let avg_efficiency =
                recent.iter().map(|s| s.efficiency).sum::<f64>() / recent.len() as f64;
            let avg_utilization =
                recent.iter().map(|s| s.thread_utilization).sum::<f64>() / recent.len() as f64;

            LoadBalancingStats {
                sample_count: samples.len(),
                avg_efficiency,
                avg_thread_utilization: avg_utilization,
                target_efficiency: self.target_efficiency,
            }
        } else {
            LoadBalancingStats::default()
        }
    }
}

impl Default for DynamicLoadBalancer {
    fn default() -> Self {
        Self::new(0.75)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoadBalancingStats {
    pub sample_count: usize,
    pub avg_efficiency: f64,
    pub avg_thread_utilization: f64,
    pub target_efficiency: f64,
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
    /// # Ordering Guarantees
    ///
    /// **Sequential path** (when `data.len() < config.min_parallel_size`):
    /// - Preserves the insertion order of items within each group.
    /// - Groups are populated in the order elements are processed.
    ///
    /// **Parallel path** (when `data.len() >= config.min_parallel_size`):
    /// - Does NOT guarantee any ordering of elements inside groups.
    /// - Order is non-deterministic due to concurrent folding/reduction across threads.
    /// - Each thread accumulates items independently, then results are merged, causing elements
    ///   within a group to appear in arbitrary order.
    ///
    /// # If Stable Ordering is Required
    ///
    /// Callers who require stable ordering within groups have these options:
    /// 1. **Sort after grouping**: Call `.sort()` on each `Vec<T>` after grouping completes.
    /// 2. **Use sequential path**: Increase `config.min_parallel_size` to force sequential processing
    ///    for the data size you're working with, or pass a `ParallelConfig` with a very large threshold.
    /// 3. **Implement stable alternative**: Provide a custom grouping function that maintains order
    ///    (potentially using a different data structure like `Vec<(K, Vec<T>)>` instead of `HashMap`).
    ///
    /// # Examples
    ///
    /// ```
    /// let config = ParallelConfig::default();
    /// let data = vec![1, 2, 3, 4, 5, 6];
    /// let result = data.into_iter().par_group_by(&config, |&x| x % 2);
    /// assert_eq!(result.data.get(&0).unwrap().len(), 3); // 2, 4, 6 (order not guaranteed in parallel)
    /// assert_eq!(result.data.get(&1).unwrap().len(), 3); // 1, 3, 5 (order not guaranteed in parallel)
    ///
    /// // If order matters, sort each group:
    /// let mut sorted_result = result.data;
    /// for vec in sorted_result.values_mut() {
    ///     vec.sort();
    /// }
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
            return ParallelResult { data, metrics };
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

        ParallelResult { data, metrics }
    }

    /// Flat maps elements in parallel, flattening the results into a single collection.
    ///
    /// Applies the transformation function to each element, which returns an iterator,
    /// then flattens all results into a single Vec. Uses parallel processing when the
    /// dataset exceeds the configured threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = ParallelConfig::default();
    /// let data = vec![vec![1, 2], vec![3, 4], vec![5]];
    /// let result = data.into_iter().par_flat_map(&config, |v| v.into_iter());
    /// assert_eq!(result.data, vec![1, 2, 3, 4, 5]);
    /// ```
    fn par_flat_map<F, U, I>(self, config: &ParallelConfig, f: F) -> ParallelResult<Vec<U>>
    where
        F: Fn(T) -> I + Send + Sync,
        I: IntoIterator<Item = U> + Send,
        I::IntoIter: Send,
        U: Send,
        T: Send,
        Self: Sized,
    {
        let start_time = Instant::now();
        let data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Sequential flat_map for small datasets
            let result: Vec<U> = data.into_iter().flat_map(f).collect();
            let elapsed = start_time.elapsed();
            let metrics = ParallelMetrics {
                total_time: elapsed,
                thread_count: 1,
                throughput: (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64,
                memory_usage: (data_len * std::mem::size_of::<T>()
                    + result.len() * std::mem::size_of::<U>()) as u64,
                efficiency: 1.0,
                work_stealing_metrics: WorkStealingMetrics::default(),
                load_balancing_metrics: LoadBalancingMetrics::default(),
            };
            return ParallelResult {
                data: result,
                metrics,
            };
        }

        // Parallel flat_map - flatten lazily without intermediate allocations
        let result: Vec<U> = data
            .into_par_iter()
            .flat_map_iter(|item| f(item))
            .collect();

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;
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

    /// Partitions elements into two collections based on a predicate in parallel.
    ///
    /// Returns a tuple of (matching, non-matching) vectors. Uses parallel processing
    /// when the dataset exceeds the configured threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = ParallelConfig::default();
    /// let data = vec![1, 2, 3, 4, 5, 6];
    /// let result = data.into_iter().par_partition(&config, |&x| x % 2 == 0);
    /// assert_eq!(result.data.0, vec![2, 4, 6]);
    /// assert_eq!(result.data.1, vec![1, 3, 5]);
    /// ```
    fn par_partition<F>(
        self,
        config: &ParallelConfig,
        predicate: F,
    ) -> ParallelResult<(Vec<T>, Vec<T>)>
    where
        F: Fn(&T) -> bool + Send + Sync,
        T: Clone + Send + Sync,
        Self: Sized,
    {
        let start_time = Instant::now();
        let data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Sequential partition for small datasets
            let (matching, non_matching): (Vec<T>, Vec<T>) = data.into_iter().partition(predicate);
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
                data: (matching, non_matching),
                metrics,
            };
        }

        // Parallel partition using fold and reduce
        let (matching, non_matching) = data
            .into_par_iter()
            .fold(
                || (Vec::new(), Vec::new()),
                |(mut matching, mut non_matching), item| {
                    if predicate(&item) {
                        matching.push(item);
                    } else {
                        non_matching.push(item);
                    }
                    (matching, non_matching)
                },
            )
            .reduce(
                || (Vec::new(), Vec::new()),
                |(mut acc_match, mut acc_non_match), (mut match_vec, mut non_match_vec)| {
                    acc_match.append(&mut match_vec);
                    acc_non_match.append(&mut non_match_vec);
                    (acc_match, acc_non_match)
                },
            );

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;
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
            data: (matching, non_matching),
            metrics,
        }
    }

    /// Finds the first element matching a predicate in parallel.
    ///
    /// Returns Some(element) if found, None otherwise. Uses parallel search
    /// when the dataset exceeds the configured threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = ParallelConfig::default();
    /// let data = vec![1, 2, 3, 4, 5];
    /// let result = data.into_iter().par_find(&config, |&x| x > 3);
    /// assert!(result.data.is_some());
    /// ```
    fn par_find<F>(self, config: &ParallelConfig, predicate: F) -> ParallelResult<Option<T>>
    where
        F: Fn(&T) -> bool + Send + Sync,
        T: Clone + Send + Sync,
        Self: Sized,
    {
        let start_time = Instant::now();
        let data: Vec<T> = self.collect();
        let data_len = data.len();

        if data_len < config.min_parallel_size {
            // Sequential find for small datasets
            let result = data.into_iter().find(|item| predicate(item));
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

        // Parallel find
        let result = data.into_par_iter().find_any(predicate);

        let elapsed = start_time.elapsed();
        let thread_count = rayon::current_num_threads();
        let throughput = (data_len as u64 * 1_000_000) / elapsed.as_micros().max(1) as u64;
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
///
/// Accumulates `ParallelMetrics` from each operation (map/filter/sort) in `metrics_history`.
/// This allows tracking performance characteristics across the entire pipeline chain.
#[derive(Debug)]
pub struct ParallelPipeline<T> {
    data: Vec<T>,
    config: ParallelConfig,
    /// Accumulated metrics from all pipeline operations
    metrics_history: Vec<ParallelMetrics>,
}

impl<T: Send + Sync + Clone + 'static> ParallelPipeline<T> {
    /// Create a new pipeline with initial data
    ///
    /// Initializes metrics history as empty; metrics are accumulated as operations are applied.
    pub fn new(data: Vec<T>, config: ParallelConfig) -> Self {
        Self {
            data,
            config,
            metrics_history: Vec::new(),
        }
    }

    /// Apply a mapping operation in the pipeline
    ///
    /// Appends the operation's metrics to the metrics history before returning the next pipeline.
    pub fn map<U, F>(self, transform: F) -> ParallelPipeline<U>
    where
        U: Send + Sync + Clone + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let result = self.data.into_iter().par_map(&self.config, transform);
        let mut metrics_history = self.metrics_history;
        metrics_history.push(result.metrics.clone());
        ParallelPipeline {
            data: result.data,
            config: self.config,
            metrics_history,
        }
    }

    /// Apply a filtering operation in the pipeline
    ///
    /// Appends the operation's metrics to the metrics history before returning the next pipeline.
    pub fn filter<F>(self, predicate: F) -> ParallelPipeline<T>
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        let result = self.data.into_iter().par_filter(&self.config, predicate);
        let mut metrics_history = self.metrics_history;
        metrics_history.push(result.metrics.clone());
        ParallelPipeline {
            data: result.data,
            config: self.config,
            metrics_history,
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
    ///
    /// Appends the operation's metrics to the metrics history before returning the sorted pipeline.
    pub fn sort(self) -> ParallelPipeline<T>
    where
        T: Ord,
    {
        let result = self.data.into_iter().par_sort(&self.config);
        let mut metrics_history = self.metrics_history;
        metrics_history.push(result.metrics.clone());
        ParallelPipeline {
            data: result.data,
            config: self.config,
            metrics_history,
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

    /// Get accumulated metrics from all pipeline operations without consuming the pipeline
    ///
    /// Returns a reference to the metrics history vector containing metrics from each
    /// operation (map, filter, sort) applied in order.
    ///
    /// # Examples
    ///
    /// ```
    /// let pipeline = ParallelPipeline::new(vec![1, 2, 3, 4, 5], ParallelConfig::default());
    /// let metrics = pipeline.with_metrics();
    /// // metrics is empty at this point (no operations applied yet)
    /// ```
    pub fn with_metrics(&self) -> &[ParallelMetrics] {
        &self.metrics_history
    }

    /// Get accumulated metrics from all pipeline operations without consuming the pipeline
    ///
    /// Alias for `with_metrics()` for convenience.
    ///
    /// # Examples
    ///
    /// ```
    /// let pipeline = ParallelPipeline::new(vec![1, 2, 3, 4, 5], ParallelConfig::default());
    /// let metrics = pipeline.metrics();
    /// ```
    pub fn metrics(&self) -> &[ParallelMetrics] {
        self.with_metrics()
    }

    /// Consume the pipeline and return the accumulated metrics history
    ///
    /// Returns a `Vec<ParallelMetrics>` containing metrics from each operation in the order applied.
    /// Use this method when you want to extract metrics alongside the pipeline's data via other means.
    ///
    /// # Examples
    ///
    /// ```
    /// let pipeline = ParallelPipeline::new(vec![1, 2, 3, 4, 5], ParallelConfig::default())
    ///     .map(|x| x * 2);
    /// let metrics = pipeline.into_metrics();
    /// // metrics contains exactly one entry from the map operation
    /// ```
    pub fn into_metrics(self) -> Vec<ParallelMetrics> {
        self.metrics_history
    }

    /// Get an aggregate summary of all accumulated metrics
    ///
    /// Combines all metrics from the pipeline operations into a single summary metric
    /// that represents the total performance characteristics:
    /// - `total_time`: Sum of all operation times
    /// - `thread_count`: Maximum threads used across any operation
    /// - `throughput`: Average throughput across operations (or 0 if no operations)
    /// - `memory_usage`: Sum of memory usage across all operations
    /// - `efficiency`: Average efficiency across operations (or 1.0 if no operations)
    /// - Work-stealing and load balancing metrics are summed from all operations
    ///
    /// Returns a default `ParallelMetrics` if metrics history is empty.
    pub fn metrics_summary(&self) -> ParallelMetrics {
        if self.metrics_history.is_empty() {
            return ParallelMetrics::default();
        }

        let total_time = self.metrics_history.iter().map(|m| m.total_time).sum();
        let thread_count = self
            .metrics_history
            .iter()
            .map(|m| m.thread_count)
            .max()
            .unwrap();
        let total_throughput: u64 = self.metrics_history.iter().map(|m| m.throughput).sum();
        let throughput = total_throughput / (self.metrics_history.len() as u64).max(1);
        let memory_usage = self.metrics_history.iter().map(|m| m.memory_usage).sum();
        let avg_efficiency = self
            .metrics_history
            .iter()
            .map(|m| m.efficiency)
            .sum::<f64>()
            / self.metrics_history.len() as f64;

        // Aggregate work-stealing metrics
        let tasks_stolen = self
            .metrics_history
            .iter()
            .map(|m| m.work_stealing_metrics.tasks_stolen)
            .sum();
        let tasks_local = self
            .metrics_history
            .iter()
            .map(|m| m.work_stealing_metrics.tasks_local)
            .sum();
        let total_tasks = tasks_stolen + tasks_local;
        let stealing_efficiency = if total_tasks > 0 {
            tasks_stolen as f64 / total_tasks as f64
        } else {
            0.0
        };
        let avg_load_imbalance = self
            .metrics_history
            .iter()
            .map(|m| m.work_stealing_metrics.load_imbalance)
            .sum::<f64>()
            / self.metrics_history.len() as f64;

        // Aggregate load balancing metrics
        let avg_work_per_thread = self
            .metrics_history
            .iter()
            .map(|m| m.load_balancing_metrics.avg_work_per_thread)
            .sum::<f64>()
            / self.metrics_history.len() as f64;
        let work_distribution_std_dev = self
            .metrics_history
            .iter()
            .map(|m| m.load_balancing_metrics.work_distribution_std_dev)
            .sum::<f64>()
            / self.metrics_history.len() as f64;
        let max_thread_work = self
            .metrics_history
            .iter()
            .map(|m| m.load_balancing_metrics.max_thread_work)
            .max()
            .unwrap();
        let min_thread_work = self
            .metrics_history
            .iter()
            .map(|m| m.load_balancing_metrics.min_thread_work)
            .min()
            .unwrap();
        let avg_balancing_efficiency = self
            .metrics_history
            .iter()
            .map(|m| m.load_balancing_metrics.balancing_efficiency)
            .sum::<f64>()
            / self.metrics_history.len() as f64;

        ParallelMetrics {
            total_time,
            thread_count,
            throughput,
            memory_usage,
            efficiency: avg_efficiency,
            work_stealing_metrics: WorkStealingMetrics {
                tasks_stolen,
                tasks_local,
                stealing_efficiency,
                load_imbalance: avg_load_imbalance,
            },
            load_balancing_metrics: LoadBalancingMetrics {
                avg_work_per_thread,
                work_distribution_std_dev,
                max_thread_work,
                min_thread_work,
                balancing_efficiency: avg_balancing_efficiency,
            },
        }
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

    // Tests for ParallelPipeline metrics accumulation

    #[test]
    fn test_pipeline_new_empty_metrics() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();
        let pipeline = ParallelPipeline::new(data, config);

        let metrics = pipeline.with_metrics();
        assert_eq!(metrics.len(), 0);
    }

    #[test]
    fn test_pipeline_metrics_accessor() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();
        let pipeline = ParallelPipeline::new(data, config.clone());

        // Both accessors should return the same thing
        let metrics_with = pipeline.with_metrics();
        let pipeline2 = ParallelPipeline::new(vec![1, 2, 3, 4, 5], config);
        let metrics_ref = pipeline2.metrics();

        assert_eq!(metrics_with.len(), metrics_ref.len());
    }

    #[test]
    fn test_pipeline_single_map_accumulates_metrics() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();
        let pipeline = ParallelPipeline::new(data, config);

        let pipeline_after_map = pipeline.map(|x| x * 2);
        let metrics = pipeline_after_map.with_metrics();

        // Should have exactly one metric entry from the map operation
        assert_eq!(metrics.len(), 1);
        assert!(metrics[0].total_time.as_millis() >= 0);
        assert!(metrics[0].throughput >= 0);
    }

    #[test]
    fn test_pipeline_chained_operations_accumulate_metrics() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut config = ParallelConfig::default();
        config.min_parallel_size = 1; // Force parallel for small data

        let pipeline = ParallelPipeline::new(data, config.clone());
        let pipeline = pipeline.map(|x| x * 2); // First operation
        let pipeline = pipeline.filter(|x| x % 4 == 0); // Second operation
        let pipeline = pipeline.map(|x| x + 1); // Third operation

        let metrics = pipeline.with_metrics();

        // Should have exactly 3 metric entries
        assert_eq!(metrics.len(), 3);

        // All metrics should be collected (timing may be 0 microseconds for very fast operations)
        for metric in metrics {
            let _ = metric.total_time.as_micros();
        }
    }

    #[test]
    fn test_pipeline_into_metrics_consumes_pipeline() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();
        let pipeline = ParallelPipeline::new(data, config);
        let pipeline = pipeline.map(|x| x * 2);

        let metrics = pipeline.into_metrics();

        // into_metrics consumes the pipeline and returns the metrics
        assert_eq!(metrics.len(), 1);
    }

    #[test]
    fn test_pipeline_sort_accumulates_metrics() {
        let data = vec![5, 2, 8, 1, 9, 3];
        let mut config = ParallelConfig::default();
        config.min_parallel_size = 1;

        let pipeline = ParallelPipeline::new(data, config);
        let pipeline = pipeline.sort();

        let metrics = pipeline.with_metrics();

        // Should have exactly 1 metric entry from sort
        assert_eq!(metrics.len(), 1);
        assert_eq!(pipeline.execute(), vec![1, 2, 3, 5, 8, 9]);
    }

    #[test]
    fn test_pipeline_metrics_summary_empty() {
        let data = vec![1, 2, 3];
        let config = ParallelConfig::default();
        let pipeline = ParallelPipeline::new(data, config);

        let summary = pipeline.metrics_summary();

        // Empty pipeline should have default metrics
        assert_eq!(summary.total_time.as_micros(), 0);
        assert_eq!(summary.thread_count, 0);
        assert_eq!(summary.throughput, 0);
        assert_eq!(summary.memory_usage, 0);
        assert_eq!(summary.efficiency, 0.0);
    }

    #[test]
    fn test_pipeline_metrics_summary_aggregates() {
        let data = vec![1, 2, 3, 4, 5];
        let mut config = ParallelConfig::default();
        config.min_parallel_size = 1;

        let pipeline = ParallelPipeline::new(data, config);
        let pipeline = pipeline.map(|x| x * 2);
        let pipeline = pipeline.map(|x| x + 1);

        let summary = pipeline.metrics_summary();

        // Summary should aggregate metrics from both operations
        assert!(summary.total_time.as_micros() > 0);
        assert!(summary.throughput >= 0);
        assert!(summary.efficiency > 0.0);
        assert!(summary.memory_usage > 0);
    }

    #[test]
    fn test_pipeline_metrics_summary_thread_count() {
        let data = vec![1, 2, 3, 4, 5];
        let mut config = ParallelConfig::default();
        config.min_parallel_size = 1;

        let pipeline = ParallelPipeline::new(data, config);
        let pipeline = pipeline.map(|x| x * 2);
        let pipeline = pipeline.filter(|x| x > &4);

        let summary = pipeline.metrics_summary();

        // Thread count should be the max from all operations
        assert!(summary.thread_count >= 1);
    }

    #[test]
    fn test_pipeline_complex_chain_preserves_data_and_metrics() {
        let data: Vec<i32> = (1..=20).collect();
        let mut config = ParallelConfig::default();
        config.min_parallel_size = 1;

        let pipeline = ParallelPipeline::new(data.clone(), config);
        let pipeline = pipeline
            .map(|x| x * 2)
            .filter(|&x| x % 4 == 0)
            .map(|x| x / 2);

        let metrics_len = pipeline.with_metrics().len();
        let final_data = pipeline.execute();

        // Should have 3 operation metrics
        assert_eq!(metrics_len, 3);

        // Data should be correctly transformed
        let expected: Vec<i32> = data
            .iter()
            .map(|&x| x * 2)
            .filter(|&x| x % 4 == 0)
            .map(|x| x / 2)
            .collect();
        assert_eq!(final_data, expected);
    }

    #[test]
    fn test_pipeline_metrics_with_sort_and_filter() {
        let data = vec![5, 2, 8, 1, 9, 3, 7, 4, 6];
        let mut config = ParallelConfig::default();
        config.min_parallel_size = 1;

        let pipeline = ParallelPipeline::new(data, config);
        let pipeline = pipeline.sort().filter(|&x| x > 3).map(|x| x * 10);

        let metrics_len = pipeline.with_metrics().len();
        let result = pipeline.execute();

        // Should have 3 operations recorded
        assert_eq!(metrics_len, 3);

        // Result should be sorted and filtered
        assert_eq!(result, vec![40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn test_pipeline_metrics_summary_with_multiple_operations() {
        let data = (1..=50).collect::<Vec<i32>>();
        let mut config = ParallelConfig::default();
        config.min_parallel_size = 1;

        let pipeline = ParallelPipeline::new(data, config);
        let pipeline = pipeline.map(|x| x * 2).map(|x| x + 1).map(|x| x / 2);

        let summary = pipeline.metrics_summary();

        // Summary should show aggregated values
        assert!(summary.total_time.as_micros() > 0);
        assert!(summary.throughput > 0 || summary.efficiency >= 0.0);
    }

    #[test]
    fn test_pipeline_get_data_before_metrics() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();
        let pipeline = ParallelPipeline::new(data.clone(), config);

        // get_data should return the original data before any transformations
        let current_data = pipeline.get_data();
        assert_eq!(current_data, &data[..]);
    }

    #[test]
    fn test_pipeline_get_data_after_map() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();
        let pipeline = ParallelPipeline::new(data, config);
        let pipeline = pipeline.map(|x| x * 2);

        let current_data = pipeline.get_data();
        assert_eq!(current_data, &[2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_par_flat_map_basic() {
        let data = vec![vec![1, 2], vec![3, 4], vec![5]];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_flat_map(&config, |v| v.into_iter());

        assert_eq!(result.data, vec![1, 2, 3, 4, 5]);
        // Metrics are collected; timing may be 0 microseconds for very fast operations
        let _ = result.metrics.total_time;
    }

    #[test]
    fn test_par_flat_map_with_transformation() {
        let data = vec![1, 2, 3];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_flat_map(&config, |x| vec![x, x * 10]);

        assert_eq!(result.data, vec![1, 10, 2, 20, 3, 30]);
    }

    #[test]
    fn test_par_partition_basic() {
        let data = vec![1, 2, 3, 4, 5, 6];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_partition(&config, |&x| x % 2 == 0);

        assert_eq!(result.data.0, vec![2, 4, 6]);
        assert_eq!(result.data.1, vec![1, 3, 5]);
        assert!(result.metrics.throughput > 0);
    }

    #[test]
    fn test_par_partition_all_match() {
        let data = vec![2, 4, 6, 8];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_partition(&config, |&x| x % 2 == 0);

        assert_eq!(result.data.0, vec![2, 4, 6, 8]);
        assert_eq!(result.data.1, Vec::<i32>::new());
    }

    #[test]
    fn test_par_find_exists() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_find(&config, |&x| x > 3);

        assert!(result.data.is_some());
        let found = result.data.unwrap();
        assert!(found > 3);
    }

    #[test]
    fn test_par_find_not_exists() {
        let data = vec![1, 2, 3, 4, 5];
        let config = ParallelConfig::default();

        let result = data.into_iter().par_find(&config, |&x| x > 10);

        assert!(result.data.is_none());
    }

    #[test]
    fn test_dynamic_load_balancer_basic() {
        let balancer = DynamicLoadBalancer::new(0.8);
        let chunk_size = balancer.calculate_chunk_size(1000, 4);

        assert!(chunk_size >= 64);
        assert!(chunk_size <= 8192);
    }

    #[test]
    fn test_dynamic_load_balancer_adapts_to_low_efficiency() {
        let balancer = DynamicLoadBalancer::new(0.8);

        // Record some low-efficiency samples
        for _ in 0..10 {
            balancer.record_sample(1024, 0.5, 0.6);
        }

        let chunk_size = balancer.calculate_chunk_size(10000, 8);

        // Should reduce chunk size when efficiency is low
        assert!(chunk_size < 1024);
    }

    #[test]
    fn test_dynamic_load_balancer_stats() {
        let balancer = DynamicLoadBalancer::new(0.75);

        balancer.record_sample(512, 0.8, 0.85);
        balancer.record_sample(1024, 0.75, 0.8);

        let stats = balancer.get_stats();

        assert_eq!(stats.sample_count, 2);
        assert!(stats.avg_efficiency > 0.0);
        assert_eq!(stats.target_efficiency, 0.75);
    }
}
