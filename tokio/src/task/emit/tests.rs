//! Tests for the emitting task system in Tokio implementation

#[cfg(test)]
mod tests {
    use super::*;
    use sweet_async_api::task::TaskId;
    use sweet_async_api::task::builder::{ReceiverStrategy, SenderStrategy};
    use sweet_async_api::task::emit::EmittingTaskBuilder as ApiEmittingTaskBuilder;
    use std::time::Duration;
    use tokio::runtime::Handle;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_emits_with_sender_and_collector() {
        // This test verifies the pattern used in the README example:
        // let batch_results = AsyncTask::emits::<ApiResult>()
        //     .sender(|collector| {
        //         collector.of(user_ids)
        //             .into_chunks(10.items());
        //     })

        // Setup test data
        let user_ids = (0..25).collect::<Vec<_>>();
        let active_tasks = Arc::new(AtomicUsize::new(0));
        
        // Create a channel for the sender to produce items
        let (tx, mut rx) = mpsc::channel(32);
        
        // Spawn a task to send items in chunks of 10
        tokio::spawn({
            let tx = tx.clone();
            async move {
                for chunk in user_ids.chunks(10) {
                    if let Err(_) = tx.send(chunk.to_vec()).await {
                        break;
                    }
                }
            }
        });
        
        // Create the emitting task builder using the API's builder pattern
        let builder = TokioEmittingTaskBuilder::<_, Vec<usize>, (), ()>::new(
            Handle::current(),
            active_tasks.clone()
        );
        
        // Build the task with sender and receiver
        let task = builder
            .sender(
                |mut collector| async move {
                    while let Some(chunk) = rx.recv().await {
                        collector.collect_items(chunk).await;
                    }
                },
                SenderStrategy::new().with_batch_size(10),
            )
            .receiver(
                |chunk: Vec<usize>| async move {
                    // Process each chunk (just return it for this test)
                    chunk
                },
                ReceiverStrategy::new().parallel(),
            )
            .run()
            .await
            .expect("Task should complete successfully");
        
        // Verify the results
        let results = task.finish().expect("Should get final results");
        let all_items: Vec<usize> = results.into_values().flatten().collect();
        
        // Should have processed all 25 items
        assert_eq!(all_items.len(), 25);
        assert_eq!(all_items, (0..25).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn test_basic_emit_receive() {
        // Test basic emit/receive functionality
        let active_tasks = Arc::new(AtomicUsize::new(0));
        
        // Create a channel for the sender to produce items
        let (tx, mut rx) = mpsc::channel(1);
        
        // Spawn a task to send a single item
        tokio::spawn(async move {
            tx.send("test".to_string()).await.unwrap();
        });
        
        let builder = TokioEmittingTaskBuilder::<_, String, (), ()>::new(
            Handle::current(),
            active_tasks.clone()
        );
        
        let task = builder
            .sender(
                |mut collector| async move {
                    while let Some(item) = rx.recv().await {
                        collector.collect_item(item).await;
                    }
                },
                SenderStrategy::new(),
            )
            .receiver(
                |item: String| async move {
                    format!("received: {}", item)
                },
                ReceiverStrategy::new(),
            )
            .run()
            .await
            .expect("Task should complete successfully");
        
        let results = task.finish().expect("Should get final results");
        let all_items: Vec<String> = results.into_values().flatten().collect();
        
        assert_eq!(all_items, vec!["received: test"]);
    }

    #[tokio::test]
    async fn test_cancellation() {
        // Test that cancellation works properly
        let active_tasks = Arc::new(AtomicUsize::new(0));
        
        // Create a channel that will never complete
        let (_tx, rx) = tokio::sync::oneshot::channel::<()>();
        
        let builder = TokioEmittingTaskBuilder::<_, (), (), ()>::new(
            Handle::current(),
            active_tasks.clone()
        );
        
        let task = builder
            .sender(
                |_collector| async move {
                    // This will block forever since the sender was dropped
                    rx.await.ok();
                },
                SenderStrategy::new(),
            )
            .receiver(
                |_| async {},
                ReceiverStrategy::new(),
            )
            .run()
            .await
            .expect("Task should start");
        
        // Cancel the task
        task.cancel().expect("Should be able to cancel");
        
        // The task should be marked as complete after cancellation
        assert!(task.is_complete());
    }
}
