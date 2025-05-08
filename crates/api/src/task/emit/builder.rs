use crate::api::task::emit::EmittingTask;
use crate::api::task::emit::{Collector, ReceiverEvent};
use crate::api::task::builder::{SenderStrategy, ReceiverStrategy, AsyncTaskBuilder};

pub trait EmittingTaskBuilder<T: Send + 'static, U: Send + 'static>: AsyncTaskBuilder {
    type Task: EmittingTask<T, U>;
    type SenderFn: FnOnce(&mut Collector<T, U>) + Send + 'static;
    type ReceiverFn: FnMut(&ReceiverEvent<T>, &mut Collector<T, U>) + Send + 'static;
    fn with_sender(self, strategy: SenderStrategy, sender: Self::SenderFn) -> Self;
    fn with_receiver(self, strategy: ReceiverStrategy, receiver: Self::ReceiverFn) -> Self;
    fn execute(self) -> Self::Task;
} 