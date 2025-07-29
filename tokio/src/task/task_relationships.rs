//! Task relationship management

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use sweet_async_api::task::TaskId;

/// Manages parent-child and dependency relationships between tasks
#[derive(Debug)]
pub struct TaskRelationshipManager<I: TaskId> {
    /// Map of task ID to its parent
    parents: Arc<RwLock<HashMap<I, I>>>,
    /// Map of task ID to its children
    children: Arc<RwLock<HashMap<I, HashSet<I>>>>,
    /// Map of task ID to its dependencies
    dependencies: Arc<RwLock<HashMap<I, HashSet<I>>>>,
    /// Map of task ID to tasks that depend on it
    dependents: Arc<RwLock<HashMap<I, HashSet<I>>>>,
}

impl<I: TaskId> TaskRelationshipManager<I> {
    pub fn new() -> Self {
        Self {
            parents: Arc::new(RwLock::new(HashMap::new())),
            children: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            dependents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a parent-child relationship
    pub async fn add_child(&self, parent_id: I, child_id: I) {
        {
            let mut parents = self.parents.write().await;
            parents.insert(child_id, parent_id);
        }
        {
            let mut children = self.children.write().await;
            children.entry(parent_id).or_insert_with(HashSet::new).insert(child_id);
        }
    }

    /// Add a dependency relationship
    pub async fn add_dependency(&self, task_id: I, depends_on: I) {
        {
            let mut dependencies = self.dependencies.write().await;
            dependencies.entry(task_id).or_insert_with(HashSet::new).insert(depends_on);
        }
        {
            let mut dependents = self.dependents.write().await;
            dependents.entry(depends_on).or_insert_with(HashSet::new).insert(task_id);
        }
    }

    /// Get all children of a task
    pub async fn get_children(&self, task_id: &I) -> Vec<I> {
        let children = self.children.read().await;
        children.get(task_id).map(|set| set.iter().copied().collect()).unwrap_or_default()
    }

    /// Get parent of a task
    pub async fn get_parent(&self, task_id: &I) -> Option<I> {
        let parents = self.parents.read().await;
        parents.get(task_id).copied()
    }

    /// Remove all relationships for a task
    pub async fn remove_task(&self, task_id: &I) {
        // Remove from parents
        {
            let mut parents = self.parents.write().await;
            parents.remove(task_id);
        }
        
        // Remove from children maps
        {
            let mut children = self.children.write().await;
            children.remove(task_id);
            for child_set in children.values_mut() {
                child_set.remove(task_id);
            }
        }
        
        // Remove from dependencies
        {
            let mut dependencies = self.dependencies.write().await;
            dependencies.remove(task_id);
            for dep_set in dependencies.values_mut() {
                dep_set.remove(task_id);
            }
        }
        
        // Remove from dependents
        {
            let mut dependents = self.dependents.write().await;
            dependents.remove(task_id);
            for dep_set in dependents.values_mut() {
                dep_set.remove(task_id);
            }
        }
    }
}

impl<I: TaskId> Default for TaskRelationshipManager<I> {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for Tokio-specific task relationships
/// Provides a consistent naming convention with other Tokio types
pub type TokioTaskRelationships<T, I> = TaskRelationshipManager<I>;