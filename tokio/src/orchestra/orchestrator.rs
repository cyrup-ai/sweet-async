//! Production-grade Tokio implementation of TaskOrchestrator trait

use sweet_async_api::orchestra::{OrchestratorError, TaskOrchestrator};
use sweet_async_api::task::{AsyncTaskError, TaskStatus};
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tokio::sync::oneshot;

/// Production-grade Tokio implementation of TaskOrchestrator trait
#[derive(Debug)]
pub struct TokioOrchestrator<T, Task, I> {
    /// Task registry with thread-safe access
    tasks: Arc<RwLock<HashMap<I, Task>>>,
    /// Task dependency graph
    dependencies: Arc<RwLock<HashMap<I, HashSet<I>>>>,
    /// Reverse dependency lookup for cascade cancellation
    dependents: Arc<RwLock<HashMap<I, HashSet<I>>>>,
    /// Task groups for batch operations
    groups: Arc<RwLock<HashMap<String, HashSet<I>>>>,
    /// Task status tracking
    task_statuses: Arc<RwLock<HashMap<I, TaskStatus>>>,
    /// Completion channels for task coordination
    completion_channels: Arc<RwLock<HashMap<I, oneshot::Sender<Result<T, AsyncTaskError>>>>>,
    /// Task result storage
    task_results: Arc<RwLock<HashMap<I, Result<T, AsyncTaskError>>>>,
}

impl<T, Task, I> TokioOrchestrator<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: sweet_async_api::AsyncTask<T, I>,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            dependents: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            task_statuses: Arc::new(RwLock::new(HashMap::new())),
            completion_channels: Arc::new(RwLock::new(HashMap::new())),
            task_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check for dependency cycles using topological sort
    fn has_dependency_cycle(&self, dependent_id: &I, dependency_id: &I) -> bool {
        let dependencies = self.dependencies.read().unwrap();
        
        // Build adjacency list including the new dependency
        let mut graph: HashMap<I, Vec<I>> = HashMap::new();
        for (task, deps) in dependencies.iter() {
            graph.insert(task.clone(), deps.iter().cloned().collect());
        }
        
        // Add the proposed new dependency
        graph.entry(dependent_id.clone())
            .or_default()
            .push(dependency_id.clone());
        
        // Kahn's algorithm for cycle detection
        let mut in_degree: HashMap<I, usize> = HashMap::new();
        for task in graph.keys() {
            in_degree.insert(task.clone(), 0);
        }
        
        for deps in graph.values() {
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }
        
        let mut queue: VecDeque<I> = VecDeque::new();
        for (task, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(task.clone());
            }
        }
        
        let mut processed = 0;
        while let Some(task) = queue.pop_front() {
            processed += 1;
            if let Some(deps) = graph.get(&task) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }
        
        processed != graph.len()
    }

    /// Get tasks ready to execute (no pending dependencies)
    fn get_ready_tasks(&self) -> Vec<I> {
        let dependencies = self.dependencies.read().unwrap();
        let statuses = self.task_statuses.read().unwrap();
        let mut ready = Vec::new();
        
        for (task_id, deps) in dependencies.iter() {
            if let Some(status) = statuses.get(task_id) {
                if *status == TaskStatus::Pending {
                    let all_deps_complete = deps.iter().all(|dep_id| {
                        statuses.get(dep_id)
                            .map(|s| *s == TaskStatus::Completed)
                            .unwrap_or(false)
                    });
                    
                    if all_deps_complete {
                        ready.push(task_id.clone());
                    }
                }
            }
        }
        
        ready
    }
}

impl<T, Task, I> Default for TokioOrchestrator<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: sweet_async_api::AsyncTask<T, I>,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Task, I> TaskOrchestrator<T, Task, I> for TokioOrchestrator<T, Task, I>
where
    T: Clone + Send + 'static,
    Task: sweet_async_api::AsyncTask<T, I>,
    I: sweet_async_api::TaskId + std::hash::Hash + Eq + Clone,
{
    type RegisterTaskReturn = Result<(), OrchestratorError>;
    type StartTaskFuture = Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>>;
    type StartAllFuture = Pin<Box<dyn Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send>>;
    type JoinAllFuture = Pin<Box<dyn Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send>>;
    type StartGroupFuture = Pin<Box<dyn Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send>>;

    fn register_task(&self, task: Task) -> Self::RegisterTaskReturn {
        let task_id = task.task_id();
        
        let mut tasks = self.tasks.write().unwrap();
        let mut statuses = self.task_statuses.write().unwrap();
        let mut dependencies = self.dependencies.write().unwrap();
        
        if tasks.contains_key(&task_id) {
            return Err(OrchestratorError::TaskAlreadyExists(format!("{:?}", task_id)));
        }
        
        tasks.insert(task_id.clone(), task);
        statuses.insert(task_id.clone(), TaskStatus::Pending);
        dependencies.insert(task_id, HashSet::new());
        
        Ok(())
    }

    fn add_dependency(&self, dependent_id: &I, dependency_id: &I) -> Result<(), OrchestratorError> {
        let tasks = self.tasks.read().unwrap();
        
        if !tasks.contains_key(dependent_id) {
            return Err(OrchestratorError::TaskNotFound(format!("{:?}", dependent_id)));
        }
        
        if !tasks.contains_key(dependency_id) {
            return Err(OrchestratorError::TaskNotFound(format!("{:?}", dependency_id)));
        }
        
        // Check for cycles before adding dependency
        if self.has_dependency_cycle(dependent_id, dependency_id) {
            return Err(OrchestratorError::DependencyCycle(
                format!("{:?} -> {:?}", dependent_id, dependency_id)
            ));
        }
        
        let mut dependencies = self.dependencies.write().unwrap();
        let mut dependents = self.dependents.write().unwrap();
        
        dependencies.entry(dependent_id.clone())
            .or_default()
            .insert(dependency_id.clone());
        
        dependents.entry(dependency_id.clone())
            .or_default()
            .insert(dependent_id.clone());
        
        Ok(())
    }

    fn start_task(&self, task_id: &I) -> Self::StartTaskFuture {
        let task_id = task_id.clone();
        let tasks = self.tasks.clone();
        let statuses = self.task_statuses.clone();
        let results = self.task_results.clone();
        
        Box::pin(async move {
            // Update status to running
            {
                let mut statuses = statuses.write().unwrap();
                statuses.insert(task_id.clone(), TaskStatus::Running);
            }
            
            // Execute the task
            let result = {
                let tasks = tasks.read().unwrap();
                if let Some(task) = tasks.get(&task_id) {
                    // Execute the task by calling its run method
                    task.run().await
                } else {
                    Err(AsyncTaskError::TaskNotFound)
                }
            };
            
            // Update status and store result
            {
                let mut statuses = statuses.write().unwrap();
                let mut results = results.write().unwrap();
                
                match &result {
                    Ok(_) => statuses.insert(task_id.clone(), TaskStatus::Completed),
                    Err(_) => statuses.insert(task_id.clone(), TaskStatus::Failed),
                };
                
                results.insert(task_id, result.clone());
            }
            
            result
        })
    }

    fn start_all(&self) -> Self::StartAllFuture {
        let tasks = self.tasks.clone();
        let statuses = self.task_statuses.clone();
        let results = self.task_results.clone();
        
        Box::pin(async move {
            let task_ids: Vec<I> = {
                let tasks = tasks.read().unwrap();
                tasks.keys().cloned().collect()
            };
            
            let mut task_results = Vec::new();
            
            for task_id in task_ids {
                // Update status to running
                {
                    let mut statuses = statuses.write().unwrap();
                    statuses.insert(task_id.clone(), TaskStatus::Running);
                }
                
                // Execute the task
                let result = {
                    let tasks = tasks.read().unwrap();
                    if let Some(task) = tasks.get(&task_id) {
                        // Execute the task by polling its future
                        task.poll_to_completion().await
                    } else {
                        Err(AsyncTaskError::TaskNotFound)
                    }
                };
                
                // Update status and store result
                {
                    let mut statuses = statuses.write().unwrap();
                    let mut results = results.write().unwrap();
                    
                    match &result {
                        Ok(_) => statuses.insert(task_id.clone(), TaskStatus::Completed),
                        Err(_) => statuses.insert(task_id.clone(), TaskStatus::Failed),
                    };
                    
                    results.insert(task_id.clone(), result.clone());
                }
                
                task_results.push((task_id, result));
            }
            
            task_results
        })
    }

    fn cancel_task(&self, task_id: &I) -> Result<(), OrchestratorError> {
        let mut statuses = self.task_statuses.write().unwrap();
        
        if !statuses.contains_key(task_id) {
            return Err(OrchestratorError::TaskNotFound(format!("{:?}", task_id)));
        }
        
        // Cancel the task and all its dependents
        let dependents = self.dependents.read().unwrap();
        let mut to_cancel = vec![task_id.clone()];
        
        if let Some(deps) = dependents.get(task_id) {
            to_cancel.extend(deps.iter().cloned());
        }
        
        for id in to_cancel {
            statuses.insert(id, TaskStatus::Cancelled);
        }
        
        Ok(())
    }

    fn task_status(&self, task_id: &I) -> Option<TaskStatus> {
        let statuses = self.task_statuses.read().unwrap();
        statuses.get(task_id).copied()
    }

    fn all_task_statuses(&self) -> Vec<(I, TaskStatus)> {
        let statuses = self.task_statuses.read().unwrap();
        statuses.iter().map(|(id, status)| (id.clone(), *status)).collect()
    }

    fn join_all(&self) -> Self::JoinAllFuture {
        let results = self.task_results.clone();
        
        Box::pin(async move {
            let results = results.read().unwrap();
            results.iter().map(|(id, result)| (id.clone(), result.clone())).collect()
        })
    }

    fn create_group(&self, group_name: &str) -> Result<(), OrchestratorError> {
        let mut groups = self.groups.write().unwrap();
        
        if groups.contains_key(group_name) {
            return Err(OrchestratorError::GroupAlreadyExists(group_name.to_string()));
        }
        
        groups.insert(group_name.to_string(), HashSet::new());
        Ok(())
    }

    fn add_task_to_group(&self, task_id: &I, group_name: &str) -> Result<(), OrchestratorError> {
        let tasks = self.tasks.read().unwrap();
        let mut groups = self.groups.write().unwrap();
        
        if !tasks.contains_key(task_id) {
            return Err(OrchestratorError::TaskNotFound(format!("{:?}", task_id)));
        }
        
        let group = groups.get_mut(group_name)
            .ok_or_else(|| OrchestratorError::GroupNotFound(group_name.to_string()))?;
        
        group.insert(task_id.clone());
        Ok(())
    }

    fn start_group(&self, group_name: &str) -> Self::StartGroupFuture {
        let group_name = group_name.to_string();
        let groups = self.groups.clone();
        let tasks = self.tasks.clone();
        let statuses = self.task_statuses.clone();
        let results = self.task_results.clone();
        
        Box::pin(async move {
            let task_ids: Vec<I> = {
                let groups = groups.read().unwrap();
                groups.get(&group_name)
                    .map(|group| group.iter().cloned().collect())
                    .unwrap_or_default()
            };
            
            let mut task_results = Vec::new();
            
            for task_id in task_ids {
                // Update status to running
                {
                    let mut statuses = statuses.write().unwrap();
                    statuses.insert(task_id.clone(), TaskStatus::Running);
                }
                
                // Execute the task
                let result = {
                    let tasks = tasks.read().unwrap();
                    if let Some(task) = tasks.get(&task_id) {
                        // Execute the task by polling its future
                        task.poll_to_completion().await
                    } else {
                        Err(AsyncTaskError::TaskNotFound)
                    }
                };
                
                // Update status and store result
                {
                    let mut statuses = statuses.write().unwrap();
                    let mut results = results.write().unwrap();
                    
                    match &result {
                        Ok(_) => statuses.insert(task_id.clone(), TaskStatus::Completed),
                        Err(_) => statuses.insert(task_id.clone(), TaskStatus::Failed),
                    };
                    
                    results.insert(task_id.clone(), result.clone());
                }
                
                task_results.push((task_id, result));
            }
            
            task_results
        })
    }

    fn cancel_group(&self, group_name: &str) -> usize {
        let groups = self.groups.read().unwrap();
        let mut statuses = self.task_statuses.write().unwrap();
        
        if let Some(group) = groups.get(group_name) {
            let mut cancelled_count = 0;
            for task_id in group {
                statuses.insert(task_id.clone(), TaskStatus::Cancelled);
                cancelled_count += 1;
            }
            cancelled_count
        } else {
            0
        }
    }
}
