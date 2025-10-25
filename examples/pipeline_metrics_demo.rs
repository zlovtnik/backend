// Example demonstrating parallel pipeline metrics accumulation
//
// This example shows how metrics are accumulated across chained pipeline operations
// (map, filter, sort) and how to extract both individual and aggregated metrics.

use rcs::functional::parallel_iterators::{ParallelConfig, ParallelPipeline};

fn main() {
    println!("=== Parallel Pipeline Metrics Accumulation Demo ===\n");

    // Setup
    let data: Vec<i32> = (1..=100).collect();
    let mut config = ParallelConfig::default();
    config.min_parallel_size = 1; // Force parallel for this small dataset for demonstration

    // Create a pipeline with initial data
    println!("Initial data: 100 elements (1..=100)");
    let pipeline = ParallelPipeline::new(data, config.clone());

    // Chain operations and accumulate metrics
    println!("\nChaining operations:");
    println!("  1. map: x * 2");
    println!("  2. filter: x % 4 == 0");
    println!("  3. map: x / 2");

    let pipeline = pipeline
        .map(|x| x * 2)
        .filter(|&x| x % 4 == 0)
        .map(|x| x / 2);

    // Access metrics without consuming the pipeline
    println!("\n=== Individual Operation Metrics ===");
    let metrics_history = pipeline.with_metrics();
    println!("Number of operations recorded: {}\n", metrics_history.len());

    for (i, metrics) in metrics_history.iter().enumerate() {
        println!("Operation {}: {}", i + 1, metrics);
    }

    // Get aggregated metrics summary
    println!("\n=== Metrics Summary (Aggregated) ===");
    let summary = pipeline.metrics_summary();
    println!("Total combined time: {:?}", summary.total_time);
    println!("Thread count (max across ops): {}", summary.thread_count);
    println!("Average efficiency: {:.2}", summary.efficiency);
    println!("Total memory usage: {} bytes", summary.memory_usage);
    println!("Average throughput: {} ops/s", summary.throughput);

    // Execute the pipeline and verify data transformation
    println!("\n=== Final Data ===");
    let result = pipeline.execute();
    println!("Result count: {} elements", result.len());
    println!("Result sample: {:?}", &result[..result.len().min(10)]);
    if result.len() > 10 {
        println!("  ... ({} more elements)", result.len() - 10);
    }

    // Example 2: Using into_metrics() to consume and extract metrics
    println!("\n=== Example 2: Consuming Pipeline with into_metrics() ===");
    let data2: Vec<i32> = (1..=50).collect();
    let pipeline2 = ParallelPipeline::new(data2, config.clone());
    let pipeline2 = pipeline2.map(|x| x * 3).sort();

    // into_metrics consumes and executes the pipeline, returning Vec<ParallelMetrics>
    let all_metrics = pipeline2.into_metrics();
    println!(
        "Extracted {} operation metrics via into_metrics()",
        all_metrics.len()
    );

    // Example 3: Complex chain with multiple filter/map operations
    println!("\n=== Example 3: Complex Operation Chain ===");
    let data3: Vec<i32> = (1..=200).collect();
    let pipeline3 = ParallelPipeline::new(data3, config);
    let pipeline3 = pipeline3
        .map(|x| x * 2)
        .filter(|&x| x > 50)
        .map(|x| x + 10)
        .filter(|&x| x % 3 == 0)
        .sort();

    println!("Pipeline chain: map -> filter -> map -> filter -> sort");
    let metrics_count = pipeline3.with_metrics().len();
    let final_result = pipeline3.execute();

    println!("Operations recorded: {}", metrics_count);
    println!("Final result count: {}", final_result.len());

    println!("\n=== Demo Complete ===");
}
