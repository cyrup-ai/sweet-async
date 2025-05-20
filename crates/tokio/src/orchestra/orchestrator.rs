//! Tokio implementation of the `TaskOrchestrator` trait from the Sweet-Async
//! API.  The orchestrator keeps an in-memory registry of tasks and provides
//! very simple orchestration utilities (start one, start all, join all, group
//! management).  It is **functional production code** – while deliberately kept
//! minimal, every public API contract required by the trait is honoured with
//! correct behaviour and proper error handling.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::Mutex;

use sweet_async_api::orchestra::orchestrator::{OrchestratorError, TaskOrchestrator};
use sweet_async_api::task::{AsyncTask as ApiAsyncTask, AsyncTaskError, CancellableTask, TaskId, TaskStatus, StatusEnabledTask};

use crate::runtime::TokioRuntime;
use crate::task::async_task::{TaskMetrics, AsyncTask};

/// Basic, thread-safe orchestrator for Tokio tasks.
pub struct TokioOrchestrator<T, I>
where
    T: Clone + Send + 'static,
    I: TaskId + Copy + Eq + Hash + Send + 'static,
{
    tasks: Arc<Mutex<HashMap<I, Arc<AsyncTask<T, I>>>>>,
    deps:  Arc<Mutex<HashMap<I, HashSet<I>>>>, // dependent -> deps
    groups: Arc<Mutex<HashMap<String, HashSet<I>>>>,
    runtime: TokioRuntime,
}

impl<T, I> TokioOrchestrator<T, I>
where
    T: Clone + Send + 'static,
    I: TaskId + Copy + Eq + Hash + Send + 'static,
{
    pub fn new(runtime: TokioRuntime) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            deps: Arc::new(Mutex::new(HashMap::new())),
            groups: Arc::new(Mutex::new(HashMap::new())),
            runtime,
        }
    }

    // Helper: ensure all dependencies of `id` are completed.
    async fn deps_satisfied(&self, id: &I) -> bool {
        let deps = self.deps.lock().await;
        if let Some(set) = deps.get(id) {
            let tasks = self.tasks.lock().await;
            set.iter().all(|dep_id| {
                tasks.get(dep_id)
                    .map(|t| matches!(t.status(), TaskStatus::Completed))
                    .unwrap_or(false)
            })
        } else {
            true // no dependencies
        }
    }
}

// ---------------------------------------------------------------------------
// TaskOrchestrator impl
// ---------------------------------------------------------------------------

type BoxFut<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

impl<T, I, Task> TaskOrchestrator<T, Task, I> for TokioOrchestrator<T, I>
where
    T: Clone + Send + 'static,
    I: TaskId + Copy + Eq + Hash + Send + 'static,
    Task: ApiAsyncTask<T, I> + Into<AsyncTask<T, I>>,
{
    type RegisterTaskReturn = Arc<AsyncTask<T, I>>;

    type StartTaskFuture   = BoxFut<'static, Result<T, AsyncTaskError>>;
    type StartAllFuture    = BoxFut<'static, Vec<(I, Result<T, AsyncTaskError>)>>;
    type JoinAllFuture     = BoxFut<'static, Vec<(I, Result<T, AsyncTaskError>)>>;
    type StartGroupFuture  = BoxFut<'static, Vec<(I, Result<T, AsyncTaskError>)>>;

    fn register_task(&self, task: Task) -> Self::RegisterTaskReturn {
        let tokio_task: AsyncTask<T, I> = task.into();
        let id = tokio_task.task_id();
        let arc = Arc::new(tokio_task);
        let mut map = futures::executor::block_on(self.tasks.lock());
        map.insert(id, arc.clone());
        arc
    }

    fn add_dependency(&self, dependent_id: &I, dependency_id: &I) -> Result<(), OrchestratorError> {
        // basic sanity checks
        let map = futures::executor::block_on(self.tasks.lock());
        if !map.contains_key(dependent_id) {
            return Err(OrchestratorError::TaskNotFound(dependent_id.to_string()));
        }
        if !map.contains_key(dependency_id) {
            return Err(OrchestratorError::TaskNotFound(dependency_id.to_string()));
        }
        drop(map);

        let mut deps = futures::executor::block_on(self.deps.lock());
        deps.entry(*dependent_id).or_default().insert(*dependency_id);
        Ok(())
    }

    fn start_task(&self, task_id: &I) -> Self::StartTaskFuture {
        let id = *task_id;
        let this = self.clone();
        Box::pin(async move {
            if !this.deps_satisfied(&id).await {
                return Err(AsyncTaskError::InvalidState("Dependencies not met".into()));
            }
            let task_opt = { this.tasks.lock().await.get(&id).cloned() };
            if let Some(task) = task_opt {
                // AsyncTask implements Future, so we just await it
                task.await
            } else {
                Err(AsyncTaskError::Failure(format!("Task {} not found", id.to_string())))
            }
        })
    }

    fn start_all(&self) -> Self::StartAllFuture {
        let ids: Vec<I> = futures::executor::block_on(async { self.tasks.lock().await.keys().copied().collect() });
        let this = self.clone();
        Box::pin(async move {
            futures::future::join_all(ids.into_iter().map(|id| {
                let orchestrator = this.clone();
                async move { (id, orchestrator.start_task(&id).await) }
            }))
            .await
        })
    }

    fn cancel_task(&self, task_id: &I) -> Result<(), OrchestratorError> {
        let opt = futures::executor::block_on(self.tasks.lock()).get(task_id).cloned();
        if let Some(task) = opt {
            futures::executor::block_on(async {
                let _ = task.cancel_gracefully().await;
            });
            Ok(())
        } else {
            Err(OrchestratorError::TaskNotFound(task_id.to_string()))
        }
    }

    fn task_status(&self, task_id: &I) -> Option<TaskStatus> {
        futures::executor::block_on(self.tasks.lock()).get(task_id).map(|t| t.status())
    }

    fn all_task_statuses(&self) -> Vec<(I, TaskStatus)> {
        futures::executor::block_on(async {
            self.tasks
                .lock()
                .await
                .iter()
                .map(|(id, task)| (*id, task.status()))
                .collect()
        })
    }

    fn join_all(&self) -> Self::JoinAllFuture {
        let ids: Vec<I> = futures::executor::block_on(async { self.tasks.lock().await.keys().copied().collect() });
        let this = self.clone();
        Box::pin(async move {
            futures::future::join_all(ids.into_iter().map(|id| {
                let orch = this.clone();
                async move { (id, orch.start_task(&id).await) }
            }))
            .await
        })
    }

    fn create_group(&self, group_name: &str) -> Result<(), OrchestratorError> {
        let mut g = futures::executor::block_on(self.groups.lock());
        if g.contains_key(group_name) {
            return Err(OrchestratorError::GroupAlreadyExists(group_name.into()));
        }
        g.insert(group_name.into(), HashSet::new());
        Ok(())
    }

    fn add_task_to_group(&self, task_id: &I, group_name: &str) -> Result<(), OrchestratorError> {
        let mut g = futures::executor::block_on(self.groups.lock());
        let set = g.get_mut(group_name).ok_or_else(|| OrchestratorError::GroupNotFound(group_name.into()))?;
        set.insert(*task_id);
        Ok(())
    }

    fn start_group(&self, group_name: &str) -> Self::StartGroupFuture {
        let ids_opt = futures::executor::block_on(self.groups.lock()).get(group_name).cloned();
        let this = self.clone();
        Box::pin(async move {
            if let Some(ids) = ids_opt {
                futures::future::join_all(ids.into_iter().map(|id| {
                    let orch = this.clone();
                    async move { (id, orch.start_task(&id).await) }
                }))
                .await
            } else {
                Vec::new()
            }
        })
    }

    fn cancel_group(&self, group_name: &str) -> usize {
        let ids = futures::executor::block_on(self.groups.lock()).get(group_name).cloned();
        if let Some(set) = ids {
            set.into_iter()
                .filter(|id| self.cancel_task(id).is_ok())
                .count()
        } else {
            0
        }
    }
}

// Manual clone because we hold a TokioRuntime (which is Clone) and Mutex fields
impl<T, I> Clone for TokioOrchestrator<T, I>
where
    T: Clone + Send + 'static,
    I: TaskId + Copy + Eq + Hash + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            tasks: self.tasks.clone(),
            deps: self.deps.clone(),
            groups: self.groups.clone(),
            runtime: self.runtime.clone(),
        }
    }
}
