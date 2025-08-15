//! High-performance task priority implementation with zero allocation

use crate::task::{TokioCpuUsage, TokioIoUsage, TokioMemoryUsage};
use sweet_async_api::task::{MetricsEnabledTask, PrioritizedTask, TaskPriority};

/// Zero-allocation task priority implementation
#[derive(Debug)]
pub struct TokioPrioritizedTask {
    priority: TaskPriority,
    cpu_usage: TokioCpuUsage,
    memory_usage: TokioMemoryUsage,
    io_usage: TokioIoUsage,
}

impl TokioPrioritizedTask {
    #[inline]
    pub fn new(priority: TaskPriority) -> Self {
        Self {
            priority,
            cpu_usage: TokioCpuUsage::new(),
            memory_usage: TokioMemoryUsage::new(),
            io_usage: TokioIoUsage::new(),
        }
    }
}

impl Default for TokioPrioritizedTask {
    #[inline]
    fn default() -> Self {
        Self::new(TaskPriority::Normal)
    }
}

impl<T> PrioritizedTask<T> for TokioPrioritizedTask
where
    T: Clone + Send + 'static,
{
    #[inline]
    fn priority(&self) -> &TaskPriority {
        &self.priority
    }
}

impl<T> MetricsEnabledTask<T> for TokioPrioritizedTask
where
    T: Send + 'static,
{
    type Cpu = TokioCpuUsage;
    type Memory = TokioMemoryUsage;
    type Io = TokioIoUsage;

    #[inline]
    fn cpu_usage(&self) -> &Self::Cpu {
        &self.cpu_usage
    }

    #[inline]
    fn memory_usage(&self) -> &Self::Memory {
        &self.memory_usage
    }

    #[inline]
    fn io_usage(&self) -> &Self::Io {
        &self.io_usage
    }
}
