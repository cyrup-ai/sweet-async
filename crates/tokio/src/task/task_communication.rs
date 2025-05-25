use futures::channel::mpsc;
use std::sync::Arc;
use sweet_async_api::task::{
    AsyncTaskError, TaskEnvelope, TaskId, TaskMessage, TaskSender, TaskStatus,
    TaskRelationships, ChannelConfig,
};
use sweet_async_api::task::encryption::EncryptionProvider;

/// Extension trait for TaskSender operations
pub trait TaskSenderExt<T: Clone + Send + 'static, I: TaskId> {
    /// Send a message to the task
    async fn send(&mut self, message: TaskMessage<T>) -> Result<(), AsyncTaskError>;
    
    /// Send a message with encryption and correlation options
    async fn send_with_options(
        &mut self, 
        message: TaskMessage<T>,
        encrypt: bool,
        correlation_id: Option<String>,
    ) -> Result<(), AsyncTaskError>;
    
    /// Send data to the task
    async fn send_data(&mut self, data: T) -> Result<(), AsyncTaskError>;
    
    /// Request task cancellation
    async fn cancel(&mut self) -> Result<(), AsyncTaskError>;
    
    /// Send a status update
    async fn update_status(&mut self, status: TaskStatus) -> Result<(), AsyncTaskError>;
}

impl<T: Clone + Send + 'static, I: TaskId> TaskSenderExt<T, I> for TaskSender<T, I> {
    async fn send(&mut self, message: TaskMessage<T>) -> Result<(), AsyncTaskError> {
        self.send_with_options(message, false, None).await
    }
    
    async fn send_with_options(
        &mut self, 
        message: TaskMessage<T>,
        encrypt: bool,
        correlation_id: Option<String>,
    ) -> Result<(), AsyncTaskError> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
            
        let hostname = hostname::get()
            .ok()
            .and_then(|name| name.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
        
        // Handle encryption if requested and provider is available
        let (final_message, is_encrypted) = if encrypt && self.encryption_provider.is_some() {
            // For now, we'll skip actual encryption implementation
            // This would serialize T and encrypt it
            (message, false)
        } else {
            (message, false)
        };
        
        let envelope = TaskEnvelope {
            sender_id: self.sender_id.clone(),
            sender_hostname: hostname,
            timestamp,
            message: final_message,
            is_encrypted,
            correlation_id,
        };
        
        self.sender
            .try_send(envelope)
            .map_err(|_| AsyncTaskError::InvalidState("Channel closed".to_string()))
    }
    
    async fn send_data(&mut self, data: T) -> Result<(), AsyncTaskError> {
        self.send(TaskMessage::Data(data)).await
    }
    
    async fn cancel(&mut self) -> Result<(), AsyncTaskError> {
        self.send(TaskMessage::CancelRequest).await
    }
    
    async fn update_status(&mut self, status: TaskStatus) -> Result<(), AsyncTaskError> {
        self.send(TaskMessage::StatusUpdate(status)).await
    }
}

/// Extension trait for TaskRelationships operations
pub trait TaskRelationshipsExt<T: Clone + Send + 'static, I: TaskId> {
    /// Add a child task sender
    fn add_child(&mut self, sender: TaskSender<T, I>);
    
    /// Remove a child task by its sender
    fn remove_child(&mut self, sender: &TaskSender<T, I>) -> bool;
    
    /// Broadcast a message to all children
    async fn broadcast_to_children(&mut self, message: TaskMessage<T>) -> Result<(), AsyncTaskError>;
    
    /// Send a message to parent if it exists
    async fn send_to_parent(&mut self, message: TaskMessage<T>) -> Result<(), AsyncTaskError>;
}

impl<T: Clone + Send + 'static, I: TaskId> TaskRelationshipsExt<T, I> for TaskRelationships<T, I> {
    fn add_child(&mut self, sender: TaskSender<T, I>) {
        self.children.push(sender);
    }
    
    fn remove_child(&mut self, sender: &TaskSender<T, I>) -> bool {
        // Compare by sender pointer since channels are unique
        self.children
            .iter()
            .position(|s| std::ptr::eq(&s.sender, &sender.sender))
            .map(|idx| self.children.remove(idx))
            .is_some()
    }
    
    async fn broadcast_to_children(&mut self, message: TaskMessage<T>) -> Result<(), AsyncTaskError> {
        let mut errors = Vec::new();
        
        for (idx, child) in self.children.iter_mut().enumerate() {
            if let Err(e) = TaskSenderExt::send(child, message.clone()).await {
                let name = child.target_name.as_deref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("child_{}", idx));
                errors.push(format!("Failed to send to {}: {}", name, e));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(AsyncTaskError::Failure(errors.join("; ")))
        }
    }
    
    async fn send_to_parent(&mut self, message: TaskMessage<T>) -> Result<(), AsyncTaskError> {
        if let Some(ref mut parent) = self.parent {
            TaskSenderExt::send(parent, message).await
        } else {
            Err(AsyncTaskError::InvalidState("No parent task".to_string()))
        }
    }
}

/// Create a pair of connected task channels
pub fn create_task_channel_pair<T: Clone + Send + 'static, I: TaskId>(
    parent_id: I,
    child_id: I,
    config: &ChannelConfig,
) -> (TaskSender<T, I>, TaskSender<T, I>) {
    let (parent_to_child_tx, _parent_to_child_rx) = mpsc::channel(config.buffer_size);
    let (child_to_parent_tx, _child_to_parent_rx) = mpsc::channel(config.buffer_size);
    
    let parent_sender = TaskSender {
        sender: parent_to_child_tx,
        sender_id: parent_id.clone(),
        target_name: None,
        encryption_provider: config.encryption_provider.clone(),
    };
    
    let child_sender = TaskSender {
        sender: child_to_parent_tx,
        sender_id: child_id,
        target_name: None,
        encryption_provider: config.encryption_provider.clone(),
    };
    
    (parent_sender, child_sender)
}