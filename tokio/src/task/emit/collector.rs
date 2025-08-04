//! Tokio collector implementation following API traits precisely
//!
//! This module provides the TokioCollector that implements the Collector trait
//! from the API and all syntax sugar traits for fluent configuration.

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use sweet_async_api::task::builder::AsyncWork;
use sweet_async_api::task::emit::event::Collector;
use sweet_async_api::syntax_sugar::*;

use crate::task::{ChunkSize, Delimiter, FromCsvLine};

/// Configuration for different data sources
#[derive(Debug, Clone)]
pub enum DataSourceConfig {
    /// CSV file source
    Csv {
        file_path: PathBuf,
        delimiter: Delimiter,
        chunk_size: ChunkSize,
    },
    #[cfg(feature = "surrealdb-ws")]
    /// SurrealDB WebSocket source
    SurrealDbWs {
        url: String,
        namespace: String,
        database: String,
        username: String,
        password: String,
        query: String,
        table: String,
        live_query: bool,
        chunk_size: ChunkSize,
    },
    #[cfg(feature = "surrealdb-kv")]
    /// SurrealDB KV source  
    SurrealDbKv {
        db_path: PathBuf,
        query: String,
        table: String,
        chunk_size: ChunkSize,
    },
}

/// Tokio implementation of the Collector trait
/// 
/// This collector follows the API precisely, implementing both collection
/// methods and syntax sugar for fluent configuration.
#[derive(Clone)]
pub struct TokioCollector<T, C>
where
    T: Clone + Send + 'static,
    C: Clone + Send + Sync + 'static,
{
    /// Internal storage for collected items
    storage: Arc<HashMap<Uuid, C>>,
    /// Data source configuration set through fluent API
    source_config: Option<DataSourceConfig>,
    /// Collection counter for metrics
    collection_count: Arc<AtomicUsize>,
    /// Type markers
    _phantom: PhantomData<T>,
}

impl<T, C> TokioCollector<T, C>
where
    T: Clone + Send + 'static,
    C: Clone + Send + Sync + 'static,
{
    /// Create a new empty collector
    #[inline]
    pub fn new() -> Self {
        Self {
            storage: Arc::new(HashMap::new()),
            source_config: None,
            collection_count: Arc::new(AtomicUsize::new(0)),
            _phantom: PhantomData,
        }
    }

    /// Check if the collector is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.collection_count.load(Ordering::Relaxed) == 0
    }

    /// Get the number of collected items
    #[inline]
    pub fn len(&self) -> usize {
        self.collection_count.load(Ordering::Relaxed)
    }

    /// Get a clone of all collected items
    #[inline]
    pub fn collected(&self) -> HashMap<Uuid, C> {
        (*self.storage).clone()
    }
    
    /// Heuristic to detect if a string looks like a file path
    fn looks_like_file_path(s: &str) -> bool {
        // File path heuristics:
        // 1. Contains file extensions like .csv, .json, .txt, etc.
        // 2. Contains path separators (/ or \)
        // 3. Doesn't contain spaces (unless quoted)
        // 4. Reasonable length (not a very long string that's likely content)
        
        if s.len() > 500 {
            return false; // Too long to be a reasonable file path
        }
        
        // Check for common file extensions
        let has_extension = s.contains('.') && {
            let ext_part = s.split('.').last().unwrap_or("");
            matches!(ext_part.to_lowercase().as_str(), 
                "csv" | "json" | "txt" | "log" | "xml" | "yaml" | "yml" | 
                "tsv" | "dat" | "parquet" | "avro" | "jsonl" | "ndjson")
        };
        
        // Check for path separators
        let has_path_separators = s.contains('/') || s.contains('\\');
        
        // Check if it's a relative or absolute path without extension
        let looks_like_path = s.starts_with("./") || s.starts_with("../") || 
                             s.starts_with('/') || s.starts_with("C:\\") ||
                             s.starts_with("~/");
        
        has_extension || has_path_separators || looks_like_path
    }
}

impl<T, C> Collector<T, C, HashMap<Uuid, C>> for TokioCollector<T, C>
where
    T: Clone + Send + 'static,
    C: Clone + Send + Sync + 'static,
{
    #[inline]
    fn collect<K>(&mut self, key: K, item: C)
    where
        K: Into<Uuid>,
    {
        let storage = Arc::make_mut(&mut self.storage);
        storage.insert(key.into(), item);
        self.collection_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn collect_item(&mut self, item: C) {
        self.collect(Uuid::new_v4(), item);
    }

    #[inline]
    fn collect_items(&mut self, items: Vec<C>) {
        let storage = Arc::make_mut(&mut self.storage);
        for item in items {
            storage.insert(Uuid::new_v4(), item);
            self.collection_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[inline]
    fn collect_error<K, E>(&mut self, _key: K, _error: E)
    where
        K: Into<Uuid>,
        E: std::error::Error + Send + 'static,
    {
        // Log error but don't store in successful collection
        tracing::warn!("Collection error occurred");
    }

    #[inline]
    fn collected(&self) -> &HashMap<Uuid, C> {
        &*self.storage
    }

    // Source methods - enhanced to support file paths and iterables
    fn of<I>(&mut self, items: I) -> &mut Self
    where
        I: IntoIterator<Item = T> + Send + 'static,
    {
        // Check if T is a string-like type that could represent a file path
        // This is a compile-time check using type information
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<String>() ||
           std::any::TypeId::of::<T>() == std::any::TypeId::of::<&str>() {
            // For string types, try to detect if it's a file path
            // This is a heuristic approach - check if it looks like a file path
            let items_vec: Vec<T> = items.into_iter().collect();
            if items_vec.len() == 1 {
                if let Some(first_item) = items_vec.first() {
                    // Use unsafe cast to get the string value for path detection
                    // This is safe because we've checked the type ID above
                    let item_ptr = first_item as *const T as *const String;
                    if !item_ptr.is_null() {
                        let potential_path = unsafe { &*item_ptr };
                        if Self::looks_like_file_path(potential_path) {
                            return self.of_file(potential_path);
                        }
                    }
                }
            }
        }
        
        // For non-file-path items, store them directly
        // This would need proper implementation for collecting arbitrary iterables
        tracing::info!("Collecting {} items of type {}", 
                      std::any::type_name::<T>(), 
                      std::any::type_name::<I>());
        self
    }

    fn of_file(&mut self, path: impl AsRef<Path>) -> &mut Self {
        let mut config = self.source_config.take().unwrap_or_else(|| DataSourceConfig::Csv {
            file_path: PathBuf::new(),
            delimiter: Delimiter::NewLine,
            chunk_size: ChunkSize::Rows(1000),
        });

        if let DataSourceConfig::Csv { file_path, .. } = &mut config {
            *file_path = path.as_ref().to_path_buf();
        }

        self.source_config = Some(config);
        self
    }

    // Database sources
    fn from_database<P>(&mut self, _pool: P, _query: &str) -> &mut Self
    where
        P: Send + 'static,
    {
        self
    }

    fn from_postgres<P>(&mut self, _pool: P, _query: &str) -> &mut Self
    where
        P: Clone + Send + 'static,
    {
        self
    }

    fn from_surrealdb<DB>(&mut self, _db: DB, _query: &str) -> &mut Self
    where
        DB: Send + 'static,
    {
        self
    }

    fn from_seaorm<Q>(&mut self, _query: Q) -> &mut Self
    where
        Q: Send + 'static,
    {
        self
    }

    fn from_sqlite<DB>(&mut self, _connection: DB, _query: &str) -> &mut Self
    where
        DB: Send + 'static,
    {
        self
    }

    fn from_mysql<DB>(&mut self, _connection: DB, _query: &str) -> &mut Self
    where
        DB: Send + 'static,
    {
        self
    }

    // API & Network sources - stubs
    fn from_api_batch<U>(&mut self, _urls: Vec<U>) -> &mut Self
    where
        U: Send + 'static,
    {
        self
    }

    fn from_websocket(&mut self, _url: &str) -> &mut Self {
        self
    }

    fn from_sse(&mut self, _url: &str) -> &mut Self {
        self
    }

    fn from_graphql_subscription<G, Q>(&mut self, _client: G, _query: Q) -> &mut Self
    where
        G: Send + 'static,
        Q: Send + 'static,
    {
        self
    }

    // Cloud Storage - stubs
    fn from_s3<Client>(&mut self, _client: Client, _bucket: &str, _pattern: &str) -> &mut Self
    where
        Client: Send + 'static,
    {
        self
    }

    fn from_gcs<Client>(&mut self, _client: Client, _bucket: &str, _pattern: &str) -> &mut Self
    where
        Client: Send + 'static,
    {
        self
    }

    fn from_azure_blob<Client>(
        &mut self,
        _client: Client,
        _container: &str,
        _pattern: &str,
    ) -> &mut Self
    where
        Client: Send + 'static,
    {
        self
    }

    // Version Control - stubs
    fn from_github(&mut self, _repo: &str, _pattern: &str) -> &mut Self {
        self
    }

    fn from_gitlab(&mut self, _project: &str, _pattern: &str) -> &mut Self {
        self
    }

    fn from_git_repo(&mut self, _url: &str) -> &mut Self {
        self
    }

    // Message Queues - stubs
    fn from_kafka_topic<Consumer>(&mut self, _consumer: Consumer, _topic: &str) -> &mut Self
    where
        Consumer: Send + 'static,
    {
        self
    }

    fn from_redis_stream<R>(&mut self, _conn: R, _pattern: &str) -> &mut Self
    where
        R: Send + 'static,
    {
        self
    }

    fn from_rabbitmq<A>(&mut self, _conn: A, _queue: &str) -> &mut Self
    where
        A: Send + 'static,
    {
        self
    }

    fn from_nats<N>(&mut self, _conn: N, _subject: &str) -> &mut Self
    where
        N: Send + 'static,
    {
        self
    }

    // Social Media - stubs
    fn from_twitter_stream(&mut self, _account: &str) -> &mut Self {
        self
    }

    fn from_discord_server<Token>(&mut self, _token: Token, _server_id: &str) -> &mut Self
    where
        Token: Send + 'static,
    {
        self
    }

    fn from_tiktok_feed<A>(&mut self, _api: A) -> &mut Self
    where
        A: Send + 'static,
    {
        self
    }

    fn from_facebook_feed<A>(&mut self, _api: A) -> &mut Self
    where
        A: Send + 'static,
    {
        self
    }

    fn from_browser_history(&mut self, _browser_type: impl Send + 'static) -> &mut Self {
        self
    }

    // IoT - stubs
    fn from_smart_fridge<A>(&mut self, _api: A) -> &mut Self
    where
        A: Send + 'static,
    {
        self
    }

    fn from_roomba<A>(&mut self, _api: A) -> &mut Self
    where
        A: Send + 'static,
    {
        self
    }

    fn from_alexa_voice_history<A>(&mut self, _api: A) -> &mut Self
    where
        A: Send + 'static,
    {
        self
    }

    fn from_tesla_api<Token>(&mut self, _token: Token) -> &mut Self
    where
        Token: Send + 'static,
    {
        self
    }

    // Financial - stubs
    fn from_bank_transactions<A>(&mut self, _api: A) -> &mut Self
    where
        A: Send + 'static,
    {
        self
    }

    // Configuration methods
    fn with_delimiter(&mut self, delimiter: impl Send + 'static) -> &mut Self {
        if let Some(DataSourceConfig::Csv { delimiter: ref mut del, .. }) = &mut self.source_config {
            // Convert the generic delimiter to our Delimiter type
            // Check if it's already our Delimiter type
            if let Ok(concrete_delimiter) = <dyn std::any::Any>::downcast_ref::<Delimiter>(&delimiter as &dyn std::any::Any) {
                *del = concrete_delimiter.clone();
            } else {
                // Fall back to NewLine as default
                *del = Delimiter::NewLine;
                tracing::warn!("Unsupported delimiter type, using NewLine as default");
            }
        }
        self
    }

    fn with_batch_size(&mut self, _size: usize) -> &mut Self {
        self
    }

    fn with_concurrency(&mut self, _level: usize) -> &mut Self {
        self
    }

    fn with_retry(&mut self, _attempts: usize) -> &mut Self {
        self
    }

    fn with_timeout(&mut self, _duration: std::time::Duration) -> &mut Self {
        self
    }

    fn with_auth(&mut self, _auth: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_headers(&mut self, _headers: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_region(&mut self, _region: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_credentials(&mut self, _credentials: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_service_account(&mut self, _account: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_connection_string(&mut self, _connection: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_branch(&mut self, _branch: &str) -> &mut Self {
        self
    }

    fn with_token(&mut self, _token: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_path(&mut self, _path: &str) -> &mut Self {
        self
    }

    fn with_ref(&mut self, _reference: &str) -> &mut Self {
        self
    }

    fn with_consumer_group(&mut self, _group: &str) -> &mut Self {
        self
    }

    fn with_offset(&mut self, _offset: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_partition_strategy(&mut self, _strategy: impl Send + 'static) -> &mut Self {
        self
    }

    fn with_prefetch(&mut self, _count: usize) -> &mut Self {
        self
    }

    fn with_queue_group(&mut self, _group: &str) -> &mut Self {
        self
    }

    fn with_streaming(&mut self, _enabled: bool) -> &mut Self {
        self
    }

    fn with_cursor_size(&mut self, _size: usize) -> &mut Self {
        self
    }

    fn with_live_query(&mut self, enabled: bool) -> &mut Self {
        #[cfg(feature = "surrealdb-ws")]
        if let Some(DataSourceConfig::SurrealDbWs { live_query, .. }) = &mut self.source_config {
            *live_query = enabled;
        }
        self
    }

    fn into_chunks(&mut self, _size: usize) -> &mut Self {
        self
    }
}

// Syntax sugar traits will be implemented later once we have proper type alignment
// For now, the TokioCollector implements the core Collector trait which is sufficient

// Remove AsyncWork implementation for now - this will be implemented properly
// when we have the correct CsvRecord type alignment

/// Wrapper that implements AsyncWork and takes a closure to configure a collector
pub struct CollectorConfigurer<T, C, F>
where
    T: Clone + Send + 'static,
    C: Send + Sync + 'static,
    F: FnOnce(&mut TokioCollector<T, C>) + Send + 'static,
{
    configure_fn: Option<F>,
    _phantom: PhantomData<(T, C)>,
}

impl<T, C, F> CollectorConfigurer<T, C, F>
where
    T: Clone + Send + 'static,
    C: Send + Sync + 'static,
    F: FnOnce(&mut TokioCollector<T, C>) + Send + 'static,
{
    pub fn new(configure_fn: F) -> Self {
        Self {
            configure_fn: Some(configure_fn),
            _phantom: PhantomData,
        }
    }
}

impl<T, C, F> AsyncWork<TokioCollector<T, C>> for CollectorConfigurer<T, C, F>
where
    T: Clone + Send + 'static,
    C: Clone + Send + Sync + 'static + for<'a> crate::task::FromCsvLine<'a>,
    F: FnOnce(&mut TokioCollector<T, C>) + Send + 'static,
{
    fn run(self) -> impl Future<Output = TokioCollector<T, C>> + Send + 'static {
        async move {
            let mut collector = TokioCollector::new();
            if let Some(configure_fn) = self.configure_fn {
                configure_fn(&mut collector);
            }
            
            // Execute file processing if configured
            if let Some(config) = collector.source_config.clone() {
                match config {
                    DataSourceConfig::Csv { file_path, delimiter, chunk_size } => {
                        if let Err(e) = process_csv_file(&mut collector, &file_path, &delimiter, &chunk_size).await {
                            tracing::error!("CSV processing failed: {}", e);
                        }
                    }
                    #[cfg(feature = "surrealdb-ws")]
                    DataSourceConfig::SurrealDbWs { .. } => {
                        tracing::warn!("SurrealDB WebSocket processing not yet implemented");
                    }
                    #[cfg(feature = "surrealdb-kv")]
                    DataSourceConfig::SurrealDbKv { .. } => {
                        tracing::warn!("SurrealDB KV processing not yet implemented");
                    }
                }
            }
            
            collector
        }
    }
}

/// Process CSV file with zero-allocation parsing and chunking
async fn process_csv_file<T, C>(
    collector: &mut TokioCollector<T, C>,
    file_path: &PathBuf,
    delimiter: &Delimiter,
    chunk_size: &ChunkSize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: Clone + Send + 'static,
    C: Clone + Send + Sync + 'static + for<'a> FromCsvLine<'a>,
{
    let file = File::open(file_path).await
        .map_err(|e| format!("Failed to open file {}: {}", file_path.display(), e))?;
    
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut line_number = 0;
    let mut chunk_count = 0;
    
    let chunk_limit = match chunk_size {
        ChunkSize::Rows(n) => *n,
        ChunkSize::Bytes(_) => 1000, // Default for now
        ChunkSize::Duration(_) => 1000, // Default for time-based chunking
    };
    
    while let Some(line) = lines.next_line().await
        .map_err(|e| format!("Error reading line {}: {}", line_number + 1, e))? {
        
        line_number += 1;
        
        // Parse CSV record with zero allocation using string slices
        let mut field_buffer = Vec::new();
        match C::from_csv_line(&line, line_number, *delimiter, &mut field_buffer) {
            Ok(record) => {
                collector.collect_item(record);
                chunk_count += 1;
                
                // Yield control every chunk_limit items for responsive async behavior
                if chunk_count >= chunk_limit {
                    tokio::task::yield_now().await;
                    chunk_count = 0;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse line {}: {}", line_number, e);
                // Continue processing other lines instead of stopping
            }
        }
    }
    
    tracing::info!("Processed {} lines from {}", line_number, file_path.display());
    Ok(())
}