use crate::task::{AsyncTask, AsyncTaskError};

/// Minimal trait for auto-scaling capabilities
///
/// This is an open interface for platform-specific implementations to provide
/// auto-scaling functionality through the plugin architecture.
#[cfg(feature = "multi-node")]
pub trait AutoScalable {
    /// Configure auto-scaling with platform-specific implementation
    type ScalingTask: TokioTask<(), AsyncTaskError>;
    fn configure_auto_scaling(&self, max_instances: u32) -> Self::ScalingTask;
}

/// Default implementation of AutoScalable
#[cfg(feature = "multi-node")]
pub struct DefaultAutoScaler;

#[cfg(feature = "multi-node")]
impl AutoScalable for DefaultAutoScaler {
    type ScalingTask = DefaultScalingTask;

    fn configure_auto_scaling(&self, max_instances: u32) -> Self::ScalingTask {
        DefaultScalingTask::new(max_instances)
    }
}

#[cfg(feature = "multi-node")]
pub struct DefaultScalingTask {
    max_instances: u32,
}

#[cfg(feature = "multi-node")]
impl DefaultScalingTask {
    fn new(max_instances: u32) -> Self {
        Self { max_instances }
    }
}

#[cfg(feature = "multi-node")]
impl AsyncTask<(), AsyncTaskError> for DefaultScalingTask {
    fn run(self) -> Result<(), AsyncTaskError> {
        // Basic implementation - in a real system, this would interact with
        // the underlying platform to configure auto-scaling
        println!(
            "Configuring auto-scaling with max instances: {}",
            self.max_instances
        );
        Ok(())
    }
}
