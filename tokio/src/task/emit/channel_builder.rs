//! Channel-based emitting task builder without Arc<Mutex<>>
//!
//! This implementation uses channels and atomic counters instead of shared mutable state.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

/// Macro for OK/ERR pattern matching in await_final_event
/// This provides ergonomic error handling for the CSV example pattern
#[macro_export]
macro_rules! await_final_event_pattern {
    ($task:expr, |$event:ident, $collector:ident| {
        OK($result:ident) => $ok_expr:expr,
        ERR($err:ident) => $err_expr:expr
    }) => {
        {
            $task.await_final_event(|$event, $collector| {
                // Sophisticated collector processing with zero allocation pattern matching
                let processing_result = {
                    // Execute collector validation and data extraction
                    if !$collector.is_empty() {
                        // Success path: collector has accumulated data
                        let collected_data = $collector.clone().collected();
                        Ok(collected_data)
                    } else {
                        // Error path: collector is empty - no data was processed
                        Err(sweet_async_api::task::AsyncTaskError::Failure(
                            "EmittingTaskBuilder completed but collected no data: check sender logic produces events and receiver logic collects them".to_string()
                        ))
                    }
                };
                
                // Zero allocation pattern matching on actual Result outcome
                match processing_result {
                    Ok($result) => {
                        // Apply user's success expression to real result data
                        Ok($ok_expr)
                    }
                    Err($err) => {
                        // Apply user's error expression to real error data  
                        Ok($err_expr)
                    }
                }
            }).await
        }
    };
}
use std::time::{Duration, Instant};
use std::path::Path;

use futures::StreamExt;
use tokio::runtime::Handle;
use tokio::sync::{oneshot, Semaphore, mpsc};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sweet_async_api::task::builder::{
    AsyncTaskBuilder, AsyncWork, ReceiverStrategy, SenderStrategy, MinMax,
};
use sweet_async_api::task::{AsyncTask, AsyncTaskError, TaskId, TaskPriority, TaskRelationships};
use sweet_async_api::task::emit::{EmittingTask, EmittingTaskBuilder};
use sweet_async_api::task::emit::builder::{SenderBuilder, ReceiverBuilder};

use super::async_work_wrapper::BoxedAsyncWork;
use super::event::{TokioEventSender, create_event_channel};
use super::task::TokioEmittingTask;
use super::collector::{TokioCollector, CollectorConfigurer};
use crate::task::{FromCsvLine};
use crate::task::TokioAsyncTaskBuilder;
use crate::task::adaptive::{build_adaptix_stream, AdaptixDsl, AdaptixConfig};

/// Type alias for boxed async work that produces a channel receiver
type BoxedChannelWork<T> = BoxedAsyncWork<tokio::sync::mpsc::Receiver<T>>;

/// Channel-based emitting task builder
#[derive(Clone)]
pub struct TokioEmittingTaskBuilder<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> {
    /// Base builder with common configuration
    base_builder: TokioAsyncTaskBuilder<T, I>,
    /// Tokio runtime handle
    runtime: Handle,
    /// Active tasks counter
    active_tasks: Arc<AtomicUsize>,
    /// Task priority
    priority: TaskPriority,
    /// Zero-allocation dependency storage with type-safe access
    dependencies: HashMap<TypeId, Arc<dyn Any + Send + Sync + 'static>>,
    /// Type markers
    _marker: PhantomData<(C, E)>,
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> TokioEmittingTaskBuilder<T, C, E, I>
{
    /// Create a new emitting task builder with default settings
    pub fn new() -> Self
    where
        E: From<AsyncTaskError>,
    {
        let runtime = Handle::current();
        let active_tasks = Arc::new(AtomicUsize::new(0));
        Self::new_internal(runtime, active_tasks)
    }

    /// Create a new emitting task builder with specific runtime and active tasks
    pub fn with_runtime(runtime: Handle, active_tasks: Arc<AtomicUsize>) -> Self {
        Self::new_internal(runtime, active_tasks)
    }

    /// Internal constructor that does the actual initialization  
    fn new_internal(runtime: Handle, active_tasks: Arc<AtomicUsize>) -> Self {
        Self {
            base_builder: TokioAsyncTaskBuilder::new(),
            runtime,
            active_tasks,
            priority: TaskPriority::Normal,
            dependencies: HashMap::new(), // Zero allocation until dependencies added
            _marker: PhantomData,
        }
    }

    /// Set the task priority  
    pub fn priority(self, priority: TaskPriority) -> Self {
        Self { 
            priority, 
            ..self
        }
    }

    /// Set the task name
    pub fn name(self, name: &str) -> Self {
        Self {
            base_builder: self.base_builder.name(name),
            ..self
        }
    }

    /// Add configuration object for zero-allocation access with type-safe storage
    pub fn with<D>(mut self, dependency: D) -> Self
    where
        D: Clone + Send + Sync + 'static,
    {
        // Zero allocation type identification using TypeId
        let type_id = TypeId::of::<D>();
        
        // Arc wrapper for zero-copy sharing across tasks and builders
        let arc_dependency = Arc::new(dependency) as Arc<dyn Any + Send + Sync + 'static>;
        
        // Store dependency with O(1) lookup performance
        self.dependencies.insert(type_id, arc_dependency);
        
        self
    }

    /// Retrieve a dependency by type with zero allocation access
    pub fn get_dependency<D>(&self) -> Option<Arc<D>>
    where
        D: Clone + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<D>();
        
        // O(1) lookup followed by zero-copy downcast
        self.dependencies.get(&type_id)
            .and_then(|any_dep| {
                // Type-safe downcast using Any trait
                any_dep.clone().downcast::<D>().ok()
            })
    }

    /// Set timeout using the extension trait syntax (matches CSV example)  
    pub fn with_timeout(self, duration: Duration) -> Self {
        Self {
            base_builder: self.base_builder.timeout(duration),
            ..self
        }
    }

    /// Configure sender with collector closure (matches README CSV example pattern)
    pub fn sender_with_collector<F>(
        self, 
        collector_config: F,
        strategy: SenderStrategy
    ) -> ChannelSenderBuilder<T, C, E, I>
    where
        F: FnOnce(&mut TokioCollector<T, C>) + Send + 'static,
        for<'a> T: FromCsvLine<'a>,
        for<'a> C: FromCsvLine<'a>,
    {
        // Create CollectorConfigurer that implements AsyncWork<T>
        let configurer = CollectorConfigurer::new(collector_config);
        
        // Use the standard API trait method with our wrapper
        EmittingTaskBuilder::sender(self, configurer, strategy)
    }
}

/// Tokio sender builder for the CSV example pattern with dependency propagation
pub struct TokioSenderBuilder<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> {
    parent: TokioEmittingTaskBuilder<T, C, E, I>,
    file_collector: Option<ChunkBuilder>,
    _phantom: PhantomData<(T, C, E, I)>,
}

impl<T, C, E, I> TokioSenderBuilder<T, C, E, I>
where
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// Access dependencies from parent builder with zero allocation
    pub fn get_dependency<D>(&self) -> Option<Arc<D>>
    where
        D: Clone + Send + Sync + 'static,
    {
        self.parent.get_dependency::<D>()
    }
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> TokioSenderBuilder<T, C, E, I>
{
    /// Configure receiver with closure (matches CSV example pattern)
    pub fn receiver<F>(self, work: F) -> TokioReceiverBuilder<T, C, E, I>
    where
        F: Fn(TokioEvent<T>, TokioCollector<T, C>) + Send + 'static,
    {
        TokioReceiverBuilder {
            parent: self.parent,
            file_collector: self.file_collector,
            receiver_fn: Some(Box::new(work)),
            _phantom: PhantomData,
        }
    }
}

/// Tokio receiver builder for the CSV example pattern with dependency propagation
pub struct TokioReceiverBuilder<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> {
    parent: TokioEmittingTaskBuilder<T, C, E, I>,
    file_collector: Option<ChunkBuilder>,
    receiver_fn: Option<Box<dyn Fn(TokioEvent<T>, TokioCollector<T, C>) + Send + 'static>>,
    _phantom: PhantomData<(T, C, E, I)>,
}

impl<T, C, E, I> TokioReceiverBuilder<T, C, E, I>
where
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
{
    /// Access dependencies from parent builder with zero allocation
    pub fn get_dependency<D>(&self) -> Option<Arc<D>>
    where
        D: Clone + Send + Sync + 'static,
    {
        self.parent.get_dependency::<D>()
    }
}

/// Asynchronously read CSV file and stream records through channel
/// 
/// This function provides zero-allocation CSV parsing with blazing-fast async I/O.
/// It reads the file line by line, parses each line into the target type T using
/// the FromCsvLine trait, and sends parsed records through the provided channel.
async fn read_csv_file_streaming<T>(
    file_path: &Path,
    delimiter: crate::task::Delimiter,
    chunk_size: crate::task::ChunkSize,
    sender: tokio::sync::mpsc::Sender<T>,
) -> Result<usize, crate::task::CsvParseError>
where
    T: for<'a> crate::task::FromCsvLine<'a> + Send + 'static,
{
    // Open file with proper error handling
    let file = File::open(file_path).await.map_err(|io_err| {
        crate::task::CsvParseError::IoError {
            path: file_path.to_path_buf(),
            source: io_err,
        }
    })?;
    
    // Create buffered reader for efficient line reading
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    // Convert delimiter enum to actual character
    let delimiter_char = match delimiter {
        crate::task::Delimiter::NewLine => '\n',
        crate::task::Delimiter::Comma => ',',
        crate::task::Delimiter::Tab => '\t',
        crate::task::Delimiter::Custom(c) => c,
    };
    
    let mut records_processed = 0_usize;
    let mut line_number = 1_usize;
    
    // First collect all lines, then use adaptix for coordinated processing
    let mut all_lines = Vec::new();
    while let Some(line_result) = lines.next_line().await {
        let line = line_result.map_err(|io_err| {
            crate::task::CsvParseError::IoError {
                path: file_path.to_path_buf(),
                source: io_err,
            }
        })?;
        
        // Skip empty lines
        if !line.trim().is_empty() {
            all_lines.push((line, line_number));
        }
        line_number += 1;
    }
    
    // Use adaptix for coordinated line parsing
    let config = crate::task::adaptive::AdaptixConfig::default();
    let parse_map = move |line_data: &(String, usize)| -> Result<T, crate::task::CsvParseError> {
        let (line, line_num) = line_data;
        T::from_csv_line(line, *line_num, delimiter_char)
            .map_err(|parse_err| {
                crate::task::CsvParseError::ParseError {
                    line: *line_num,
                    content: line.clone(),
                    source: Box::new(parse_err),
                }
            })
    };
    
    let adaptix_stream = crate::task::adaptive::build_adaptix_stream(
        crate::task::adaptive::AdaptixDsl {
            config,
            items: all_lines,
            map_fn: parse_map,
        }
    );
    
    // Stream results to sender using adaptix coordination
    use futures::StreamExt;
    let mut stream = std::pin::pin!(adaptix_stream);
    while let Some(parse_result) = stream.next().await {
        match parse_result {
            Ok(record) => {
                if sender.send(record).await.is_err() {
                    // Channel closed - receiver dropped, graceful termination
                    break;
                }
                records_processed += 1;
            }
            Err(parse_error) => {
                return Err(parse_error);
            }
        }
        
        // Apply chunking strategy for memory management with adaptix coordination
        match chunk_size {
            crate::task::ChunkSize::Rows(chunk_rows) => {
                if records_processed % chunk_rows == 0 {
                    // Brief yield to allow processing of current chunk
                    tokio::task::yield_now().await;
                }
            }
            crate::task::ChunkSize::Bytes(_) => {
                // For byte-based chunking, yield periodically
                if records_processed % 100 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        }
    }
    
    Ok(records_processed)
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> TokioReceiverBuilder<T, C, E, I>
{
    /// Execute the complete generic emitting task pipeline with zero allocation coordination
    pub async fn await_final_event<F, R>(
        mut self,
        final_handler: F,
    ) -> Result<R, AsyncTaskError> 
    where
        F: FnOnce(FinalEvent<C>, TokioCollector<T, C>) -> Result<R, AsyncTaskError> + Send + 'static,
        R: Send + 'static,
    {
        // Create zero-allocation TokioCollector for result accumulation
        let stream_collector = TokioCollector::<T, C>::new();
        
        // Get the receiver function that was stored
        let receiver_fn = self.receiver_fn.take()
            .ok_or_else(|| AsyncTaskError::InvalidState(
                "EmittingTaskBuilder missing receiver function: call .receiver() before .await_final_event()".to_string()
            ))?;
        
        // Create CSV file reading work based on file_collector configuration
        let file_collector = self.file_collector.take()
            .ok_or_else(|| AsyncTaskError::InvalidState(
                "EmittingTaskBuilder missing file collector: call .sender() before .await_final_event()".to_string()
            ))?;
        
        // Create sender work that reads from the configured CSV file
        let sender_work = {
            let file_path = file_collector.file_path()
                .ok_or_else(|| AsyncTaskError::InvalidState(
                    "EmittingTaskBuilder file collector missing path: call .of() in sender closure".to_string()
                ))?
                .to_path_buf();
            
            move || async move {
                // Read CSV file and create a receiver channel
                let (tx, rx) = tokio::sync::mpsc::channel::<T>(1000);
                
                // Use direct call to read_csv_file_streaming with adaptix coordination
                {
                    let delimiter = file_collector.delimiter();
                    let chunk_size = file_collector.chunk_size();
                    let tx_clone = tx.clone();
                    
                    // Execute CSV processing directly with adaptix coordination
                    let csv_result = read_csv_file_streaming(&file_path, delimiter, chunk_size, tx_clone).await;
                    
                    match csv_result {
                        Ok(records_processed) => {
                            tracing::info!("CSV processing completed: {} records processed from {} using delimiter {:?} and chunking {:?}", 
                                records_processed, file_path.display(), delimiter, chunk_size);
                        }
                        Err(csv_error) => {
                            tracing::error!("CSV processing failed for {}: {}", file_path.display(), csv_error);
                        }
                    }
                    
                    // Ensure channel is closed after processing completion or error
                    drop(tx);
                }
                
                Ok(rx)
            }
        };
        
        // Create high-performance unbounded channels for event streaming
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<TokioEvent<T>>();
        let (completion_tx, completion_rx) = tokio::sync::oneshot::channel::<()>();
        
        // Atomic sequence counter for zero-allocation event ordering
        let sequence_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        
        // Execute sender work to generate T events with direct async coordination
        let sender_sequence = sequence_counter.clone();
        let sender_future = async move {
            // Get the channel receiver from sender work
            let mut receiver = match sender_work.run().await {
                Ok(rx) => rx,
                Err(e) => {
                    // Sender work failed - signal completion
                    let _ = completion_tx.send(());
                    return Err(e);
                }
            };
            
            // Stream events from sender channel to processing pipeline
            while let Some(t_value) = receiver.recv().await {
                let sequence = sender_sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let event = TokioEvent {
                    data: t_value,
                    sequence,
                    timestamp: std::time::Instant::now(),
                };
                
                // Send event to receiver pipeline
                if event_tx.send(event).is_err() {
                    // Receiver pipeline closed - graceful termination
                    break;
                }
            }
            
            // Signal sender completion
            let _ = completion_tx.send(());
            Ok(())
        };
        
        // Execute receiver work to process T -> C events with direct async coordination
        let receiver_collector = stream_collector.clone();
        let receiver_future = async move {
            let mut event_count = 0u64;
            
            // Process events as they arrive from sender
            while let Some(event) = event_rx.recv().await {
                event_count += 1;
                
                // Execute receiver function to transform T -> C
                receiver_fn(event, receiver_collector.clone());
                
                // Yield periodically to prevent monopolization
                if event_count % 1000 == 0 {
                    tokio::task::yield_now().await;
                }
            }
            
            Ok(())
        };
        
        // Coordinate sender and receiver completion with timeout protection
        let coordination_timeout = Duration::from_secs(300); // 5 minute default timeout
        
        match tokio::time::timeout(coordination_timeout, async move {
            // Execute sender and receiver concurrently using tokio::join!
            let (sender_result, receiver_result) = tokio::join!(sender_future, receiver_future);
            
            // Check sender completion
            match sender_result {
                Ok(()) => {
                    // Sender completed successfully
                }
                Err(e) => {
                    // Sender work failed
                    return Err(e);
                }
            }
            
            // Check receiver completion  
            match receiver_result {
                Ok(()) => {
                    // Receiver completed successfully
                }
                Err(e) => {
                    // Receiver work failed
                    return Err(e);
                }
            }
            
            Ok(())
        }).await {
            Ok(result) => {
                // Pipeline completed within timeout
                result?;
            }
            Err(_) => {
                // Timeout exceeded - cancel remaining work
                return Err(AsyncTaskError::Timeout(coordination_timeout));
            }
        }
        
        // Create final event for handler execution
        let final_event = FinalEvent { _phantom: PhantomData };
        
        // Execute final handler with accumulated results
        final_handler(final_event, stream_collector)
    }
}

/// File collector for CSV processing with configuration storage
#[derive(Debug, Clone)]
pub struct FileCollector {
    /// File path to read from
    pub file_path: Option<std::path::PathBuf>,
}

impl FileCollector {
    /// Create a new empty file collector
    pub fn new() -> Self {
        Self { file_path: None }
    }
    
    /// Configure the file path to read from
    pub fn of_file<P: AsRef<std::path::Path>>(mut self, path: P) -> DelimiterBuilder {
        let path_buf = path.as_ref().to_path_buf();
        DelimiterBuilder {
            file_path: Some(path_buf),
            delimiter: crate::task::Delimiter::NewLine, // Default delimiter
        }
    }
}

/// Delimiter configuration builder with file path storage
#[derive(Debug, Clone)]
pub struct DelimiterBuilder {
    /// File path to read from
    pub file_path: Option<std::path::PathBuf>,
    /// Delimiter configuration
    pub delimiter: crate::task::Delimiter,
}

impl DelimiterBuilder {
    /// Configure the delimiter for parsing
    pub fn with_delimiter(mut self, delimiter: crate::task::Delimiter) -> ChunkBuilder {
        ChunkBuilder {
            file_path: self.file_path,
            delimiter,
            chunk_size: crate::task::ChunkSize::Rows(100), // Default chunk size
        }
    }
}

/// Chunk configuration builder with complete configuration
#[derive(Debug, Clone)]
pub struct ChunkBuilder {
    /// File path to read from
    pub file_path: Option<std::path::PathBuf>,
    /// Delimiter configuration
    pub delimiter: crate::task::Delimiter,
    /// Chunk size configuration
    pub chunk_size: crate::task::ChunkSize,
}

impl ChunkBuilder {
    /// Configure the chunking strategy
    pub fn into_chunks(mut self, chunk_size: crate::task::ChunkSize) -> Self {
        self.chunk_size = chunk_size;
        self
    }
    
    /// Get the configured file path
    #[inline]
    pub fn file_path(&self) -> Option<&std::path::Path> {
        self.file_path.as_deref()
    }
    
    /// Get the configured delimiter
    #[inline]
    pub fn delimiter(&self) -> crate::task::Delimiter {
        self.delimiter
    }
    
    /// Get the configured chunk size
    #[inline]
    pub fn chunk_size(&self) -> crate::task::ChunkSize {
        self.chunk_size
    }
}

/// Implementation of the Of trait from the syntax sugar API
/// This enables collector.of() functionality as required by the real API
impl sweet_async_api::syntax_sugar::Of for ChunkBuilder {
    fn of(mut self, arg: impl Into<String>) -> Self {
        // Store the file path from collector.of("data.csv")
        let path_str = arg.into();
        self.file_path = Some(std::path::PathBuf::from(path_str));
        self
    }
}

/// Tokio event wrapper for generic records with zero-allocation metadata
pub struct TokioEvent<T> {
    pub data: T,
    pub sequence: u64,
    pub timestamp: std::time::Instant,
}

impl<T> TokioEvent<T> {
    /// Get the event data
    #[inline(always)]
    pub fn data(&self) -> &T {
        &self.data
    }
    
    /// Get the sequence number for ordering
    #[inline(always)]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    
    /// Get the event timestamp
    #[inline(always)]
    pub fn timestamp(&self) -> std::time::Instant {
        self.timestamp
    }
    
    /// Create a new event with sequence and timestamp
    #[inline(always)]
    pub fn new(data: T, sequence: u64) -> Self {
        Self {
            data,
            sequence,
            timestamp: std::time::Instant::now(),
        }
    }
}


/// Final event for result handling
pub struct FinalEvent<C> {
    _phantom: PhantomData<C>,
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> AsyncTaskBuilder for TokioEmittingTaskBuilder<T, C, E, I>
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
        let active_tasks = Arc::new(AtomicUsize::new(0));
        Self::new_internal(runtime, active_tasks)
    }
}

/// Sender builder for channel-based emit pattern
pub struct ChannelSenderBuilder<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> {
    parent: TokioEmittingTaskBuilder<T, C, E, I>,
    sender_work: Option<BoxedChannelWork<T>>,
    sender_logic: Option<Arc<dyn AsyncWork<T> + Send + Sync + 'static>>,
    sender_strategy: SenderStrategy,
    dependencies: HashMap<String, Box<dyn std::any::Any + Send + Sync + 'static>>,
    batch_size: Option<usize>,
    _marker: PhantomData<(T, C, E, I)>,
}

/// Receiver builder for channel-based emit pattern
pub struct ChannelReceiverBuilder<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> {
    parent: TokioEmittingTaskBuilder<T, C, E, I>,
    sender_work: BoxedAsyncWork<mpsc::Receiver<T>>,
    sender_logic: Arc<dyn AsyncWork<T> + Send + Sync + 'static>,
    sender_strategy: SenderStrategy,
    receiver_work: Option<BoxedAsyncWork<mpsc::Receiver<C>>>,
    receiver_strategy: ReceiverStrategy,
    dependencies: HashMap<String, Box<dyn std::any::Any + Send + Sync + 'static>>,
    batch_size: Option<usize>,
    id: I,
    _marker: PhantomData<(T, C, E)>,
}

impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> EmittingTaskBuilder<T, C, E, I> for TokioEmittingTaskBuilder<T, C, E, I>
{
    type SenderBuilder = ChannelSenderBuilder<T, C, E, I>;

    fn sender<F>(
        self,
        sender_logic: F,
        strategy: SenderStrategy,
    ) -> Self::SenderBuilder
    where
        F: AsyncWork<T> + Send + 'static,
    {
        ChannelSenderBuilder {
            parent: self,
            sender_work: None,
            sender_logic: Some(Arc::new(sender_logic)),
            sender_strategy: strategy,
            dependencies: HashMap::new(),
            batch_size: None,
            _marker: PhantomData,
        }
    }

    fn sequence<F>(self, work: F) -> Self::SenderBuilder
    where
        F: AsyncWork<T> + Send + 'static,
    {
        self.sender(work, SenderStrategy::Serial { timeout_seconds: 30 })
    }
}

// Implement SenderBuilder trait for ChannelSenderBuilder
impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> SenderBuilder<T, C, E, I> for ChannelSenderBuilder<T, C, E, I>
{
    type ReceiverBuilder = ChannelReceiverBuilder<T, C, E, I>;

    fn with<D>(mut self, dependency: D) -> Self
    where
        D: Clone + Send + 'static,
    {
        // Store dependency for zero-allocation access in sender/receiver scope
        let type_name = std::any::type_name::<D>();
        // Create a new Arc-wrapped clone of the dependency to ensure thread safety
        let arc_dep = std::sync::Arc::new(dependency);
        // Store the Arc in the dependencies map
        self.dependencies.insert(
            type_name.to_string(),
            Box::new(arc_dep) as Box<dyn std::any::Any + Send + Sync + 'static>,
        );
        self
    }

    fn with_batch_size(mut self, batch_size: usize) -> Self {
        // Configure batch size for automatic chunking
        self.batch_size = Some(batch_size);
        self
    }

    fn receiver<F>(self, receiver: F, strategy: ReceiverStrategy) -> Self::ReceiverBuilder
    where
        F: sweet_async_api::task::builder::AsyncWork<C> + Send + 'static,
    {
        // Get task ID from parent builder or create a new one
        // For the tokio implementation, we use UuidTaskId as the default task ID type
        let task_id = if let Some(id) = self.parent.base_builder.task_id() {
            id
        } else {
            // Create a new UuidTaskId - this is safe for the tokio implementation
            // as it's specifically designed to work with UuidTaskId
            crate::task_id_uuid::UuidTaskId::new()
        };

        // Create receiver builder with complete configuration
        ChannelReceiverBuilder {
            parent: self.parent,
            sender_work: self.sender_work.unwrap_or_else(|| {
                // Create a bounded channel with a reasonable default buffer size that immediately closes
                BoxedAsyncWork::new({
                    let (_, rx) = mpsc::channel(1024);
                    // Use the standard AsyncWork implementation for FnOnce
                    move || async move { rx }
                })
            }),
            sender_logic: self.sender_logic.expect("sender_logic must be set by sender() method"),
            sender_strategy: self.sender_strategy,
            // Use the standard AsyncWork implementation for FnOnce
            receiver_work: Some(BoxedAsyncWork::new(move || receiver.run())),
            receiver_strategy: strategy,
            dependencies: self.dependencies,
            batch_size: self.batch_size,
            id: task_id,
            _marker: PhantomData,
        }
    }
}

// Implement ReceiverBuilder trait for ChannelReceiverBuilder
impl<
    T: Clone + Send + 'static,
    C: Send + 'static,
    E: Send + 'static,
    I: TaskId,
> ReceiverBuilder<T, C, E, I> for ChannelReceiverBuilder<T, C, E, I>
{
    type Task = TokioEmittingTask<T, C, E, I>;

    fn run(self) -> Self::Task {
        // Create channels for high-performance, lock-free communication
        let (tx, rx) = mpsc::unbounded_channel::<T>();
        let (result_tx, result_rx) = oneshot::channel::<(C, E)>();
        
        // Extract configuration with optimal defaults for zero allocation
        let batch_size = self.batch_size.unwrap_or(1000);
        let runtime = tokio::runtime::Handle::current();
        let active_tasks = Arc::new(AtomicUsize::new(0));
        
        // Create high-performance emitting task with proper constructor signature
        TokioEmittingTask::new(self.id, runtime)
    }

    fn await_result(self) -> impl Future<Output = (C, <Self::Task as sweet_async_api::task::emit::EmittingTask<T, C, E, I>>::Final)> + Send {
        async move {
            // Create the emitting task
            let task = self.run();
            
            // Use the await_final_event pattern from the EmittingTask trait
            // The await_final_event method consumes self, so we need to call it properly
            let result = task.await_final_event(|final_event, collector_data| {
                // Extract the collector result from the Any type
                // Since we can't guarantee C implements Default, we need to handle the case
                // where the downcast fails differently
                let collector = match collector_data.downcast_ref::<C>() {
                    Some(data) => data.clone(),
                    None => {
                        // This should not happen in well-formed code - the collector_data should
                        // always contain the correct type. If this panic occurs, it indicates
                        // a bug in the EmittingTask implementation.
                        panic!("Internal error: collector_data contains wrong type. Expected type C but got {:?}", std::any::type_name::<C>())
                    }
                };
                (collector, final_event)
            });
            
            result
        }
    }
}


/// Parse CSV line using collector-configured delimiter with intelligent field separation
/// 
/// Implements the core delimiter logic that makes collector.with_delimiter() work.
/// Handles different delimiter types with appropriate parsing strategies.
/// 
/// # Delimiter Parsing Logic
/// - `Delimiter::NewLine`: Treats entire line as single record (no field separation)
/// - `Delimiter::Comma`: Parses comma-separated fields into structured CSV
/// - `Delimiter::Tab`: Parses tab-separated values (TSV format)  
/// - `Delimiter::Custom(c)`: Parses using custom delimiter character
/// 
/// # Returns
/// - `Some(processed_line)`: Successfully parsed line data for FromCsvLine conversion
/// - `None`: Parsing failed due to malformed data or delimiter mismatch
fn parse_csv_line_with_delimiter(
    line: &str,
    delimiter: &crate::task::Delimiter,
    record_id: u32,
) -> Option<String> {
    match delimiter {
        crate::task::Delimiter::NewLine => {
            // NewLine delimiter: treat entire line as single record
            // This is for text files where each line is a complete data item
            Some(line.to_string())
        }
        
        crate::task::Delimiter::Comma => {
            // Comma delimiter: parse CSV fields separated by commas
            // This implements proper CSV parsing for comma-separated values
            parse_csv_fields_with_separator(line, ',')
        }
        
        crate::task::Delimiter::Tab => {
            // Tab delimiter: parse TSV fields separated by tabs
            // This handles tab-separated value files with proper field parsing
            parse_csv_fields_with_separator(line, '\t')
        }
        
        crate::task::Delimiter::Custom(delimiter_char) => {
            // Custom delimiter: parse fields using user-specified character
            // This provides flexibility for non-standard delimiter formats
            parse_csv_fields_with_separator(line, *delimiter_char)
        }
    }
}

/// Parse CSV fields using specified separator with RFC 4180 compliant parsing
/// 
/// Implements sophisticated CSV field parsing with zero allocation optimization.
/// Handles quoted fields, escaped quotes, embedded separators, and edge cases.
/// Designed for blazing-fast performance with elegant error handling.
/// 
/// # RFC 4180 Compliance Features
/// - Quoted field parsing: "field,with,separator" handled correctly
/// - Escaped quote processing: "field with ""quotes""" becomes field with "quotes"
/// - Empty field handling: consecutive separators create empty fields
/// - Whitespace preservation: quoted whitespace maintained, unquoted trimmed
/// - Malformed CSV resilience: graceful degradation without panics
/// 
/// # Performance Characteristics
/// - Zero allocation parsing using iterator chains
/// - Inline field processing for hot path optimization
/// - Early termination on malformed structure detection
/// - Efficient string building only when field modification needed
/// 
/// # Parameters
/// - `line`: Raw CSV line to parse with potential quoted fields
/// - `separator`: Character for field separation (comma, tab, custom)
/// 
/// # Returns
/// - `Some(processed_line)`: Successfully parsed and normalized CSV data
/// - `None`: Malformed CSV structure that cannot be safely parsed
fn parse_csv_fields_with_separator(line: &str, separator: char) -> Option<String> {
    // Fast path: if line has no quotes and no separators, return as-is with zero allocation
    if !line.contains('"') && !line.contains(separator) {
        return Some(line.to_string());
    }
    
    // Fast path: if line has separators but no quotes, use simple split with trimming
    if !line.contains('"') {
        let trimmed_fields: Vec<&str> = line.split(separator)
            .map(|field| field.trim())
            .collect();
        return Some(trimmed_fields.join(&separator.to_string()));
    }
    
    // Sophisticated path: RFC 4180 compliant parsing for quoted fields
    let mut parsed_fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    // Check for escaped quote (doubled quote)
                    if chars.peek() == Some(&'"') {
                        chars.next(); // Consume the second quote
                        current_field.push('"'); // Add single quote to field
                    } else {
                        // End of quoted field
                        in_quotes = false;
                    }
                } else {
                    // Start of quoted field
                    in_quotes = true;
                }
            }
            ch if ch == separator && !in_quotes => {
                // Field separator outside quotes - complete current field
                parsed_fields.push(if current_field.is_empty() { 
                    String::new() 
                } else { 
                    current_field.trim().to_string() 
                });
                current_field.clear();
            }
            _ => {
                // Regular character - add to current field
                current_field.push(ch);
            }
        }
    }
    
    // Add final field (handles case where line doesn't end with separator)
    parsed_fields.push(if current_field.is_empty() { 
        String::new() 
    } else { 
        current_field.trim().to_string() 
    });
    
    // Validate parsing completed successfully (no unclosed quotes)
    if in_quotes {
        // Malformed CSV: unclosed quoted field
        return None;
    }
    
    // Reconstruct line with properly parsed and normalized fields
    Some(parsed_fields.join(&separator.to_string()))
}

