//! Performance Monitoring for Functional Programming Operations
//!
//! This module provides comprehensive performance tracking and monitoring capabilities
//! for functional programming patterns, including iterator chains, immutable data structures,
//! and pipeline operations. It integrates with the existing health check system to provide
//! real-time insights into functional operation performance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Performance metrics for functional operations
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Total number of operations performed
    pub operation_count: u64,
    /// Average execution time per operation
    pub avg_execution_time: Duration,
    /// Minimum execution time recorded
    pub min_execution_time: Duration,
    /// Maximum execution time recorded
    pub max_execution_time: Duration,
    /// Memory usage statistics
    pub memory_stats: MemoryStats,
    /// Error count for this operation type
    pub error_count: u64,
    /// Timestamp of last update
    pub last_updated: Instant,
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Average memory usage per operation
    pub avg_memory_per_operation: u64,
    /// Number of allocations tracked
    pub allocation_count: u64,
    /// Total allocated memory across all operations
    pub total_allocated: u64,
}

/// Types of functional operations we monitor
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum OperationType {
    /// Iterator chain operations
    IteratorChain,
    /// Pure function registry operations
    PureFunctionCall,
    /// Immutable state transitions
    StateTransition,
    /// Query composition operations
    QueryComposition,
    /// Validation engine operations
    ValidationPipeline,
    /// Lazy pipeline operations
    LazyPipeline,
    /// Concurrent processing operations
    ConcurrentProcessing,
    /// Response transformation operations
    ResponseTransformation,
    /// Custom operation type
    Custom(String),
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationType::IteratorChain => write!(f, "iterator_chain"),
            OperationType::PureFunctionCall => write!(f, "pure_function_call"),
            OperationType::StateTransition => write!(f, "state_transition"),
            OperationType::QueryComposition => write!(f, "query_composition"),
            OperationType::ValidationPipeline => write!(f, "validation_pipeline"),
            OperationType::LazyPipeline => write!(f, "lazy_pipeline"),
            OperationType::ConcurrentProcessing => write!(f, "concurrent_processing"),
            OperationType::ResponseTransformation => write!(f, "response_transformation"),
            OperationType::Custom(name) => write!(f, "custom_{}", name),
        }
    }
}

/// Performance measurement context for tracking individual operations
#[derive(Debug)]
pub struct PerformanceMeasurement {
    operation_type: OperationType,
    start_time: Instant,
    initial_memory: u64,
    monitor: Arc<PerformanceMonitor>,
}

impl PerformanceMeasurement {
    /// Complete the measurement and record the results
    pub fn complete(self) {
        let duration = self.start_time.elapsed();
        let memory_used = self.get_current_memory_usage() - self.initial_memory;

        self.monitor.record_operation(
            self.operation_type,
            duration,
            memory_used,
            false, // no error
        );
    }

    /// Complete the measurement with an error
    pub fn complete_with_error(self) {
        let duration = self.start_time.elapsed();
        let memory_used = self.get_current_memory_usage() - self.initial_memory;

        self.monitor.record_operation(
            self.operation_type,
            duration,
            memory_used,
            true, // error occurred
        );
    }

    /// Get current memory usage (simplified - in production might use jemalloc or similar)
    fn get_current_memory_usage(&self) -> u64 {
        // In a real implementation, this would use a proper memory profiler
        // For now, we'll estimate based on allocations
        std::mem::size_of::<Self>() as u64
    }
}

/// Main performance monitoring system
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Metrics storage by operation type
    metrics: RwLock<HashMap<OperationType, PerformanceMetrics>>,
    /// Global configuration
    config: PerformanceConfig,
    /// Operation thresholds for alerting
    thresholds: RwLock<HashMap<OperationType, PerformanceThreshold>>,
}

/// Configuration for performance monitoring
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Enable performance monitoring
    pub enabled: bool,
    /// Maximum number of operation types to track
    pub max_operation_types: usize,
    /// Memory tracking enabled
    pub memory_tracking_enabled: bool,
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_operation_types: 100,
            memory_tracking_enabled: true,
            sampling_rate: 1.0, // Track all operations by default
        }
    }
}

/// Performance thresholds for alerting
#[derive(Debug, Clone)]
pub struct PerformanceThreshold {
    /// Maximum acceptable execution time
    pub max_execution_time: Duration,
    /// Maximum acceptable memory usage per operation
    pub max_memory_per_operation: u64,
    /// Maximum acceptable error rate (0.0 to 1.0)
    pub max_error_rate: f64,
}

impl Default for PerformanceThreshold {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_millis(1000), // 1 second
            max_memory_per_operation: 1024 * 1024,           // 1 MB
            max_error_rate: 0.05,                            // 5% error rate
        }
    }
}

/// Alert types for threshold violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Alert {
    SlowOperation {
        operation_type: OperationType,
        actual_time: Duration,
        threshold: Duration,
    },
    HighMemoryUsage {
        operation_type: OperationType,
        actual_memory: u64,
        threshold: u64,
    },
    HighErrorRate {
        operation_type: OperationType,
        actual_rate: f64,
        threshold: f64,
    },
}

impl PerformanceMonitor {
    /// Create a new performance monitor with default configuration
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            metrics: RwLock::new(HashMap::new()),
            config: PerformanceConfig::default(),
            thresholds: RwLock::new(HashMap::new()),
        })
    }

    /// Create a new performance monitor with custom configuration
    pub fn with_config(config: PerformanceConfig) -> Arc<Self> {
        Arc::new(Self {
            metrics: RwLock::new(HashMap::new()),
            config,
            thresholds: RwLock::new(HashMap::new()),
        })
    }

    /// Start measuring a functional operation
    pub fn start_measurement(
        self: &Arc<Self>,
        operation_type: OperationType,
    ) -> Option<PerformanceMeasurement> {
        if !self.config.enabled {
            return None;
        }

        // Apply sampling rate
        if rand::random::<f64>() > self.config.sampling_rate {
            return None;
        }

        Some(PerformanceMeasurement {
            operation_type,
            start_time: Instant::now(),
            initial_memory: self.get_current_memory_usage(),
            monitor: Arc::clone(self),
        })
    }

    /// Record a completed operation
    pub fn record_operation(
        &self,
        operation_type: OperationType,
        duration: Duration,
        memory_used: u64,
        is_error: bool,
    ) {
        let mut metrics = self.metrics.write().unwrap();

        let metric = metrics
            .entry(operation_type.clone())
            .or_insert_with(|| PerformanceMetrics {
                operation_count: 0,
                avg_execution_time: Duration::from_nanos(0),
                min_execution_time: Duration::from_secs(u64::MAX),
                max_execution_time: Duration::from_nanos(0),
                memory_stats: MemoryStats {
                    peak_memory_bytes: 0,
                    avg_memory_per_operation: 0,
                    allocation_count: 0,
                    total_allocated: 0,
                },
                error_count: 0,
                last_updated: Instant::now(),
            });

        // Get previous count before incrementing
        let prev_count = metric.operation_count;

        // Update operation count
        metric.operation_count += 1;

        // Update timing statistics
        if prev_count == 0 {
            // First sample - set avg, min, and max to duration
            metric.avg_execution_time = duration;
            metric.min_execution_time = duration;
            metric.max_execution_time = duration;
        } else {
            // Rolling average: new_avg = (old_avg * prev_count + duration) / new_count
            metric.avg_execution_time = (metric.avg_execution_time * prev_count as u32 + duration)
                / metric.operation_count as u32;

            if duration < metric.min_execution_time {
                metric.min_execution_time = duration;
            }

            if duration > metric.max_execution_time {
                metric.max_execution_time = duration;
            }
        }

        // Update memory statistics
        metric.memory_stats.allocation_count += 1;
        metric.memory_stats.total_allocated += memory_used;
        metric.memory_stats.avg_memory_per_operation =
            metric.memory_stats.total_allocated / metric.memory_stats.allocation_count;

        if memory_used > metric.memory_stats.peak_memory_bytes {
            metric.memory_stats.peak_memory_bytes = memory_used;
        }

        // Update error count
        if is_error {
            metric.error_count += 1;
        }

        metric.last_updated = Instant::now();

        // Check thresholds and generate alerts if necessary
        self.check_thresholds(&operation_type, metric);
    }

    /// Get performance metrics for a specific operation type
    pub fn get_metrics(&self, operation_type: &OperationType) -> Option<PerformanceMetrics> {
        self.metrics.read().unwrap().get(operation_type).cloned()
    }

    /// Get all performance metrics
    pub fn get_all_metrics(&self) -> HashMap<OperationType, PerformanceMetrics> {
        self.metrics.read().unwrap().clone()
    }

    /// Set performance threshold for an operation type
    pub fn set_threshold(&self, operation_type: OperationType, threshold: PerformanceThreshold) {
        self.thresholds
            .write()
            .unwrap()
            .insert(operation_type, threshold);
    }

    /// Get performance summary for health checks
    pub fn get_health_summary(&self) -> HealthSummary {
        let metrics = self.metrics.read().unwrap();
        let mut total_operations = 0u64;
        let mut total_errors = 0u64;
        let mut slowest_operation = Duration::from_nanos(0);
        let mut highest_memory_usage = 0u64;

        for metric in metrics.values() {
            total_operations += metric.operation_count;
            total_errors += metric.error_count;

            if metric.max_execution_time > slowest_operation {
                slowest_operation = metric.max_execution_time;
            }

            if metric.memory_stats.peak_memory_bytes > highest_memory_usage {
                highest_memory_usage = metric.memory_stats.peak_memory_bytes;
            }
        }

        let error_rate = if total_operations > 0 {
            total_errors as f64 / total_operations as f64
        } else {
            0.0
        };

        HealthSummary {
            total_operations,
            error_rate,
            slowest_operation,
            highest_memory_usage,
            operation_types_tracked: metrics.len(),
            monitoring_enabled: self.config.enabled,
        }
    }

    /// Check performance thresholds and generate alerts
    fn check_thresholds(&self, operation_type: &OperationType, metric: &PerformanceMetrics) {
        let thresholds = self.thresholds.read().unwrap();

        if let Some(threshold) = thresholds.get(operation_type) {
            let error_rate = metric.error_count as f64 / metric.operation_count as f64;

            // Check execution time threshold
            if metric.avg_execution_time > threshold.max_execution_time {
                log::warn!(
                    "Performance alert: {} operation exceeds time threshold. Average: {:?}, Threshold: {:?}",
                    operation_type,
                    metric.avg_execution_time,
                    threshold.max_execution_time
                );
            }

            // Check memory usage threshold
            if metric.memory_stats.avg_memory_per_operation > threshold.max_memory_per_operation {
                log::warn!(
                    "Performance alert: {} operation exceeds memory threshold. Average: {} bytes, Threshold: {} bytes",
                    operation_type,
                    metric.memory_stats.avg_memory_per_operation,
                    threshold.max_memory_per_operation
                );
            }

            // Check error rate threshold
            if error_rate > threshold.max_error_rate {
                log::warn!(
                    "Performance alert: {} operation exceeds error rate threshold. Rate: {:.2}%, Threshold: {:.2}%",
                    operation_type,
                    error_rate * 100.0,
                    threshold.max_error_rate * 100.0
                );
            }
        }
    }

    /// Get current memory usage (simplified implementation)
    fn get_current_memory_usage(&self) -> u64 {
        // In a real implementation, this would use a proper memory profiler
        // For now, we'll return a placeholder value
        0
    }

    /// Reset all metrics (useful for testing)
    pub fn reset_metrics(&self) {
        self.metrics.write().unwrap().clear();
    }
}

/// Health summary for integration with health check endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    /// Total number of functional operations performed
    pub total_operations: u64,
    /// Overall error rate across all operations
    pub error_rate: f64,
    /// Slowest operation recorded
    pub slowest_operation: Duration,
    /// Highest memory usage recorded
    pub highest_memory_usage: u64,
    /// Number of operation types being tracked
    pub operation_types_tracked: usize,
    /// Whether monitoring is currently enabled
    pub monitoring_enabled: bool,
}

/// Global performance monitor instance
static GLOBAL_MONITOR: std::sync::OnceLock<Arc<PerformanceMonitor>> = std::sync::OnceLock::new();

/// Initialize the global performance monitor
pub fn init_performance_monitor(config: PerformanceConfig) {
    GLOBAL_MONITOR
        .set(PerformanceMonitor::with_config(config))
        .unwrap_or_else(|_| panic!("Performance monitor already initialized"));
}

/// Get the global performance monitor instance
pub fn get_performance_monitor() -> &'static Arc<PerformanceMonitor> {
    GLOBAL_MONITOR.get_or_init(|| PerformanceMonitor::new())
}

/// Convenience macro for measuring functional operations
#[macro_export]
macro_rules! measure_operation {
    ($operation_type:expr, $block:block) => {{
        let monitor = $crate::functional::performance_monitoring::get_performance_monitor();
        let measurement = monitor.start_measurement($operation_type);

        let result = $block;

        if let Some(m) = measurement {
            match result {
                Ok(v) => {
                    m.complete();
                    Ok(v)
                }
                Err(e) => {
                    m.complete_with_error();
                    Err(e)
                }
            }
        } else {
            result
        }
    }};
}

/// Integration traits for existing functional components
pub trait Measurable {
    /// Get the operation type for performance monitoring
    fn operation_type(&self) -> OperationType;

    /// Execute with performance monitoring
    fn execute_with_monitoring<T, F>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let monitor = get_performance_monitor();
        let measurement = monitor.start_measurement(self.operation_type());

        let result = f();

        if let Some(m) = measurement {
            m.complete();
        }

        result
    }
}

// Implementation for iterator engine
impl Measurable for crate::functional::iterator_engine::IteratorEngine {
    fn operation_type(&self) -> OperationType {
        OperationType::IteratorChain
    }
}

// Implementation for pure function registry
impl Measurable for crate::functional::pure_function_registry::PureFunctionRegistry {
    fn operation_type(&self) -> OperationType {
        OperationType::PureFunctionCall
    }
}

// Implementation for validation engine
// impl Measurable for crate::functional::validation_engine::ValidationEngine {
//     fn operation_type(&self) -> OperationType {
//         OperationType::ValidationPipeline
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        assert!(monitor.config.enabled);
        assert_eq!(monitor.get_all_metrics().len(), 0);
    }

    #[test]
    fn test_operation_measurement() {
        let monitor = PerformanceMonitor::new();

        // Start measurement
        let measurement = monitor
            .start_measurement(OperationType::IteratorChain)
            .unwrap();

        // Simulate some work
        thread::sleep(Duration::from_millis(10));

        // Complete measurement
        measurement.complete();

        // Check metrics
        let metrics = monitor.get_metrics(&OperationType::IteratorChain).unwrap();
        assert_eq!(metrics.operation_count, 1);
        assert!(metrics.avg_execution_time >= Duration::from_millis(10));
    }

    #[test]
    fn test_operation_with_error() {
        let monitor = PerformanceMonitor::new();

        let measurement = monitor
            .start_measurement(OperationType::PureFunctionCall)
            .unwrap();
        measurement.complete_with_error();

        let metrics = monitor
            .get_metrics(&OperationType::PureFunctionCall)
            .unwrap();
        assert_eq!(metrics.error_count, 1);
    }

    #[test]
    fn test_health_summary() {
        let monitor = PerformanceMonitor::new();

        // Record some operations
        monitor.record_operation(
            OperationType::IteratorChain,
            Duration::from_millis(100),
            1024,
            false,
        );

        monitor.record_operation(
            OperationType::ValidationPipeline,
            Duration::from_millis(50),
            512,
            true,
        );

        let summary = monitor.get_health_summary();
        assert_eq!(summary.total_operations, 2);
        assert_eq!(summary.error_rate, 0.5);
        assert_eq!(summary.slowest_operation, Duration::from_millis(100));
        assert_eq!(summary.highest_memory_usage, 1024);
    }

    #[test]
    fn test_performance_thresholds() {
        let monitor = PerformanceMonitor::new();

        // Set a threshold
        let threshold = PerformanceThreshold {
            max_execution_time: Duration::from_millis(50),
            max_memory_per_operation: 500,
            max_error_rate: 0.1,
        };

        monitor.set_threshold(OperationType::IteratorChain, threshold);

        // Record an operation that exceeds thresholds
        monitor.record_operation(
            OperationType::IteratorChain,
            Duration::from_millis(100), // Exceeds time threshold
            1000,                       // Exceeds memory threshold
            false,
        );

        // Threshold violations would be logged (we can't easily test log output)
        let metrics = monitor.get_metrics(&OperationType::IteratorChain).unwrap();
        assert!(metrics.avg_execution_time > Duration::from_millis(50));
        assert!(metrics.memory_stats.avg_memory_per_operation > 500);
    }

    #[test]
    fn test_multiple_operations() {
        let monitor = PerformanceMonitor::new();

        // Record multiple operations of the same type
        for i in 1..=5 {
            monitor.record_operation(
                OperationType::QueryComposition,
                Duration::from_millis(i * 10),
                i * 100,
                false,
            );
        }

        let metrics = monitor
            .get_metrics(&OperationType::QueryComposition)
            .unwrap();
        assert_eq!(metrics.operation_count, 5);
        assert_eq!(metrics.min_execution_time, Duration::from_millis(10));
        assert_eq!(metrics.max_execution_time, Duration::from_millis(50));

        // Check average calculation
        let expected_avg = Duration::from_millis((10 + 20 + 30 + 40 + 50) / 5);
        assert_eq!(metrics.avg_execution_time, expected_avg);
    }

    #[test]
    fn test_sampling_rate() {
        let config = PerformanceConfig {
            enabled: true,
            sampling_rate: 0.0, // No sampling
            ..Default::default()
        };

        let monitor = PerformanceMonitor::with_config(config);

        // Should return None due to sampling rate
        let measurement = monitor.start_measurement(OperationType::IteratorChain);
        assert!(measurement.is_none());
    }

    #[test]
    fn test_disabled_monitoring() {
        let config = PerformanceConfig {
            enabled: false,
            ..Default::default()
        };

        let monitor = PerformanceMonitor::with_config(config);

        // Should return None when disabled
        let measurement = monitor.start_measurement(OperationType::IteratorChain);
        assert!(measurement.is_none());
    }

    #[test]
    fn test_reset_metrics() {
        let monitor = PerformanceMonitor::new();

        // Add some metrics
        monitor.record_operation(
            OperationType::IteratorChain,
            Duration::from_millis(100),
            1024,
            false,
        );

        assert_eq!(monitor.get_all_metrics().len(), 1);

        // Reset and verify
        monitor.reset_metrics();
        assert_eq!(monitor.get_all_metrics().len(), 0);
    }

    #[test]
    fn test_operation_type_display() {
        assert_eq!(OperationType::IteratorChain.to_string(), "iterator_chain");
        assert_eq!(
            OperationType::Custom("test".to_string()).to_string(),
            "custom_test"
        );
    }
}
