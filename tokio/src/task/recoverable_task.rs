//! Recoverable task implementation with retry logic and error recovery

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

use sweet_async_api::task::RecoverableTask;
use sweet_async_api::task::recoverable_task::RetryStrategy;
use sweet_async_api::task::AsyncTaskError;
use sweet_async_api::task::builder::AsyncWork;

/// Backoff strategy for retry attempts
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed(Duration),
    /// Linear backoff with base interval
    Linear(Duration),
    /// Exponential backoff with base and max delay
    Exponential { base: Duration, max_delay: Duration },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::Exponential {
            base: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u8,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
    pub strategy: BackoffStrategy,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 30_000,
            backoff_multiplier: 2.0,
            jitter: true,
            strategy: BackoffStrategy::default(),
        }
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_ms: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_ms: 60_000,
        }
    }
}

/// Tokio-specific recoverable task implementation
#[derive(Debug)]
pub struct TokioRecoverableTask<T: Send + 'static> {
    retry_config: RetryConfig,
    circuit_config: CircuitBreakerConfig,
    attempt_count: Arc<AtomicU32>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    circuit_state: Arc<std::sync::RwLock<CircuitState>>,
    last_failure_time: Arc<std::sync::RwLock<Option<std::time::Instant>>>,
    fallback_work: crate::task::error_fallback::ErrorFallback<T>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static> TokioRecoverableTask<T> {
    /// Create a new recoverable task
    pub fn new() -> Self {
        Self {
            retry_config: RetryConfig::default(),
            circuit_config: CircuitBreakerConfig::default(),
            attempt_count: Arc::new(AtomicU32::new(0)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            circuit_state: Arc::new(std::sync::RwLock::new(CircuitState::Closed)),
            last_failure_time: Arc::new(std::sync::RwLock::new(None)),
            fallback_work: crate::task::error_fallback::ErrorFallback::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Configure retry behavior
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Configure circuit breaker
    pub fn with_circuit_breaker(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_config = config;
        self
    }

    /// Check if circuit breaker allows execution
    fn can_execute(&self) -> bool {
        let state = match self.circuit_state.read() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                tracing::error!("Circuit breaker state lock poisoned, defaulting to closed");
                *poisoned.into_inner()
            }
        };
        
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = match self.last_failure_time.read() {
                    Ok(guard) => *guard,
                    Err(poisoned) => {
                        tracing::error!("Last failure time lock poisoned");
                        *poisoned.into_inner()
                    }
                };
                
                if let Some(failure_time) = last_failure {
                    let elapsed = failure_time.elapsed().as_millis() as u64;
                    if elapsed >= self.circuit_config.timeout_ms {
                        // Transition to half-open
                        match self.circuit_state.write() {
                            Ok(mut guard) => {
                                *guard = CircuitState::HalfOpen;
                                self.success_count.store(0, Ordering::SeqCst);
                                true
                            }
                            Err(poisoned) => {
                                tracing::error!("Circuit breaker state write lock poisoned");
                                *poisoned.into_inner() = CircuitState::HalfOpen;
                                self.success_count.store(0, Ordering::SeqCst);
                                true
                            }
                        }
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record execution success
    fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::SeqCst);
        
        let state = match self.circuit_state.read() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                tracing::error!("Circuit breaker state lock poisoned in record_success");
                *poisoned.into_inner()
            }
        };
        
        if state == CircuitState::HalfOpen {
            let success_count = self.success_count.load(Ordering::SeqCst);
            if success_count >= self.circuit_config.success_threshold {
                match self.circuit_state.write() {
                    Ok(mut guard) => {
                        *guard = CircuitState::Closed;
                        self.failure_count.store(0, Ordering::SeqCst);
                        debug!("Circuit breaker closed after {} successes", success_count);
                    }
                    Err(poisoned) => {
                        tracing::error!("Circuit breaker state write lock poisoned in record_success");
                        *poisoned.into_inner() = CircuitState::Closed;
                        self.failure_count.store(0, Ordering::SeqCst);
                        debug!("Circuit breaker closed after {} successes (recovered from poison)", success_count);
                    }
                }
            }
        }
    }

    /// Record execution failure
    fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::SeqCst);
        
        match self.last_failure_time.write() {
            Ok(mut guard) => {
                *guard = Some(std::time::Instant::now());
            }
            Err(poisoned) => {
                tracing::error!("Last failure time write lock poisoned in record_failure");
                *poisoned.into_inner() = Some(std::time::Instant::now());
            }
        }
        
        let failure_count = self.failure_count.load(Ordering::SeqCst);
        if failure_count >= self.circuit_config.failure_threshold {
            match self.circuit_state.write() {
                Ok(mut guard) => {
                    *guard = CircuitState::Open;
                    warn!("Circuit breaker opened after {} failures", failure_count);
                }
                Err(poisoned) => {
                    tracing::error!("Circuit breaker state write lock poisoned in record_failure");
                    *poisoned.into_inner() = CircuitState::Open;
                    warn!("Circuit breaker opened after {} failures (recovered from poison)", failure_count);
                }
            }
        }
    }

    /// Calculate delay for retry attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.retry_config.base_delay_ms as f64;
        let multiplier = self.retry_config.backoff_multiplier;
        let mut delay_ms = base_delay * multiplier.powi(attempt as i32);
        
        // Apply maximum delay limit
        delay_ms = delay_ms.min(self.retry_config.max_delay_ms as f64);
        
        // Add jitter if enabled
        if self.retry_config.jitter {
            let jitter = rand::random::<f64>() * 0.1; // ±10% jitter
            delay_ms *= 1.0 + jitter;
        }
        
        Duration::from_millis(delay_ms as u64)
    }
}

impl<T: Send + 'static> Default for TokioRecoverableTask<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send + 'static> RecoverableTask<T> for TokioRecoverableTask<T> {
    type FallbackWork = crate::task::error_fallback::ErrorFallback<T>;

    fn recover(&self, error: AsyncTaskError) -> impl Future<Output = Result<T, AsyncTaskError>> + Send {
        let fallback_work = self.fallback_work().clone();
        let can_recover = self.can_recover_from(&error);
        let current_retry = self.current_retry();
        let max_retries = self.max_retries();
        
        async move {
            if can_recover && current_retry < max_retries {
                // Use fallback work to generate a recovery value
                return fallback_work.run().await;
            }
            Err(error)
        }
    }

    fn can_recover_from(&self, error: &AsyncTaskError) -> bool {
        // Recoverable from timeout, IO, and resource limit errors
        matches!(error, 
            AsyncTaskError::Timeout(_) | 
            AsyncTaskError::Io(_) | 
            AsyncTaskError::ResourceLimit(_)
        )
    }

    fn fallback_work(&self) -> &Self::FallbackWork {
        &self.fallback_work
    }

    fn max_retries(&self) -> u8 {
        self.retry_config.max_attempts
    }

    fn current_retry(&self) -> u8 {
        self.attempt_count.load(Ordering::SeqCst) as u8
    }

    fn retry_strategy(&self) -> RetryStrategy {
        match self.retry_config.strategy {
            BackoffStrategy::Fixed(duration) => RetryStrategy::Fixed(duration),
            BackoffStrategy::Linear(base) => RetryStrategy::Linear { 
                base, 
                increment: Duration::from_millis(100) // Default increment of 100ms
            },
            BackoffStrategy::Exponential { base, max_delay } => RetryStrategy::Exponential { 
                base, 
                factor: 2.0, // Default factor of 2.0
                max: max_delay 
            },
        }
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[tokio::test]
    async fn test_successful_retry() {
        let task = TokioRecoverableTask::<i32>::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = task.retry(move || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(AsyncTaskError::Failure("Temporary failure".to_string()))
                } else {
                    Ok(42)
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exhaustion() {
        let task = TokioRecoverableTask::<i32>::new()
            .with_retry_config(RetryConfig {
                max_attempts: 2,
                base_delay_ms: 1,
                ..Default::default()
            });
        
        let result = task.retry(|| async {
            Err(AsyncTaskError::Failure("Always fails".to_string()))
        }).await;
        
        assert!(result.is_err());
        assert_eq!(task.retry_count(), 2);
    }
}