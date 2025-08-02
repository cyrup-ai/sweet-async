//! Task relationship management

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;
use sweet_async_api::task::TaskId;

/// Manages parent-child and dependency relationships between tasks
#[derive(Debug)]
pub struct TaskRelationshipManager<I: TaskId + Hash> {
    /// Map of task ID to its parent
    parents: Arc<RwLock<HashMap<I, I>>>,
    /// Map of task ID to its children
    children: Arc<RwLock<HashMap<I, HashSet<I>>>>,
    /// Map of task ID to its dependencies
    dependencies: Arc<RwLock<HashMap<I, HashSet<I>>>>,
    /// Map of task ID to tasks that depend on it
    dependents: Arc<RwLock<HashMap<I, HashSet<I>>>>,
}

impl<I: TaskId + Hash> TaskRelationshipManager<I> {
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
    
    /// Get all dependencies of a task
    pub async fn get_dependencies(&self, task_id: &I) -> Vec<I> {
        let deps = self.dependencies.read().await;
        deps.get(task_id).map(|set| set.iter().copied().collect()).unwrap_or_default()
    }
    
    /// Check if a task has any dependencies
    pub async fn has_dependencies(&self, task_id: &I) -> bool {
        let deps = self.dependencies.read().await;
        deps.get(task_id).map(|set| !set.is_empty()).unwrap_or(false)
    }
    
    /// Remove a completed dependency from a task
    pub async fn remove_completed_dependency(&self, task_id: I, completed_dependency: I) {
        let mut deps = self.dependencies.write().await;
        if let Some(dep_set) = deps.get_mut(&task_id) {
            dep_set.remove(&completed_dependency);
        }
        
        let mut dependents = self.dependents.write().await;
        if let Some(dependent_set) = dependents.get_mut(&completed_dependency) {
            dependent_set.remove(&task_id);
        }
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

impl<I: TaskId + Hash> Default for TaskRelationshipManager<I> {
    fn default() -> Self {
        Self::new()
    }
}

/// Tokio-specific task relationships that track the result type T
pub struct TokioTaskRelationships<T, I: TaskId + Hash> {
    manager: TaskRelationshipManager<I>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, I: TaskId + Hash> TokioTaskRelationships<T, I> {
    pub fn new() -> Self {
        Self {
            manager: TaskRelationshipManager::new(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    pub async fn add_dependency(&self, task_id: I, depends_on: I) {
        self.manager.add_dependency(task_id, depends_on).await;
    }
    
    pub async fn add_child(&self, parent_id: I, child_id: I) {
        self.manager.add_child(parent_id, child_id).await;
    }
    
    pub async fn get_children(&self, task_id: &I) -> Vec<I> {
        self.manager.get_children(task_id).await
    }
    
    pub async fn get_dependencies(&self, task_id: &I) -> Vec<I> {
        self.manager.get_dependencies(task_id).await
    }
    
    pub async fn has_dependencies(&self, task_id: &I) -> bool {
        self.manager.has_dependencies(task_id).await
    }
    
    pub async fn remove_completed_dependency(&self, task_id: I, completed_dependency: I) {
        self.manager.remove_completed_dependency(task_id, completed_dependency).await;
    }
}

impl<T, I: TaskId + Hash> Default for TokioTaskRelationships<T, I> {
    fn default() -> Self {
        Self::new()
    }
}