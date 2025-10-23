//! Concurrent Functional Processing Engine
//!
//! Provides high-level abstractions for executing functional data
//! transformations concurrently across multiple threads while
//! integrating cleanly with the Actix Web async runtime.
//!
//! # Key Capabilities
//!
//! - `ConcurrentProcessor` orchestrates a dedicated Rayon thread pool
//!   with functional-friendly configuration knobs
//! - Parallel map, filter, fold, and group-by helpers that capture rich
//!   performance metrics via `ParallelResult`
//! - Async bridging helpers that connect Rayon workloads to Actix/Tokio
//!   runtimes (`map_async`, `fold_async`, `map_actix_blocking`)
//! - Batch orchestration utilities such as `process_batch` for service
//!   pipelines that need convenient parallelisation
//! - Metrics aggregation helpers (`aggregate_metrics`) to summarise
//!   concurrent workloads at runtime
//!
//! The module emphasises thread-safe parallel composition, runtime
//! observability, and ergonomic async bridging so functional pipelines
//! can scale without manual thread management.

use std::collections::HashMap;
use std::sync::Arc;
#[cfg(test)]
use std::time::Duration;

use actix_web::error::BlockingError;
use actix_web::web;
use rayon::{ThreadPool, ThreadPoolBuilder};
use tokio::task;

use super::parallel_iterators::{
    ParallelConfig, ParallelIteratorExt, ParallelMetrics, ParallelResult,
};

use thiserror::Error;

/// Error type for concurrent processing failures
#[derive(Debug, Error)]
pub enum ConcurrentProcessingError {
    /// Error raised when a custom Rayon thread pool cannot be constructed
    #[error("Failed to build Rayon thread pool: {0}")]
    ThreadPoolBuild(String),
    /// Error raised when a spawned blocking task is cancelled or panics
    #[error("Blocking task was cancelled or panicked")]
    JoinError,
    /// Error raised when Actix's blocking adapter fails to execute the task
    #[error("Actix blocking task failed: {0}")]
    ActixBlocking(String),
}

/// Immutable dataset wrapper that ensures data can be shared safely across threads.
///
/// Internally uses `Arc<[T]>` to provide cheap clones while preventing mutation.
#[derive(Clone)]
pub struct ImmutableDataset<T> {
    inner: Arc<[T]>,
}

impl<T> ImmutableDataset<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Construct an ImmutableDataset from an owned `Vec`.
    ///
    /// # Examples
    ///
    /// ```
    /// let ds = ImmutableDataset::new(vec![1, 2, 3]);
    /// assert_eq!(ds.len(), 3);
    /// ```
    pub fn new(data: Vec<T>) -> Self {
        Self {
            inner: Arc::<[T]>::from(data),
        }
    }

    /// Create an immutable dataset from an existing shared slice.
    pub fn from_arc(inner: Arc<[T]>) -> Self {
        Self { inner }
    }

    /// Get the number of elements in the dataset.
    ///
    /// # Returns
    ///
    /// The number of elements in the dataset.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let ds = ImmutableDataset::new(vec![1, 2, 3]);
    /// assert_eq!(ds.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` when the dataset has no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Materialises the shared data into an owned vector.
    ///
    /// This clones each element, preserving immutability of the shared backing slice.
    pub fn to_vec(&self) -> Vec<T> {
        self.inner.iter().cloned().collect()
    }

    /// Provides a read-only slice view of the dataset's contents.
    ///
    /// The returned slice borrows from the internal shared storage and does not clone or move the data.
    ///
    /// # Examples
    ///
    /// ```
    /// let ds = ImmutableDataset::new(vec![1, 2, 3]);
    /// let s = ds.as_slice();
    /// assert_eq!(s, &[1, 2, 3]);
    /// ```
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }
}

/// High-level coordinator for executing functional workloads concurrently.
#[derive(Clone)]
pub struct ConcurrentProcessor {
    pool: Arc<ThreadPool>,
    config: ParallelConfig,
}

impl ConcurrentProcessor {
    /// Creates a ConcurrentProcessor configured with the provided `ParallelConfig`.
    ///
    /// On success returns the constructed `ConcurrentProcessor`. On failure returns
    /// `ConcurrentProcessingError::ThreadPoolBuild` containing a diagnostic message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// // Assuming ParallelConfig implements Default
    /// let cfg = ParallelConfig::default();
    /// let proc = ConcurrentProcessor::new(cfg).expect("failed to build processor");
    /// assert_eq!(proc.config().thread_pool_size, ParallelConfig::default().thread_pool_size);
    /// ```
    pub fn new(config: ParallelConfig) -> Result<Self, ConcurrentProcessingError> {
        let mut builder = ThreadPoolBuilder::new();

        if config.thread_pool_size > 0 {
            builder = builder.num_threads(config.thread_pool_size);
        }

        let pool = builder
            .build()
            .map_err(|err| ConcurrentProcessingError::ThreadPoolBuild(err.to_string()))?;

        Ok(Self {
            pool: Arc::new(pool),
            config,
        })
    }

    /// Create a ConcurrentProcessor using the default parallel configuration.
    ///
    /// # Returns
    ///
    /// `Ok(processor)` initialized with `ParallelConfig::default()`, or an `Err(ConcurrentProcessingError::ThreadPoolBuild(_))` if the Rayon thread pool cannot be built.
    ///
    /// # Examples
    ///
    /// ```
    /// let proc = ConcurrentProcessor::try_default().expect("failed to build default processor");
    /// assert_eq!(proc.config(), &ParallelConfig::default());
    /// ```
    pub fn try_default() -> Result<Self, ConcurrentProcessingError> {
        Self::new(ParallelConfig::default())
    }

    /// Returns the active parallel configuration.
    pub fn config(&self) -> &ParallelConfig {
        &self.config
    }

    /// Creates a new processor with a different configuration.
    pub fn with_config(&self, config: ParallelConfig) -> Result<Self, ConcurrentProcessingError> {
        Self::new(config)
    }

    /// Applies a transformation to each item of an input iterator in parallel.
    ///
    /// Consumes the input iterator and returns a `Vec` containing one transformed value per input element.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::concurrent_processing::ConcurrentProcessor;
    ///
    /// let proc = ConcurrentProcessor::try_default().unwrap();
    /// let out = proc.map(vec![1, 2, 3], |x| x * 2).unwrap();
    /// assert_eq!(out, vec![2, 4, 6]);
    /// ```
    pub fn map<I, T, U, F>(&self, data: I, transform: F) -> ParallelResult<Vec<U>>
    where
        I: IntoIterator<Item = T>,
        T: Send + Sync + 'static,
        U: Send + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let items: Vec<T> = data.into_iter().collect();

        self.pool
            .install(|| items.into_iter().par_map(&config, transform))
    }

    /// Applies `transform` to each element of an immutable dataset in parallel and collects the results.
    ///
    /// Returns a `ParallelResult<Vec<U>>` containing the transformed items on success.
    ///
    /// # Examples
    ///
    /// ```
    /// let processor = ConcurrentProcessor::try_default().unwrap();
    /// let dataset = ImmutableDataset::new(vec![1, 2, 3]);
    /// let doubled = processor.map_dataset(&dataset, |x| x * 2).unwrap();
    /// assert_eq!(doubled, vec![2, 4, 6]);
    /// ```
    pub fn map_dataset<T, U, F>(
        &self,
        data: &ImmutableDataset<T>,
        transform: F,
    ) -> ParallelResult<Vec<U>>
    where
        T: Clone + Send + Sync + 'static,
        U: Send + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let owned = data.to_vec();
        self.map(owned, transform)
    }

    /// Reduce an iterator to a single value using a parallel fold on the processor's Rayon thread pool.
    ///
    /// The `fold` closure accumulates items into a partition-local accumulator of type `B`; the `combine`
    /// closure merges two partition accumulators into one to produce the final result.
    ///
    /// # Returns
    ///
    /// `ParallelResult<B>` containing the final accumulated value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let proc = ConcurrentProcessor::try_default().expect("build processor");
    /// let items = vec![1, 2, 3, 4];
    /// let result = proc
    ///     .fold(items, 0, |acc, x| acc + x, |a, b| a + b)
    ///     .expect("parallel fold succeeded");
    /// assert_eq!(result, 10);
    /// ```
    pub fn fold<I, T, B, F, C>(&self, data: I, init: B, fold: F, combine: C) -> ParallelResult<B>
    where
        I: IntoIterator<Item = T>,
        T: Send + Sync + 'static,
        B: Send + Clone + Sync + 'static,
        F: Fn(B, T) -> B + Send + Sync + 'static,
        C: Fn(B, B) -> B + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let items: Vec<T> = data.into_iter().collect();

        self.pool
            .install(|| items.into_iter().par_fold(&config, init, fold, combine))
    }

    /// Filters items in parallel while preserving the input order.
    ///
    /// Applies `predicate` to each item concurrently and collects all items for which the predicate
    /// returns `true`, keeping the same relative ordering as the input.
    ///
    /// # Returns
    ///
    /// A `ParallelResult<Vec<T>>` containing the items for which `predicate` returns `true`,
    /// in the same order they appeared in the input.
    ///
    /// # Examples
    ///
    /// ```
    /// let processor = ConcurrentProcessor::try_default().unwrap();
    /// let input = vec![1, 2, 3, 4];
    /// let result = processor.filter(input, |&x| x % 2 == 0).unwrap();
    /// assert_eq!(result, vec![2, 4]);
    /// ```
    pub fn filter<I, T, Filt>(&self, data: I, predicate: Filt) -> ParallelResult<Vec<T>>
    where
        I: IntoIterator<Item = T>,
        T: Clone + Send + Sync + 'static,
        Filt: Fn(&T) -> bool + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let items: Vec<T> = data.into_iter().collect();

        self.pool
            .install(|| items.into_iter().par_filter(&config, predicate))
    }

    /// Groups items by a key produced from each element using the provided key function.
    ///
    /// Returns a `HashMap` that maps each distinct key to a `Vec<T>` containing the items that produced that key.
    ///
    /// # Examples
    ///
    /// ```
    /// let proc = ConcurrentProcessor::try_default().unwrap();
    /// let items = vec![1, 2, 3, 4, 5];
    /// let grouped = proc.group_by(items, |&n| n % 2 == 0);
    /// assert_eq!(grouped.get(&true).unwrap().len(), 2); // 2 and 4
    /// assert_eq!(grouped.get(&false).unwrap().len(), 3); // 1, 3, 5
    /// ```
    pub fn group_by<I, T, K, KeyFn>(
        &self,
        data: I,
        key_fn: KeyFn,
    ) -> ParallelResult<HashMap<K, Vec<T>>>
    where
        I: IntoIterator<Item = T>,
        T: Clone + Send + Sync + 'static,
        K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
        KeyFn: Fn(&T) -> K + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let items: Vec<T> = data.into_iter().collect();

        self.pool
            .install(|| items.into_iter().par_group_by(&config, key_fn))
    }

    /// Runs a parallel map over the input using the processor's Rayon thread pool and returns the collected results.
    ///
    /// The input iterator is consumed and each item is transformed in parallel on the processor's pool; the final collection
    /// preserves the order produced by the parallel iterator.
    ///
    /// # Returns
    ///
    /// `Ok(ParallelResult<Vec<U>>)` with the transformed items on success, `Err(ConcurrentProcessingError::JoinError)` if the spawned blocking task was cancelled or panicked.
    ///
    /// # Examples
    ///
    /// ```
    /// // Run inside a Tokio runtime in tests or examples.
    /// use crate::functional::concurrent_processing::{ConcurrentProcessor, ParallelConfig};
    ///
    /// let rt = tokio::runtime::Runtime::new().unwrap();
    /// let proc = ConcurrentProcessor::try_default().unwrap();
    ///
    /// let out = rt.block_on(async {
    ///     proc.map_async(0..4, |x| x * 2).await.unwrap().into_inner()
    /// });
    ///
    /// assert_eq!(out, vec![0, 2, 4, 6]);
    /// ```
    pub async fn map_async<I, T, U, F>(
        &self,
        data: I,
        transform: F,
    ) -> Result<ParallelResult<Vec<U>>, ConcurrentProcessingError>
    where
        I: IntoIterator<Item = T> + Send + 'static,
        T: Send + Sync + 'static,
        U: Send + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let pool = self.pool.clone();
        let items: Vec<T> = data.into_iter().collect();

        task::spawn_blocking(move || pool.install(|| items.into_iter().par_map(&config, transform)))
            .await
            .map_err(|_| ConcurrentProcessingError::JoinError)
    }

    /// Performs a parallel fold across the provided items using the processor's Rayon thread pool.
    ///
    /// The folding operation is applied to items in parallel; `fold` produces partial accumulators
    /// for each partition and `combine` merges them into the final result.
    ///
    /// # Returns
    ///
    /// `Ok(ParallelResult<B>)` with the final accumulated value on success, or
    /// `Err(ConcurrentProcessingError::JoinError)` if the spawned blocking task was cancelled or panicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// # use crate::functional::concurrent_processing::{ConcurrentProcessor, ParallelConfig};
    /// // create a runtime and a processor (adjust to your crate's public API as needed)
    /// let rt = tokio::runtime::Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     let proc = ConcurrentProcessor::try_default().unwrap();
    ///     let items = vec![1u32, 2, 3, 4];
    ///     let result = proc
    ///         .fold_async(items, 0u32, |acc, x| acc + x, |a, b| a + b)
    ///         .await
    ///         .unwrap()
    ///         .into_inner();
    ///     assert_eq!(result, 10);
    /// });
    /// ```
    pub async fn fold_async<I, T, B, F, C>(
        &self,
        data: I,
        init: B,
        fold: F,
        combine: C,
    ) -> Result<ParallelResult<B>, ConcurrentProcessingError>
    where
        I: IntoIterator<Item = T> + Send + 'static,
        T: Send + Sync + 'static,
        B: Send + Clone + Sync + 'static,
        F: Fn(B, T) -> B + Send + Sync + 'static,
        C: Fn(B, B) -> B + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let pool = self.pool.clone();
        let items: Vec<T> = data.into_iter().collect();

        task::spawn_blocking(move || {
            pool.install(|| items.into_iter().par_fold(&config, init, fold, combine))
        })
        .await
        .map_err(|_| ConcurrentProcessingError::JoinError)
    }

    /// Execute a parallel map over the provided input using this processor's Rayon thread pool via Actix Web's blocking adapter.
    ///
    /// This runs the given `transform` in parallel on the input values inside `web::block`, allowing the work to be awaited safely from an Actix handler while keeping Rayon work on the processor's pool.
    ///
    /// # Returns
    ///
    /// `Ok(ParallelResult<Vec<U>>)` with the mapped results on success, `Err(ConcurrentProcessingError::ActixBlocking(_))` if the Actix blocking adapter fails.
    ///
    /// # Examples
    ///
    /// ```
    /// // Called from an async Actix handler:
    /// // let res = proc.map_actix_blocking(vec![1,2,3], |n| n * 2).await.unwrap().unwrap();
    /// ```
    pub async fn map_actix_blocking<I, T, U, F>(
        &self,
        data: I,
        transform: F,
    ) -> Result<ParallelResult<Vec<U>>, ConcurrentProcessingError>
    where
        I: IntoIterator<Item = T> + Send + 'static,
        T: Send + Sync + 'static,
        U: Send + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let pool = self.pool.clone();
        let items: Vec<T> = data.into_iter().collect();

        web::block(move || pool.install(|| items.into_iter().par_map(&config, transform)))
            .await
            .map_err(|err: BlockingError| ConcurrentProcessingError::ActixBlocking(err.to_string()))
    }

    /// Apply a processor function to every element of a batch and return the mapped results.
    ///
    /// Runs the mapping on the processor's configured thread pool and returns the collected
    /// parallel execution result or an error if execution fails.
    ///
    /// # Returns
    /// `Ok(ParallelResult<Vec<U>>)` with mapped values on success, `Err(ConcurrentProcessingError)` if execution fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example(proc: &super::ConcurrentProcessor) {
    /// let input = vec![1, 2, 3];
    /// let result = proc.process_batch(input, |x| x * 2).await.unwrap();
    /// assert_eq!(result.len(), 3);
    /// assert_eq!(result[0], 2);
    /// # }
    /// ```
    pub async fn process_batch<T, U, F>(
        &self,
        data: Vec<T>,
        processor: F,
    ) -> Result<ParallelResult<Vec<U>>, ConcurrentProcessingError>
    where
        T: Send + Sync + 'static,
        U: Send + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        self.map_async(data, processor).await
    }
}

/// Produces a consolidated ParallelMetrics by combining an iterator of metrics.
///
/// Returns a `ParallelMetrics` whose `total_time`, `throughput`, and `memory_usage` are summed,
/// whose `thread_count` is the maximum observed, and whose `efficiency` is the arithmetic mean
/// of the input efficiencies (0.0 if the iterator is empty).
///
/// # Examples
///
/// ```
/// # use std::time::Duration;
/// use crate::functional::parallel_iterators::ParallelMetrics;
/// use crate::functional::concurrent_processing::aggregate_metrics;
///
/// let mut a = ParallelMetrics::default();
/// a.total_time = Duration::from_millis(10);
/// a.thread_count = 4;
/// a.throughput = 100;
/// a.memory_usage = 256;
/// a.efficiency = 0.8;
///
/// let mut b = ParallelMetrics::default();
/// b.total_time = Duration::from_millis(5);
/// b.thread_count = 8;
/// b.throughput = 50;
/// b.memory_usage = 128;
/// b.efficiency = 0.6;
///
/// let combined = aggregate_metrics([&a, &b].into_iter());
/// assert_eq!(combined.total_time, Duration::from_millis(15));
/// assert_eq!(combined.thread_count, 8);
/// assert_eq!(combined.throughput, 150);
/// assert_eq!(combined.memory_usage, 384);
/// // efficiency is averaged: (0.8 + 0.6) / 2 = 0.7
/// assert!((combined.efficiency - 0.7).abs() < 1e-12);
/// ```
pub fn aggregate_metrics<'a, I>(metrics: I) -> ParallelMetrics
where
    I: IntoIterator<Item = &'a ParallelMetrics>,
{
    let mut aggregated = ParallelMetrics::default();
    let mut count = 0_u64;

    for metric in metrics {
        aggregated.total_time += metric.total_time;
        aggregated.thread_count = aggregated.thread_count.max(metric.thread_count);
        aggregated.throughput = aggregated.throughput.saturating_add(metric.throughput);
        aggregated.memory_usage = aggregated.memory_usage.saturating_add(metric.memory_usage);
        aggregated.efficiency += metric.efficiency;
        count += 1;
    }

    if count > 0 {
        aggregated.efficiency /= count as f64;
    }

    aggregated
}

/// Create a default `ConcurrentProcessor` for tests and examples.
///
/// The returned processor is configured with `ParallelConfig::default()`.
///
/// # Panics
///
/// Panics if the underlying Rayon thread pool cannot be constructed.
///
/// # Examples
///
/// ```
/// let proc = processor();
/// // use `proc` to run parallel operations in tests
/// ```
pub fn processor() -> ConcurrentProcessor {
    ConcurrentProcessor::try_default().expect("thread pool should build")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_parallel_basic() {
        let processor = processor();
        let data = vec![1, 2, 3, 4, 5];

        let result = processor.map(data, |x| x * 2);

        assert_eq!(result.data, vec![2, 4, 6, 8, 10]);
        assert!(result.metrics.efficiency >= 0.0);
    }

    #[test]
    fn map_parallel_with_dataset() {
        let processor = processor();
        let dataset = ImmutableDataset::new(vec![10, 20, 30]);

        let result = processor.map_dataset(&dataset, |x| x / 10);

        assert_eq!(result.data, vec![1, 2, 3]);
        assert_eq!(dataset.len(), 3);
    }

    #[test]
    fn aggregate_metrics_combines_values() {
        let mut a = ParallelMetrics::default();
        a.throughput = 100;
        a.memory_usage = 10;
        a.efficiency = 0.8;
        a.total_time = Duration::from_millis(10);

        let mut b = ParallelMetrics::default();
        b.throughput = 200;
        b.memory_usage = 20;
        b.efficiency = 0.6;
        b.total_time = Duration::from_millis(20);

        let aggregated = aggregate_metrics([&a, &b]);

        assert_eq!(aggregated.throughput, 300);
        assert_eq!(aggregated.memory_usage, 30);
        assert_eq!(aggregated.total_time, Duration::from_millis(30));
        assert!((aggregated.efficiency - 0.7).abs() < f64::EPSILON);
    }

    #[actix_rt::test]
    async fn map_async_executes_on_thread_pool() {
        let processor = processor();
        let data = (0..1000).collect::<Vec<_>>();

        let result = processor
            .map_async(data, |x| x * x)
            .await
            .expect("async map should succeed");

        assert_eq!(result.data.len(), 1000);
        assert_eq!(result.data[10], 100);
    }

    #[actix_rt::test]
    async fn map_actix_blocking_integrates_with_runtime() {
        let processor = processor();
        let dataset = ImmutableDataset::new((1..=64).collect::<Vec<_>>());

        let result = processor
            .map_actix_blocking(dataset.to_vec(), |x| x * 3)
            .await
            .expect("actix blocking map should succeed");

        assert_eq!(result.data[0], 3);
        assert_eq!(result.data.last().copied(), Some(192));
    }
}

#[test]
fn immutable_dataset_creation() {
    let data = vec![1, 2, 3, 4, 5];
    let dataset = ImmutableDataset::new(data);

    assert_eq!(dataset.len(), 5);
    assert!(!dataset.is_empty());
    assert_eq!(dataset.as_slice()[0], 1);
}

#[test]
fn immutable_dataset_to_vec_clones_data() {
    let dataset = ImmutableDataset::new(vec![10, 20, 30]);
    let cloned = dataset.to_vec();

    assert_eq!(cloned, vec![10, 20, 30]);
    assert_eq!(dataset.len(), 3); // Original still intact
}

#[test]
fn immutable_dataset_from_arc() {
    let arc_data = Arc::<[i32]>::from(vec![1, 2, 3]);
    let dataset = ImmutableDataset::from_arc(arc_data);

    assert_eq!(dataset.len(), 3);
    assert_eq!(dataset.as_slice(), &[1, 2, 3]);
}

#[test]
fn immutable_dataset_empty() {
    let dataset: ImmutableDataset<i32> = ImmutableDataset::new(vec![]);

    assert_eq!(dataset.len(), 0);
    assert!(dataset.is_empty());
}

#[test]
fn concurrent_processor_config() {
    let config = ParallelConfig {
        thread_pool_size: 4,
        min_parallel_size: 10,
        enable_work_stealing: true,
        chunk_size: 100,
    };

    let processor = ConcurrentProcessor::new(config).expect("should build processor");
    assert_eq!(processor.config().thread_pool_size, 4);
    assert_eq!(processor.config().min_parallel_size, 10);
}

#[test]
fn concurrent_processor_with_config() {
    let processor = ConcurrentProcessor::try_default().unwrap();
    let new_config = ParallelConfig {
        thread_pool_size: 2,
        min_parallel_size: 50,
        enable_work_stealing: false,
        chunk_size: 200,
    };

    let new_processor = processor.with_config(new_config).expect("should build");
    assert_eq!(new_processor.config().thread_pool_size, 2);
}

#[test]
fn map_empty_collection() {
    let processor = processor();
    let data: Vec<i32> = vec![];

    let result = processor.map(data, |x| x * 2);

    assert!(result.data.is_empty());
    assert_eq!(result.metrics.thread_count, 1);
}

#[test]
fn map_large_dataset() {
    let processor = processor();
    let data: Vec<i32> = (0..10000).collect();

    let result = processor.map(data, |x| x + 1);

    assert_eq!(result.data.len(), 10000);
    assert_eq!(result.data[0], 1);
    assert_eq!(result.data[9999], 10000);
}

#[test]
fn fold_accumulates_values() {
    let processor = processor();
    let data = vec![1, 2, 3, 4, 5];

    let result = processor.fold(data, 0, |acc, x| acc + x, |a, b| a + b);

    assert_eq!(result.data, 15);
}

#[test]
fn fold_with_complex_accumulator() {
    let processor = processor();
    let data = vec!["hello", "world", "rust"];

    let result = processor.fold(
        data,
        String::new(),
        |mut acc, word| {
            if !acc.is_empty() {
                acc.push(' ');
            }
            acc.push_str(word);
            acc
        },
        |mut a, b| {
            if !a.is_empty() && !b.is_empty() {
                a.push(' ');
            }
            a.push_str(&b);
            a
        },
    );

    assert!(result.data.contains("hello"));
    assert!(result.data.contains("world"));
    assert!(result.data.contains("rust"));
}

#[test]
fn filter_preserves_matching_items() {
    let processor = processor();
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let result = processor.filter(data, |x| x % 2 == 0);

    assert_eq!(result.data, vec![2, 4, 6, 8, 10]);
}

#[test]
fn filter_all_match() {
    let processor = processor();
    let data = vec![2, 4, 6, 8];

    let result = processor.filter(data, |x| x % 2 == 0);

    assert_eq!(result.data, vec![2, 4, 6, 8]);
}

#[test]
fn filter_none_match() {
    let processor = processor();
    let data = vec![1, 3, 5, 7];

    let result = processor.filter(data, |x| x % 2 == 0);

    assert!(result.data.is_empty());
}

#[test]
fn group_by_creates_buckets() {
    let processor = processor();
    let data = vec![1, 2, 3, 4, 5, 6];

    let result = processor.group_by(data, |x| x % 3);

    assert_eq!(result.data.len(), 3); // Groups: 0, 1, 2
    assert!(result.data.contains_key(&0));
    assert!(result.data.contains_key(&1));
    assert!(result.data.contains_key(&2));
}

#[test]
fn group_by_string_keys() {
    let processor = processor();
    let data = vec!["apple", "apricot", "banana", "blueberry", "cherry"];

    let result = processor.group_by(data, |word| word.chars().next().unwrap_or('_'));

    assert_eq!(result.data.get(&'a').unwrap().len(), 2);
    assert_eq!(result.data.get(&'b').unwrap().len(), 2);
    assert_eq!(result.data.get(&'c').unwrap().len(), 1);
}

#[actix_rt::test]
async fn fold_async_with_strings() {
    let processor = processor();
    let data = vec!["a", "b", "c"];

    let result = processor
        .fold_async(
            data,
            String::new(),
            |mut acc, s| {
                acc.push_str(s);
                acc
            },
            |mut a, b| {
                a.push_str(&b);
                a
            },
        )
        .await
        .expect("fold_async should succeed");

    assert!(result.data.contains('a'));
    assert!(result.data.contains('b'));
    assert!(result.data.contains('c'));
}

#[actix_rt::test]
async fn process_batch_legacy_api() {
    let processor = processor();
    let data = vec![10, 20, 30, 40];

    let result = processor
        .process_batch(data, |x| x / 10)
        .await
        .expect("process_batch should succeed");

    assert_eq!(result.data, vec![1, 2, 3, 4]);
}

#[test]
fn aggregate_metrics_empty_iterator() {
    let metrics: Vec<&ParallelMetrics> = vec![];
    let aggregated = aggregate_metrics(metrics);

    assert_eq!(aggregated.throughput, 0);
    assert_eq!(aggregated.memory_usage, 0);
    assert_eq!(aggregated.total_time, Duration::from_secs(0));
}

#[test]
fn aggregate_metrics_single_metric() {
    let mut m = ParallelMetrics::default();
    m.throughput = 500;
    m.memory_usage = 1024;
    m.efficiency = 0.95;
    m.total_time = Duration::from_millis(100);
    m.thread_count = 4;

    let aggregated = aggregate_metrics([&m]);

    assert_eq!(aggregated.throughput, 500);
    assert_eq!(aggregated.memory_usage, 1024);
    assert_eq!(aggregated.thread_count, 4);
    assert!((aggregated.efficiency - 0.95).abs() < f64::EPSILON);
}

#[test]
fn concurrent_processing_error_invalid_thread_pool() {
    let config = ParallelConfig {
        thread_pool_size: 0,
        min_parallel_size: 10,
        enable_work_stealing: true,
        chunk_size: 100,
    };

    // Should succeed with 0 threads (uses default)
    let result = ConcurrentProcessor::new(config);
    assert!(result.is_ok());
}
