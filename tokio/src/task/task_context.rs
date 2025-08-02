//! Task context implementation

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use sweet_async_api::task::{ContextualizedTask, TaskId};
use crate::orchestra::runtime::TokioRuntime;
use crate::task::relationships::TokioTaskRelationships;

/// Tokio-specific task context that implements ContextualizedTask
#[derive(Debug, Clone)]
pub struct TokioTaskContext<T: Clone + Send + Sync + 'static, I: TaskId> {
    /// Task data storage
    data: Arc<RwLock<HashMap<String, Value>>>,
    /// Task metadata storage
    metadata: Arc<RwLock<HashMap<String, String>>>,
    /// Runtime handle for this task context
    runtime: TokioRuntime,
    /// Task relationships for parent-child communication
    relationships: TokioTaskRelationships<T, I>,
    /// Current working directory
    working_directory: PathBuf,
}

impl<T: Clone + Send + Sync + 'static, I: TaskId> TokioTaskContext<T, I> {
    /// Create a new task context with default values
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            runtime: TokioRuntime::new(),
            relationships: TokioTaskRelationships::new(),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
        }
    }

    /// Create a new task context with custom runtime and relationships
    pub fn new_with_config(
        runtime: TokioRuntime,
        relationships: TokioTaskRelationships<T, I>,
        working_directory: Option<PathBuf>,
    ) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            runtime,
            relationships,
            working_directory: working_directory.unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
            }),
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

impl<T: Clone + Send + Sync + 'static, I: TaskId> Default for TokioTaskContext<T, I> {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of ContextualizedTask trait for TokioTaskContext
impl<T: Clone + Send + Sync + 'static, I: TaskId> ContextualizedTask<T, I> for TokioTaskContext<T, I> {
    type RuntimeType = TokioRuntime;
    type RelationshipsType = TokioTaskRelationships<T, I>;

    /// Get communication handles for this task's relationships
    fn relationships(&self) -> &Self::RelationshipsType {
        &self.relationships
    }

    /// Get mutable communication handles for this task's relationships
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        &mut self.relationships
    }

    /// Get the runtime this task is running in
    fn runtime(&self) -> &Self::RuntimeType {
        &self.runtime
    }

    /// Get the current working directory for task execution
    fn cwd(&self) -> PathBuf {
        self.working_directory.clone()
    }
}

impl<T: Clone + Send + Sync + 'static, I: TaskId> TokioTaskContext<T, I> {
    /// Set the working directory for this task context
    pub fn set_working_directory(&mut self, path: PathBuf) {
        self.working_directory = path;
    }

    /// Get all data keys currently stored in the context
    pub async fn get_all_data_keys(&self) -> Vec<String> {
        let data = self.data.read().await;
        data.keys().cloned().collect()
    }

    /// Get all metadata keys currently stored in the context
    pub async fn get_all_metadata_keys(&self) -> Vec<String> {
        let metadata = self.metadata.read().await;
        metadata.keys().cloned().collect()
    }

    /// Clear all data from the context
    pub async fn clear_data(&self) {
        let mut data = self.data.write().await;
        data.clear();
    }

    /// Clear all metadata from the context
    pub async fn clear_metadata(&self) {
        let mut metadata = self.metadata.write().await;
        metadata.clear();
    }

    /// Remove a specific data key
    pub async fn remove_data(&self, key: &str) -> Option<Value> {
        let mut data = self.data.write().await;
        data.remove(key)
    }

    /// Remove a specific metadata key
    pub async fn remove_metadata(&self, key: &str) -> Option<String> {
        let mut metadata = self.metadata.write().await;
        metadata.remove(key)
    }

    /// Check if a data key exists
    pub async fn has_data(&self, key: &str) -> bool {
        let data = self.data.read().await;
        data.contains_key(key)
    }

    /// Check if a metadata key exists
    pub async fn has_metadata(&self, key: &str) -> bool {
        let metadata = self.metadata.read().await;
        metadata.contains_key(key)
    }

    /// Get the number of data entries
    pub async fn data_count(&self) -> usize {
        let data = self.data.read().await;
        data.len()
    }

    /// Get the number of metadata entries
    pub async fn metadata_count(&self) -> usize {
        let metadata = self.metadata.read().await;
        metadata.len()
    }

    /// Set multiple data entries at once
    pub async fn set_data_batch(&self, entries: HashMap<String, Value>) {
        let mut data = self.data.write().await;
        for (key, value) in entries {
            data.insert(key, value);
        }
    }

    /// Set multiple metadata entries at once
    pub async fn set_metadata_batch(&self, entries: HashMap<String, String>) {
        let mut metadata = self.metadata.write().await;
        for (key, value) in entries {
            metadata.insert(key, value);
        }
    }

    /// Create a snapshot of all current data
    pub async fn snapshot_data(&self) -> HashMap<String, Value> {
        let data = self.data.read().await;
        data.clone()
    }

    /// Create a snapshot of all current metadata
    pub async fn snapshot_metadata(&self) -> HashMap<String, String> {
        let metadata = self.metadata.read().await;
        metadata.clone()
    }
}