//! Full API test to validate all Sweet Async features

use crate::task::async_task::AsyncTask;
use crate::UuidTaskId;
use sweet_async_api::task::AsyncTask as ApiAsyncTask;
use sweet_async_api::task::spawn::SpawningTaskBuilder;
use sweet_async_api::task::emit::EmittingTaskBuilder;
use sweet_async_api::task::builder::{SenderStrategy, ReceiverStrategy};
use std::time::Duration;

#[tokio::test]
async fn test_to_with_await() {
    // Test 1: Basic AsyncTask::to() with await
    let result = AsyncTask::<String, UuidTaskId>::to()
        .timeout(Duration::from_secs(30))
        .run(|| async {
            "Hello from async task!".to_string()
        })
        .await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello from async task!");
}

#[tokio::test]
async fn test_await_result_pattern() {
    // Test 2: await_result() leap frog pattern
    let result = AsyncTask::<String, UuidTaskId>::to()
        .timeout(Duration::from_secs(30))
        .await_result(|| async {
            "Result from await_result".to_string()
        })
        .await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_emits_pattern() {
    // Test 3: AsyncTask::emits() with sender/receiver
    use crate::task::builder::MinMax;
    
    let builder = AsyncTask::<String, UuidTaskId>::emits();
    
    // This should compile if properly implemented
    let _emitter = builder
        .sender(
            || async {
                tokio::sync::mpsc::channel::<String>(10).1
            },
            SenderStrategy::Parallel { 
                workers: MinMax(1, 4),
                rate_limit: 10.0,
            }
        );
        
    // Test would continue with .receiver() and .await_final_event()
    // but we're just testing compilation for now
}

#[tokio::test]
async fn test_block_reduction_syntax() {
    // Test 4: Block reduction syntax (simplified closure)
    // This is more about ergonomics than functionality
    let _result = AsyncTask::<String, UuidTaskId>::to()
        .timeout(Duration::from_secs(30))
        .run(|| async {
            // Simulating block syntax
            let value = "Block syntax result";
            value.to_string()
        });
}

// Extension trait for duration helpers (should be in a utils module)
trait DurationExt {
    fn seconds(self) -> Duration;
}

impl DurationExt for u64 {
    fn seconds(self) -> Duration {
        Duration::from_secs(self)
    }
}

#[tokio::test]
async fn test_duration_extension() {
    // Test 5: Duration extension methods
    let thirty_seconds = 30.seconds();
    assert_eq!(thirty_seconds, Duration::from_secs(30));
}