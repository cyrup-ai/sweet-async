use async_task::AsyncTask;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn test_spawn_with_block_syntax() {
    let value = Arc::new(Mutex::new(0));
    let value_clone = value.clone();

    let result = AsyncTask::<i32>::spawn(move || async move {
        // Simulate work
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Update shared value
        let mut data = value_clone.lock().unwrap();
        *data = 42;

        42
    })
    .await
    .unwrap();

    assert_eq!(result, 42);
    assert_eq!(*value.lock().unwrap(), 42);
}

#[tokio::test]
async fn test_spawn_with_closure_syntax() {
    let result = AsyncTask::<String>::spawn(|| async { "Hello, world!".to_string() })
        .await
        .unwrap();

    assert_eq!(result, "Hello, world!");
}

#[tokio::test]
async fn test_spawn_with_error_handling() {
    let condition = false;

    let result = AsyncTask::<i32>::spawn(move || async move {
        if condition {
            Ok(42)
        } else {
            Err::<i32, String>("Failed to compute value".into())
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Failed to compute value");
}

#[tokio::test]
async fn test_and_then_chaining() {
    let final_result = AsyncTask::<String>::spawn(|| async { "hello".to_string() })
        .and_then(|s| async move { Ok(s.to_uppercase()) })
        .and_then(|s| async move { Ok(format!("{}, WORLD!", s)) })
        .await
        .unwrap();

    assert_eq!(final_result, "HELLO, WORLD!");
}

#[tokio::test]
async fn test_map_transformation() {
    let length = AsyncTask::<String>::spawn(|| async { "hello".to_string() })
        .map(|s| s.len())
        .await
        .unwrap();

    assert_eq!(length, 5);
}

#[tokio::test]
async fn test_with_context() {
    let result = AsyncTask::<i32>::spawn(|| async { Err::<i32, String>("Original error".into()) })
        .with_context("Operation failed")
        .await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Operation failed: Original error"
    );
}

#[tokio::test]
async fn test_inspect() {
    let value = Arc::new(Mutex::new(0));
    let value_clone = value.clone();

    let result = AsyncTask::<i32>::spawn(|| async { 42 })
        .inspect(move |val| {
            let mut data = value_clone.lock().unwrap();
            *data = *val;
        })
        .await
        .unwrap();

    assert_eq!(result, 42);
    assert_eq!(*value.lock().unwrap(), 42);
}

#[tokio::test]
async fn test_multiple_concurrent_tasks() {
    let tasks: Vec<_> = (0..100)
        .map(|i| {
            AsyncTask::<i32>::spawn(move || async move {
                tokio::time::sleep(Duration::from_millis(5)).await;
                i
            })
        })
        .collect();

    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.unwrap());
    }

    assert_eq!(results.len(), 100);
    for i in 0..100 {
        assert!(results.contains(&i));
    }
}

#[tokio::test]
async fn test_explicit_task_cancellation() {
    let completed = Arc::new(Mutex::new(false));
    let completed_clone = completed.clone();

    let mut task = AsyncTask::<String>::spawn(move || async move {
        let completed = completed_clone;

        // This long task should be cancelled before completion
        tokio::time::sleep(Duration::from_secs(10)).await;

        // If we reach this point, the task wasn't cancelled
        let mut data = completed.lock().unwrap();
        *data = true;

        "Task completed!".to_string()
    });

    // Explicitly cancel the task
    task.cancel();

    // Wait a moment to ensure the cancellation has been processed
    tokio::time::sleep(Duration::from_millis(50)).await;

    // The task should have been cancelled, so completed should still be false
    assert_eq!(*completed.lock().unwrap(), false);

    // The result should be an error
    let result = task.await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_task_cancellation_on_drop() {
    let completed = Arc::new(Mutex::new(false));

    // Create a scope to ensure the task is dropped
    {
        let completed_clone = completed.clone();

        let _task = AsyncTask::<String>::spawn(move || async move {
            let completed = completed_clone;

            // This long task should be cancelled when task is dropped
            tokio::time::sleep(Duration::from_secs(10)).await;

            // If we reach this point, the task wasn't cancelled
            let mut data = completed.lock().unwrap();
            *data = true;

            "Task completed!".to_string()
        });

        // Let task go out of scope here
    }

    // Wait a moment to ensure the cancellation has been processed
    tokio::time::sleep(Duration::from_millis(50)).await;

    // The task should have been cancelled when dropped, so completed should still be false
    assert_eq!(*completed.lock().unwrap(), false);
}
