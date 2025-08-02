//! Channel-based orchestrator implementation without Arc<Mutex<>>
//! 
//! This implementation uses message passing for all state management,
//! eliminating the need for shared mutable state.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{mpsc, oneshot};

use sweet_async_api::orchestra::orchestrator::{OrchestratorError, TaskOrchestrator};
use sweet_async_api::task::{AsyncTask as ApiAsyncTask, AsyncTaskError, CancellableTask, TaskId, TaskStatus, StatusEnabledTask};

use crate::orchestra::runtime::TokioRuntime;
use crate::task::tokio_task::TokioTask;

/// Messages for orchestrator operations
enum OrchestratorMessage<T: Clone + Send + Sync + 'static, I: TaskId> {
    RegisterTask {
        task: TokioTask<T, I>,
        respond: oneshot::Sender<I>,
    },
    AddDependency {
        dependent_id: I,
        dependency_id: I,
        respond: oneshot::Sender<Result<(), OrchestratorError>>,
    },
    GetTask {
        id: I,
        respond: oneshot::Sender<Option<TokioTask<T, I>>>,
    },
    GetTaskStatus {
        id: I,
        respond: oneshot::Sender<Option<TaskStatus>>,
    },
    GetAllStatuses {
        respond: oneshot::Sender<Vec<(I, TaskStatus)>>,
    },
    CreateGroup {
        name: String,
        respond: oneshot::Sender<Result<(), OrchestratorError>>,
    },
    AddToGroup {
        task_id: I,
        group_name: String,
        respond: oneshot::Sender<Result<(), OrchestratorError>>,
    },
    GetGroup {
        name: String,
        respond: oneshot::Sender<Option<HashSet<I>>>,
    },
    GetDependencies {
        id: I,
        respond: oneshot::Sender<Option<HashSet<I>>>,
    },
    CancelTask {
        id: I,
        respond: oneshot::Sender<Result<(), OrchestratorError>>,
    },
}

/// Channel-based orchestrator that uses message passing instead of Arc<Mutex<>>
pub struct ChannelOrchestrator<T, I>
where
    T: Clone + Send + Sync + 'static,
    I: TaskId + Clone + Copy + Eq + Hash + Send + 'static,
{
    sender: mpsc::UnboundedSender<OrchestratorMessage<T, I>>,
    runtime: TokioRuntime,
    is_running: AtomicBool,
}

impl<T, I> ChannelOrchestrator<T, I>
where
    T: Clone + Send + Sync + 'static,
    I: TaskId + Clone + Copy + Eq + Hash + Send + 'static,
{
    pub fn new(runtime: TokioRuntime) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let is_running = AtomicBool::new(true);
        
        // Spawn the actor loop
        runtime.handle().spawn(async move {
            let mut tasks: HashMap<I, TokioTask<T, I>> = HashMap::new();
            let mut deps: HashMap<I, HashSet<I>> = HashMap::new();
            let mut groups: HashMap<String, HashSet<I>> = HashMap::new();
            
            while let Some(msg) = receiver.recv().await {
                match msg {
                    OrchestratorMessage::RegisterTask { task, respond } => {
                        let id = task.task_id();
                        tasks.insert(id, task);
                        let _ = respond.send(id);
                    }
                    OrchestratorMessage::AddDependency { dependent_id, dependency_id, respond } => {
                        let result = if !tasks.contains_key(&dependent_id) {
                            Err(OrchestratorError::TaskNotFound(dependent_id.to_string()))
                        } else if !tasks.contains_key(&dependency_id) {
                            Err(OrchestratorError::TaskNotFound(dependency_id.to_string()))
                        } else {
                            deps.entry(dependent_id).or_default().insert(dependency_id);
                            Ok(())
                        };
                        let _ = respond.send(result);
                    }
                    OrchestratorMessage::GetTask { id, respond } => {
                        let _ = respond.send(tasks.remove(&id));
                    }
                    OrchestratorMessage::GetTaskStatus { id, respond } => {
                        let _ = respond.send(tasks.get(&id).map(|t| t.status()));
                    }
                    OrchestratorMessage::GetAllStatuses { respond } => {
                        let statuses = tasks.iter()
                            .map(|(id, task)| (*id, task.status()))
                            .collect();
                        let _ = respond.send(statuses);
                    }
                    OrchestratorMessage::CreateGroup { name, respond } => {
                        let result = if groups.contains_key(&name) {
                            Err(OrchestratorError::GroupAlreadyExists(name))
                        } else {
                            groups.insert(name, HashSet::new());
                            Ok(())
                        };
                        let _ = respond.send(result);
                    }
                    OrchestratorMessage::AddToGroup { task_id, group_name, respond } => {
                        let result = if let Some(group) = groups.get_mut(&group_name) {
                            group.insert(task_id);
                            Ok(())
                        } else {
                            Err(OrchestratorError::GroupNotFound(group_name))
                        };
                        let _ = respond.send(result);
                    }
                    OrchestratorMessage::GetGroup { name, respond } => {
                        let _ = respond.send(groups.get(&name).cloned());
                    }
                    OrchestratorMessage::GetDependencies { id, respond } => {
                        let _ = respond.send(deps.get(&id).cloned());
                    }
                    OrchestratorMessage::CancelTask { id, respond } => {
                        let result = if let Some(task) = tasks.get(&id) {
                            // Clone task to cancel it
                            let task_clone = task.clone();
                            tokio::spawn(async move {
                                let _ = task_clone.cancel_gracefully().await;
                            });
                            Ok(())
                        } else {
                            Err(OrchestratorError::TaskNotFound(id.to_string()))
                        };
                        let _ = respond.send(result);
                    }
                }
            }
        });
        
        Self {
            sender,
            runtime,
            is_running,
        }
    }
    
    async fn check_dependencies_satisfied(&self, id: &I) -> bool {
        let (tx, rx) = oneshot::channel();
        let _ = self.sender.send(OrchestratorMessage::GetDependencies {
            id: *id,
            respond: tx,
        });
        
        if let Ok(Some(deps)) = rx.await {
            for dep_id in deps {
                let (tx, rx) = oneshot::channel();
                let _ = self.sender.send(OrchestratorMessage::GetTaskStatus {
                    id: dep_id,
                    respond: tx,
                });
                
                if let Ok(Some(status)) = rx.await {
                    if !matches!(status, TaskStatus::Completed) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }
}

type BoxFut<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

impl<T, I, Task> TaskOrchestrator<T, Task, I> for ChannelOrchestrator<T, I>
where
    T: Clone + Send + Sync + 'static,
    I: TaskId + Copy + Eq + Hash + Send + 'static,
    Task: ApiAsyncTask<T, I> + Into<TokioTask<T, I>>,
{
    type RegisterTaskReturn = I;
    type StartTaskFuture = BoxFut<'static, Result<T, AsyncTaskError>>;
    type StartAllFuture = BoxFut<'static, Vec<(I, Result<T, AsyncTaskError>)>>;
    type JoinAllFuture = BoxFut<'static, Vec<(I, Result<T, AsyncTaskError>)>>;
    type StartGroupFuture = BoxFut<'static, Vec<(I, Result<T, AsyncTaskError>)>>;

    fn register_task(&self, task: Task) -> Self::RegisterTaskReturn {
        let tokio_task: TokioTask<T, I> = task.into();
        let (tx, rx) = oneshot::channel();
        
        let _ = self.sender.send(OrchestratorMessage::RegisterTask {
            task: tokio_task,
            respond: tx,
        });
        
        // Block on receiving the ID with error handling
        rx.blocking_recv().unwrap_or_else(|e| {
            tracing::error!("Orchestrator actor communication failed: {}", e);
            // Return a default ID in case of actor failure
            I::default()
        })
    }

    fn add_dependency(&self, dependent_id: &I, dependency_id: &I) -> Result<(), OrchestratorError> {
        let (tx, rx) = oneshot::channel();
        
        let _ = self.sender.send(OrchestratorMessage::AddDependency {
            dependent_id: *dependent_id,
            dependency_id: *dependency_id,
            respond: tx,
        });
        
        rx.blocking_recv().unwrap_or(Err(OrchestratorError::OperationFailed("Actor died".into())))
    }

    fn start_task(&self, task_id: &I) -> Self::StartTaskFuture {
        let id = *task_id;
        let sender = self.sender.clone();
        let orchestrator = self.clone();
        
        Box::pin(async move {
            // Check dependencies
            if !orchestrator.check_dependencies_satisfied(&id).await {
                return Err(AsyncTaskError::InvalidState("Dependencies not met".into()));
            }
            
            // Get and remove the task
            let (tx, rx) = oneshot::channel();
            let _ = sender.send(OrchestratorMessage::GetTask {
                id,
                respond: tx,
            });
            
            if let Ok(Some(task)) = rx.await {
                task.await
            } else {
                Err(AsyncTaskError::Failure(format!("Task {} not found", id.to_string())))
            }
        })
    }

    fn start_all(&self) -> Self::StartAllFuture {
        let orchestrator = self.clone();
        
        Box::pin(async move {
            let (tx, rx) = oneshot::channel();
            let _ = orchestrator.sender.send(OrchestratorMessage::GetAllStatuses {
                respond: tx,
            });
            
            let statuses = rx.await.unwrap_or_default();
            let ids: Vec<I> = statuses.into_iter().map(|(id, _)| id).collect();
            
            futures::future::join_all(ids.into_iter().map(|id| {
                let orch = orchestrator.clone();
                async move { (id, orch.start_task(&id).await) }
            }))
            .await
        })
    }

    fn cancel_task(&self, task_id: &I) -> Result<(), OrchestratorError> {
        let (tx, rx) = oneshot::channel();
        
        let _ = self.sender.send(OrchestratorMessage::CancelTask {
            id: *task_id,
            respond: tx,
        });
        
        rx.blocking_recv().unwrap_or(Err(OrchestratorError::OperationFailed("Actor died".into())))
    }

    fn task_status(&self, task_id: &I) -> Option<TaskStatus> {
        let (tx, rx) = oneshot::channel();
        
        let _ = self.sender.send(OrchestratorMessage::GetTaskStatus {
            id: *task_id,
            respond: tx,
        });
        
        rx.blocking_recv().ok().flatten()
    }

    fn all_task_statuses(&self) -> Vec<(I, TaskStatus)> {
        let (tx, rx) = oneshot::channel();
        
        let _ = self.sender.send(OrchestratorMessage::GetAllStatuses {
            respond: tx,
        });
        
        rx.blocking_recv().unwrap_or_default()
    }

    fn join_all(&self) -> Self::JoinAllFuture {
        self.start_all()
    }

    fn create_group(&self, group_name: &str) -> Result<(), OrchestratorError> {
        let (tx, rx) = oneshot::channel();
        
        let _ = self.sender.send(OrchestratorMessage::CreateGroup {
            name: group_name.to_string(),
            respond: tx,
        });
        
        rx.blocking_recv().unwrap_or(Err(OrchestratorError::OperationFailed("Actor died".into())))
    }

    fn add_task_to_group(&self, task_id: &I, group_name: &str) -> Result<(), OrchestratorError> {
        let (tx, rx) = oneshot::channel();
        
        let _ = self.sender.send(OrchestratorMessage::AddToGroup {
            task_id: *task_id,
            group_name: group_name.to_string(),
            respond: tx,
        });
        
        rx.blocking_recv().unwrap_or(Err(OrchestratorError::OperationFailed("Actor died".into())))
    }

    fn start_group(&self, group_name: &str) -> Self::StartGroupFuture {
        let sender = self.sender.clone();
        let group_name = group_name.to_string();
        let orchestrator = self.clone();
        
        Box::pin(async move {
            let (tx, rx) = oneshot::channel();
            let _ = sender.send(OrchestratorMessage::GetGroup {
                name: group_name,
                respond: tx,
            });
            
            if let Ok(Some(ids)) = rx.await {
                futures::future::join_all(ids.into_iter().map(|id| {
                    let orch = orchestrator.clone();
                    async move { (id, orch.start_task(&id).await) }
                }))
                .await
            } else {
                Vec::new()
            }
        })
    }

    fn cancel_group(&self, group_name: &str) -> usize {
        let (tx, rx) = oneshot::channel();
        
        let _ = self.sender.send(OrchestratorMessage::GetGroup {
            name: group_name.to_string(),
            respond: tx,
        });
        
        if let Ok(Some(ids)) = rx.blocking_recv() {
            ids.into_iter()
                .filter(|id| self.cancel_task(id).is_ok())
                .count()
        } else {
            0
        }
    }
}

impl<T, I> Clone for ChannelOrchestrator<T, I>
where
    T: Clone + Send + Sync + 'static,
    I: TaskId + Clone + Copy + Eq + Hash + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            runtime: self.runtime.clone(),
            is_running: AtomicBool::new(self.is_running.load(Ordering::SeqCst)),
        }
    }
}