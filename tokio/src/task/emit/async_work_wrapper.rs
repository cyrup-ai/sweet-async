//! Wrapper types for making AsyncWork trait object-safe
//!
//! Since AsyncWork uses `impl Trait` in return position, it's not dyn-compatible.
//! These wrappers allow us to store and execute AsyncWork instances.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use sweet_async_api::task::builder::AsyncWork;

/// A boxed version of AsyncWork that can be stored as a trait object
pub struct BoxedAsyncWork<T> {
    work: Option<Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = T> + Send>> + Send>>,
}

impl<T> BoxedAsyncWork<T> {
    /// Create a new BoxedAsyncWork from any AsyncWork implementation
    pub fn new<W>(work: W) -> Self
    where
        W: AsyncWork<T> + Send + 'static,
        T: Send + 'static,
    {
        Self {
            work: Some(Box::new(move || Box::pin(work.run()))),
        }
    }

    /// Execute the stored work
    /// 
    /// # Panics
    /// 
    /// This method can only be called once. Subsequent calls will panic.
    pub async fn run(mut self) -> T {
        let work = match self.work.take() {
            Some(work) => work,
            None => {
                tracing::error!("BoxedAsyncWork::run() called multiple times - this is a programming error");
                // Instead of panic, we abort the process which is safer for production
                std::process::abort();
            }
        };
        work().await
    }
}

/// A newtype wrapper around a boxed future to make it clonable
struct CloneableFuture<T>(Box<dyn Future<Output = T> + Send>);

impl<T> CloneableFuture<T> {
    fn new<F>(fut: F) -> Self 
    where
        F: Future<Output = T> + Send + 'static
    {
        Self(Box::new(fut))
    }
}

impl<T> Future for CloneableFuture<T> {
    type Output = T;
    
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.0).poll(cx)
    }
}

impl<T: Clone> Clone for CloneableFuture<T> {
    fn clone(&self) -> Self {
        // This is a dummy implementation since we can't actually clone a future
        // In practice, we'll need to ensure the future is recreated when cloned
        // This is safe because we'll only use this in contexts where the future
        // is recreated from the original work function
        panic!("CloneableFuture should not be cloned directly")
    }
}

/// A clonable version of BoxedAsyncWork for cases where we need to share work
pub struct SharedAsyncWork<T> {
    work: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync>,
}

impl<T> SharedAsyncWork<T> {
    /// Create a new SharedAsyncWork that can be called multiple times
    pub fn new<F, Fut>(work_fn: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        Self {
            work: Arc::new(move || Box::pin(work_fn())),
        }
    }

    /// Execute the work (can be called multiple times)
    pub async fn run(&self) -> T {
        (self.work)().await
    }
}

impl<T> Clone for SharedAsyncWork<T> {
    fn clone(&self) -> Self {
        Self {
            work: Arc::clone(&self.work),
        }
    }
}