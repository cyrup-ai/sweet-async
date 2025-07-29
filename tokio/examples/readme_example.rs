//! Examples mirroring the README.md fluent builder syntax
//! 
//! This example demonstrates the various builder patterns shown in the README
//! using the Tokio implementation.

use std::time::Duration;
use sweet_async_api::task::{
    AsyncTask, TaskPriority, TaskRelationships, 
    builder::{ReceiverStrategy, SenderStrategy}
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Basic task with timeout and error handling
    println!("=== Example 1: Basic Task ===");
    let result = AsyncTask::new()
        .with_name("basic_task")
        .with_priority(TaskPriority::High)
        .with_timeout(Duration::from_secs(5))
        .run(|| async {
            println!("Running basic task");
            "Task completed successfully"
        })
        .await?;
    
    println!("Result: {}", result);
    
    // Example 2: Task with dependencies
    println!("\n=== Example 2: Task with Dependencies ===");
    let dep1 = AsyncTask::new()
        .with_name("dependency_1")
        .run(|| async { 42 })
        .await?;
    
    let dep2 = AsyncTask::new()
        .with_name("dependency_2")
        .run(|| async { 10 })
        .await?;
    
    let result = AsyncTask::new()
        .with_name("dependent_task")
        .with_dependencies(vec![&dep1, &dep2])
        .run(move |deps: TaskRelationships| async move {
            let sum: i32 = deps.iter()
                .map(|r| r.downcast_ref::<i32>().unwrap())
                .sum();
            sum
        })
        .await?;
    
    println!("Sum of dependencies: {}", result);
    
    // Example 3: Emitting task with sender/receiver pattern
    println!("\n=== Example 3: Emitting Task with Sender/Receiver ===");
    let results = AsyncTask::emits::<usize>()
        .sender(|collector| {
            collector.of(0..10)
                .into_chunks(3)
                .with_name("number_emitter")
        })
        .receiver(|chunk: Vec<usize>| async move {
            println!("Processing chunk: {:?}", chunk);
            chunk.into_iter().sum::<usize>()
        })
        .run()
        .await?;
    
    let total: usize = results.into_values().sum();
    println!("Total sum: {}", total);
    
    // Example 4: Task with error handling and retries
    println!("\n=== Example 4: Task with Error Handling ===");
    let mut attempt = 0;
    let result = AsyncTask::new()
        .with_name("retry_task")
        .with_retries(3)
        .with_timeout(Duration::from_secs(2))
        .run(move || {
            attempt += 1;
            println!("Attempt {}", attempt);
            async move {
                if attempt < 3 {
                    Err("Temporary failure".into())
                } else {
                    Ok("Success after retries")
                }
            }
        })
        .await?;
    
    println!("Final result: {}", result);
    
    // Example 5: Parallel task execution
    println!("\n=== Example 5: Parallel Task Execution ===");
    let tasks = (0..5).map(|i| {
        AsyncTask::new()
            .with_name(format!("parallel_task_{}", i))
            .with_priority(TaskPriority::Normal)
            .run(move || async move {
                tokio::time::sleep(Duration::from_millis(100 * (5 - i) as u64)).await;
                i * i
            })
    });
    
    let results = futures::future::join_all(tasks).await;
    println!("Parallel task results: {:?}", results);
    
    Ok(())
}
