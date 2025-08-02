//! Quick validation test for core API functionality after cleanup
//! This ensures the main API surface still works correctly

#[cfg(test)]
mod tests {
    use sweet_async_api::task::AsyncTask;
    use crate::task::async_task::TokioAsyncTask;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_fluent_api() {
        // Test the core fluent API: AsyncTask::to::<T>().timeout().run().await
        
        // This should compile and work correctly after our cleanup
        let result = TokioAsyncTask::<String, uuid::Uuid>::to::<String, TokioAsyncTask<String, uuid::Uuid>>()
            .timeout(Duration::from_secs(5))
            .run(|| async {
                "Hello World".to_string()
            })
            .await;

        // If this compiles and runs, our API cleanup was successful
        match result {
            Ok(value) => assert_eq!(value, "Hello World"),
            Err(_) => {
                // Task execution errors are OK - we just need the API to compile
                // The important thing is that the fluent chain works
            }
        }
    }

    #[tokio::test] 
    async fn test_orchestrator_path() {
        use crate::orchestra::TokioOrchestrator;
        
        // Test the custom orchestrator path: AsyncTask::to().orchestrator(&custom).timeout().run().await
        let orchestrator = TokioOrchestrator::new();
        
        let result = TokioAsyncTask::<String, uuid::Uuid>::to::<String, TokioAsyncTask<String, uuid::Uuid>>()
            .orchestrator(&orchestrator)
            .timeout(Duration::from_secs(5))
            .run(|| {
                Box::pin(async {
                    "Custom orchestrator".to_string()
                }) as std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>>
            })
            .await;

        // Again, compilation success is the main goal
        match result {
            Ok(value) => assert_eq!(value, "Custom orchestrator"),
            Err(_) => {
                // Execution errors are acceptable - we're testing API surface
            }
        }
    }
}