use crate::task::TaskId;

/// Defines relationships between tasks for communication
pub trait TaskRelationships<T: Clone + Send + 'static, I: TaskId>: Send + Sync {
    /// Associated type for parent channel sender
    type ParentSender: Send + Sync;

    /// Associated type for parent channel receiver
    type ParentReceiver: Send + Sync;

    /// Associated type for child channel sender
    type ChildSender: Send + Sync;

    /// Associated type for child channel receiver
    type ChildReceiver: Send + Sync;

    /// Get access to the parent channel sender
    fn parent_sender(&self) -> Option<&Self::ParentSender>;

    /// Get access to the parent channel receiver
    fn parent_receiver(&self) -> Option<&Self::ParentReceiver>;

    /// Get access to child channel senders
    fn child_senders(&self) -> &[Self::ChildSender];

    /// Get access to child channel receivers
    fn child_receivers(&self) -> &[Self::ChildReceiver];

    /// Add a child channel to this task's relationships
    fn add_child_channel(&mut self, sender: Self::ChildSender, receiver: Self::ChildReceiver);

    /// Set the parent channel for this task
    fn set_parent_channel(&mut self, sender: Self::ParentSender, receiver: Self::ParentReceiver);

    /// Check if this task has a parent
    fn has_parent(&self) -> bool;

    /// Get the number of children
    fn child_count(&self) -> usize;
}
