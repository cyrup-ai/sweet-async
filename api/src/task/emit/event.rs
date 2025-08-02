use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

/// The type of event in the streaming event system
#[derive(Debug, Clone)]
pub enum StreamingEventType<T> {
    /// A final event, containing the final data
    Final(T),
    /// A continuation event, processing continues
    Continue,
    /// An error occurred during processing
    Error(String),
    /// The processing was cancelled
    Cancellation,
}

/// Base trait for all streaming events in the system
#[allow(dead_code)]
pub trait StreamingEvent<T>: Send + 'static {
    /// When the event was created
    fn created_timestamp(&self) -> &DateTime<Utc>;

    /// When the event processing started
    fn started_timestamp(&self) -> &DateTime<Utc>;

    /// When the event processing completed
    fn completed_timestamp(&self) -> &DateTime<Utc>;

    /// Unique identifier for this event
    fn event_id(&self) -> &Uuid;

    /// Identifier of the task this event belongs to
    fn task_id(&self) -> &Uuid;

    /// Access the data payload of the event
    fn data(&self) -> &T;

    /// The type of this event
    fn event_type(&self) -> &StreamingEventType<T>;

    /// Whether this is the final event in a sequence
    fn is_final(&self) -> bool;
}

/// Collects and manages event results from various data sources
#[allow(dead_code)]
pub trait Collector<T, C, Collection = HashMap<Uuid, C>>: Send + 'static {
    /// Collect an item with a specific key
    fn collect<K>(&mut self, key: K, item: C)
    where
        K: Into<Uuid>;

    /// Collect a single item (automatically generates a UUID key)
    fn collect_item(&mut self, item: C);

    /// Collect multiple items
    fn collect_items(&mut self, items: Vec<C>);

    /// Collect an error
    fn collect_error<K, E>(&mut self, key: K, error: E)
    where
        K: Into<Uuid>,
        E: std::error::Error + Send + 'static;

    /// Get all collected items
    fn collected(&self) -> &Collection;

    // Collection sources
    fn of<I>(&mut self, items: I) -> &mut Self
    where
        I: IntoIterator<Item = T> + Send + 'static;

    fn of_file(&mut self, path: impl AsRef<Path>) -> &mut Self;

    // Database sources
    fn from_database<P>(&mut self, pool: P, query: &str) -> &mut Self
    where
        P: Send + 'static;

    fn from_postgres<P>(&mut self, pool: P, query: &str) -> &mut Self
    where
        P: Clone + Send + 'static;

    fn from_surrealdb<DB>(&mut self, db: DB, query: &str) -> &mut Self
    where
        DB: Send + 'static;

    fn from_seaorm<Q>(&mut self, query: Q) -> &mut Self
    where
        Q: Send + 'static;

    fn from_sqlite<DB>(&mut self, connection: DB, query: &str) -> &mut Self
    where
        DB: Send + 'static;

    fn from_mysql<DB>(&mut self, connection: DB, query: &str) -> &mut Self
    where
        DB: Send + 'static;

    // API & Network sources
    fn from_api_batch<U>(&mut self, urls: Vec<U>) -> &mut Self
    where
        U: Send + 'static;

    fn from_websocket(&mut self, url: &str) -> &mut Self;

    fn from_sse(&mut self, url: &str) -> &mut Self;

    fn from_graphql_subscription<G, Q>(&mut self, client: G, query: Q) -> &mut Self
    where
        G: Send + 'static,
        Q: Send + 'static;

    // Cloud Storage
    fn from_s3<Client>(&mut self, client: Client, bucket: &str, pattern: &str) -> &mut Self
    where
        Client: Send + 'static;

    fn from_gcs<Client>(&mut self, client: Client, bucket: &str, pattern: &str) -> &mut Self
    where
        Client: Send + 'static;

    fn from_azure_blob<Client>(
        &mut self,
        client: Client,
        container: &str,
        pattern: &str,
    ) -> &mut Self
    where
        Client: Send + 'static;

    // Version Control
    fn from_github(&mut self, repo: &str, pattern: &str) -> &mut Self;

    fn from_gitlab(&mut self, project: &str, pattern: &str) -> &mut Self;

    fn from_git_repo(&mut self, url: &str) -> &mut Self;

    // Message Queues & Streaming
    fn from_kafka_topic<Consumer>(&mut self, consumer: Consumer, topic: &str) -> &mut Self
    where
        Consumer: Send + 'static;

    fn from_redis_stream<R>(&mut self, conn: R, pattern: &str) -> &mut Self
    where
        R: Send + 'static;

    fn from_rabbitmq<A>(&mut self, conn: A, queue: &str) -> &mut Self
    where
        A: Send + 'static;

    fn from_nats<N>(&mut self, conn: N, subject: &str) -> &mut Self
    where
        N: Send + 'static;

    // Social Media & Communication
    fn from_twitter_stream(&mut self, account: &str) -> &mut Self;

    fn from_discord_server<Token>(&mut self, token: Token, server_id: &str) -> &mut Self
    where
        Token: Send + 'static;

    fn from_tiktok_feed<A>(&mut self, api: A) -> &mut Self
    where
        A: Send + 'static;

    fn from_facebook_feed<A>(&mut self, api: A) -> &mut Self
    where
        A: Send + 'static;

    // Browser & History sources
    fn from_browser_history(&mut self, browser_type: impl Send + 'static) -> &mut Self;

    // IoT & Smart Devices
    fn from_smart_fridge<A>(&mut self, api: A) -> &mut Self
    where
        A: Send + 'static;

    fn from_roomba<A>(&mut self, api: A) -> &mut Self
    where
        A: Send + 'static;

    fn from_alexa_voice_history<A>(&mut self, api: A) -> &mut Self
    where
        A: Send + 'static;

    fn from_tesla_api<Token>(&mut self, token: Token) -> &mut Self
    where
        Token: Send + 'static;

    // Financial & Shopping
    fn from_bank_transactions<A>(&mut self, api: A) -> &mut Self
    where
        A: Send + 'static;

    // Configuration methods
    fn with_delimiter(&mut self, delimiter: impl Send + 'static) -> &mut Self;
    fn with_batch_size(&mut self, size: usize) -> &mut Self;
    fn with_concurrency(&mut self, level: usize) -> &mut Self;
    fn with_retry(&mut self, attempts: usize) -> &mut Self;
    fn with_timeout(&mut self, duration: Duration) -> &mut Self;
    fn with_auth(&mut self, auth: impl Send + 'static) -> &mut Self;
    fn with_headers(&mut self, headers: impl Send + 'static) -> &mut Self;
    fn with_region(&mut self, region: impl Send + 'static) -> &mut Self;
    fn with_credentials(&mut self, credentials: impl Send + 'static) -> &mut Self;
    fn with_service_account(&mut self, account: impl Send + 'static) -> &mut Self;
    fn with_connection_string(&mut self, connection: impl Send + 'static) -> &mut Self;
    fn with_branch(&mut self, branch: &str) -> &mut Self;
    fn with_token(&mut self, token: impl Send + 'static) -> &mut Self;
    fn with_path(&mut self, path: &str) -> &mut Self;
    fn with_ref(&mut self, reference: &str) -> &mut Self;
    fn with_consumer_group(&mut self, group: &str) -> &mut Self;
    fn with_offset(&mut self, offset: impl Send + 'static) -> &mut Self;
    fn with_partition_strategy(&mut self, strategy: impl Send + 'static) -> &mut Self;
    fn with_prefetch(&mut self, count: usize) -> &mut Self;
    fn with_queue_group(&mut self, group: &str) -> &mut Self;
    fn with_streaming(&mut self, enabled: bool) -> &mut Self;
    fn with_cursor_size(&mut self, size: usize) -> &mut Self;
    fn with_live_query(&mut self, enabled: bool) -> &mut Self;

    // Finalization methods
    fn into_chunks(&mut self, size: usize) -> &mut Self;
}

/// Builder for sender events - uses a fluent API without explicit build steps
#[allow(dead_code)]
pub trait SenderEventBuilder<T>: SenderEvent<T> + Send + 'static {
    /// Create a new sender event builder
    fn new(task_id: Uuid, event_id: Uuid, data: T) -> Self;

    /// Set the event type
    fn event_type(self, event_type: StreamingEventType<T>) -> Self;

    /// Update the event data
    fn data(self, data: T) -> Self;

    /// Mark this as the final event
    fn is_final(self) -> Self;
}

/// Event that can be sent through the system
#[allow(dead_code)]
pub trait SenderEvent<T>: StreamingEvent<T> + Send + 'static {
    type Builder: SenderEventBuilder<T>;
    /// Create a new builder for this event type
    fn builder() -> Self::Builder;

    /// Create a builder with specified event type
    fn event_type(event_type: StreamingEventType<T>) -> Self::Builder;

    /// Create a builder with specified data
    fn data(data: T) -> Self::Builder;

    /// Create a builder for a final event
    fn is_final() -> Self::Builder;
}

/// Event that can be received for processing
pub trait ReceiverEvent<T, C>: Send + 'static {
    /// Event identifier
    fn event_id(&self) -> &Uuid;

    /// Task identifier
    fn task_id(&self) -> &Uuid;

    /// Access the event data
    fn data(&self) -> &T;

    /// Get the event type
    fn event_type(&self) -> &StreamingEventType<T>;

    /// Whether this is the final event
    fn is_final(&self) -> bool;

    /// Access the collector for this event
    fn collector(&self) -> &C;
}

/// The final event in a sequence, containing all collected results
#[allow(dead_code)]
pub trait FinalEvent<T, C, Item, Collection = HashMap<Uuid, Item>>:
    ReceiverEvent<T, C> + Send + 'static
{
    /// Access all collected items in their native collection
    fn collected(&self) -> &Collection;

    /// Get all collected items as a vector (for convenience)
    fn yield_results(&self) -> Vec<Item>;
}
