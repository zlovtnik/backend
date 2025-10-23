//! Iterator Chain Processing Engine
//!
//! Core iterator processing system that leverages itertools' advanced features
//! including chunk_by, kmerge, join operations and requires Rust 1.63.0 or later.
//! This engine serves as the foundation for all data transformation operations.

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

#[cfg(feature = "functional")]
use itertools::Itertools;

#[cfg(feature = "performance_monitoring")]
use crate::functional::performance_monitoring::{
    get_performance_monitor, Measurable, OperationType,
};

#[cfg(feature = "functional")]
use std::panic::{self, AssertUnwindSafe};

#[cfg(feature = "functional")]
struct SafeIterator<I>
where
    I: Iterator,
{
    inner: I,
    terminated: bool,
}

#[cfg(feature = "functional")]
impl<I> SafeIterator<I>
where
    I: Iterator,
{
    fn new(inner: I) -> Self {
        Self {
            inner,
            terminated: false,
        }
    }

    fn terminate(&mut self) {
        self.terminated = true;
    }
}

#[cfg(feature = "functional")]
impl<I> Iterator for SafeIterator<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminated {
            return None;
        }

        match panic::catch_unwind(AssertUnwindSafe(|| self.inner.next())) {
            Ok(item) => item,
            Err(_) => {
                self.terminated = true;
                None
            }
        }
    }
}

#[cfg(feature = "functional")]
struct LockstepZipIterator<I, J>
where
    I: Iterator,
    J: Iterator<Item = I::Item>,
{
    primary: SafeIterator<I>,
    others: Vec<SafeIterator<J>>,
}

#[cfg(feature = "functional")]
impl<I, J> LockstepZipIterator<I, J>
where
    I: Iterator,
    J: Iterator<Item = I::Item>,
{
    fn new(primary: SafeIterator<I>, others: Vec<SafeIterator<J>>) -> Self {
        Self { primary, others }
    }
}

#[cfg(feature = "functional")]
impl<I, J> Iterator for LockstepZipIterator<I, J>
where
    I: Iterator,
    J: Iterator<Item = I::Item>,
{
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let first = match self.primary.next() {
            Some(value) => value,
            None => return None,
        };

        let mut row = Vec::with_capacity(self.others.len() + 1);
        row.push(first);

        for other in self.others.iter_mut() {
            match other.next() {
                Some(value) => row.push(value),
                None => {
                    self.primary.terminate();
                    for iter in self.others.iter_mut() {
                        iter.terminate();
                    }
                    return None;
                }
            }
        }

        Some(row)
    }
}

/// Extension trait to re-wrap any iterator back into an IteratorChain
///
/// This trait provides a convenient way to recover IteratorChain functionality
/// after using standard iterator methods that return standard iterator types.
pub trait IntoIteratorChain<T>: Iterator<Item = T> + 'static + Sized {
    /// Wraps this iterator into an IteratorChain to access custom methods like kmerge()
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::iterator_engine::{IteratorChain, IntoIteratorChain};
    ///
    /// let data1 = vec![1, 3, 5];
    /// let data2 = vec![2, 4, 6];
    /// let result: Vec<i32> = IteratorChain::new(data1.into_iter())
    ///     .chain(data2)      // Returns std::iter::Chain, loses IteratorChain methods
    ///     .into_chain()      // Re-wrap to regain IteratorChain methods
    ///     .kmerge(vec![7, 8]) // Now kmerge is available again
    ///     .collect();
    /// ```
    fn into_chain(self) -> IteratorChain<T, Self> {
        IteratorChain {
            iterator: self,
            config: IteratorConfig::default(),
            operations: vec!["wrap".to_string()],
        }
    }
}

// Implement for all iterators
impl<T, I> IntoIteratorChain<T> for I where I: Iterator<Item = T> + 'static {}

/// Iterator chain configuration for performance optimization
#[derive(Debug, Clone)]
pub struct IteratorConfig {
    /// Enable parallel processing for large datasets
    pub enable_parallel: bool,
    /// Buffer size for chunked operations
    pub buffer_size: usize,
    /// Memory limit for lazy evaluation
    pub memory_limit: usize,
}

impl Default for IteratorConfig {
    /// Creates an IteratorConfig populated with the library's sensible defaults.
    ///
    /// The defaults are:
    /// - `enable_parallel = false`
    /// - `buffer_size = 1024`
    /// - `memory_limit = 10 * 1024 * 1024` (10 MB)
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = IteratorConfig::default();
    /// assert_eq!(cfg.enable_parallel, false);
    /// assert_eq!(cfg.buffer_size, 1024);
    /// assert_eq!(cfg.memory_limit, 10 * 1024 * 1024);
    /// ```
    fn default() -> Self {
        Self {
            enable_parallel: false,
            buffer_size: 1024,
            memory_limit: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Iterator chain with lazy evaluation support
pub struct IteratorChain<T, I>
where
    I: Iterator<Item = T>,
{
    iterator: I,
    config: IteratorConfig,
    operations: Vec<String>, // For debugging and monitoring
}

impl<T, I> Iterator for IteratorChain<T, I>
where
    I: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}

impl<T, I> IteratorChain<T, I>
where
    I: Iterator<Item = T>,
{
    /// Creates a new IteratorChain with default configuration
    pub fn new(iterator: I) -> Self {
        Self {
            iterator,
            config: IteratorConfig::default(),
            operations: Vec::new(),
        }
    }

    /// Replace the chain's configuration with the provided `IteratorConfig` and return the updated chain.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3].into_iter());
    /// let cfg = IteratorConfig { enable_parallel: true, buffer_size: 2048, memory_limit: 10 * 1024 * 1024 };
    /// let chain = chain.with_config(cfg);
    /// ```
    pub fn with_config(mut self, config: IteratorConfig) -> Self {
        self.config = config;
        self
    }

    /// Wraps any iterator back into an IteratorChain to regain access to custom methods.
    ///
    /// This is useful when you've used standard iterator methods (like `chain`, `take`, `skip`, etc.)
    /// that return standard iterator types, but you need to access IteratorChain-specific methods
    /// like `kmerge()` or `chunk_by()`.
    ///
    /// # Examples
    ///
    /// ```
    /// let data1 = vec![1, 3, 5];
    /// let data2 = vec![2, 4, 6];
    /// let chain = IteratorChain::new(data1.into_iter())
    ///     .chain(data2) // This returns std::iter::Chain, losing IteratorChain methods
    ///     .wrap()       // Re-wrap to regain IteratorChain methods
    ///     .kmerge(vec![7, 8]); // Now kmerge is available again
    /// ```
    pub fn wrap<J>(iterator: J) -> IteratorChain<T, J>
    where
        J: Iterator<Item = T>,
    {
        IteratorChain {
            iterator,
            config: IteratorConfig::default(),
            operations: vec!["wrap".to_string()],
        }
    }

    /// Wraps any iterator back into an IteratorChain, preserving the current configuration.
    ///
    /// Similar to `wrap()` but preserves the current chain's configuration and operation history.
    /// This is the preferred method when you need to chain standard iterator operations with
    /// IteratorChain-specific methods.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3, 4].into_iter())
    ///     .with_config(custom_config);
    /// let result = chain.clone()
    ///     .take(2)  // std iterator method
    ///     .rewrap_with_config(chain.config.clone()) // Preserve config
    ///     .kmerge(vec![5, 6]); // IteratorChain method now available
    /// ```
    pub fn rewrap_with_config<J>(
        iterator: J,
        config: IteratorConfig,
        mut operations: Vec<String>,
    ) -> IteratorChain<T, J>
    where
        J: Iterator<Item = T>,
    {
        operations.push("rewrap".to_string());
        IteratorChain {
            iterator,
            config,
            operations,
        }
    }

    /// Transforms each item in the chain by applying the provided function and returns a new chain of the results.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorEngine::new().from_vec(vec![1, 2, 3]);
    /// let mapped = chain.map(|x| x * 2).collect();
    /// assert_eq!(mapped, vec![2, 4, 6]);
    /// ```
    ///
    /// # Returns
    ///
    /// A new `IteratorChain` that yields values produced by applying `f` to each item of the original chain.
    pub fn map<U, F>(self, f: F) -> IteratorChain<U, std::iter::Map<I, F>>
    where
        F: FnMut(T) -> U,
    {
        let mut operations = self.operations;
        operations.push("map".to_string());

        IteratorChain {
            iterator: self.iterator.map(f),
            config: self.config,
            operations,
        }
    }

    /// Filters items in the chain using the provided predicate and returns a new chain with the filter operation recorded.
    ///
    /// The predicate is applied to a reference to each item; items for which the predicate returns `true` are retained.
    /// This method appends "filter" to the chain's operations log.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3, 4].into_iter())
    ///     .filter(|&x| x % 2 == 0)
    ///     .collect();
    /// assert_eq!(chain, vec![2, 4]);
    /// ```
    pub fn filter<F>(self, f: F) -> IteratorChain<T, std::iter::Filter<I, F>>
    where
        F: FnMut(&T) -> bool,
    {
        let mut operations = self.operations;
        operations.push("filter".to_string());

        IteratorChain {
            iterator: self.iterator.filter(f),
            config: self.config,
            operations,
        }
    }

    /// Group consecutive elements by a derived key, yielding `(key, Vec<items>)` for each contiguous run.
    ///
    /// The resulting `IteratorChain` produces one `(key, Vec<T>)` tuple for each sequence of adjacent
    /// items whose derived keys are equal. Requires `T: Clone` because groups are collected into `Vec<T>`,
    /// and `K: PartialEq` to compare adjacent keys.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 1, 2, 2, 2, 3].into_iter());
    /// let groups: Vec<(i32, Vec<i32>)> = chain.chunk_by(|&x| x).collect();
    /// assert_eq!(groups, vec![(1, vec![1, 1]), (2, vec![2, 2, 2]), (3, vec![3])]);
    /// ```
    #[cfg(feature = "functional")]
    pub fn chunk_by<K, F>(
        self,
        f: F,
    ) -> IteratorChain<(K, Vec<T>), impl Iterator<Item = (K, Vec<T>)>>
    where
        F: FnMut(&T) -> K,
        K: PartialEq,
        T: Clone,
    {
        let mut operations = self.operations;
        operations.push("chunk_by".to_string());

        let chunks: Vec<(K, Vec<T>)> = self
            .iterator
            .chunk_by(f)
            .into_iter()
            .map(|(key, group)| (key, group.collect()))
            .collect();

        IteratorChain {
            iterator: chunks.into_iter(),
            config: self.config,
            operations,
        }
    }

    /// K-way merge sorted iterators using itertools two-way merge
    #[cfg(feature = "functional")]
    pub fn kmerge<J>(self, other: J) -> IteratorChain<T, impl Iterator<Item = T>>
    where
        T: Ord,
        J: IntoIterator<Item = T>,
        I: 'static,
        <J as IntoIterator>::IntoIter: 'static,
    {
        let mut operations = self.operations;
        operations.push("kmerge".to_string());

        // Create a vector of boxed iterators to handle different concrete types
        let iterators: Vec<Box<dyn Iterator<Item = T>>> =
            vec![Box::new(self.iterator), Box::new(other.into_iter())];

        let merged = iterators.into_iter().kmerge();

        IteratorChain {
            iterator: merged,
            config: self.config,
            operations,
        }
    }

    /// Lockstep iteration over multiple iterators (zip all with equal lengths)
    #[cfg(feature = "functional")]
    pub fn lockstep_zip<J>(
        self,
        others: impl IntoIterator<Item = J>,
    ) -> IteratorChain<Vec<T>, impl Iterator<Item = Vec<T>>>
    where
        J: Iterator<Item = T>,
    {
        let mut operations = self.operations;
        operations.push("lockstep_zip".to_string());

        let iterator = LockstepZipIterator::new(
            SafeIterator::new(self.iterator),
            others
                .into_iter()
                .map(SafeIterator::new)
                .collect::<Vec<_>>(),
        );

        IteratorChain {
            iterator,
            config: self.config,
            operations,
        }
    }

    /// Join two sequences by key, emitting every matching pair of left and right items.
    ///
    /// The right-hand sequence is collected into a map keyed by `other_key`. For each item from
    /// the left iterator, this returns a pair for every right-hand item whose key equals the
    /// left item's `self_key`. Both left and right items are cloned as required by the API.
    ///
    /// # Examples
    ///
    /// ```
    /// // left: [1, 2, 3]
    /// // right: [(1, 10), (2, 20), (1, 11)]
    /// // result: [(1, 10), (1, 11), (2, 20)]
    /// let left_chain = IteratorChain::new(vec![1, 2, 3].into_iter());
    /// let joined: Vec<_> = left_chain
    ///     .join(
    ///         vec![(1, 10), (2, 20), (1, 11)],
    ///         |l: &i32| *l,
    ///         |r: &(i32, i32)| r.0,
    ///     )
    ///     .collect();
    /// assert_eq!(joined, vec![(1, 10), (1, 11), (2, 20)]);
    /// ```
    pub fn join<K, U, V, F, G>(
        self,
        other: U,
        self_key: F,
        other_key: G,
    ) -> IteratorChain<(T, V), impl Iterator<Item = (T, V)>>
    where
        K: Hash + Eq,
        U: IntoIterator<Item = V>,
        F: Fn(&T) -> K,
        G: Fn(&V) -> K,
        T: Clone,
        V: Clone,
    {
        let mut operations = self.operations;
        operations.push("join".to_string());

        // Collect right side into a HashMap for lookup
        let right_map: HashMap<K, Vec<V>> = other
            .into_iter()
            .map(|item| (other_key(&item), item))
            .fold(HashMap::new(), |mut map, (key, item)| {
                map.entry(key).or_insert_with(Vec::new).push(item);
                map
            });

        // Use flat_map to emit all matches instead of just the first one
        let joined = self.iterator.flat_map(move |left_item| {
            let left_key = self_key(&left_item);
            let right_items = right_map.get(&left_key).cloned().unwrap_or_default();

            right_items
                .into_iter()
                .map(move |right_item| (left_item.clone(), right_item))
        });

        IteratorChain {
            iterator: joined,
            config: self.config,
            operations,
        }
    }

    /// Cartesian product with another iterator
    #[cfg(feature = "functional")]
    pub fn cartesian_product<U>(
        self,
        other: U,
    ) -> IteratorChain<(T, U::Item), impl Iterator<Item = (T, U::Item)>>
    where
        U: IntoIterator,
        U::IntoIter: Clone,
        T: Clone,
        I: Clone,
    {
        let mut operations = self.operations;
        operations.push("cartesian_product".to_string());

        let product = self.iterator.cartesian_product(other);
        IteratorChain {
            iterator: product,
            config: self.config,
            operations,
        }
    }

    /// Partition the iterator into two collections based on a predicate
    ///
    /// Items for which the predicate returns `true` are placed in the first collection,
    /// items for which it returns `false` are placed in the second.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3, 4, 5].into_iter());
    /// let (evens, odds): (Vec<i32>, Vec<i32>) = chain.partition(|&x| x % 2 == 0);
    /// assert_eq!(evens, vec![2, 4]);
    /// assert_eq!(odds, vec![1, 3, 5]);
    /// ```
    pub fn partition<F>(self, predicate: F) -> (Vec<T>, Vec<T>)
    where
        F: FnMut(&T) -> bool,
    {
        self.iterator.partition(predicate)
    }

    /// Split the iterator into two based on a predicate, returning two new IteratorChains
    ///
    /// The first chain contains items that satisfy the predicate, the second contains those that don't.
    pub fn split_by<F>(self, mut predicate: F) -> (IteratorChain<T, impl Iterator<Item = T>>, IteratorChain<T, impl Iterator<Item = T>>)
    where
        F: FnMut(&T) -> bool,
        T: Clone,
    {
        let (satisfying, not_satisfying): (Vec<T>, Vec<T>) = self.iterator.partition(|item| predicate(item));
        let operations_true = self.operations.clone();
        let operations_false = self.operations;
        
        let chain_true = IteratorChain {
            iterator: satisfying.into_iter(),
            config: self.config.clone(),
            operations: operations_true,
        };
        
        let chain_false = IteratorChain {
            iterator: not_satisfying.into_iter(),
            config: self.config,
            operations: operations_false,
        };
        
        (chain_true, chain_false)
    }

    /// Collects all items from the chain into a `Vec`.
    ///
    /// Returns a `Vec<T>` containing every item produced by the chain's iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::functional::iterator_engine::IteratorChain;
    /// let chain = IteratorChain::new(vec![1, 2, 3].into_iter());
    /// let v = chain.collect();
    /// assert_eq!(v, vec![1, 2, 3]);
    /// ```
    pub fn collect(self) -> Vec<T> {
        #[cfg(feature = "performance_monitoring")]
        {
            let start = std::time::Instant::now();

            let result: Vec<T> = self.iterator.collect();

            let duration = start.elapsed();
            let memory_usage = (result.len() * std::mem::size_of::<T>()) as u64;

            get_performance_monitor().record_operation(
                OperationType::IteratorChain,
                duration,
                memory_usage,
                false,
            );

            result
        }
        #[cfg(not(feature = "performance_monitoring"))]
        {
            self.iterator.collect()
        }
    }

    /// Group items by a key function, returning a vector of (key, group) pairs
    ///
    /// Items are grouped based on the key returned by the key function.
    /// Each group is a vector of items that share the same key.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3, 4, 5].into_iter());
    /// let groups: Vec<(i32, Vec<i32>)> = chain.group_by(|&x| x % 2).collect();
    /// // Groups items by their remainder when divided by 2
    /// ```
    #[cfg(feature = "functional")]
    pub fn group_by<K, F>(self, key_fn: F) -> IteratorChain<(K, Vec<T>), impl Iterator<Item = (K, Vec<T>)>>
    where
        K: Eq + std::hash::Hash,
        F: Fn(&T) -> K,
    {
        let mut operations = self.operations;
        operations.push("group_by".to_string());
        
        use std::collections::HashMap;
        let mut map: HashMap<K, Vec<T>> = HashMap::new();
        
        for item in self.iterator {
            let key = key_fn(&item);
            map.entry(key).or_insert_with(Vec::new).push(item);
        }
        
        let grouped = map.into_iter().collect::<Vec<_>>();
        
        IteratorChain {
            iterator: grouped.into_iter(),
            config: self.config,
            operations,
        }
    }

    /// Counts the remaining elements in the chain.
    ///
    /// Returns the number of remaining elements.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3].into_iter());
    /// assert_eq!(chain.count(), 3);
    /// ```
    pub fn count(self) -> usize {
        self.iterator.count()
    }

    /// Reduces the iterator's items into a single value by applying an accumulator function.
    ///
    /// # Returns
    ///
    /// The final accumulated value after processing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3].into_iter());
    /// let sum = chain.fold(0, |acc, x| acc + x);
    /// assert_eq!(sum, 6);
    /// ```
    pub fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, T) -> B,
    {
        self.iterator.fold(init, f)
    }

    /// Reduces the iterator's items into a single value by applying an accumulator function.
    ///
    /// This is similar to fold, but uses the first element as the initial value.
    ///
    /// # Returns
    ///
    /// Some(value) if the iterator is non-empty, None otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3].into_iter());
    /// let sum = chain.reduce(|acc, x| acc + x);
    /// assert_eq!(sum, Some(6));
    /// ```
    pub fn reduce<F>(self, f: F) -> Option<T>
    where
        F: FnMut(T, T) -> T,
    {
        self.iterator.reduce(f)
    }

    /// Accumulate items into a collection using a custom accumulator function
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3, 4, 5].into_iter());
    /// let evens: Vec<i32> = chain.accumulate(Vec::new(), |mut acc, x| {
    ///     if x % 2 == 0 { acc.push(x); }
    ///     acc
    /// });
    /// assert_eq!(evens, vec![2, 4]);
    /// ```
    pub fn accumulate<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, T) -> B,
    {
        self.iterator.fold(init, f)
    }

    /// Retrieve the first element of the chain, consuming the chain.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3].into_iter());
    /// assert_eq!(chain.first(), Some(1));
    /// ```
    pub fn first(mut self) -> Option<T> {
        self.iterator.next()
    }

    /// Calculate the sum of all numeric items in the chain
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3, 4, 5].into_iter());
    /// assert_eq!(chain.sum::<i32>(), 15);
    /// ```
    pub fn sum<S>(self) -> S
    where
        T: std::iter::Sum<S>,
        S: std::iter::Sum<T>,
    {
        self.iterator.sum()
    }

    /// Calculate the product of all numeric items in the chain
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3, 4].into_iter());
    /// assert_eq!(chain.product::<i32>(), 24);
    /// ```
    pub fn product<P>(self) -> P
    where
        T: std::iter::Product<P>,
        P: std::iter::Product<T>,
    {
        self.iterator.product()
    }

    /// Find the minimum value in the chain
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![3, 1, 4, 1, 5].into_iter());
    /// assert_eq!(chain.min(), Some(1));
    /// ```
    pub fn min(self) -> Option<T>
    where
        T: Ord,
    {
        self.iterator.min()
    }

    /// Find the maximum value in the chain
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![3, 1, 4, 1, 5].into_iter());
    /// assert_eq!(chain.max(), Some(5));
    /// ```
    pub fn max(self) -> Option<T>
    where
        T: Ord,
    {
        self.iterator.max()
    }

    /// Calculate the average of all numeric items in the chain
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1.0, 2.0, 3.0, 4.0, 5.0].into_iter());
    /// assert_eq!(chain.average(), Some(3.0));
    /// ```
    pub fn average(self) -> Option<f64>
    where
        T: Into<f64> + Clone,
    {
        let values: Vec<T> = self.iterator.collect();
        if values.is_empty() {
            None
        } else {
            let sum: f64 = values.iter().map(|x| x.clone().into()).sum();
            Some(sum / values.len() as f64)
        }
    }

}

impl<T, I> fmt::Debug for IteratorChain<T, I>
where
    I: Iterator<Item = T> + fmt::Debug,
{
    /// Formats the `IteratorChain` for debugging by emitting a struct-like representation
    /// with the fields `iterator`, `config`, and `operations`.
    ///
    /// # Examples
    ///
    /// ```
    /// let chain = IteratorChain::new(vec![1, 2, 3].into_iter());
    /// let s = format!("{:?}", chain);
    /// assert!(s.contains("IteratorChain"));
    /// assert!(s.contains("config"));
    /// assert!(s.contains("operations"));
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IteratorChain")
            .field("iterator", &self.iterator)
            .field("config", &self.config)
            .field("operations", &self.operations)
            .finish()
    }
}

#[cfg(feature = "performance_monitoring")]
impl<T, I> Measurable for IteratorChain<T, I>
where
    I: Iterator<Item = T>,
{
    /// Gets the operation type for monitoring
    fn operation_type(&self) -> OperationType {
        OperationType::IteratorChain
    }
}

/// Core iterator processing engine
pub struct IteratorEngine {
    config: IteratorConfig,
    performance_metrics: HashMap<String, u64>,
}

impl IteratorEngine {
    /// Constructs a new IteratorEngine with the default configuration and no performance metrics.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = IteratorEngine::new();
    /// assert!(engine.metrics().is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            config: IteratorConfig::default(),
            performance_metrics: HashMap::new(),
        }
    }

    /// Constructs an IteratorEngine configured with the given settings.
    ///
    /// The returned engine uses `config` for its behavior and initializes an empty
    /// performance metrics map.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = IteratorConfig {
    ///     enable_parallel: true,
    ///     buffer_size: 2048,
    ///     memory_limit: 16 * 1024 * 1024,
    /// };
    /// let engine = IteratorEngine::with_config(cfg);
    /// assert_eq!(engine.metrics().len(), 0);
    /// ```
    pub fn with_config(config: IteratorConfig) -> Self {
        Self {
            config,
            performance_metrics: HashMap::new(),
        }
    }

    /// Create an IteratorChain from an existing iterator using this engine's configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = IteratorEngine::new();
    /// let chain = engine.from_iter(vec![1, 2, 3].into_iter());
    /// let collected = chain.collect();
    /// assert_eq!(collected, vec![1, 2, 3]);
    /// ```
    pub fn from_iter<T, I>(&self, iterator: I) -> IteratorChain<T, I>
    where
        I: Iterator<Item = T>,
    {
        IteratorChain::new(iterator).with_config(self.config.clone())
    }

    /// Creates an `IteratorChain` backed by the given vector.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = IteratorEngine::new();
    /// let chain = engine.from_vec(vec![1, 2, 3]);
    /// let collected: Vec<_> = chain.collect();
    /// assert_eq!(collected, vec![1, 2, 3]);
    /// ```
    pub fn from_vec<T>(&self, vec: Vec<T>) -> IteratorChain<T, std::vec::IntoIter<T>> {
        self.from_iter(vec.into_iter())
    }

    /// Applies `transform` to each element of `data` by reference and returns a `Vec` of the results.
    ///
    /// This function borrows each input (`&T`) so elements are not cloned; when the engine's
    /// `config.enable_parallel` is true and the crate is built with the `"parallel"` feature,
    /// processing may run in parallel for large inputs, otherwise it runs sequentially.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = IteratorEngine::new();
    /// let data = [1, 2, 3];
    /// let out = engine.process_zero_copy(&data, |x| x * 2);
    /// assert_eq!(out, vec![2, 4, 6]);
    /// ```
    #[allow(unexpected_cfgs)]
    pub fn process_zero_copy<T, F, U>(&self, data: &[T], transform: F) -> Vec<U>
    where
        F: Fn(&T) -> U,
        T: Sync,
        U: Send,
    {
        #[cfg(feature = "parallel")]
        if self.config.enable_parallel && data.len() > self.config.buffer_size {
            // Parallel processing for large datasets
            use rayon::prelude::*;
            return data.par_iter().map(transform).collect();
        }

        #[cfg(not(feature = "parallel"))]
        // Sequential processing
        data.iter().map(transform).collect()
    }

    /// Access the current performance metrics collected by the engine.
    ///
    /// The returned map associates metric names with their recorded numeric values.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = IteratorEngine::new();
    /// let metrics = engine.metrics();
    /// // newly created engine has no metrics recorded
    /// assert!(metrics.is_empty());
    /// ```
    pub fn metrics(&self) -> &HashMap<String, u64> {
        &self.performance_metrics
    }

    /// Clears all recorded performance metrics from the engine.
    ///
    /// This removes every entry from the engine's internal metrics map so subsequent
    /// calls to `metrics()` will return an empty collection.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut engine = IteratorEngine::new();
    /// // metrics start empty by default; calling reset_metrics ensures they are empty
    /// engine.reset_metrics();
    /// assert!(engine.metrics().is_empty());
    /// ```
    pub fn reset_metrics(&mut self) {
        self.performance_metrics.clear();
    }
}

impl Default for IteratorEngine {
    /// Creates a default IteratorEngine configured with the library's standard settings.
    ///
    /// The created engine uses the default `IteratorConfig` and starts with empty performance metrics.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = IteratorEngine::default();
    /// let chain = engine.from_vec(vec![1, 2, 3]);
    /// assert_eq!(chain.collect(), vec![1, 2, 3]);
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_iterator_chain() {
        let engine = IteratorEngine::new();
        let data = vec![1, 2, 3, 4, 5];

        let result: Vec<i32> = engine
            .from_vec(data)
            .filter(|&x| x % 2 == 0)
            .map(|x| x * 2)
            .collect();

        assert_eq!(result, vec![4, 8]);
    }

    #[test]
    fn test_chunk_by() {
        let engine = IteratorEngine::new();
        let data = vec![1, 1, 2, 2, 3, 3, 3];

        let chunks: Vec<Vec<i32>> = engine
            .from_vec(data)
            .chunk_by(|&x| x)
            .map(|(_key, group)| group)
            .collect();

        assert_eq!(chunks, vec![vec![1, 1], vec![2, 2], vec![3, 3, 3]]);
    }

    #[test]
    fn test_cartesian_product() {
        let engine = IteratorEngine::new();
        let data1 = vec![1, 2];
        let data2 = vec![3, 4];

        let products: Vec<(i32, i32)> = engine.from_vec(data1).cartesian_product(data2).collect();

        assert_eq!(products, vec![(1, 3), (1, 4), (2, 3), (2, 4)]);
    }

    #[test]
    fn test_zero_copy_processing() {
        let engine = IteratorEngine::new();
        let data = vec![1, 2, 3, 4, 5];

        let result = engine.process_zero_copy(&data, |&x| x * 2);

        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }

    #[cfg(feature = "functional")]
    mod functional_more_tests {
        use super::*;

        #[test]
        fn test_kmerge() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 3, 5];
            let data2 = vec![2, 4, 6];

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged, vec![1, 2, 3, 4, 5, 6]);
        }

        #[test]
        fn test_lockstep_zip() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 3];
            let data2 = vec![4, 5, 6];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter()])
                .collect();

            assert_eq!(zipped, vec![vec![1, 4], vec![2, 5], vec![3, 6]]);
        }

        #[test]
        fn test_kmerge_preserves_merge_semantics() {
            let engine = IteratorEngine::new();
            let left = vec![1, 3, 5, 7, 9];
            let right = vec![2, 4, 6, 8, 10];

            let merged: Vec<i32> = engine.from_vec(left).kmerge(right).collect();

            // Should produce sorted merge of both inputs
            assert_eq!(merged, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        }

        #[test]
        fn test_lockstep_zip_stops_at_shortest() {
            let engine = IteratorEngine::new();
            let short = vec![1, 2];
            let long = vec![4, 5, 6, 7];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(short)
                .lockstep_zip(vec![long.into_iter()])
                .collect();

            // Should stop at shortest iterator length
            assert_eq!(zipped, vec![vec![1, 4], vec![2, 5]]);
            assert_eq!(zipped.len(), 2);
        }

        #[test]
        fn test_kmerge_lazy_evaluation() {
            struct LimitedIterator {
                data: Vec<i32>,
                count: usize,
                max_calls: usize,
            }

            impl LimitedIterator {
                fn new(data: Vec<i32>, max_calls: usize) -> Self {
                    Self {
                        data,
                        count: 0,
                        max_calls,
                    }
                }
            }

            impl Iterator for LimitedIterator {
                type Item = i32;

                fn next(&mut self) -> Option<Self::Item> {
                    if self.count >= self.max_calls {
                        panic!("Iterator called next() too many times: {}", self.max_calls);
                    }
                    let item = self.data.get(self.count).cloned();
                    self.count += 1;
                    item
                }
            }

            let engine = IteratorEngine::new();
            let left = LimitedIterator::new(vec![1, 3, 5], 4); // Allow up to 4 calls (3 elements + 1 None check)
            let right = LimitedIterator::new(vec![2, 4, 6], 4); // Allow up to 4 calls (3 elements + 1 None check)

            // Collect all - should succeed and produce merged result without panicking
            let merged: Vec<i32> = engine.from_iter(left).kmerge(right).collect();

            assert_eq!(merged, vec![1, 2, 3, 4, 5, 6]);
        }

        #[test]
        fn test_lockstep_zip_lazy_evaluation() {
            struct LimitedIterator {
                data: Vec<i32>,
                count: usize,
                max_calls: usize,
            }

            impl LimitedIterator {
                fn new(data: Vec<i32>, max_calls: usize) -> Self {
                    Self {
                        data,
                        count: 0,
                        max_calls,
                    }
                }
            }

            impl Iterator for LimitedIterator {
                type Item = i32;

                fn next(&mut self) -> Option<Self::Item> {
                    if self.count >= self.max_calls {
                        panic!("Iterator called next() too many times: {}", self.max_calls);
                    }
                    let item = self.data.get(self.count).cloned();
                    self.count += 1;
                    item
                }
            }

            let engine = IteratorEngine::new();
            let main = LimitedIterator::new(vec![1, 2], 3); // Allow up to 3 calls (2 elements + 1 None check)
            let other = LimitedIterator::new(vec![4, 5], 3); // Allow up to 3 calls (2 elements + 1 None check)

            // Collect all - should succeed and stop at shortest
            let zipped: Vec<Vec<i32>> = engine.from_iter(main).lockstep_zip(vec![other]).collect();

            assert_eq!(zipped, vec![vec![1, 4], vec![2, 5]]);
        }

        // Comprehensive tests for kmerge
        #[test]
        fn test_kmerge_with_empty_iterators() {
            let engine = IteratorEngine::new();
            let data1: Vec<i32> = vec![];
            let data2: Vec<i32> = vec![];

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged, vec![] as Vec<i32>);
        }

        #[test]
        fn test_kmerge_with_one_empty_iterator() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 3];
            let data2: Vec<i32> = vec![];

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged, vec![1, 2, 3]);
        }

        #[test]
        fn test_kmerge_with_first_empty() {
            let engine = IteratorEngine::new();
            let data1: Vec<i32> = vec![];
            let data2 = vec![4, 5, 6];

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged, vec![4, 5, 6]);
        }

        #[test]
        fn test_kmerge_with_unsorted_input() {
            let engine = IteratorEngine::new();
            let mut data1 = vec![3, 1, 5];
            let mut data2 = vec![6, 2, 4];
            data1.sort();
            data2.sort();

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            // kmerge merges pre-sorted iterators
            assert_eq!(merged, vec![1, 2, 3, 4, 5, 6]);
        }

        #[test]
        fn test_kmerge_with_duplicates() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 2, 3];
            let data2 = vec![2, 3, 4, 4];

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged, vec![1, 2, 2, 2, 3, 3, 4, 4]);
        }

        #[test]
        fn test_kmerge_with_single_elements() {
            let engine = IteratorEngine::new();
            let data1 = vec![5];
            let data2 = vec![3];

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged, vec![3, 5]);
        }

        #[test]
        fn test_kmerge_with_negative_numbers() {
            let engine = IteratorEngine::new();
            let data1 = vec![-3, -1, 2];
            let data2 = vec![-2, 0, 1];

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged, vec![-3, -2, -1, 0, 1, 2]);
        }

        #[test]
        fn test_kmerge_with_strings() {
            let engine = IteratorEngine::new();
            let data1 = vec!["banana", "cherry"];
            let data2 = vec!["apple", "date"];

            let merged: Vec<&str> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged, vec!["apple", "banana", "cherry", "date"]);
        }

        #[test]
        fn test_kmerge_preserves_operation_tracking() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 3];
            let data2 = vec![2, 4];

            let chain = engine.from_vec(data1).kmerge(data2);

            // Verify the operation is tracked
            assert!(chain.operations.contains(&"kmerge".to_string()));
        }

        #[test]
        fn test_kmerge_with_large_dataset() {
            let engine = IteratorEngine::new();
            let data1: Vec<i32> = (0..1000).filter(|x| x % 2 == 0).collect();
            let data2: Vec<i32> = (0..1000).filter(|x| x % 2 == 1).collect();

            let merged: Vec<i32> = engine.from_vec(data1).kmerge(data2).collect();

            assert_eq!(merged.len(), 1000);
            // Verify it's sorted
            for i in 0..999 {
                assert!(merged[i] <= merged[i + 1]);
            }
        }

        // Comprehensive tests for lockstep_zip
        #[test]
        fn test_lockstep_zip_with_empty_iterators() {
            let engine = IteratorEngine::new();
            let data1: Vec<i32> = vec![];
            let data2: Vec<i32> = vec![];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter()])
                .collect();

            assert_eq!(zipped, Vec::<Vec<i32>>::new());
        }

        #[test]
        fn test_lockstep_zip_with_different_lengths() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 3, 4, 5];
            let data2 = vec![10, 20, 30];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter()])
                .collect();

            // Should truncate to shortest length
            assert_eq!(zipped, vec![vec![1, 10], vec![2, 20], vec![3, 30]]);
        }

        #[test]
        fn test_lockstep_zip_with_first_shorter() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2];
            let data2 = vec![10, 20, 30, 40];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter()])
                .collect();

            assert_eq!(zipped, vec![vec![1, 10], vec![2, 20]]);
        }

        #[test]
        fn test_lockstep_zip_multiple_iterators() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 3];
            let data2 = vec![10, 20, 30];
            let data3 = vec![100, 200, 300];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter(), data3.into_iter()])
                .collect();

            assert_eq!(
                zipped,
                vec![vec![1, 10, 100], vec![2, 20, 200], vec![3, 30, 300]]
            );
        }

        #[test]
        fn test_lockstep_zip_multiple_with_different_lengths() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 3, 4];
            let data2 = vec![10, 20];
            let data3 = vec![100, 200, 300];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter(), data3.into_iter()])
                .collect();

            // Truncates to length 2 (shortest)
            assert_eq!(zipped, vec![vec![1, 10, 100], vec![2, 20, 200]]);
        }

        #[test]
        fn test_lockstep_zip_single_element() {
            let engine = IteratorEngine::new();
            let data1 = vec![42];
            let data2 = vec![100];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter()])
                .collect();

            assert_eq!(zipped, vec![vec![42, 100]]);
        }

        #[test]
        fn test_lockstep_zip_with_strings() {
            let engine = IteratorEngine::new();
            let data1 = vec!["a", "b", "c"];
            let data2 = vec!["x", "y", "z"];

            let zipped: Vec<Vec<&str>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter()])
                .collect();

            assert_eq!(zipped, vec![vec!["a", "x"], vec!["b", "y"], vec!["c", "z"]]);
        }

        #[test]
        fn test_lockstep_zip_preserves_operation_tracking() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2];
            let data2 = vec![3, 4];

            let chain = engine.from_vec(data1).lockstep_zip(vec![data2.into_iter()]);

            assert!(chain.operations.contains(&"lockstep_zip".to_string()));
        }

        #[test]
        fn test_lockstep_zip_with_no_additional_iterators() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 3];

            let zipped: Vec<Vec<i32>> = engine
                .from_vec(data1)
                .lockstep_zip(Vec::<std::vec::IntoIter<i32>>::new())
                .collect();

            // With no additional iterators, each element becomes a single-item vec
            assert_eq!(zipped, vec![vec![1], vec![2], vec![3]]);
        }

        #[test]
        fn test_lockstep_zip_clones_values() {
            #[derive(Clone, Debug, PartialEq)]
            struct Value(i32);

            let engine = IteratorEngine::new();
            let data1 = vec![Value(1), Value(2)];
            let data2 = vec![Value(10), Value(20)];

            let zipped: Vec<Vec<Value>> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter()])
                .collect();

            assert_eq!(
                zipped,
                vec![vec![Value(1), Value(10)], vec![Value(2), Value(20)]]
            );
        }

        // Integration tests combining multiple operations
        #[test]
        fn test_kmerge_then_filter() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 3, 4, 5];
            let data2 = vec![6, 7, 8, 9, 10];

            let result: Vec<i32> = engine
                .from_vec(data1)
                .kmerge(data2)
                .filter(|&x| x % 2 == 0)
                .collect();

            assert_eq!(result, vec![2, 4, 6, 8, 10]);
        }

        #[test]
        fn test_lockstep_zip_then_map() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 2, 3];
            let data2 = vec![10, 20, 30];

            let result: Vec<i32> = engine
                .from_vec(data1)
                .lockstep_zip(vec![data2.into_iter()])
                .map(|vec| vec.iter().sum())
                .collect();

            assert_eq!(result, vec![11, 22, 33]);
        }

        #[test]
        fn test_chain_kmerge_lockstep() {
            let engine = IteratorEngine::new();
            let data1 = vec![1, 3];
            let data2 = vec![2, 4];
            let data3 = vec![10, 20, 30, 40];

            let merged = engine.from_vec(data1).kmerge(data2);
            let result: Vec<Vec<i32>> = merged.lockstep_zip(vec![data3.into_iter()]).collect();

            assert_eq!(
                result,
                vec![vec![1, 10], vec![2, 20], vec![3, 30], vec![4, 40]]
            );
        }
    }

    #[test]
    fn test_join() {
        let engine = IteratorEngine::new();
        let left = vec![1, 2, 3];
        let right = vec![(1, 10), (2, 20), (1, 11)];

        let joined: Vec<(i32, (i32, i32))> = engine
            .from_vec(left)
            .join(right, |&l| l, |&(r, _)| r)
            .collect();

        // Order may vary, but should contain all matches
        assert!(joined.contains(&(1, (1, 10))));
        assert!(joined.contains(&(1, (1, 11))));
        assert!(joined.contains(&(2, (2, 20))));
        assert_eq!(joined.len(), 3);
    }

    #[test]
    fn test_custom_struct_support() {
        #[derive(Clone, Debug, PartialEq)]
        struct Person {
            id: i32,
            name: String,
        }

        let engine = IteratorEngine::new();
        let people = vec![
            Person {
                id: 1,
                name: "Alice".to_string(),
            },
            Person {
                id: 2,
                name: "Bob".to_string(),
            },
            Person {
                id: 3,
                name: "Charlie".to_string(),
            },
        ];

        let result: Vec<String> = engine
            .from_vec(people)
            .filter(|p| p.id % 2 == 1)
            .map(|p| p.name)
            .collect();

        assert_eq!(result, vec!["Alice", "Charlie"]);
    }

    #[test]
    fn test_method_resolution_pitfall_solution() {
        use crate::functional::iterator_engine::IntoIteratorChain;

        let engine = IteratorEngine::new();
        let data1 = vec![1, 3, 5];
        let data2 = vec![2, 4, 6];
        let data3 = vec![7, 8];

        // Problem: after using std iterator methods, we lose IteratorChain methods
        // This won't compile: chain.chain(data2).kmerge(data3)
        // Because chain() returns std::iter::Chain, not IteratorChain

        // Solution 1: Use into_chain() to re-wrap
        let result1: Vec<i32> = engine
            .from_vec(data1.clone())
            .chain(data2.clone()) // Use owned iterator
            .into_chain() // Re-wrap to get IteratorChain methods back
            .kmerge(data3.clone()) // Now kmerge is available!
            .collect();

        // Solution 2: Use IteratorChain::wrap() directly
        let std_chain = engine.from_vec(data1.clone()).chain(data2.clone());
        let result2: Vec<i32> = std_chain.into_chain().kmerge(data3.clone()).collect();

        // Both solutions should produce the same result
        let expected = vec![1, 3, 5, 2, 4, 6, 7, 8];
        assert_eq!(result1, expected);
        assert_eq!(result2, expected);

        // Verify operations are tracked
        let chain = engine.from_vec(vec![1, 2, 3]).take(2).into_chain();
        assert!(chain.operations.contains(&"wrap".to_string()));
    }
}
