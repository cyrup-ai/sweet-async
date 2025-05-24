//! Async utilities for task implementations
//!
//! This module provides common patterns for dealing with async operations
//! in contexts where we need synchronous interfaces.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Spawn a detached task to update a value in an Arc<Mutex<T>>
/// This allows builder methods to remain synchronous while performing async operations
pub fn spawn_update<T, F>(target: Arc<Mutex<T>>, f: F)
where
    T: Send + Sync + 'static,
    F: FnOnce(&mut T) + Send + 'static,
{
    tokio::spawn(async move {
        let mut guard = target.lock().await;
        f(&mut *guard);
    });
}

/// Spawn a detached task to update a value in an Arc<RwLock<T>>
pub fn spawn_write<T, F>(target: Arc<RwLock<T>>, f: F)
where
    T: Send + Sync + 'static,
    F: FnOnce(&mut T) + Send + 'static,
{
    tokio::spawn(async move {
        let mut guard = target.write().await;
        f(&mut *guard);
    });
}

/// Create an async future that reads from an Arc<Mutex<T>>
pub fn async_read<T, R, F>(target: Arc<Mutex<T>>, f: F) -> Pin<Box<dyn Future<Output = R> + Send>>
where
    T: Send + Sync + 'static,
    R: Send + 'static,
    F: FnOnce(&T) -> R + Send + 'static,
{
    Box::pin(async move {
        let guard = target.lock().await;
        f(&*guard)
    })
}

/// Create an async future that reads from an Arc<RwLock<T>>
pub fn async_read_rw<T, R, F>(target: Arc<RwLock<T>>, f: F) -> Pin<Box<dyn Future<Output = R> + Send>>
where
    T: Send + Sync + 'static,
    R: Send + 'static,
    F: FnOnce(&T) -> R + Send + 'static,
{
    Box::pin(async move {
        let guard = target.read().await;
        f(&*guard)
    })
}