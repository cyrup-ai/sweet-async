//! Root orchestrator task that owns the runtime

use std::sync::Arc;
use sweet_async_api::task::{
    ContextualizedTask, TaskId
};
use crate::orchestra::runtime::TokioOrchestraRuntime;
use crate::task::TokioTaskRelationships;

/// Root orchestrator task that owns the runtime
pub struct TokioOrchestratorTask<T, I> {
    runtime: TokioOrchestraRuntime<T, I>,
    relationships: TokioTaskRelationships<T, I>,
}

impl<T, I> TokioOrchestratorTask<T, I>
where
    T: Clone + Send + Sync + Default + 'static,
    I: TaskId + std::hash::Hash,
{
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            runtime: TokioOrchestraRuntime::new(),
            relationships: TokioTaskRelationships::new(),
        })
    }
}

impl<T, I> ContextualizedTask<T, I> for TokioOrchestratorTask<T, I>
where
    T: Clone + Send + Sync + Default + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type RuntimeType = TokioOrchestraRuntime<T, I>;
    type RelationshipsType = TokioTaskRelationships<T, I>;

    fn runtime(&self) -> &Self::RuntimeType {
        &self.runtime
    }

    fn relationships(&self) -> &Self::RelationshipsType {
        &self.relationships
    }

    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        &mut self.relationships
    }

    fn cwd(&self) -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
    }
}