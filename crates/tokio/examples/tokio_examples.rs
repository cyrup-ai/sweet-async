use sweet_async_api::task::{TaskPriority, CancellableTask, TaskStatus};
use sweet_async_api::task::spawn::SpawningTask;
use sweet_async_tokio::{TokioRuntime, TokioOrchestrator, process_adaptive, AdaptiveConfig};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Example demonstrating basic task execution
async fn basic_task_example() {
    println!("=== Basic Task Example ===");
    
    // Create a new runtime
    let runtime = TokioRuntime::new();
    
    // Create a simple task
    let task = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
            println!("Task executing...");
            std::thread::sleep(Duration::from_millis(100));
            42
        }),
        TaskPriority::Normal,
    );
    
    // Wait for the task to complete
    let result = task.into_future().await;
    println!("Task result: {:?}", result);
}

/// Example demonstrating task cancellation
async fn cancellation_example() {
    println!("\n=== Cancellation Example ===");
    
    // Create a new runtime
    let runtime = TokioRuntime::new();
    
    // Create a counter to track progress
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();
    
    // Create a task that can be cancelled
    let task = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(move || {
            // Simulate long-running work
            for i in 0..10 {
                std::thread::sleep(Duration::from_millis(200));
                println!("Working... step {}", i);
                
                // Update counter
                let runtime = tokio::runtime::Handle::current();
                runtime.block_on(async {
                    let mut counter = counter_clone.lock().await;
                    *counter += 1;
                });
            }
            
            "Task completed"
        }),
        TaskPriority::Normal,
    );
    
    // Register cancellation callback
    task.on_cancel(|| async {
        println!("Task cancellation callback executed!");
    });
    
    // Wait a bit then cancel the task
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("Cancelling task...");
    
    task.cancel_gracefully().await.unwrap();
    
    // Check status and counter
    println!("Task status: {:?}", task.status());
    println!("Counter value: {}", *counter.lock().await);
}

/// Example demonstrating the orchestrator
async fn orchestrator_example() {
    println!("\n=== Orchestrator Example ===");
    
    // Create a new runtime and orchestrator
    let runtime = TokioRuntime::new();
    let orchestrator = TokioOrchestrator::new(runtime.clone());
    
    // Create tasks with dependencies
    let task1 = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
            println!("Task 1 executing...");
            std::thread::sleep(Duration::from_millis(100));
            1
        }),
        TaskPriority::Normal,
    );
    
    let task2 = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
            println!("Task 2 executing...");
            std::thread::sleep(Duration::from_millis(100));
            2
        }),
        TaskPriority::Normal,
    );
    
    let task3 = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
            println!("Task 3 executing...");
            std::thread::sleep(Duration::from_millis(100));
            3
        }),
        TaskPriority::High, // Higher priority
    );
    
    // Register tasks with orchestrator
    let task1_ref = orchestrator.register_task(task1.clone());
    let task2_ref = orchestrator.register_task(task2.clone());
    let task3_ref = orchestrator.register_task(task3.clone());
    
    // Add dependencies
    let id1 = task1.task_id();
    let id2 = task2.task_id();
    let id3 = task3.task_id();
    
    // task3 depends on task1 and task2
    orchestrator.add_dependency(&id3, &id1).unwrap();
    orchestrator.add_dependency(&id3, &id2).unwrap();
    
    // Create a task group
    orchestrator.create_group("example_group").unwrap();
    orchestrator.add_task_to_group(&id1, "example_group").unwrap();
    orchestrator.add_task_to_group(&id2, "example_group").unwrap();
    orchestrator.add_task_to_group(&id3, "example_group").unwrap();
    
    // Start all tasks
    println!("Starting all tasks...");
    let results = orchestrator.start_all().await;
    
    // Report results
    for (id, result) in results {
        println!("Task {} result: {:?}", id.to_string(), result);
    }
    
    // Report task statuses
    let statuses = orchestrator.all_task_statuses();
    for (id, status) in statuses {
        println!("Task {} status: {:?}", id.to_string(), status);
    }
}

/// Example demonstrating adaptive concurrency
async fn adaptive_example() {
    println!("\n=== Adaptive Concurrency Example ===");
    
    // Create a list of 1000 items to process
    let items: Vec<usize> = (0..1000).collect();
    
    // Custom adaptive config
    let config = AdaptiveConfig {
        min_workers: 2,
        max_workers: num_cpus::get() * 2,
        sample_size: 20,
        io_threshold_ms: 30,
        adapt_interval_ms: 500,
        cpu_chunk_size: 50,
        io_chunk_size: 1,
        mixed_chunk_size: 10,
        partial_cancel: true,
    };
    
    // Process items with CPU-bound work
    println!("Processing CPU-bound work...");
    let start = Instant::now();
    let cpu_results = process_adaptive(
        items.clone(),
        |x| {
            // Simulate CPU-bound work with a complex calculation
            let mut result = 0;
            for i in 0..10000 {
                result = result.wrapping_add((i + x * 7) % 131);
            }
            result
        },
        Some(config.clone()),
    ).await;
    println!("CPU-bound processing completed in {:?}", start.elapsed());
    println!("Processed {} items", cpu_results.len());
    
    // Process items with IO-bound work
    println!("\nProcessing IO-bound work...");
    let start = Instant::now();
    let io_results = process_adaptive(
        items.clone(),
        |x| {
            // Simulate IO-bound work with sleep
            tokio::runtime::Handle::current().block_on(async {
                tokio::time::sleep(Duration::from_millis(5)).await;
                *x
            })
        },
        Some(config.clone()),
    ).await;
    println!("IO-bound processing completed in {:?}", start.elapsed());
    println!("Processed {} items", io_results.len());
    
    // Process items with mixed work
    println!("\nProcessing mixed work...");
    let start = Instant::now();
    let mixed_results = process_adaptive(
        items,
        |x| {
            // Mixed CPU/IO work
            if x % 3 == 0 {
                // CPU-bound
                let mut result = 0;
                for i in 0..1000 {
                    result = result.wrapping_add((i + x * 7) % 131);
                }
                result
            } else {
                // IO-bound
                tokio::runtime::Handle::current().block_on(async {
                    tokio::time::sleep(Duration::from_millis(2)).await;
                    *x
                })
            }
        },
        Some(config),
    ).await;
    println!("Mixed processing completed in {:?}", start.elapsed());
    println!("Processed {} items", mixed_results.len());
}

/// Example demonstrating timeout and fallback
async fn timeout_fallback_example() {
    println!("\n=== Timeout and Fallback Example ===");
    
    // Create a new runtime
    let runtime = TokioRuntime::new();
    
    // Create a task with timeout
    let task_with_timeout = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
            println!("Task with timeout executing...");
            std::thread::sleep(Duration::from_secs(2));
            "Task completed"
        }),
        TaskPriority::Normal,
    ).with_timeout(Duration::from_millis(500));
    
    // Create a task with fallback
    let task_with_fallback = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
            println!("Task with fallback executing...");
            if true {
                panic!("Simulating task failure");
            }
            "Task completed"
        }),
        TaskPriority::Normal,
    ).with_fallback("Fallback value");
    
    // Wait for tasks to complete
    let timeout_result = task_with_timeout.into_future().await;
    let fallback_result = task_with_fallback.into_future().await;
    
    println!("Task with timeout result: {:?}", timeout_result);
    println!("Task with fallback result: {:?}", fallback_result);
}

#[tokio::main]
async fn main() {
    // Run all examples
    basic_task_example().await;
    cancellation_example().await;
    orchestrator_example().await;
    adaptive_example().await;
    timeout_fallback_example().await;
}
