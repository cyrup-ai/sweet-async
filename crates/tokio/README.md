# Sweet Async Tokio

![Sweet Async Logo](/assets/sweet_async.png)

This crate provides a Tokio implementation for the Sweet Async library.

## Features

- **Full API implementation**: Implements all traits defined in `sweet_async_api`
- **Tokio-specific optimizations**: Leverages Tokio's capabilities for efficient task execution
- **Adaptive concurrency**: Automatically switches between CPU and IO bound workload handling
- **Structured task orchestration**: Manages task dependencies and lifecycle

## Implementation Status

This crate is currently under active development. See the [implementation plans](plans/README.md) for details on the current status and upcoming work.

- ✅ Basic `AsyncTask` implementation
- ✅ Basic `TokioRuntime` implementation 
- ✅ Basic `TokioOrchestrator` implementation
- ✅ Adaptive concurrency utilities
- 🚧 Builder pattern implementation (in progress)
- 🚧 Complete AsyncTask trait implementation (partial)
- 🚧 Parent-child relationship implementation (stubbed)
- 🚧 Task chaining (planned)
- 🚧 Event processing system (planned)

## Basic Usage

```rust
use sweet_async_tokio::{TokioRuntime, spawn, process_adaptive};
use sweet_async_api::task::TaskPriority;
use sweet_async_api::task::spawn::SpawningTask;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Create a runtime
    let runtime = TokioRuntime::new();
    
    // Spawn a task
    let task = runtime.spawn(
        sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
            // Task logic here
            42
        }),
        TaskPriority::Normal,
    );
    
    // Wait for the task to complete
    let result = task.into_future().await;
    println!("Result: {:?}", result);
    
    // Use adaptive concurrency for processing collections
    let items: Vec<i32> = (1..100).collect();
    let results = process_adaptive(
        items,
        |x| x * 2,
        None, // Use default configuration
    ).await;
    
    println!("Processed {} items", results.len());
}
```

## Advanced Features

### Task Orchestration

```rust
let runtime = TokioRuntime::new();
let orchestrator = TokioOrchestrator::new(runtime.clone());

// Create and register tasks
let task1 = runtime.spawn(...);
let task2 = runtime.spawn(...);

orchestrator.register_task(task1.clone());
orchestrator.register_task(task2.clone());

// Add dependencies
orchestrator.add_dependency(&task2.task_id(), &task1.task_id()).unwrap();

// Execute all tasks
let results = orchestrator.start_all().await;
```

### Adaptive Concurrency

```rust
// Custom configuration
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

// Process items with adaptive concurrency
let results = process_adaptive(
    items,
    |item| {
        // Process each item
        // Will automatically adapt between CPU and IO bound processing
    },
    Some(config),
).await;
```

## Upcoming Features

The following features are currently in development:

1. **Builder Pattern**: A fluent, immutable builder pattern for task creation
   ```rust
   // Coming soon
   let task = AsyncTask::to::<String>()
       .timeout(30.seconds())
       .run(|| async { "Hello, world!".to_string() })
       .await?;
   ```

2. **Event Processing**: Stream-based processing with different strategies
   ```rust
   // Coming soon
   let results = AsyncTask::emits::<UserData>()
       .sender(SenderStrategy::Parallel { workers: MinMax(1, 4) })
       .receiver(ReceiverStrategy::Serial { timeout_seconds: 30 })
       .await_final_event()
       .await?;
   ```

3. **Task Chaining**: Functional composition of tasks
   ```rust
   // Coming soon
   let result = task1
       .chain(|result| process_result(result))
       .chain(|processed| finalize(processed))
       .await?;
   ```

## Contributing

See the [implementation plans](plans/README.md) for guidance on contributing to this crate.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

![Book](/assets/book.png)
