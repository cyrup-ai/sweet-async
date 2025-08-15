#[derive(Clone, Debug)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    PendingCancellation,
    Cancelled,
}

impl TaskStatus {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => TaskStatus::Pending,
            1 => TaskStatus::Running,
            2 => TaskStatus::Completed,
            3 => TaskStatus::Failed,
            4 => TaskStatus::PendingCancellation,
            5 => TaskStatus::Cancelled,
            _ => TaskStatus::Pending, // Default fallback
        }
    }
}

pub trait StatusEnabledTask<T: Send + 'static> {
    fn status(&self) -> TaskStatus;
}
