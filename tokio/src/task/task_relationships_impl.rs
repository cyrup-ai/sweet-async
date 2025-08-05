//! Tokio implementation of TaskRelationships trait
//!
//! This module provides task relationship management using Tokio's channel system
//! for parent-child task communication patterns.

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use sweet_async_api::task::task_relationships::TaskRelationships;
use sweet_async_api::task::TaskId;

/// Tokio implementation of TaskRelationships
/// 
/// Manages parent-child relationships between tasks using Tokio channels.
#[derive(Debug)]
pub struct TokioTaskRelationships<T: Clone + Send + 'static, I: TaskId> {
    parent_sender: Option<mpsc::UnboundedSender<T>>,
    parent_receiver: Option<Arc<Mutex<mpsc::UnboundedReceiver<T>>>>,
    child_senders: Vec<mpsc::UnboundedSender<T>>,
    child_receivers: Vec<Arc<Mutex<mpsc::UnboundedReceiver<T>>>>,
    task_id: I,
}

impl<T: Clone + Send + 'static, I: TaskId> TokioTaskRelationships<T, I> {
    /// Create a new task relationships manager
    pub fn new(task_id: I) -> Self {
        Self {
            parent_sender: None,
            parent_receiver: None,
            child_senders: Vec::new(),
            child_receivers: Vec::new(),
            task_id,
        }
    }
    
    /// Create a new parent-child channel pair
    pub fn create_channel() -> (mpsc::UnboundedSender<T>, Arc<Mutex<mpsc::UnboundedReceiver<T>>>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (sender, Arc::new(Mutex::new(receiver)))
    }
    
    /// Get the task ID
    pub fn task_id(&self) -> &I {
        &self.task_id
    }
    
    /// Send a message to the parent task
    pub async fn send_to_parent(&self, message: T) -> Result<(), mpsc::error::SendError<T>> {
        if let Some(sender) = &self.parent_sender {
            sender.send(message)?;
        }
        Ok(())
    }
    
    /// Try to receive a message from the parent task
    pub async fn try_receive_from_parent(&self) -> Option<T> {
        if let Some(receiver) = &self.parent_receiver {
            if let Ok(mut receiver_guard) = receiver.try_lock() {
                receiver_guard.try_recv().ok()
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Send a message to all child tasks
    pub async fn broadcast_to_children(&self, message: T) -> Vec<Result<(), mpsc::error::SendError<T>>> {
        let mut results = Vec::new();
        for sender in &self.child_senders {
            results.push(sender.send(message.clone()));
        }
        results
    }
    
    /// Send a message to a specific child by index
    pub async fn send_to_child(&self, child_index: usize, message: T) -> Result<(), mpsc::error::SendError<T>> {
        if let Some(sender) = self.child_senders.get(child_index) {
            sender.send(message)?;
        }
        Ok(())
    }
}

impl<T: Clone + Send + 'static, I: TaskId> Clone for TokioTaskRelationships<T, I> {
    fn clone(&self) -> Self {
        Self {
            parent_sender: self.parent_sender.clone(),
            parent_receiver: self.parent_receiver.clone(),
            child_senders: self.child_senders.clone(),
            child_receivers: self.child_receivers.clone(),
            task_id: self.task_id,
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TaskRelationships<T, I> for TokioTaskRelationships<T, I> {
    type ParentSender = mpsc::UnboundedSender<T>;
    type ParentReceiver = Arc<Mutex<mpsc::UnboundedReceiver<T>>>;
    type ChildSender = mpsc::UnboundedSender<T>;
    type ChildReceiver = Arc<Mutex<mpsc::UnboundedReceiver<T>>>;
    
    fn parent_sender(&self) -> Option<&Self::ParentSender> {
        self.parent_sender.as_ref()
    }
    
    fn parent_receiver(&self) -> Option<&Self::ParentReceiver> {
        self.parent_receiver.as_ref()
    }
    
    fn child_senders(&self) -> &[Self::ChildSender] {
        &self.child_senders
    }
    
    fn child_receivers(&self) -> &[Self::ChildReceiver] {
        &self.child_receivers
    }
    
    fn add_child_channel(&mut self, sender: Self::ChildSender, receiver: Self::ChildReceiver) {
        self.child_senders.push(sender);
        self.child_receivers.push(receiver);
    }
    
    fn set_parent_channel(&mut self, sender: Self::ParentSender, receiver: Self::ParentReceiver) {
        self.parent_sender = Some(sender);
        self.parent_receiver = Some(receiver);
    }
    
    fn has_parent(&self) -> bool {
        self.parent_sender.is_some()
    }
    
    fn child_count(&self) -> usize {
        self.child_senders.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TokioTaskId;
    
    #[tokio::test]
    async fn test_parent_child_communication() {
        let parent_id = TokioTaskId::new();
        let child_id = TokioTaskId::new();
        
        let mut parent_relationships = TokioTaskRelationships::new(parent_id);
        let mut child_relationships = TokioTaskRelationships::new(child_id);
        
        // Create channels
        let (parent_to_child_sender, parent_to_child_receiver) = TokioTaskRelationships::create_channel();
        let (child_to_parent_sender, child_to_parent_receiver) = TokioTaskRelationships::create_channel();
        
        // Set up relationships
        parent_relationships.add_child_channel(parent_to_child_sender, child_to_parent_receiver);
        child_relationships.set_parent_channel(child_to_parent_sender, parent_to_child_receiver);
        
        // Test communication
        assert!(parent_relationships.child_count() == 1);
        assert!(child_relationships.has_parent());
        
        // Test sending from child to parent
        child_relationships.send_to_parent("Hello Parent!".to_string()).await.unwrap();
        
        // Note: In a real test, we'd need to properly handle the async receiving
        // This is just demonstrating the structure
    }
    
    #[test]
    fn test_relationship_structure() {
        let task_id = TokioTaskId::new();
        let relationships = TokioTaskRelationships::<String, _>::new(task_id);
        
        assert!(!relationships.has_parent());
        assert_eq!(relationships.child_count(), 0);
        assert_eq!(relationships.task_id(), &task_id);
    }
}