use std::fmt::Debug;
use std::time::Duration;
use crate::api::task::TaskPriority;
use crate::api::task::spawn::SpawningTask;
use crate::api::task::builder::AsyncTaskBuilder;

pub trait SpawningTaskBuilder<T: Send + 'static>: AsyncTaskBuilder {
    fn spawn(self, work: impl FnOnce() -> T + Send + 'static) -> Box<dyn SpawningTask<T>>;
} 