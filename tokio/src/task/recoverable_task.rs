//! Recoverable task implementation with retry logic and error recovery

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

use sweet_async_api::task::RecoverableTask;
use crate::task::task_error::{AsyncTaskError, RecoveryStrategy};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 30_000,
            backoff_multiplier: 2.0,
            jitter: true,
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
        let state = *self.circuit_state.read().unwrap();
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = *self.last_failure_time.read().unwrap() {
                    let elapsed = last_failure.elapsed().as_millis() as u64;
                    if elapsed >= self.circuit_config.timeout_ms {
                        // Transition to half-open
                        *self.circuit_state.write().unwrap() = CircuitState::HalfOpen;
                        self.success_count.store(0, Ordering::SeqCst);
                        true
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
        
        let state = *self.circuit_state.read().unwrap();
        if state == CircuitState::HalfOpen {
            let success_count = self.success_count.load(Ordering::SeqCst);
            if success_count >= self.circuit_config.success_threshold {
                *self.circuit_state.write().unwrap() = CircuitState::Closed;
                self.failure_count.store(0, Ordering::SeqCst);
                debug!("Circuit breaker closed after {} successes", success_count);
            }
        }
    }

    /// Record execution failure
    fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::SeqCst);
        *self.last_failure_time.write().unwrap() = Some(std::time::Instant::now());
        
        let failure_count = self.failure_count.load(Ordering::SeqCst);
        if failure_count >= self.circuit_config.failure_threshold {
            *self.circuit_state.write().unwrap() = CircuitState::Open;
            warn!("Circuit breaker opened after {} failures", failure_count);
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

impl<T: Send + 'static> RecoverableTask<T> for TokioRecoverableTask<T> {
    async fn retry<F, Fut>(&self, operation: F) -> Result<T, AsyncTaskError>
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, AsyncTaskError>> + Send + 'static,
    {
        let mut last_error = AsyncTaskError::failure("No attempts made");
        
        for attempt in 0..self.retry_config.max_attempts {
            // Check circuit breaker
            if !self.can_execute() {
                return Err(AsyncTaskError::failure("Circuit breaker is open"));
            }
            
            self.attempt_count.fetch_add(1, Ordering::SeqCst);
            debug!("Retry attempt {} of {}", attempt + 1, self.retry_config.max_attempts);
            
            match operation().await {
                Ok(result) => {
                    self.record_success();
                    debug!("Operation succeeded on attempt {}", attempt + 1);
                    return Ok(result);
                }
                Err(error) => {
                    last_error = error.clone();
                    self.record_failure();
                    
                    // Check if error is retryable
                    if !error.is_retryable() {
                        warn!("Non-retryable error: {}", error);
                        return Err(error);
                    }
                    
                    // Don't delay on the last attempt
                    if attempt < self.retry_config.max_attempts - 1 {
                        let delay = self.calculate_delay(attempt);
                        debug!("Waiting {:?} before retry attempt {}", delay, attempt + 2);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        warn!("All retry attempts exhausted");
        Err(last_error)
    }

    async fn recover_with_fallback<F, Fut, FB, FutB>(
        &self,
        primary: F,
        fallback: FB,
    ) -> Result<T, AsyncTaskError>
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, AsyncTaskError>> + Send + 'static,
        FB: Fn() -> FutB + Send + 'static,
        FutB: Future<Output = Result<T, AsyncTaskError>> + Send + 'static,
    {
        // Try primary operation with retry
        match self.retry(primary).await {
            Ok(result) => Ok(result),
            Err(primary_error) => {
                warn!("Primary operation failed, trying fallback: {}", primary_error);
                
                // Try fallback operation
                match fallback().await {
                    Ok(result) => {
                        debug!("Fallback operation succeeded");
                        Ok(result)
                    }
                    Err(fallback_error) => {
                        warn!("Fallback operation also failed: {}", fallback_error);
                        Err(AsyncTaskError::failure(format!(
                            "Both primary and fallback failed. Primary: {}. Fallback: {}",
                            primary_error, fallback_error
                        )))
                    }
                }
            }
        }
    }

    fn retry_count(&self) -> u32 {
        self.attempt_count.load(Ordering::SeqCst)
    }

    fn reset_retry_count(&self) {
        self.attempt_count.store(0, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        *self.circuit_state.write().unwrap() = CircuitState::Closed;
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
                    Err(AsyncTaskError::failure("Temporary failure"))
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
            Err(AsyncTaskError::failure("Always fails"))
        }).await;
        
        assert!(result.is_err());
        assert_eq!(task.retry_count(), 2);
    }
}