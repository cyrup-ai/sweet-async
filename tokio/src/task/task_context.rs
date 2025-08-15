//! High-performance task context implementation with zero allocation

use crate::orchestra::runtime::TokioOrchestraRuntime;
use crate::task::TokioTaskRelationships;
use std::path::PathBuf;
use sweet_async_api::task::{ContextualizedTask, TaskId};

/// Zero-allocation task context implementation
#[derive(Debug)]
pub struct TokioTaskContext<T, I> {
    task_id: I,
    relationships: TokioTaskRelationships<T, I>,
    cwd: PathBuf,
}

impl<T, I> TokioTaskContext<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + 'static,
    I: TaskId + std::hash::Hash,
{
    #[inline]
    pub fn new(task_id: I) -> Self {
        Self {
            task_id,
            relationships: TokioTaskRelationships::new(),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
        }
    }

    #[inline]
    pub fn relationships(&self) -> &TokioTaskRelationships<T, I> {
        &self.relationships
    }

    #[inline]
    pub fn relationships_mut(&mut self) -> &mut TokioTaskRelationships<T, I> {
        &mut self.relationships
    }

    #[inline]
    pub fn runtime(&self) -> &TokioOrchestraRuntime<T, I> {
        // Runtime is created on-demand only when run() is called
        &TokioOrchestraRuntime::<T, I>::current()
    }

    #[inline]
    pub fn cwd(&self) -> PathBuf {
        self.cwd.clone()
    }
}

impl<T, I> ContextualizedTask<T, I> for TokioTaskContext<T, I>
where
    T: Clone + Send + Sync + Default + std::fmt::Debug + Unpin + 'static,
    I: TaskId + std::hash::Hash + Unpin,
{
    type RuntimeType = TokioOrchestraRuntime<T, I>;
    type RelationshipsType = TokioTaskRelationships<T, I>;

    #[inline]
    fn relationships(&self) -> &Self::RelationshipsType {
        &self.relationships
    }

    #[inline]
    fn relationships_mut(&mut self) -> &mut Self::RelationshipsType {
        &mut self.relationships
    }

    #[inline]
    fn runtime(&self) -> &Self::RuntimeType {
        // Runtime is created on-demand only when run() is called
        &TokioOrchestraRuntime::<T, I>::current()
    }

    #[inline]
    fn cwd(&self) -> PathBuf {
        self.cwd.clone()
    }
}
