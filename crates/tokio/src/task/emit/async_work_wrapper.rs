//! Wrapper types for making AsyncWork trait object-safe
//!
//! Since AsyncWork uses `impl Trait` in return position, it's not dyn-compatible.
//! These wrappers allow us to store and execute AsyncWork instances.

use std::future::Future;
use std::pin::Pin;
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
    pub async fn run(mut self) -> T {
        let work = self.work.take().expect("BoxedAsyncWork can only be run once");
        work().await
    }
}

/// A clonable version of BoxedAsyncWork for cases where we need to share work
pub struct SharedAsyncWork<T> {
    work: Box<dyn Fn() -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync>,
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
            work: Box::new(move || Box::pin(work_fn())),
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
            work: self.work.clone(),
        }
    }
}