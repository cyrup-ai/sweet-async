#[cfg(test)]
mod tests {
    use sweet_async_api::task::ContextualizedTask;
    use sweet_async_tokio::task::spawn::task::{TokioSpawningTask, TokioAsyncWork};
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::sync::Arc;
    
    // Helper to create a no-op waker for testing
    fn noop_waker() -> std::task::Waker {
        use std::task::{RawWaker, RawWakerVTable, Waker};
        
        fn clone(_: *const ()) -> RawWaker { noop_raw_waker() }
        fn wake(_: *const ()) {}
        fn wake_by_ref(_: *const ()) {}
        fn drop(_: *const ()) {}
        
        fn noop_raw_waker() -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        
        const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        
        unsafe { Waker::from_raw(noop_raw_waker()) }
    }
    
    #[tokio::test]
    async fn test_runtime_access_through_parent_chain() {
        // Create a task without parent - should trigger orchestrator creation on first poll
        let mut task = TokioSpawningTask::<String, u64>::new(1);
        
        // Create a simple work that returns a value
        let work = TokioAsyncWork::from_value("test_result".to_string());
        task = task.run(work);
        
        // First poll should create orchestrator and set it as parent
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        
        // Poll once to trigger orchestrator creation
        let _ = Pin::new(&mut task).poll(&mut cx);
        
        // Now runtime should be accessible via parent chain
        let runtime = task.runtime();
        assert!(runtime.is_running());
        assert_eq!(runtime.active_task_count(), 0); // No active tasks initially
    }
    
    #[tokio::test]
    async fn test_lazy_orchestrator_creation() {
        // Create task with direct value (no work to spawn)
        let mut task = TokioSpawningTask::<i32, u32>::with_value(42, 100);
        
        // Before polling, the task should have no parent
        // (We can't test this directly due to private fields, but runtime() would panic)
        
        // Poll the task - should return the value immediately
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        
        match Pin::new(&mut task).poll(&mut cx) {
            Poll::Ready(result) => {
                match result.into_result() {
                    Ok(value) => assert_eq!(value, 100),
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            }
            Poll::Pending => panic!("Expected immediate result for direct value"),
        }
    }
    
    #[tokio::test]
    async fn test_work_execution_through_runtime() {
        // Create a task with actual async work
        let work = TokioAsyncWork::new(Arc::new(|| {
            Box::pin(async {
                // Simulate some async work
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                "async_result".to_string()
            })
        }));
        
        let mut task = TokioSpawningTask::<String, u64>::new(123).run(work);
        
        // Poll the task to completion
        let result = (&mut task).await;
        
        match result.into_result() {
            Ok(value) => assert_eq!(value, "async_result"),
            Err(e) => panic!("Work execution failed: {:?}", e),
        }
        
        // Verify runtime is accessible after completion
        let runtime = task.runtime();
        assert!(runtime.is_running());
    }
}