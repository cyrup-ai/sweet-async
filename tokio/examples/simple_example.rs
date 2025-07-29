//! A simple example demonstrating the fluent builder syntax
//! without relying on the full workspace setup.

use std::time::Duration;
use sweet_async_api::task::{
    AsyncTask, TaskPriority, 
    builder::{ReceiverStrategy, SenderStrategy}
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Basic task with timeout
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
    
    // Example 2: Simple emitting task
    println!("\n=== Example 2: Emitting Task ===");
    let results = AsyncTask::emits::<usize>()
        .sender(|collector| {
            collector.of(0..5)
                .into_chunks(2)
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
    
    Ok(())
}
