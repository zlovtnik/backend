//! Fluent API for Building Complex Iterator Chains
//!
//! Provides a builder pattern for constructing sophisticated iterator chains
//! with advanced itertools operations, lazy evaluation, and performance optimization.

use itertools::{Dedup, Itertools, Unique};
use std::hash::Hash;
use std::iter::{Filter, FilterMap, FlatMap, Map, Skip, Take};

/// Fluent API for building iterator chains with immediate application
pub struct ChainBuilder<I> {
    iterator: I,
}

impl<I> ChainBuilder<I>
where
    I: Iterator,
{
    /// Wraps an iterator in a ChainBuilder to start a fluent iterator pipeline.
    ///
    /// # Examples
    ///
    /// ```
    /// let iter = vec![1, 2, 3].into_iter();
    /// let result: Vec<_> = ChainBuilder::from_iter(iter)
    ///     .map(|x| x * 2)
    ///     .collect();
    /// assert_eq!(result, vec![2, 4, 6]);
    /// ```
    pub fn from_iter(iterator: I) -> Self {
        Self { iterator }
    }

    /// Applies a predicate filter to the wrapped iterator and returns a new ChainBuilder that yields only items for which the predicate returns `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3, 4];
    /// let result: Vec<_> = ChainBuilder::from_vec(data)
    ///     .filter(|&x| x % 2 == 0)
    ///     .collect();
    /// assert_eq!(result, vec![2, 4]);
    /// ```
    pub fn filter<F>(self, predicate: F) -> ChainBuilder<Filter<I, F>>
    where
        F: FnMut(&I::Item) -> bool,
    {
        ChainBuilder {
            iterator: self.iterator.filter(predicate),
        }
    }

    /// Applies a mapping function to each item of the underlying iterator.
    ///
    /// Returns a `ChainBuilder` wrapping the iterator after applying `f`.
    ///
    /// # Examples
    ///
    /// ```
    /// let v = vec![1, 2, 3];
    /// let out: Vec<_> = ChainBuilder::from_vec(v).map(|x| x * 2).collect();
    /// assert_eq!(out, vec![2, 4, 6]);
    /// ```
    pub fn map<B, F>(self, f: F) -> ChainBuilder<Map<I, F>>
    where
        F: FnMut(I::Item) -> B,
    {
        ChainBuilder {
            iterator: self.iterator.map(f),
        }
    }

    /// Maps each item to an `Option` and yields the contained values, discarding `None` results.
    ///
    /// The provided closure receives each item and may return `Some(mapped)` to emit a value
    /// or `None` to skip the item.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3, 4];
    /// let result: Vec<_> = ChainBuilder::from_vec(data)
    ///     .filter_map(|x| if x % 2 == 0 { Some(x * 2) } else { None })
    ///     .collect();
    /// assert_eq!(result, vec![4, 8]);
    /// ```
    pub fn filter_map<B, F>(self, f: F) -> ChainBuilder<FilterMap<I, F>>
    where
        F: FnMut(I::Item) -> Option<B>,
    {
        ChainBuilder {
            iterator: self.iterator.filter_map(f),
        }
    }

    /// Maps each item to an iterable and concatenates the resulting iterators into a single sequence.
    ///
    /// The closure `f` should produce a value that implements `IntoIterator` for each item; the
    /// produced iterators are flattened in order.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3];
    /// let result: Vec<_> = ChainBuilder::from_vec(data)
    ///     .flat_map(|x| vec![x, x * 2])
    ///     .collect();
    /// assert_eq!(result, vec![1, 2, 2, 4, 3, 6]);
    /// ```
    pub fn flat_map<J, U, F>(self, f: F) -> ChainBuilder<FlatMap<I, J, F>>
    where
        F: FnMut(I::Item) -> J,
        J: IntoIterator<IntoIter = U, Item = U::Item>,
        U: Iterator,
    {
        ChainBuilder {
            iterator: self.iterator.flat_map(f),
        }
    }

    /// Limits the iterator to at most `n` items.
    ///
    /// Returns a new `ChainBuilder` that yields at most `n` elements from the wrapped iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// let result = crate::functional::chain_builder::ChainBuilder::from_iter(vec![1, 2, 3].into_iter())
    ///     .take(2)
    ///     .collect::<Vec<_>>();
    /// assert_eq!(result, vec![1, 2]);
    /// ```
    pub fn take(self, n: usize) -> ChainBuilder<Take<I>> {
        ChainBuilder {
            iterator: self.iterator.take(n),
        }
    }

    /// Creates a ChainBuilder that skips the first `n` items of the underlying iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// let builder = ChainBuilder::from_vec(vec![1, 2, 3, 4]).skip(2);
    /// let v: Vec<_> = builder.collect();
    /// assert_eq!(v, vec![3, 4]);
    /// ```
    ///
    /// # Returns
    ///
    /// A `ChainBuilder` that yields the original iterator's items after skipping the first `n`.
    pub fn skip(self, n: usize) -> ChainBuilder<Skip<I>> {
        ChainBuilder {
            iterator: self.iterator.skip(n),
        }
    }

    /// Produces a ChainBuilder that yields each distinct item, keeping the first occurrence.
    ///
    /// The returned builder wraps an iterator adapter that filters out duplicates seen earlier
    /// in the stream. Item types must implement `Clone`, `Eq`, and `Hash`.
    ///
    /// # Examples
    ///
    /// ```
    /// use functional::chain_builder::ChainBuilder;
    ///
    /// let data = vec![1, 2, 2, 3, 1];
    /// let result: Vec<_> = ChainBuilder::from_vec(data).unique().collect();
    /// assert_eq!(result, vec![1, 2, 3]);
    /// ```
    pub fn unique(self) -> ChainBuilder<Unique<I>>
    where
        I::Item: Clone + Eq + Hash,
    {
        ChainBuilder {
            iterator: self.iterator.unique(),
        }
    }

    /// Removes consecutive duplicate items from the iterator.
    ///
    /// Returns a new ChainBuilder wrapping an iterator that yields the same items
    /// but with consecutive equal elements collapsed into a single occurrence.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 1, 2, 2, 3, 3, 3];
    /// let result: Vec<_> = ChainBuilder::from_vec(data).dedup().collect();
    /// assert_eq!(result, vec![1, 2, 3]);
    /// ```
    pub fn dedup(self) -> ChainBuilder<Dedup<I>>
    where
        I::Item: PartialEq,
    {
        ChainBuilder {
            iterator: self.iterator.dedup(),
        }
    }

    /// Collects the remaining items from the builder's iterator into a collection.
    ///
    /// # Returns
    ///
    /// A collection of type `B` containing the remaining items yielded by the underlying iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3];
    /// let collected: Vec<i32> = ChainBuilder::from_vec(data).collect();
    /// assert_eq!(collected, vec![1, 2, 3]);
    /// ```
    pub fn collect<B>(self) -> B
    where
        B: FromIterator<I::Item>,
    {
        self.iterator.collect()
    }

    /// Counts the remaining items in the wrapped iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// let v = vec![1, 2, 3];
    /// let cnt = ChainBuilder::from_vec(v).count();
    /// assert_eq!(cnt, 3);
    /// ```
    pub fn count(self) -> usize {
        self.iterator.count()
    }

    /// Finds the first item that satisfies the given predicate.
    ///
    /// The `predicate` is called with a reference to each item; the first item
    /// for which the predicate returns `true` is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3, 4];
    /// let found = ChainBuilder::from_vec(data).find(|x| *x == 3);
    /// assert_eq!(found, Some(3));
    /// ```
    ///
    /// # Returns
    ///
    /// `Some(item)` if an item matches the predicate, `None` otherwise.
    pub fn find<P>(mut self, predicate: P) -> Option<I::Item>
    where
        P: FnMut(&I::Item) -> bool,
    {
        self.iterator.find(predicate)
    }

    /// Determines whether any item in the chained iterator satisfies the given predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// let found = ChainBuilder::from_vec(vec![1, 2, 3, 4]).any(|x| x % 2 == 0);
    /// assert!(found);
    /// ```
    ///
    /// # Returns
    ///
    /// `true` if any item satisfies `f`, `false` otherwise.
    pub fn any<F>(mut self, f: F) -> bool
    where
        F: FnMut(I::Item) -> bool,
    {
        self.iterator.any(f)
    }

    /// Determines whether every item produced by the builder satisfies a predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// let result = ChainBuilder::from_vec(vec![2, 4, 6]).all(|x| x % 2 == 0);
    /// assert!(result);
    /// ```
    pub fn all<F>(mut self, f: F) -> bool
    where
        F: FnMut(I::Item) -> bool,
    {
        self.iterator.all(f)
    }

    /// Accumulates the remaining items of the iterator into a single value using a provided closure.
    ///
    /// The `init` value is used as the starting accumulator and `f` is applied for each item
    /// to produce the next accumulator value.
    ///
    /// # Examples
    ///
    /// ```
    /// let result = ChainBuilder::from_vec(vec![1, 2, 3, 4]).fold(0, |acc, x| acc + x);
    /// assert_eq!(result, 10);
    /// ```
    pub fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, I::Item) -> B,
    {
        self.iterator.fold(init, f)
    }

    /// Retrieve the underlying iterator wrapped by this ChainBuilder.
    ///
    /// # Examples
    ///
    /// ```
    /// let cb = ChainBuilder::from_vec(vec![1, 2, 3]);
    /// let it = cb.into_inner();
    /// let collected: Vec<_> = it.collect();
    /// assert_eq!(collected, vec![1, 2, 3]);
    /// ```
    pub fn into_inner(self) -> I {
        self.iterator
    }
}

impl<T> ChainBuilder<std::vec::IntoIter<T>> {
    /// Creates a ChainBuilder from a vector by consuming the vector and using its iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// let cb = ChainBuilder::from_vec(vec![1, 2, 3]);
    /// let result: Vec<_> = cb.map(|x| x * 2).collect();
    /// assert_eq!(result, vec![2, 4, 6]);
    /// ```
    pub fn from_vec(data: Vec<T>) -> Self {
        ChainBuilder {
            iterator: data.into_iter(),
        }
    }
}

/// Utility functions for common iterator chain patterns
pub mod patterns {
    use super::*;

    /// Filter and transform elements from a vector, returning a new collected vector of results.
    ///
    /// The function builds a processing pipeline that applies `filter_fn` to borrow-checked items
    /// and then applies `transform_fn` to the owned items that pass the filter.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3, 4, 5, 6];
    /// let res = filter_transform(data, |&x| x > 3, |x| x.to_string());
    /// assert_eq!(res, vec!["4".to_string(), "5".to_string(), "6".to_string()]);
    /// ```
    pub fn filter_transform<T, U>(
        data: Vec<T>,
        filter_fn: impl Fn(&T) -> bool,
        transform_fn: impl Fn(T) -> U,
    ) -> Vec<U> {
        ChainBuilder::from_vec(data)
            .filter(filter_fn)
            .map(transform_fn)
            .collect()
    }

    /// Applies a function to each element and returns the processed results in the original input order.
    ///
    /// This pipeline currently executes the processing sequentially (a parallel implementation may be introduced later).
    ///
    /// # Returns
    ///
    /// A `Vec<U>` containing the processed outputs in the same order as the input elements.
    ///
    /// # Examples
    ///
    /// ```
    /// let out = sequential_process(vec![1, 2, 3], |x| x * 2);
    /// assert_eq!(out, vec![2, 4, 6]);
    /// ```
    pub fn sequential_process<T, U>(data: Vec<T>, process_fn: impl Fn(T) -> U) -> Vec<U> {
        // Sequential processing; parallel implementation (e.g., rayon) may be added later
        ChainBuilder::from_vec(data).map(process_fn).collect()
    }

    /// Builds a memory-efficient pipeline that processes each element of the input vector and collects the results.
    ///
    /// Returns a `Vec<U>` containing the processed items in order.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = vec![1, 2, 3];
    /// let out: Vec<String> = memory_efficient_process(data, |n| format!("n={}", n));
    /// assert_eq!(out, vec!["n=1".to_string(), "n=2".to_string(), "n=3".to_string()]);
    /// ```
    pub fn memory_efficient_process<T, U>(data: Vec<T>, process_fn: impl Fn(T) -> U) -> Vec<U> {
        ChainBuilder::from_vec(data).map(process_fn).collect()
    }

    /// Groups consecutive elements by a key function into tuples of (key, group_items).
    ///
    /// This function groups consecutive elements that produce the same key value when
    /// passed to `key_fn`. Each group is returned as a tuple containing the key and
    /// a vector of all consecutive items that produced that key.
    ///
    /// # Parameters
    ///
    /// - `data`: input vector of items to group.
    /// - `key_fn`: function that extracts a key from each item for grouping.
    ///
    /// # Returns
    ///
    /// A `Vec<(K, Vec<T>)>` containing tuples of (key, items) for each consecutive group.
    ///
    /// # Examples
    ///
    /// ```
    /// use functional::chain_builder::patterns::group_by_key;
    ///
    /// let data = vec![1, 1, 2, 2, 3, 3, 3];
    /// let result = group_by_key(data, |&x| x);
    /// assert_eq!(result, vec![(1, vec![1, 1]), (2, vec![2, 2]), (3, vec![3, 3, 3])]);
    /// ```
    pub fn group_by_key<T, K, F>(data: Vec<T>, key_fn: F) -> Vec<(K, Vec<T>)>
    where
        K: PartialEq,
        F: Fn(&T) -> K,
    {
        let mut result = Vec::new();
        let mut iter = data.into_iter();

        if let Some(first) = iter.next() {
            let mut current_key = key_fn(&first);
            let mut current_group = vec![first];

            for item in iter {
                let item_key = key_fn(&item);
                if item_key == current_key {
                    current_group.push(item);
                } else {
                    result.push((current_key, current_group));
                    current_key = item_key;
                    current_group = vec![item];
                }
            }

            // Push the last group
            result.push((current_key, current_group));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_chain_builder() {
        let data = vec![1, 2, 3, 4, 5, 6];

        let result: Vec<i32> = ChainBuilder::from_vec(data)
            .filter(|&x| x % 2 == 0)
            .map(|x| x * 2)
            .collect();

        assert_eq!(result, vec![4, 8, 12]);
    }

    #[test]
    fn test_filter_transform_pattern() {
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = patterns::filter_transform(data, |&x| x > 3, |x| x.to_string());

        assert_eq!(result, vec!["4", "5", "6"]);
    }

    #[test]
    fn test_group_by_pattern() {
        let data = vec![1, 1, 2, 2, 3, 3, 3];

        let result = patterns::group_by_key(data, |&x| x);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], (1, vec![1, 1]));
        assert_eq!(result[1], (2, vec![2, 2]));
        assert_eq!(result[2], (3, vec![3, 3, 3]));
    }
}
