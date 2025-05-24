//! Emitting task builder for Tokio implementation
//!
//! This module provides the implementation for creating and configuring stream-based tasks.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use tokio::runtime::Handle;
use tokio::sync::{Mutex, oneshot, Semaphore, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sweet_async_api::task::builder::{
    AsyncTaskBuilder, AsyncWork, ReceiverStrategy, SenderStrategy,
};
use sweet_async_api::task::emit::{EmittingTask, EmittingTaskBuilder as ApiEmittingTaskBuilder};
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskPriority};

use super::event::{TokioEventSender, create_event_channel};
use crate::task::builder::TokioAsyncTaskBuilder;

/// Tokio implementation of the EmittingTaskBuilder
#[derive(Clone)]
pub struct TokioEmittingTaskBuilder<
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + Sync + 'static,
    I: TaskId,
> {
    /// Base builder with common configuration
    base_builder: TokioAsyncTaskBuilder<T, I>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks registry
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Task priority
    priority: TaskPriority,
    /// Type markers
    _marker_c: PhantomData<C>,
    _marker_eitem: PhantomData<EItem>,
    _marker_eoverall: PhantomData<EOverall>,
}

impl<
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + Sync + 'static,
    I: TaskId,
> TokioEmittingTaskBuilder<T, C, EItem, EOverall, I>
{
    /// Create a new emitting task builder
    pub fn new(runtime: Handle, active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>) -> Self {
        Self {
            base_builder: TokioAsyncTaskBuilder::new(runtime.clone(), active_tasks.clone()),
            runtime,
            active_tasks,
            priority: TaskPriority::Normal,
            _marker_c: PhantomData,
            _marker_eitem: PhantomData,
            _marker_eoverall: PhantomData,
        }
    }

    /// Set the task priority
    pub fn priority(self, priority: TaskPriority) -> Self {
        Self { priority, ..self }
    }

    /// Set the task name
    pub fn name(self, name: &str) -> Self {
        Self {
            base_builder: self.base_builder.name(name),
            ..self
        }
    }
}

impl<
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + Sync + 'static,
    I: TaskId,
> AsyncTaskBuilder for TokioEmittingTaskBuilder<T, C, EItem, EOverall, I>
{
    fn timeout(self, duration: Duration) -> Self {
        Self {
            base_builder: self.base_builder.timeout(duration),
            ..self
        }
    }

    fn retry(self, attempts: u8) -> Self {
        Self {
            base_builder: self.base_builder.retry(attempts),
            ..self
        }
    }

    fn tracing(self, enabled: bool) -> Self {
        Self {
            base_builder: self.base_builder.tracing(enabled),
            ..self
        }
    }

    fn new() -> Self {
        let runtime = Handle::current();
        let active_tasks = Arc::new(Mutex::new(Vec::new()));
        Self::new(runtime, active_tasks)
    }
}

impl<
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + Sync + 'static,
    I: TaskId,
> ApiEmittingTaskBuilder<T, C, EItem, I> for TokioEmittingTaskBuilder<T, C, EItem, EOverall, I>
{
    type SenderBuilder = TokioSenderBuilder<T, C, EItem, EOverall, I>;

    fn sender<F>(
        self,
        sender_logic: F,
        strategy: SenderStrategy,
    ) -> Self::SenderBuilder
    where
        F: AsyncWork<T> + Send + 'static,
    {
        TokioSenderBuilder::new(
            self.base_builder,
            self.runtime,
            self.active_tasks,
            self.priority,
            Box::new(sender_logic),
            strategy,
        )
    }
}

impl<T: Clone + Send + 'static, Task: AsyncTask<T, I>, I: TaskId, E>
    sweet_async_api::orchestra::OrchestratorBuilder<T, Task, I>
    for TokioEmittingTaskBuilder<T, T, E, E, I>
where
    E: Send + Sync + 'static,
{
    type Next = TokioAsyncTaskBuilder<T, I>;

    fn orchestrator<O: sweet_async_api::orchestra::orchestrator::TaskOrchestrator<T, Task, I>>(
        self,
        orchestrator: &O,
    ) -> Self::Next {
        self.base_builder
    }
}

/// Tokio implementation of the SenderBuilder
pub struct TokioSenderBuilder<T, C, EItem, EOverall, I>
where
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
{
    /// Base builder with common configuration
    base_builder: TokioAsyncTaskBuilder<T, I>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks registry
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Task priority
    priority: TaskPriority,
    /// Event sender work function
    sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
    /// Sender strategy
    sender_strategy: SenderStrategy,
    /// Type markers
    _marker_c: PhantomData<C>,
    _marker_eitem: PhantomData<EItem>,
    _marker_eoverall: PhantomData<EOverall>,
}

impl<T, C, EItem, EOverall, I> TokioSenderBuilder<T, C, EItem, EOverall, I>
where
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
{
    /// Create a new sender builder
    pub fn new(
        base_builder: TokioAsyncTaskBuilder<T, I>,
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        priority: TaskPriority,
        sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
        sender_strategy: SenderStrategy,
    ) -> Self {
        Self {
            base_builder,
            runtime,
            active_tasks,
            priority,
            sender_work,
            sender_strategy,
            _marker_c: PhantomData,
            _marker_eitem: PhantomData,
            _marker_eoverall: PhantomData,
        }
    }
}

impl<
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + 'static,
    I: TaskId,
> sweet_async_api::task::emit::builder::SenderBuilder<T, C, EItem, I> 
    for TokioSenderBuilder<T, C, EItem, EOverall, I>
{
    type ReceiverBuilder = TokioReceiverBuilder<T, C, EItem, I>;

    fn receiver<F>(
        self,
        receiver_work: F,
        strategy: ReceiverStrategy,
    ) -> Self::ReceiverBuilder
    where
        F: AsyncWork<C> + Send + 'static,
    {
        TokioReceiverBuilder::new(
            self.base_builder,
            self.runtime,
            self.active_tasks,
            self.priority,
            self.sender_work,
            self.sender_strategy,
            Box::new(receiver_work),
            strategy,
        )
    }
}

/// Tokio implementation of ReceiverBuilder
pub struct TokioReceiverBuilder<T, C, EItem, I>
where 
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    I: TaskId,
{
    /// Base builder with common configuration
    base_builder: TokioAsyncTaskBuilder<T, I>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks registry
    active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Task priority
    priority: TaskPriority,
    /// Event sender work function
    sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
    /// Sender strategy
    sender_strategy: SenderStrategy,
    /// Event receiver work
    receiver_work: Box<dyn AsyncWork<C> + Send + 'static>,
    /// Receiver strategy
    receiver_strategy: ReceiverStrategy,
    /// Type markers
    _marker_eitem: PhantomData<EItem>,
}

impl<T, C, EItem, I> TokioReceiverBuilder<T, C, EItem, I>
where
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    I: TaskId,
{
    /// Create a new receiver builder
    pub fn new(
        base_builder: TokioAsyncTaskBuilder<T, I>,
        runtime: Handle,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        priority: TaskPriority,
        sender_work: Box<dyn AsyncWork<T> + Send + 'static>,
        sender_strategy: SenderStrategy,
        receiver_work: Box<dyn AsyncWork<C> + Send + 'static>,
        receiver_strategy: ReceiverStrategy,
    ) -> Self {
        Self {
            base_builder,
            runtime,
            active_tasks,
            priority,
            sender_work,
            sender_strategy,
            receiver_work,
            receiver_strategy,
            _marker_eitem: PhantomData,
        }
    }
}

/// Tokio implementation of EmittingTask
pub struct TokioEmittingTask<
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Send + Sync + 'static,
    EOverall: Send + Sync + 'static,
    I: TaskId,
> {
    /// Task ID
    id: I,
    /// Task priority
    priority: TaskPriority,
    /// Sender task handle
    sender_handle: Arc<Mutex<Option<JoinHandle<Result<(), AsyncTaskError>>>>>,
    /// Receiver task handle
    receiver_handle:
        Arc<Mutex<Option<JoinHandle<Result<HashMap<Uuid, Result<C, EItem>>, AsyncTaskError>>>>>,
    /// Event sender
    event_sender: Arc<TokioEventSender<T>>,
    /// Cancellation sender
    cancel_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    /// Final result from task completion
    final_result: Arc<Mutex<Option<Result<HashMap<Uuid, Result<C, EItem>>, AsyncTaskError>>>>,
    /// Task metrics
    metrics: crate::task::async_task::TaskMetrics,
    /// Task timeout
    task_timeout: Duration,
    /// Type markers
    _marker_e_overall: PhantomData<EOverall>,
}

impl<
    T: Clone + Send + 'static,
    C: Clone + Send + 'static,
    EItem: Clone + Send + Sync + 'static,
    I: TaskId + Copy + Clone + Send + Sync + 'static,
> TokioEmittingTask<T, C, EItem, AsyncTaskError, I>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: I,
        priority: TaskPriority,
        sender_work_produces_receiver: Box<
            dyn AsyncWork<tokio::sync::mpsc::Receiver<T>> + Send + 'static,
        >,
        sender_strategy: SenderStrategy,
        receiver_work_fn: fn(&T, &mut (), Uuid) -> Result<C, EItem>,
        receiver_strategy: ReceiverStrategy,
        runtime: Handle,
        task_timeout_duration: Duration,
        task_retry_count: u8,
        _tracing_enabled: bool,
        active_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
        _name: Option<String>,
    ) -> Self {
        let (internal_event_tx, internal_event_rx_for_collector) = create_event_channel::<T>(100);
        let (cancel_tx_main_oneshot, mut cancel_rx_for_sender) = oneshot::channel();
        let (_receiver_task_cancel_tx, receiver_task_cancel_rx) = oneshot::channel();

        let sender_join_handle_arc = Arc::new(Mutex::new(None));
        let receiver_join_handle_arc = Arc::new(Mutex::new(None));
        let final_result_arc = Arc::new(Mutex::new(None));

        let runtime_for_sender = runtime.clone();
        let active_tasks_for_sender = active_tasks.clone();
        let sender_join_handle_for_storage = sender_join_handle_arc.clone();
        let internal_event_tx_for_sender_task = internal_event_tx.clone();

        let original_sender_jh: JoinHandle<Result<(), AsyncTaskError>> = runtime_for_sender.spawn(async move {
            let mut user_event_source_rx: tokio::sync::mpsc::Receiver<T> = sender_work_produces_receiver.run().await;
            let overall_sender_result: Result<(), AsyncTaskError> = Ok(());

            match sender_strategy {
                SenderStrategy::Serial { timeout_seconds } => {
                    let item_send_timeout = if timeout_seconds > 0 { Some(Duration::from_secs(timeout_seconds)) } else { None };
                    loop {
                        tokio::select! {
                            biased;
                            _ = &mut cancel_rx_for_sender => { return Err(AsyncTaskError::Cancelled); }
                            maybe_event_t = user_event_source_rx.recv() => {
                                if let Some(event_t) = maybe_event_t {
                                    let send_op = internal_event_tx_for_sender_task.send(event_t);
                                    let send_result = match item_send_timeout {
                                        Some(dur) => tokio::time::timeout(dur, send_op).await.map_err(|_| AsyncTaskError::Timeout(dur)),
                                        None => Ok(send_op.await),
                                    };
                                    if let Err(send_error) = send_result.and_then(|res| res.map_err(|_| AsyncTaskError::Failure("Internal channel send failed".to_string()))) {
                                        tracing::error!("SerialSender: Failed to send event to internal channel: {:?}", send_error);
                                        return Err(send_error);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Simplified for other strategies
                    tracing::warn!("Only Serial sender strategy is implemented for now");
                }
            }
            overall_sender_result
        });

        {
            let sender_handle_clone = original_sender_jh;
            let runtime_clone = runtime.clone();
            
            tokio::spawn(async move {
                *sender_join_handle_for_storage.lock().await = Some(sender_handle_clone);
            });
        }

        let final_result_for_receiver_task = final_result_arc.clone();
        let runtime_for_receiver = runtime.clone();
        let receiver_join_handle_for_storage = receiver_join_handle_arc.clone();
        let shared_receiver_work_fn_with_result = Arc::new(receiver_work_fn);

        let original_receiver_jh: JoinHandle<Result<HashMap<Uuid, Result<C, EItem>>, AsyncTaskError>> = runtime_for_receiver.spawn(async move {
            let mut local_collector = crate::task::emit::collector::TokioEventCollector::<T, C, EItem, I>::new();
            let collector_token = CancellationToken::new();

            local_collector.start_processing(
                internal_event_rx_for_collector,
                receiver_strategy,
                shared_receiver_work_fn_with_result,
                collector_token.clone()
            );

            tokio::select! {
                biased;
                _ = receiver_task_cancel_rx => {
                    collector_token.cancel();
                    let collected_map_on_cancel = local_collector.join().await;
                    *final_result_for_receiver_task.lock().await = Some(Ok(collected_map_on_cancel.clone()));
                    Err(AsyncTaskError::Cancelled)
                }
                collected_items_map_result = local_collector.join() => {
                    *final_result_for_receiver_task.lock().await = Some(Ok(collected_items_map_result.clone()));
                    Ok(collected_items_map_result)
                }
            }
        });

        {
            let receiver_handle_clone = original_receiver_jh;
            
            tokio::spawn(async move {
                *receiver_join_handle_for_storage.lock().await = Some(receiver_handle_clone);
            });
        }

        Self {
            id,
            priority,
            sender_handle: sender_join_handle_arc,
            receiver_handle: receiver_join_handle_arc,
            event_sender: internal_event_tx,
            cancel_tx: Arc::new(Mutex::new(Some(cancel_tx_main_oneshot))),
            final_result: final_result_arc,
            metrics: crate_shared_metrics_instance(),
            task_timeout: task_timeout_duration,
            _marker_e_overall: PhantomData,
        }
    }

    /// Waits for the emitting task to complete and returns a TokioFinalEvent or an error.
    pub async fn await_final_event(
        self,
    ) -> Result<crate::task::emit::TokioFinalEvent<(), C, EItem, I>, AsyncTaskError> {
        let task_id_for_final_event = self.id;
        let overall_task_timeout = self.task_timeout;
        let final_result_arc_clone = self.final_result.clone();
        let receiver_handle_arc_clone = self.receiver_handle.clone();

        let processing_future = async {
            let receiver_jh_opt = {
                let mut handle_guard = receiver_handle_arc_clone.lock().await;
                handle_guard.take()
            };

            if let Some(receiver_jh) = receiver_jh_opt {
                match receiver_jh.await {
                    Ok(task_outcome_result) => match task_outcome_result {
                        Ok(collected_map_with_results) => {
                            let final_event = crate::task::emit::TokioFinalEvent::new(
                                (),
                                collected_map_with_results,
                                task_id_for_final_event,
                            );
                            Ok(final_event)
                        }
                        Err(async_task_error) => Err(async_task_error),
                    },
                    Err(join_error) => {
                        let async_task_error = AsyncTaskError::Failure(format!(
                            "Receiver task join error: {}",
                            join_error
                        ));
                        {
                            let mut fr_guard = final_result_arc_clone.lock().await;
                            if fr_guard.is_none() {
                                *fr_guard = Some(Err(async_task_error.clone()));
                            }
                        }
                        Err(async_task_error)
                    }
                }
            } else {
                let final_result_outcome = { final_result_arc_clone.lock().await.clone() };
                match final_result_outcome {
                    Some(Ok(collected_map_with_results)) => {
                        let final_event = crate::task::emit::TokioFinalEvent::new(
                            (),
                            collected_map_with_results,
                            task_id_for_final_event,
                        );
                        Ok(final_event)
                    }
                    Some(Err(async_task_error)) => Err(async_task_error),
                    None => Err(AsyncTaskError::Failure(
                        "Emitting task result unavailable and handle already taken/not set"
                            .to_string(),
                    )),
                }
            }
        };

        if overall_task_timeout > Duration::ZERO {
            match tokio::time::timeout(overall_task_timeout, processing_future).await {
                Ok(result_of_processing) => result_of_processing,
                Err(_) => {
                    let timeout_error = AsyncTaskError::Timeout(overall_task_timeout);
                    {
                        let mut fr_guard = final_result_arc_clone.lock().await;
                        if fr_guard.is_none() || matches!(*fr_guard, Some(Ok(_))) {
                            *fr_guard = Some(Err(timeout_error.clone()));
                        }
                    }
                    Err(timeout_error)
                }
            }
        } else {
            processing_future.await
        }
    }
}

// Implement core AsyncTask methods first (these are required by EmittingTask trait)
impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId> AsyncTask<T, I>
    for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    fn to<R: Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_spawning()
    }

    fn emits<R: Send + 'static, Task: AsyncTask<R, I>>()
    -> impl sweet_async_api::orchestra::OrchestratorBuilder<R, Task, I> {
        use crate::builder::DefaultOrchestratorBuilder;
        DefaultOrchestratorBuilder::<R, Task, I>::new_emitting()
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    EmittingTask<T, C, EItem, I> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    type Final = crate::task::emit::TokioFinalEvent<T, C, EItem, I>;

    fn is_complete(&self) -> bool {
        match self.final_result.try_lock() {
            Ok(result) => result.is_some(),
            Err(_) => false,
        }
    }

    fn cancel(&self) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        let cancel_tx = Arc::clone(&self.cancel_tx);
        
        match cancel_tx.try_lock() {
            Ok(mut guard) => {
                if let Some(sender) = guard.take() {
                    let _ = sender.send(());
                }
                Ok(())
            }
            Err(_) => {
                tokio::spawn(async move {
                    let mut guard = cancel_tx.lock().await;
                    if let Some(sender) = guard.take() {
                        let _ = sender.send(());
                    }
                });
                
                Ok(())
            }
        }
    }
}

// Implement all required AsyncTask supertraits
impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::PrioritizedTask<T> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    fn priority(&self) -> &impl sweet_async_api::task::RankableByPriority {
        &self.priority
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::CancellableTask<T> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    async fn cancel(
        &self,
        _level: sweet_async_api::task::CancellationLevel,
    ) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        let cancel_tx = self.cancel_tx.clone();

        let sender = {
            let mut guard = cancel_tx.lock().await;
            guard.take()
        };

        if let Some(tx) = sender {
            let _ = tx.send(());
        }

        Ok(())
    }

    fn is_cancelled(&self) -> bool {
        match self.final_result.try_lock() {
            Ok(result) => matches!(&*result, Some(Err(AsyncTaskError::Cancelled))),
            Err(_) => false,
        }
    }

    fn on_cancel<F, Fut>(&self, _callback: F)
    where
        F: sweet_async_api::task::builder::AsyncWork<Fut> + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // TODO: Implement callback registration
    }

    fn cancel_gracefully(&self) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        self.cancel()
    }

    fn cancel_forcefully(&self) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        self.cancel()
    }

    fn cancel_immediately(&self) -> Result<(), sweet_async_api::orchestra::OrchestratorError> {
        self.cancel()
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::TracingTask<T> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    fn handle_error(
        &self,
        error: sweet_async_api::task::AsyncTaskError,
    ) -> Result<T, sweet_async_api::task::AsyncTaskError> {
        Err(error)
    }

    fn record_error(&self, error: &sweet_async_api::task::AsyncTaskError) {
        tracing::error!("Recording error: {:?}", error);
    }

    fn is_tracing_enabled(&self) -> bool {
        false
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::TimedTask<T> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    fn created_timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }

    fn executed_timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }

    fn completed_timestamp(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }

    fn timeout(&self) -> std::time::Duration {
        self.task_timeout
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::ContextualizedTask<T, I> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    type RuntimeType = crate::runtime::TokioRuntime;

    fn child_tasks(&self) -> Vec<T> {
        Vec::new()
    }

    fn parent(&self) -> Option<T> {
        None
    }

    fn runtime(&self) -> &Self::RuntimeType {
        panic!("ContextualizedTask::runtime is not directly available")
    }

    fn cwd(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::StatusEnabledTask<T> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    fn status(&self) -> sweet_async_api::task::TaskStatus {
        if self.is_complete() {
            sweet_async_api::task::TaskStatus::Completed
        } else {
            sweet_async_api::task::TaskStatus::Running
        }
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::RecoverableTask<T> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    fn recover(
        &self,
        error: sweet_async_api::task::AsyncTaskError,
    ) -> Result<T, sweet_async_api::task::AsyncTaskError> {
        Err(error)
    }

    fn can_recover_from(&self, _error: &sweet_async_api::task::AsyncTaskError) -> bool {
        false
    }

    fn fallback_value(&self) -> Option<T> {
        None
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, EOverall: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::MetricsEnabledTask<T> for TokioEmittingTask<T, C, EItem, EOverall, I>
{
    type Cpu = crate::task::async_task::TaskMetrics;
    type Memory = crate::task::async_task::TaskMetrics;
    type Io = crate::task::async_task::TaskMetrics;

    fn cpu_usage(&self) -> &Self::Cpu {
        &self.metrics
    }

    fn memory_usage(&self) -> &Self::Memory {
        &self.metrics
    }

    fn io_usage(&self) -> &Self::Io {
        &self.metrics
    }
}

impl<T: Clone + Send + 'static, C: Clone + Send + 'static, EItem: Send + Sync + 'static, I: TaskId>
    sweet_async_api::task::emit::builder::ReceiverBuilder<T, C, EItem, I> for TokioReceiverBuilder<T, C, EItem, I>
{
    type Task = TokioEmittingTask<T, C, EItem, EItem, I>;

    fn run(self) -> Self::Task {
        let task_id_str = self.base_builder.get_name().unwrap_or_else(|| {
            format!(
                "emitting-task-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )
        });
        let task_id = I::from_string(&task_id_str).unwrap_or_else(|| {
            let fallback_id_str = format!("fallback-emitting-task-{}", uuid::Uuid::new_v4());
            I::from_string(&fallback_id_str).expect("Failed to create TaskId for emitting task")
        });

        // Convert receiver work to a function that returns Result<C, EItem>
        let receiver_work_fn = |t: &T, _collector: &mut (), _uuid: Uuid| -> Result<C, EItem> {
            // For now, we'll assume the receiver work always succeeds
            // In a real implementation, this would need proper error handling
            Ok(t.clone() as C) // This assumes T can be converted to C
        };

        TokioEmittingTask::new(
            task_id,
            self.priority,
            Box::new(|_| Box::pin(async move {
                let (tx, rx) = tokio::sync::mpsc::channel(100);
                // The sender work should populate this channel
                rx
            })),
            self.sender_strategy,
            receiver_work_fn,
            self.receiver_strategy,
            self.runtime.clone(),
            self.base_builder.get_timeout(),
            self.base_builder.get_retry_attempts(),
            self.base_builder.is_tracing_enabled(),
            self.active_tasks,
            self.base_builder.get_name(),
        )
    }

    async fn await_result(
        self,
    ) -> (C, <Self::Task as EmittingTask<T, C, EItem, I>>::Final) {
        let task = self.run();
        let final_event = task.await_final_event().await.unwrap();
        // Extract first successful result as the main result
        let first_result = final_event
            .collected_items
            .values()
            .find_map(|r| r.as_ref().ok().cloned())
            .unwrap();
        (first_result, final_event)
    }
}

// Helper function for metrics - replace with actual logic
fn crate_shared_metrics_instance() -> crate::task::async_task::TaskMetrics {
    crate::task::async_task::TaskMetrics::new()
}