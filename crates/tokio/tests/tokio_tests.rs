#[cfg(test)]
mod tests {
    use std::time::Duration;
    use sweet_async_api::task::builder::AsyncTaskBuilder;
    use sweet_async_api::task::CancellableTask;
    use sweet_async_api::task::TaskStatus;
    use sweet_async_tokio::{TokioRuntime, TokioOrchestrator, process_adaptive};
    
    #[tokio::test]
    async fn test_basic_task_execution() {
        let runtime = TokioRuntime::new();
        
        // Create a simple task that returns a value
        let task = runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| 42),
            sweet_async_api::task::TaskPriority::Normal,
        );
        
        // Verify task execution
        let result = task.into_future().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_task_cancellation() {
        let runtime = TokioRuntime::new();
        
        // Create a task that sleeps
        let task = runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
                std::thread::sleep(Duration::from_secs(1));
                42
            }),
            sweet_async_api::task::TaskPriority::Normal,
        );
        
        // Cancel the task
        let cancel_result = task.cancel_gracefully().await;
        assert!(cancel_result.is_ok());
        
        // Verify task status
        assert!(task.is_cancelled());
    }
    
    #[tokio::test]
    async fn test_task_timeout() {
        let runtime = TokioRuntime::new();
        
        // Create a task with timeout
        let task = runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
                std::thread::sleep(Duration::from_secs(2));
                42
            }),
            sweet_async_api::task::TaskPriority::Normal,
        ).with_timeout(Duration::from_millis(100));
        
        // Execute the task
        let result = task.into_future().await;
        
        // Verify timeout occurred
        assert!(result.is_err());
        match result {
            Err(sweet_async_api::task::AsyncTaskError::Timeout(_)) => {
                // Expected error
            }
            _ => panic!("Expected timeout error"),
        }
    }
    
    #[tokio::test]
    async fn test_orchestrator() {
        let runtime = TokioRuntime::new();
        let orchestrator = TokioOrchestrator::new(runtime.clone());
        
        // Create tasks
        let task1 = runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| 1),
            sweet_async_api::task::TaskPriority::Normal,
        );
        
        let task2 = runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| 2),
            sweet_async_api::task::TaskPriority::Normal,
        );
        
        // Register tasks with orchestrator
        let task1_ref = orchestrator.register_task(task1.clone());
        let task2_ref = orchestrator.register_task(task2.clone());
        
        // Start all tasks
        let results = orchestrator.start_all().await;
        
        // Verify results
        assert_eq!(results.len(), 2);
        
        // Check results individually
        for (_, result) in results {
            assert!(result.is_ok());
            assert!(result.unwrap() == 1 || result.unwrap() == 2);
        }
    }
    
    #[tokio::test]
    async fn test_adaptive_processing() {
        // Create a list of items to process
        let items: Vec<i32> = (1..100).collect();
        
        // Process items using adaptive concurrency
        let results = process_adaptive(
            items,
            |x| x * 2,
            None,
        ).await;
        
        // Verify results
        assert_eq!(results.len(), 99);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(*result, (i as i32 + 1) * 2);
        }
    }
    
    #[tokio::test]
    async fn test_task_priority() {
        let runtime = TokioRuntime::new();
        
        // Create tasks with different priorities
        let high_task = runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| 1),
            sweet_async_api::task::TaskPriority::High,
        );
        
        let low_task = runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| 2),
            sweet_async_api::task::TaskPriority::Low,
        );
        
        // Verify priorities
        assert_eq!(*high_task.priority(), sweet_async_api::task::TaskPriority::High);
        assert_eq!(*low_task.priority(), sweet_async_api::task::TaskPriority::Low);
        
        // Execute tasks
        let high_result = high_task.into_future().await;
        let low_result = low_task.into_future().await;
        
        // Verify results
        assert!(high_result.is_ok());
        assert!(low_result.is_ok());
        assert_eq!(high_result.unwrap(), 1);
        assert_eq!(low_result.unwrap(), 2);
    }
    
    #[tokio::test]
    async fn test_task_fallback() {
        let runtime = TokioRuntime::new();
        
        // Create a task that will fail
        let task = runtime.spawn(
            sweet_async_api::task::spawn::builder::BaseSpawningTask::new(|| {
                panic!("Task failure");
            }),
            sweet_async_api::task::TaskPriority::Normal,
        ).with_fallback(42);
        
        // Execute the task
        let result = task.into_future().await;
        
        // Verify fallback was used
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
