pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    PendingCancellation,
    Cancelled,
}

pub trait StatusEnabledTask<T: Send + 'static> {
    fn status(&self) -> TaskStatus;
}
