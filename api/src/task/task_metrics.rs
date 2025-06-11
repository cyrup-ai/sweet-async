use crate::task::{CpuUsage, IoUsage, MemoryUsage};

pub trait MetricsEnabledTask<T: Send + 'static> {
    type Cpu: CpuUsage;
    type Memory: MemoryUsage;
    type Io: IoUsage;

    fn cpu_usage(&self) -> &Self::Cpu;
    fn memory_usage(&self) -> &Self::Memory;
    fn io_usage(&self) -> &Self::Io;
}
