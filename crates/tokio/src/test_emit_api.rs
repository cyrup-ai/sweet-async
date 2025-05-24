//! Test the emit API implementation

use crate::task::async_task::AsyncTask;
use crate::task::builder::MinMax;
use crate::UuidTaskId;
use sweet_async_api::task::AsyncTask as ApiAsyncTask;
use sweet_async_api::task::emit::EmittingTaskBuilder;
use sweet_async_api::task::builder::{SenderStrategy, ReceiverStrategy};
use std::time::Duration;

#[tokio::test]
async fn test_basic_emit_pattern() {
    // Create a simple emit task that generates numbers and collects their squares
    let result = AsyncTask::<i32, UuidTaskId>::emits()
        .sender(
            || async {
                // Create a channel that will emit numbers
                let (tx, rx) = tokio::sync::mpsc::channel::<i32>(10);
                
                // Spawn a task to send numbers
                tokio::spawn(async move {
                    for i in 1..=5 {
                        let _ = tx.send(i).await;
                    }
                });
                
                rx
            },
            SenderStrategy::Serial { timeout_seconds: 30 }
        )
        .receiver(
            |event, collector| async move {
                // Square the number and collect it
                let number = event.data();
                let squared = number * number;
                collector.collect_item(squared);
            },
            ReceiverStrategy::Serial { timeout_seconds: 30 }
        )
        .await_final_event(|event, _collector| async move {
            // Return all the squared numbers
            event.yield_results()
        })
        .await;
    
    // Check results
    match result {
        Ok(squares) => {
            println!("Collected squares: {:?}", squares);
            assert_eq!(squares.len(), 5);
            assert!(squares.contains(&1));
            assert!(squares.contains(&4));
            assert!(squares.contains(&9));
            assert!(squares.contains(&16));
            assert!(squares.contains(&25));
        }
        Err(e) => {
            panic!("Emit task failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_parallel_emit() {
    // Test parallel processing
    let _result = AsyncTask::<String, UuidTaskId>::emits()
        .sender(
            || async {
                // Create a channel for strings
                let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);
                
                // Send many items
                tokio::spawn(async move {
                    for i in 0..100 {
                        let _ = tx.send(format!("Item {}", i)).await;
                    }
                });
                
                rx
            },
            SenderStrategy::Parallel { 
                workers: MinMax(2, 4),
                rate_limit: 50.0,
            }
        );
    
    // Just test that it compiles for now
    println!("Parallel emit test compiled");
}