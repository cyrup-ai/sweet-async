use crate::api::task::spawn::SpawningTask;
use crate::api::task::builder::AsyncTaskBuilder;
use crate::api::task::TaskId;

pub trait SpawningTaskBuilder<T: Send + 'static, I: TaskId>: AsyncTaskBuilder {
    type Task: SpawningTask<T, I>;
    type WorkFn: FnOnce() -> T + Send + 'static;
    fn spawn(self, work: Self::WorkFn) -> Self::Task;
} 