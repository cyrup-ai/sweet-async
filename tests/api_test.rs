use async_task::prelude::*;
use std::time::Duration;

// Verify the API works exactly as shown in the README
#[tokio::test]
async fn test_api_matches_readme() {
    // Basic usage with block syntax
    let result = async_task!({
        tokio::time::sleep(Duration::from_millis(10)).await;
        "Hello, world!".to_string()
    }).await;
    
    assert_eq!(result, "Hello, world!");
    
    // Builder pattern with configuration
    let result = builder!()
        .with_name("test-task")
        .with_priority(TaskPriority::High)
        .spawn({
            tokio::time::sleep(Duration::from_millis(10)).await;
            "Hello from builder!".to_string()
        }).await;
        
    assert_eq!(result, "Hello from builder!");
    
    // Error handling
    let condition = false;
    let result = async_task!({
        if condition {
            Ok(42)
        } else {
            Err(AsyncTaskError::Failure("Failed to compute value".into()))
        }
    }).await;
    
    assert!(result.is_err());
    match result {
        Err(AsyncTaskError::Failure(msg)) => assert_eq!(msg, "Failed to compute value"),
        _ => panic!("Unexpected error type")
    };
    
    // Map transform as shown in README
    let length = AsyncTask::<String>::spawn(|| async {
        "Hello, world!".to_string()
    }).map(|s| s.len()).await.unwrap();
    
    assert_eq!(length, 13);
    
    // Chaining operations with and_then as shown in README
    let final_result = AsyncTask::<String>::spawn(|| async {
        "hello".to_string()
    }).await.unwrap()
      .to_uppercase();
      
    assert_eq!(final_result, "HELLO");
}