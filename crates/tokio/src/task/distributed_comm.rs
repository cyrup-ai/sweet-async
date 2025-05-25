use std::sync::Arc;
use tokio::sync::mpsc;
use sweet_async_api::task::{TaskId, TaskMessage, TaskSender, TaskRelationships, AsyncTaskError};
use crate::task::vector_clock::VectorClock;
use crate::task::task_envelope::TaskEnvelopeExt;

/// Distributed task communication handler with vector clock support
pub struct DistributedTaskComm<T: Clone + Send + 'static, I: TaskId> {
    /// Task ID of this communicator
    pub task_id: I,
    /// Hostname
    pub hostname: String,
    /// Current vector clock
    pub vector_clock: VectorClock<I>,
    /// Channel for receiving messages
    pub inbox: mpsc::UnboundedReceiver<TaskEnvelopeExt<T, I>>,
    /// Relationships for sending messages
    pub relationships: TaskRelationships<T, I>,
}

impl<T: Clone + Send + 'static, I: TaskId> DistributedTaskComm<T, I> {
    /// Create a new distributed communication handler
    pub fn new(
        task_id: I,
        hostname: String,
        inbox: mpsc::UnboundedReceiver<TaskEnvelopeExt<T, I>>,
        relationships: TaskRelationships<T, I>,
    ) -> Self {
        Self {
            task_id,
            hostname,
            vector_clock: VectorClock::new(),
            inbox,
            relationships,
        }
    }
    
    /// Send a message to parent with vector clock
    pub async fn send_to_parent(&mut self, message: TaskMessage<T>) -> Result<(), AsyncTaskError> {
        // Tick our clock before sending
        self.vector_clock.tick(&self.task_id, &self.hostname);
        
        let envelope = TaskEnvelopeExt::new(
            self.task_id.clone(),
            message,
            self.vector_clock.clone(),
        );
        
        if let Some(ref mut parent) = self.relationships.parent {
            // Convert to API envelope for sending
            parent.send(TaskMessage::Custom("vector_clock".to_string(), 
                bincode::encode_to_vec(&envelope, bincode::config::standard())
                    .map_err(|e| AsyncTaskError::Failure(e.to_string()))?
            )).await
        } else {
            Err(AsyncTaskError::InvalidState("No parent task".to_string()))
        }
    }
    
    /// Broadcast to all children with vector clock
    pub async fn broadcast_to_children(&mut self, message: TaskMessage<T>) -> Result<(), AsyncTaskError> {
        // Tick our clock before sending
        self.vector_clock.tick(&self.task_id, &self.hostname);
        
        let envelope = TaskEnvelopeExt::new(
            self.task_id.clone(),
            message,
            self.vector_clock.clone(),
        );
        
        let serialized = bincode::encode_to_vec(&envelope, bincode::config::standard())
            .map_err(|e| AsyncTaskError::Failure(e.to_string()))?;
            
        let mut errors = Vec::new();
        for child in &mut self.relationships.children {
            if let Err(e) = child.send(TaskMessage::Custom("vector_clock".to_string(), serialized.clone())).await {
                errors.push(e.to_string());
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(AsyncTaskError::Failure(errors.join("; ")))
        }
    }
    
    /// Process incoming messages and update vector clock
    pub async fn receive(&mut self) -> Option<TaskEnvelopeExt<T, I>> {
        if let Some(envelope) = self.inbox.recv().await {
            // Update our vector clock with the received clock
            self.vector_clock.merge(&envelope.vector_clock);
            self.vector_clock.tick(&self.task_id, &self.hostname);
            
            Some(envelope)
        } else {
            None
        }
    }
    
    /// Get messages that happened before a specific envelope
    pub fn get_messages_before<'a>(&self, envelopes: &'a [TaskEnvelopeExt<T, I>], target: &TaskEnvelopeExt<T, I>) -> Vec<&'a TaskEnvelopeExt<T, I>> {
        envelopes.iter()
            .filter(|env| env.happened_before(target))
            .collect()
    }
    
    /// Get concurrent messages
    pub fn get_concurrent_messages<'a>(&self, envelopes: &'a [TaskEnvelopeExt<T, I>], target: &TaskEnvelopeExt<T, I>) -> Vec<&'a TaskEnvelopeExt<T, I>> {
        envelopes.iter()
            .filter(|env| env.is_concurrent(target))
            .collect()
    }
    
    /// Build a causal chain of messages
    pub fn build_causal_chain(&self, envelopes: &mut Vec<TaskEnvelopeExt<T, I>>) {
        // Sort by happened-before relationship
        envelopes.sort_by(|a, b| {
            if a.happened_before(b) {
                std::cmp::Ordering::Less
            } else if b.happened_before(a) {
                std::cmp::Ordering::Greater
            } else {
                // Concurrent - use timestamp as tiebreaker
                a.base.timestamp.cmp(&b.base.timestamp)
            }
        });
    }
}

/// Create a distributed communication channel pair
pub fn create_distributed_channel_pair<T: Clone + Send + 'static, I: TaskId>(
    parent_id: I,
    parent_hostname: String,
    child_id: I,
    child_hostname: String,
) -> (DistributedTaskComm<T, I>, DistributedTaskComm<T, I>) {
    // Create channels
    let (parent_to_child_tx, parent_to_child_rx) = mpsc::unbounded_channel();
    let (child_to_parent_tx, child_to_parent_rx) = mpsc::unbounded_channel();
    
    // Create senders using the channels
    let parent_sender = TaskSender {
        sender: child_to_parent_tx,
        sender_id: parent_id.clone(),
        target_name: None,
        encryption_provider: None,
    };
    
    let child_sender = TaskSender {
        sender: parent_to_child_tx,
        sender_id: child_id.clone(),
        target_name: None,
        encryption_provider: None,
    };
    
    // Create parent communication handler
    let parent_comm = DistributedTaskComm::new(
        parent_id,
        parent_hostname,
        child_to_parent_rx,
        TaskRelationships {
            parent: None,
            children: vec![child_sender],
        },
    );
    
    // Create child communication handler
    let child_comm = DistributedTaskComm::new(
        child_id,
        child_hostname,
        parent_to_child_rx,
        TaskRelationships {
            parent: Some(parent_sender),
            children: vec![],
        },
    );
    
    (parent_comm, child_comm)
}