//! High-performance named task implementation

use std::sync::Arc;
use sweet_async_api::task::named_task::NamedTask;

/// Zero-allocation named task with Arc-based string sharing
#[derive(Debug, Clone)]
pub struct TokioNamedTask {
    name: Option<Arc<str>>,
}

impl TokioNamedTask {
    #[inline]
    pub fn new() -> Self {
        Self { name: None }
    }

    #[inline]
    pub fn with_name(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: Some(name.into()),
        }
    }
}

impl Default for TokioNamedTask {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl NamedTask for TokioNamedTask {
    #[inline]
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[inline]
    fn set_name(&mut self, name: String) {
        self.name = Some(name.into());
    }
}
