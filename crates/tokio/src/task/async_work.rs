//! Wrapper types for AsyncWork trait to enable dynamic dispatch
//!
//! Since AsyncWork has associated types that make it not dyn compatible,
//! we create wrapper types that can be used with dynamic dispatch.

use std::future::Future;
use std::pin::Pin;
use sweet_async_api::task::builder::AsyncWork;

/// Wrapper type that holds a boxed future for dynamic dispatch
pub struct AsyncWorkWrapper<T> {
    work: Pin<Box<dyn Future<Output = T> + Send + 'static>>,
}

impl<T> AsyncWorkWrapper<T> {
    /// Create a new wrapper from an AsyncWork implementation
    pub fn new<W: AsyncWork<T> + 'static>(work: W) -> Self {
        Self {
            work: Box::pin(work.run()),
        }
    }
}

impl<T: Send + 'static> AsyncWork<T> for AsyncWorkWrapper<T> {
    fn run(self) -> impl Future<Output = T> + Send + 'static {
        self.work
    }
}

/// Type-erased wrapper for AsyncWork 
pub struct DynAsyncWork<T> {
    work: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + 'static>,
}

impl<T> DynAsyncWork<T> {
    /// Create a new dynamic async work from a closure
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Self {
            work: Box::new(move || Box::pin(f())),
        }
    }
}

impl<T: Send + 'static> AsyncWork<T> for DynAsyncWork<T> {
    fn run(self) -> impl Future<Output = T> + Send + 'static {
        (self.work)()
    }
}