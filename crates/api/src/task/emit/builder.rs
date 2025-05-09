use crate::task::emit::EmittingTask;
use crate::task::emit::{Collector, ReceiverEvent};
use crate::task::builder::{SenderStrategy, ReceiverStrategy, AsyncTaskBuilder};

pub trait EmittingTaskBuilder<T: Send + 'static, U: Send + 'static>: AsyncTaskBuilder {
    type Task: EmittingTask<T, U>;
    type SenderFn: FnOnce(&mut dyn Collector<T, U>) + Send + 'static + Clone;
    type ReceiverFn: FnMut(&dyn ReceiverEvent<T, U>, &dyn Collector<T, U>) + Send + 'static + Clone;
    #[allow(dead_code)]
    fn with_sender(self, strategy: SenderStrategy, sender: Self::SenderFn) -> Self::Task;
    #[allow(dead_code)]
    fn with_receiver(self, strategy: ReceiverStrategy, receiver: Self::ReceiverFn) -> Self;
    #[allow(dead_code)]
    fn execute(self) -> Self::Task;
} 
