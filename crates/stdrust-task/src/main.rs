// Example std Rust-based implementation
use async_task_api::prelude::*;
use futures::executor::block_on;

fn main() {
    // Run an example task with std Rust futures
    let future = async_task!({
        println!("Running a task with std Rust futures!");
        "Task complete!"
    });
    
    // Use the futures executor to block_on the future
    let result = block_on(future);
    println!("Result: {}", result);
}