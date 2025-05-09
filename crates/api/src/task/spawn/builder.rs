use crate::task::spawn::SpawningTask;
use crate::task::builder::AsyncTaskBuilder;
use crate::task::TaskId;

pub trait SpawningTaskBuilder<T: Send + 'static, I: TaskId>: AsyncTaskBuilder {
    type Task: SpawningTask<T, I>;
    type WorkFn: FnOnce() -> T + Send + 'static;
    #[allow(dead_code)]
    fn spawn(self, work: Self::WorkFn) -> Self::Task;
} 