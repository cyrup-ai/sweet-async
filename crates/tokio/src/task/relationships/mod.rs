use std::marker::PhantomData;
use tokio::sync::mpsc::{Sender, Receiver};
use sweet_async_api::task::{TaskId, TaskRelationships};

/// Tokio implementation of TaskRelationships trait
pub struct TokioTaskRelationships<T: Clone + Send + 'static, I: TaskId> {
    /// Parent sender channel
    parent_sender: Option<Sender<T>>,
    
    /// Parent receiver channel
    parent_receiver: Option<Receiver<T>>,
    
    /// Child sender channels
    child_senders: Vec<Sender<T>>,
    
    /// Child receiver channels
    child_receivers: Vec<Receiver<T>>,
    
    /// Type marker
    _marker: PhantomData<(T, I)>,
}

impl<T: Clone + Send + 'static, I: TaskId> Default for TokioTaskRelationships<T, I> {
    fn default() -> Self {
        Self {
            parent_sender: None,
            parent_receiver: None,
            child_senders: Vec::new(),
            child_receivers: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TokioTaskRelationships<T, I> {
    /// Create a new empty relationship set
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: Clone + Send + 'static, I: TaskId> TaskRelationships<T, I> for TokioTaskRelationships<T, I> {
    type ParentSender = Sender<T>;
    type ParentReceiver = Receiver<T>;
    type ChildSender = Sender<T>;
    type ChildReceiver = Receiver<T>;
    
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
        self.parent_sender.is_some() || self.parent_receiver.is_some()
    }
    
    fn child_count(&self) -> usize {
        self.child_senders.len()
    }
}

/// Create a pair of task relationship channels
pub fn create_task_relationship_channels<T: Clone + Send + 'static, I: TaskId>(
    buffer_size: usize
) -> (Sender<T>, Receiver<T>, Sender<T>, Receiver<T>) {
    let (parent_to_child_tx, parent_to_child_rx) = tokio::sync::mpsc::channel(buffer_size);
    let (child_to_parent_tx, child_to_parent_rx) = tokio::sync::mpsc::channel(buffer_size);
    
    (parent_to_child_tx, child_to_parent_rx, child_to_parent_tx, parent_to_child_rx)
}

/// Helper function to create parent-child relationships
pub fn create_parent_child_relationships<T: Clone + Send + 'static, I: TaskId>(
    buffer_size: usize
) -> (TokioTaskRelationships<T, I>, TokioTaskRelationships<T, I>) {
    let (parent_tx, child_rx, child_tx, parent_rx) = create_task_relationship_channels(buffer_size);
    
    let mut parent_relationships = TokioTaskRelationships::new();
    parent_relationships.child_senders.push(parent_tx);
    parent_relationships.child_receivers.push(parent_rx);
    
    let mut child_relationships = TokioTaskRelationships::new();
    child_relationships.parent_sender = Some(child_tx);
    child_relationships.parent_receiver = Some(child_rx);
    
    (parent_relationships, child_relationships)
}