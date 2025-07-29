//! Task context implementation

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use sweet_async_api::task::ContextualizedTask;

/// Tokio-specific task context
#[derive(Debug, Clone)]
pub struct TokioTaskContext {
    data: Arc<RwLock<HashMap<String, Value>>>,
    metadata: Arc<RwLock<HashMap<String, String>>>,
}

impl TokioTaskContext {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_data(&self, key: &str) -> Option<Value> {
        let data = self.data.read().await;
        data.get(key).cloned()
    }

    pub async fn set_data(&self, key: String, value: Value) {
        let mut data = self.data.write().await;
        data.insert(key, value);
    }

    pub async fn get_metadata(&self, key: &str) -> Option<String> {
        let metadata = self.metadata.read().await;
        metadata.get(key).cloned()
    }

    pub async fn set_metadata(&self, key: String, value: String) {
        let mut metadata = self.metadata.write().await;
        metadata.insert(key, value);
    }
}

impl Default for TokioTaskContext {
    fn default() -> Self {
        Self::new()
    }
}

// Implementation of ContextualizedTask would go here when we have the proper generic parameters
// For now, we'll keep the context as a simple utility struct