use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use sweet_async_api::orchestra::orchestrator::{OrchestratorError, TaskOrchestrator};
use sweet_async_api::task::{AsyncTask as ApiAsyncTask, AsyncTaskError, TaskId, TaskStatus};

use tokio::sync::Mutex;

use crate::runtime::{TokioRuntime, safe_blocking};
use crate::task::async_task::AsyncTask;

/// Tokio-specific implementation of TaskOrchestrator
pub struct TokioOrchestrator<T: Clone + Send + 'static, I: TaskId> {
    /// Registry of all tasks managed by this orchestrator
    tasks: Arc<Mutex<HashMap<I, Arc<AsyncTask<T, I>>>>>,
    /// Task dependencies (dependent_id -> set of dependency_ids)
    dependencies: Arc<Mutex<HashMap<I, HashSet<I>>>>,
    /// Reverse dependencies (dependency_id -> set of dependent_ids)
    reverse_dependencies: Arc<Mutex<HashMap<I, HashSet<I>>>>,
    /// Task groups (group_name -> set of task_ids)
    groups: Arc<Mutex<HashMap<String, HashSet<I>>>>,
    /// Runtime for executing tasks
    runtime: Arc<TokioRuntime>,
}

impl<T: Clone + Send + 'static, I: TaskId> TokioOrchestrator<T, I> {
    /// Create a new TokioOrchestrator with the given runtime
    pub fn new(runtime: TokioRuntime) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            dependencies: Arc::new(Mutex::new(HashMap::new())),
            reverse_dependencies: Arc::new(Mutex::new(HashMap::new())),
            groups: Arc::new(Mutex::new(HashMap::new())),
            runtime: Arc::new(runtime),
        }
    }
    
    /// Create an orchestrated task with the given runtime and orchestrator
    /// 
    /// Note: Currently disabled until the builder pattern is fully implemented
    #[allow(dead_code)]
    pub async fn orchestrate<R, F>(runtime: TokioRuntime, f: F) -> Result<R, sweet_async_api::task::AsyncTaskError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
        I: TaskId,
    {
        unimplemented!("orchestrate is not yet implemented - will be added in a future PR after the builder pattern is completed")
    }
    
    /// Helper method to check for dependency cycles
    async fn has_dependency_cycle(&self, dependent_id: &I, dependency_id: &I) -> bool {
        // Helper function to check if target is in the dependency chain of start
        async fn is_in_dependency_chain<Id: TaskId>(
            dependencies: &Mutex<HashMap<Id, HashSet<Id>>>,
            start: &Id,
            target: &Id,
            visited: &mut HashSet<Id>,
        ) -> bool {
            if start == target {
                return true;
            }
            
            if !visited.insert(*start) {
                return false; // Already visited
            }
            
            let deps = dependencies.lock().await;
            if let Some(direct_deps) = deps.get(start) {
                for dep in direct_deps {
                    if is_in_dependency_chain(dependencies, dep, target, visited).await {
                        return true;
                    }
                }
            }
            
            false
        }
        
        let mut visited = HashSet::new();
        is_in_dependency_chain(&self.dependencies, dependency_id, dependent_id, &mut visited).await
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TaskOrchestrator<T, AsyncTask<T, I>, I> for TokioOrchestrator<T, I> {
    type RegisterTaskReturn = Arc<AsyncTask<T, I>>;
    type StartTaskFuture = Pin<Box<dyn Future<Output = Result<T, AsyncTaskError>> + Send>>;
    type StartAllFuture = Pin<Box<dyn Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send>>;
    type JoinAllFuture = Pin<Box<dyn Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send>>;
    type StartGroupFuture = Pin<Box<dyn Future<Output = Vec<(I, Result<T, AsyncTaskError>)>> + Send>>;

    fn register_task(&self, task: AsyncTask<T, I>) -> Self::RegisterTaskReturn {
        let task_arc = Arc::new(task);
        let task_id = task_arc.task_id();
        
        // Store the task safely using block_in_place when in a Tokio context
        safe_blocking(|| {
            self.runtime.handle().block_on(async {
                let mut tasks = self.tasks.lock().await;
                tasks.insert(task_id, task_arc.clone());
            })
        });
        
        task_arc
    }

    fn add_dependency(&self, dependent_id: &I, dependency_id: &I) -> Result<(), OrchestratorError> {
        safe_blocking(|| {
            self.runtime.handle().block_on(async {
                // Check if both tasks exist
                let tasks = self.tasks.lock().await;
                if !tasks.contains_key(dependent_id) {
                    return Err(OrchestratorError::TaskNotFound(dependent_id.to_string()));
                }
                if !tasks.contains_key(dependency_id) {
                    return Err(OrchestratorError::TaskNotFound(dependency_id.to_string()));
                }
                
                // Check for dependency cycles
                if self.has_dependency_cycle(dependent_id, dependency_id).await {
                    return Err(OrchestratorError::DependencyCycle(format!(
                        "Adding dependency from {} to {} would create a cycle",
                        dependent_id.to_string(),
                        dependency_id.to_string()
                    )));
                }
                
                // Add dependency
                {
                    let mut deps = self.dependencies.lock().await;
                    deps.entry(*dependent_id)
                        .or_insert_with(HashSet::new)
                        .insert(*dependency_id);
                }
                
                // Add reverse dependency
                {
                    let mut rev_deps = self.reverse_dependencies.lock().await;
                    rev_deps.entry(*dependency_id)
                        .or_insert_with(HashSet::new)
                        .insert(*dependent_id);
                }
                
                Ok(())
            })
        })
    }

    fn start_task(&self, task_id: &I) -> Self::StartTaskFuture {
        let task_id = *task_id;
        let tasks = self.tasks.clone();
        let dependencies = self.dependencies.clone();
        
        Box::pin(async move {
            // Get the task
            let task = {
                let tasks_lock = tasks.lock().await;
                if let Some(task) = tasks_lock.get(&task_id) {
                    task.clone()
                } else {
                    return Err(AsyncTaskError::Failure(
                        format!("Task {} not found", task_id.to_string())
                    ));
                }
            };
            
            // Check if all dependencies are complete
            let deps_complete = {
                let deps_lock = dependencies.lock().await;
                let tasks_lock = tasks.lock().await;
                
                if let Some(deps) = deps_lock.get(&task_id) {
                    // Check if all dependencies are complete
                    for dep_id in deps {
                        if let Some(dep_task) = tasks_lock.get(dep_id) {
                            match dep_task.status() {
                                TaskStatus::Completed => continue,
                                _ => return false,
                            }
                        } else {
                            return false;
                        }
                    }
                }
                
                true
            };
            
            if !deps_complete {
                return Err(AsyncTaskError::InvalidState(
                    "Not all dependencies are complete".to_string()
                ));
            }
            
            // Start the task
            task.clone().into_future().await
        })
    }

    fn start_all(&self) -> Self::StartAllFuture {
        let tasks = self.tasks.clone();
        let dependencies = self.dependencies.clone();
        
        Box::pin(async move {
            let mut results = Vec::new();
            
            // Get all task IDs
            let task_ids = {
                let tasks_lock = tasks.lock().await;
                tasks_lock.keys().copied().collect::<Vec<_>>()
            };
            
            // Calculate a topological sort of tasks based on dependencies
            let mut execution_order = Vec::new();
            let mut visited = HashSet::new();
            let mut temp_visited = HashSet::new();
            
            async fn visit<Id: TaskId, T: Send + 'static>(
                task_id: Id,
                dependencies: &Mutex<HashMap<Id, HashSet<Id>>>,
                tasks: &Mutex<HashMap<Id, Arc<AsyncTask<T, Id>>>>,
                visited: &mut HashSet<Id>,
                temp_visited: &mut HashSet<Id>,
                execution_order: &mut Vec<Id>,
            ) {
                if visited.contains(&task_id) {
                    return;
                }
                
                if temp_visited.contains(&task_id) {
                    // Cycle detected, but we've already checked for cycles in add_dependency
                    return;
                }
                
                temp_visited.insert(task_id);
                
                // Visit all dependencies first
                let deps = {
                    let deps_lock = dependencies.lock().await;
                    deps_lock.get(&task_id)
                        .map(|d| d.iter().copied().collect::<Vec<_>>())
                        .unwrap_or_default()
                };
                
                for dep_id in deps {
                    visit(dep_id, dependencies, tasks, visited, temp_visited, execution_order).await;
                }
                
                temp_visited.remove(&task_id);
                visited.insert(task_id);
                execution_order.push(task_id);
            }
            
            // Build execution order
            for task_id in task_ids {
                if !visited.contains(&task_id) {
                    visit(
                        task_id,
                        &dependencies,
                        &tasks,
                        &mut visited,
                        &mut temp_visited,
                        &mut execution_order,
                    ).await;
                }
            }
            
            // Execute tasks in order
            for task_id in execution_order {
                let task = {
                    let tasks_lock = tasks.lock().await;
                    tasks_lock.get(&task_id).cloned()
                };
                
                if let Some(task) = task {
                    let result = task.clone().into_future().await;
                    results.push((task_id, result));
                }
            }
            
            results
        })
    }

    fn cancel_task(&self, task_id: &I) -> Result<(), OrchestratorError> {
        futures::executor::block_on(async {
            // Get the task
            let task = {
                let tasks_lock = self.tasks.lock().await;
                if let Some(task) = tasks_lock.get(task_id) {
                    task.clone()
                } else {
                    return Err(OrchestratorError::TaskNotFound(task_id.to_string()));
                }
            };
            
            // Cancel the task
            task.cancel_gracefully().await.map_err(|e| {
                OrchestratorError::OperationFailed(format!("Failed to cancel task: {:?}", e))
            })?;
            
            // Cancel all dependent tasks
            let dependent_tasks = {
                let rev_deps_lock = self.reverse_dependencies.lock().await;
                rev_deps_lock.get(task_id)
                    .map(|d| d.iter().copied().collect::<Vec<_>>())
                    .unwrap_or_default()
            };
            
            for dep_id in dependent_tasks {
                self.cancel_task(&dep_id)?;
            }
            
            Ok(())
        })
    }

    fn task_status(&self, task_id: &I) -> Option<TaskStatus> {
        futures::executor::block_on(async {
            let tasks_lock = self.tasks.lock().await;
            tasks_lock.get(task_id).map(|t| t.status())
        })
    }

    fn all_task_statuses(&self) -> Vec<(I, TaskStatus)> {
        futures::executor::block_on(async {
            let tasks_lock = self.tasks.lock().await;
            tasks_lock.iter()
                .map(|(id, task)| (*id, task.status()))
                .collect()
        })
    }

    fn join_all(&self) -> Self::JoinAllFuture {
        let tasks = self.tasks.clone();
        
        Box::pin(async move {
            let mut results = Vec::new();
            
            // Get all tasks
            let all_tasks = {
                let tasks_lock = tasks.lock().await;
                tasks_lock.iter()
                    .map(|(id, task)| (*id, task.clone()))
                    .collect::<Vec<_>>()
            };
            
            // Wait for all tasks to complete
            for (id, task) in all_tasks {
                // Use Future trait directly to avoid moving task
                let result = task.await;
                results.push((id, result));
            }
            
            results
        })
    }

    fn create_group(&self, group_name: &str) -> Result<(), OrchestratorError> {
        futures::executor::block_on(async {
            let mut groups_lock = self.groups.lock().await;
            if groups_lock.contains_key(group_name) {
                return Err(OrchestratorError::GroupAlreadyExists(group_name.to_string()));
            }
            
            groups_lock.insert(group_name.to_string(), HashSet::new());
            Ok(())
        })
    }

    fn add_task_to_group(&self, task_id: &I, group_name: &str) -> Result<(), OrchestratorError> {
        futures::executor::block_on(async {
            // Check if task exists
            {
                let tasks_lock = self.tasks.lock().await;
                if !tasks_lock.contains_key(task_id) {
                    return Err(OrchestratorError::TaskNotFound(task_id.to_string()));
                }
            }
            
            // Add task to group
            let mut groups_lock = self.groups.lock().await;
            if let Some(group) = groups_lock.get_mut(group_name) {
                group.insert(*task_id);
                Ok(())
            } else {
                Err(OrchestratorError::GroupNotFound(group_name.to_string()))
            }
        })
    }

    fn start_group(&self, group_name: &str) -> Self::StartGroupFuture {
        let groups = self.groups.clone();
        let tasks = self.tasks.clone();
        
        Box::pin(async move {
            let mut results = Vec::new();
            
            // Get all tasks in the group
            let group_tasks = {
                let groups_lock = groups.lock().await;
                let tasks_lock = tasks.lock().await;
                
                if let Some(group) = groups_lock.get(group_name) {
                    group.iter()
                        .filter_map(|id| tasks_lock.get(id).map(|t| (*id, t.clone())))
                        .collect::<Vec<_>>()
                } else {
                    return Vec::new(); // Group not found
                }
            };
            
            // Start all tasks
            for (id, task) in group_tasks {
                let result = task.clone().into_future().await;
                results.push((id, result));
            }
            
            results
        })
    }

    fn cancel_group(&self, group_name: &str) -> usize {
        safe_blocking(|| {
            self.runtime.handle().block_on(async {
                // Get all tasks in the group
                let group_task_ids = {
                    let groups_lock = self.groups.lock().await;
                    if let Some(group) = groups_lock.get(group_name) {
                        group.iter().copied().collect::<Vec<_>>()
                    } else {
                        return 0; // Group not found
                    }
                };
                
                // Cancel all tasks
                let mut cancelled_count = 0;
                for task_id in group_task_ids {
                    if self.cancel_task(&task_id).is_ok() {
                        cancelled_count += 1;
                    }
                }
                
                cancelled_count
            })
        })
    }
}
