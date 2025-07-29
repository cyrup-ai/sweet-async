//! Task communication implementation

use std::collections::HashMap;
use tokio::sync::mpsc;
use serde_json::Value;

/// Inter-task communication message
#[derive(Debug, Clone)]
pub struct TaskMessage {
    pub sender_id: String,
    pub recipient_id: String,
    pub payload: Value,
    pub timestamp: std::time::SystemTime,
}

/// Communication channel manager
#[derive(Debug)]
pub struct TaskCommunicationManager {
    channels: HashMap<String, mpsc::UnboundedSender<TaskMessage>>,
}

impl TaskCommunicationManager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn register_task(&mut self, task_id: String) -> mpsc::UnboundedReceiver<TaskMessage> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.channels.insert(task_id, tx);
        rx
    }

    pub async fn send_message(&self, message: TaskMessage) -> Result<(), String> {
        if let Some(sender) = self.channels.get(&message.recipient_id) {
            sender.send(message).map_err(|e| e.to_string())
        } else {
            Err("Recipient not found".to_string())
        }
    }
}

impl Default for TaskCommunicationManager {
    fn default() -> Self {
        Self::new()
    }
}