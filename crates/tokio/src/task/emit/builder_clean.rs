//! Clean emitting task builder for Tokio implementation

use std::marker::PhantomData;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;

use sweet_async_api::task::{
    AsyncTaskError, TaskId, TaskPriority,
};
use sweet_async_api::task::builder::{
    AsyncTaskBuilder, AsyncWork, ReceiverStrategy, SenderStrategy,
};

/// Clean emitting task that just spawns work without all the mutex nonsense
pub struct TokioEmittingTask<T, C, EItem, EOverall, I> 
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
{
    id: I,
    priority: TaskPriority,
    timeout: Duration,
    _phantom: PhantomData<(T, C, EItem, EOverall)>,
}

impl<T, C, EItem, EOverall, I> TokioEmittingTask<T, C, EItem, EOverall, I>
where
    T: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
{
    pub fn new(
        id: I,
        priority: TaskPriority,
        sender_work: impl AsyncWork<mpsc::Receiver<T>> + Send + 'static,
        sender_strategy: SenderStrategy,
        receiver_work: impl AsyncWork<C> + Send + 'static,
        receiver_strategy: ReceiverStrategy,
        timeout: Duration,
    ) -> Self {
        // Just spawn the work - no handles, no mutexes
        let (tx, rx) = mpsc::channel::<T>(100);
        
        // Spawn sender
        tokio::spawn(async move {
            let _rx = sender_work.run().await;
            // Process according to sender_strategy
        });
        
        // Spawn receiver  
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let _result = receiver_work.run().await;
                // Process according to receiver_strategy
            }
        });
        
        Self {
            id,
            priority,
            timeout,
            _phantom: PhantomData,
        }
    }
}