use super::parallel_iterators::{ParallelConfig, ParallelIteratorExt};

#[cfg(test)]
mod tests {
    use super::*;

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
