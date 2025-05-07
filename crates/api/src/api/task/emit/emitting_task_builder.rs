use std::fmt::Debug;
use crate::api::task::emit::EmittingTask;
use crate::api::task::TaskPriority;
use crate::api::task::emit::{Collector, ReceiverEvent};
use crate::api::task::builder::{SenderStrategy, ReceiverStrategy, AsyncTaskBuilder};

pub trait EmittingTaskBuilder<T: Send + 'static, U: Send + 'static>: AsyncTaskBuilder {
    fn with_sender(self, strategy: SenderStrategy, sender: impl FnOnce(&mut Collector<T, U>) + Send + 'static) -> Self;
    fn with_receiver(self, receiver: impl FnMut(&ReceiverEvent<T>, &mut Collector<T, U>) + Send + 'static, strategy: ReceiverStrategy) -> Self;
    fn execute(self) -> Box<dyn EmittingTask<T, U>>;
} 