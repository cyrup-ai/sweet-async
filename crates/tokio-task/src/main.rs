// Example Tokio-based implementation
use async_task_api::prelude::*;

fn main() {
    // Set up a simple Tokio runtime
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    // Run an example task
    runtime.block_on(async {
        // Using our macros for ergonomic syntax
        let result = async_task!({
            println!("Running a task with Tokio!");
            "Task complete!"
        }).await;
        
        println!("Result: {}", result);
    });
}