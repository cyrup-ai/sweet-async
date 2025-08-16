#[cfg(test)]
mod tests {
    use sweet_async_api::task::builder::{AsyncWork, AsyncTaskBuilder};
    use sweet_async_api::task::spawn::builder::SpawningTaskBuilder;
    use sweet_async_tokio::task::spawn::builder::TokioSpawningTaskBuilder;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    // Simple async work that returns a value
    struct ValueWork<T> {
        value: T,
    }
    
    impl<T: Clone + Send + 'static> AsyncWork<T> for ValueWork<T> {
        async fn run(self) -> T {
            self.value
        }
    }
    
    // Async work that returns a Result
    struct ResultWork<T, E> {
        result: Result<T, E>,
    }
    
    impl<T: Clone + Send + 'static, E: Clone + Send + 'static> AsyncWork<Result<T, E>> for ResultWork<T, E> {
        async fn run(self) -> Result<T, E> {
            self.result
        }
    }
    
    #[tokio::test]
    async fn test_await_result_success() {
        // Create a builder
        let builder = TokioSpawningTaskBuilder::<String, String, u64>::new();
        
        // Create work that succeeds
        let work = ResultWork {
            result: Ok("success".to_string())
        };
        
        // Execute synchronously with await_result
        let result = builder.await_result(work);
        
        // Verify we got the success value
        assert_eq!(result, Ok("success".to_string()));
    }
    
    #[tokio::test]
    async fn test_await_result_error() {
        // Create a builder
        let builder = TokioSpawningTaskBuilder::<i32, String, u64>::new();
        
        // Create work that returns an error
        let work = ResultWork {
            result: Err("error message".to_string())
        };
        
        // Execute synchronously with await_result
        let result = builder.await_result(work);
        
        // Verify we got the error
        assert_eq!(result, Err("error message".to_string()));
    }
    
    #[tokio::test]
    async fn test_await_result_async_work_executes() {
        // Track if async work actually ran
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();
        
        // Create async work that sets the flag
        struct TrackedWork {
            executed: Arc<AtomicBool>,
        }
        
        impl AsyncWork<Result<(), ()>> for TrackedWork {
            async fn run(self) -> Result<(), ()> {
                // Simulate async work
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                self.executed.store(true, Ordering::SeqCst);
                Ok(())
            }
        }
        
        let builder = TokioSpawningTaskBuilder::<(), (), u64>::new();
        let work = TrackedWork { executed: executed_clone };
        
        // Execute synchronously - should block until complete
        let result = builder.await_result(work);
        
        // Verify work executed and succeeded
        assert!(executed.load(Ordering::SeqCst), "Async work should have executed");
        assert_eq!(result, Ok(()));
    }
    
    #[tokio::test]
    async fn test_await_result_with_handler() {
        // Track if handler executed
        let handler_executed = Arc::new(AtomicBool::new(false));
        let handler_clone = handler_executed.clone();
        
        struct TrackingHandler {
            executed: Arc<AtomicBool>,
        }
        
        impl AsyncWork<String> for TrackingHandler {
            async fn run(self) -> String {
                self.executed.store(true, Ordering::SeqCst);
                "handler_result".to_string()
            }
        }
        
        let builder = TokioSpawningTaskBuilder::<i32, (), u64>::new();
        let work = ResultWork { result: Ok(42) };
        let handler = TrackingHandler { executed: handler_clone };
        
        // Execute with handler
        let output = builder.await_result_with_handler(work, handler);
        
        // Verify handler executed and returned its value
        assert!(handler_executed.load(Ordering::SeqCst), "Handler should have executed");
        assert_eq!(output, "handler_result");
    }
    
    #[tokio::test]
    async fn test_await_result_preserves_error_type() {
        // Custom error type to verify preservation
        #[derive(Debug, Clone, PartialEq)]
        struct CustomError {
            code: i32,
            message: String,
        }
        
        let builder = TokioSpawningTaskBuilder::<String, CustomError, u64>::new();
        let error = CustomError {
            code: 404,
            message: "Not found".to_string(),
        };
        
        let work = ResultWork {
            result: Err(error.clone())
        };
        
        // Execute and verify error type is preserved
        let result = builder.await_result(work);
        assert_eq!(result, Err(error));
    }
}