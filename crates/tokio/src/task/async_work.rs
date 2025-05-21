//! Type-erased types for AsyncWork trait to enable dynamic dispatch
//!
//! Since AsyncWork has associated types that make it not dyn compatible,
//! we create type-erased types that can be used with dynamic dispatch.

use std::future::Future;
use std::pin::Pin;
use sweet_async_api::task::builder::AsyncWork;

/// Type-erased async work for dynamic dispatch
pub struct AsyncWorkDyn<T> {
    work: Pin<Box<dyn Future<Output = T> + Send + 'static>>,
}

impl<T> AsyncWorkDyn<T> {
    /// Create a new type-erased async work from an AsyncWork implementation
    pub fn new<W: AsyncWork<T> + 'static>(work: W) -> Self {
        Self {
            work: Box::pin(work.run()),
        }
    }
}

impl<T: Send + 'static> AsyncWork<T> for AsyncWorkDyn<T> {
    fn run(self) -> impl Future<Output = T> + Send + 'static {
        self.work
    }
}

/// Type-erased dynamic async work for AsyncWork 
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