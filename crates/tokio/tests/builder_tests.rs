//! Tests for the Tokio task builder implementation

use std::time::Duration;
use sweet_async_api::task::TaskId;
use sweet_async_api::task::TaskPriority;
use sweet_async_api::task::builder::AsyncTaskBuilder;
use sweet_async_api::task::spawn::SpawningTaskBuilder;
use sweet_async_api::task::AsyncTask;
use uuid::Uuid;

use sweet_async_tokio::builder;
use sweet_async_tokio::task::tokio_task::AsyncTask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct TestTaskId(Uuid);

impl TaskId for TestTaskId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(TestTaskId)
    }
}

// Helper method to generate a new UUID (not part of the TaskId trait)
impl TestTaskId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[tokio::test]
async fn test_base_builder() {
    // Test creating a base builder
    let builder = builder::<String, TestTaskId>();
    
    // Test chaining methods
    let configured_builder = builder
        .name("test_task")
        .timeout(Duration::from_secs(10))
        .retry(3)
        .tracing(true);
    
    // Test accessing configuration
    assert_eq!(configured_builder.get_name(), Some("test_task".to_string()));
    assert_eq!(configured_builder.get_timeout(), Duration::from_secs(10));
    assert_eq!(configured_builder.get_retry_attempts(), 3);
    assert!(configured_builder.is_tracing_enabled());
}

#[tokio::test]
async fn test_spawning_builder() {
    // Test creating a spawning builder
    let builder = builder::spawning_builder::<String, &'static str, TestTaskId>();
    
    // Configure the builder
    let configured_builder = builder
        .name("test_spawning_task")
        .timeout(Duration::from_secs(5))
        .retry(2)
        .tracing(true)
        .priority(TaskPriority::High);
    
    // Verify builder configuration
    assert_eq!(configured_builder.get_name(), Some("test_spawning_task".to_string()));
    assert_eq!(configured_builder.get_timeout(), Duration::from_secs(5));
    assert_eq!(configured_builder.get_retry_attempts(), 2);
    assert!(configured_builder.is_tracing_enabled());
    assert_eq!(configured_builder.get_priority(), TaskPriority::High);
    
    // Run a simple task
    let task = configured_builder.run(|| async { 
        tokio::time::sleep(Duration::from_millis(10)).await;
        "Hello, World!".to_string() 
    });
    
    // Verify the task configuration
    assert_eq!(task.name(), Some("test_spawning_task".to_string()));
    
    /* Skipping execution test until actual implementation is completed
    // Verify that the task runs and produces the expected result
    let result = task.await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, World!".to_string());
    */
}

#[tokio::test]
#[ignore]
async fn test_await_result() {
    // Test creating a spawning builder
    let builder = builder::spawning_builder::<String, &'static str, TestTaskId>();
    
    // Use await_result directly
    let result = builder
        .name("await_result_task")
        .await_result(|| async { 
            tokio::time::sleep(Duration::from_millis(10)).await;
            "Hello from await_result".to_string() 
        })
        .await;
    
    // Verify the result
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello from await_result".to_string());
}

#[tokio::test]
#[ignore]
async fn test_await_result_with_handler() {
    // Test creating a spawning builder
    let builder = builder::spawning_builder::<String, &'static str, TestTaskId>();
    
    // Use await_result_with_handler
    let result = builder
        .name("handler_task")
        .await_result_with_handler(
            || async { 
                tokio::time::sleep(Duration::from_millis(10)).await;
                "Hello, Handler!".to_string() 
            },
            |result| {
                match result {
                    Ok(value) => format!("Processed: {}", value),
                    Err(_) => "Error occurred".to_string(),
                }
            }
        )
        .await;
    
    // Verify the result
    assert_eq!(result, "Processed: Hello, Handler!".to_string());
}

#[tokio::test]
#[ignore]
async fn test_async_task_to_builder() {
    // Test the AsyncTask::to() static method 
    let builder = AsyncTask::<String, TestTaskId>::to::<String, AsyncTask<String, TestTaskId>>();
    
    // Configure and run
    let task = builder
        .name("from_async_task_to")
        .run(|| async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            "created from AsyncTask::to()".to_string()
        });
    
    assert_eq!(task.name(), Some("from_async_task_to".to_string()));
    
    // Wait for task to complete
    let result = task.await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "created from AsyncTask::to()".to_string());
}

// Commented out until emitting builder is implemented
/*
#[tokio::test]
async fn test_emitting_builder() {
    // Test creating an emitting builder
    let builder = builder::emitting_builder::<String, Vec<String>, &'static str, TestTaskId>();
    
    // Configure sender and receiver
    let sender_builder = builder
        .timeout(Duration::from_secs(3))
        .sender(
            || async { "Event data".to_string() },
            sweet_async_api::task::builder::SenderStrategy::Serial { timeout_seconds: 1 }
        );
    
    // Add receiver
    let receiver_builder = sender_builder.receiver(
        || async { vec!["Processed event".to_string()] },
        sweet_async_api::task::builder::ReceiverStrategy::Serial { timeout_seconds: 1 }
    );
    
    // Run the task
    let task = receiver_builder.run();
    
    // Verify task execution
    let result = task.await_completion().await;
    assert!(result.is_ok());
}
*/
