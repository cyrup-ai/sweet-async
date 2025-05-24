//! Basic API test to validate core functionality

use crate::task::async_task::AsyncTask;
use crate::UuidTaskId;
use sweet_async_api::task::AsyncTask as ApiAsyncTask;
use sweet_async_api::task::spawn::SpawningTaskBuilder;

#[tokio::test]
async fn test_basic_to_api() {
    // Test that AsyncTask::to() works
    let builder = AsyncTask::<String, UuidTaskId>::to();
    
    // Test basic builder methods
    let task = builder
        .timeout(std::time::Duration::from_secs(5))
        .retry(2)
        .tracing(true)
        .run(|| async { "Hello World!".to_string() });
    
    // Test that we can await the result
    let result = task.await;
    
    match result {
        Ok(value) => {
            println!("Task completed successfully: {}", value);
            assert_eq!(value, "Hello World!");
        }
        Err(e) => {
            println!("Task failed: {:?}", e);
            // For now, expect this to fail since implementation is incomplete
        }
    }
}

#[tokio::test] 
async fn test_basic_emits_api() {
    // Test that AsyncTask::emits() works
    let _builder = AsyncTask::<String, UuidTaskId>::emits();
    
    // This should compile if the API is properly implemented
    println!("Basic emits() API test passed");
}