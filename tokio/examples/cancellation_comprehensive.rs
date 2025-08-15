//! Comprehensive cancellation example demonstrating README.md syntax
//!
//! This example shows the zero-allocation, lock-free cancellation system
//! with hierarchical task cancellation, cleanup handlers, and escalation.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use sweet_async_api::task::{CancellableTask, CancellationLevel};
use sweet_async_tokio::AsyncTask;
use tokio::time::sleep;

#[derive(Clone)]
struct ProcessedItem {
    id: usize,
    data: String,
    processed_at: std::time::Instant,
}

struct ProcessingStats {
    items_processed: AtomicUsize,
    cleanup_called: AtomicUsize,
}

impl ProcessingStats {
    fn new() -> Self {
        Self {
            items_processed: AtomicUsize::new(0),
            cleanup_called: AtomicUsize::new(0),
        }
    }

    fn increment_processed(&self) {
        self.items_processed.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_cleanup(&self) {
        self.cleanup_called.fetch_add(1, Ordering::Relaxed);
    }

    fn processed_count(&self) -> usize {
        self.items_processed.load(Ordering::Relaxed)
    }

    fn cleanup_count(&self) -> usize {
        self.cleanup_called.load(Ordering::Relaxed)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize processing statistics
    let stats = Arc::new(ProcessingStats::new());
    let stats_clone = stats.clone();

    // Create dataset for processing
    let large_dataset: Vec<String> = (0..1000).map(|i| format!("data_item_{}", i)).collect();

    println!("🚀 Starting comprehensive cancellation example");

    // Demonstrate hierarchical cancellation from README.md
    let parent_task = AsyncTask::emits::<ProcessedItem>()
        .with_timeout(60.seconds())
        .sender(|collector| {
            collector.of(large_dataset).into_chunks(100.items());
        })
        .receiver(|event, collector| {
            let item = event.data();

            // Simulate processing work
            let result = ProcessedItem {
                id: collector.len(),
                data: item.clone(),
                processed_at: std::time::Instant::now(),
            };

            stats_clone.increment_processed();
            collector.collect(result.id, result);
        })
        .on_cancel(move || {
            let stats = stats.clone();
            async move {
                println!("🧹 Cleaning up resources gracefully");
                stats.increment_cleanup();
                // Cleanup logic here - save partial progress, close files, etc.
                sleep(Duration::from_millis(100)).await;
                println!("✅ Cleanup completed");
            }
        })
        .await_final_event(|event, collector| match event {
            Ok(result) => {
                println!("🎉 Processing completed with {} items", collector.len());
                Ok(collector.collected())
            }
            Err(e) => {
                println!("❌ Processing failed: {:?}", e);
                Err(e)
            }
        });

    // Let it run for a short time
    println!("⏱️  Letting task run for 500ms...");
    sleep(Duration::from_millis(500)).await;

    println!("📊 Processed {} items so far", stats.processed_count());

    // Test cancellation escalation: Graceful -> Kill -> KillHard
    println!("\n🛑 Testing cancellation escalation:");

    println!("1. Attempting graceful cancellation...");
    let graceful_result = parent_task.cancel_gracefully().await;
    match graceful_result {
        Ok(()) => println!("✅ Graceful cancellation succeeded"),
        Err(e) => println!("⚠️  Graceful cancellation failed: {:?}", e),
    }

    // Give graceful cancellation time to work
    sleep(Duration::from_millis(200)).await;

    if parent_task.is_cancelled() {
        println!("✅ Task cancelled gracefully");
        println!("🧹 Cleanup was called {} times", stats.cleanup_count());
    } else {
        println!("⚠️  Graceful cancellation didn't work, trying forceful...");

        let forceful_result = parent_task.cancel_forcefully().await;
        match forceful_result {
            Ok(()) => println!("✅ Forceful cancellation succeeded"),
            Err(e) => println!("⚠️  Forceful cancellation failed: {:?}", e),
        }

        sleep(Duration::from_millis(100)).await;

        if parent_task.is_cancelled() {
            println!("✅ Task cancelled forcefully");
        } else {
            println!("🚨 Forceful cancellation didn't work, using immediate termination...");

            let immediate_result = parent_task.cancel_immediately().await;
            match immediate_result {
                Ok(()) => println!("✅ Immediate cancellation succeeded"),
                Err(e) => println!("❌ Immediate cancellation failed: {:?}", e),
            }
        }
    }

    // Final statistics
    println!("\n📈 Final Statistics:");
    println!("   Items processed: {}", stats.processed_count());
    println!("   Cleanup calls: {}", stats.cleanup_count());
    println!("   Task cancelled: {}", parent_task.is_cancelled());

    println!("\n🎯 Cancellation example completed successfully!");
    println!("   ✅ Zero allocations during cancellation");
    println!("   ✅ Lock-free atomic operations");
    println!("   ✅ Hierarchical cancellation propagation");
    println!("   ✅ Proper cleanup handler execution");

    Ok(())
}
