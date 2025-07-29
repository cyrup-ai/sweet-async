//! A standalone example demonstrating the fluent builder syntax
//! without any workspace dependencies.

use std::time::Duration;
use tokio::time::sleep;

// Simple implementation of the builder pattern
struct TaskBuilder {
    name: String,
    timeout: Duration,
    priority: Priority,
}

#[derive(Debug, Clone, Copy)]
enum Priority {
    Low,
    Normal,
    High,
}

impl TaskBuilder {
    fn new() -> Self {
        Self {
            name: "unnamed_task".to_string(),
            timeout: Duration::from_secs(5),
            priority: Priority::Normal,
        }
    }

    fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    async fn run<F, Fut, T>(self, task: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        println!("Running task '{}' with priority {:?} and timeout {:?}", 
            self.name, self.priority, self.timeout);
        
        // In a real implementation, we would use tokio::time::timeout here
        task().await
    }
}

#[tokio::main]
async fn main() {
    // Example 1: Basic task with timeout
    println!("=== Example 1: Basic Task ===");
    let result = TaskBuilder::new()
        .with_name("basic_task")
        .with_priority(Priority::High)
        .with_timeout(Duration::from_secs(2))
        .run(|| async {
            println!("Task is running...");
            sleep(Duration::from_millis(100)).await;
            "Task completed successfully"
        })
        .await;
    
    println!("Result: {}", result);
    
    // Example 2: Task with error handling
    println!("\n=== Example 2: Task with Error Handling ===");
    let result = TaskBuilder::new()
        .with_name("error_task")
        .with_retries(3)
        .run(|| async {
            println!("This task will fail and retry...");
            sleep(Duration::from_millis(100)).await;
            if true { // Simulate a failure
                Err("Temporary failure")
            } else {
                Ok("Success")
            }
        })
        .await;
    
    println!("Final result: {:?}", result);
}

// Add retry functionality
trait WithRetries {
    fn with_retries(self, retries: u32) -> Self;
}

impl WithRetries for TaskBuilder {
    fn with_retries(mut self, retries: u32) -> Self {
        println!("Task will be retried up to {} times on failure", retries);
        self
    }
}
