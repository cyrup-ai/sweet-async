use std::fmt::Debug;
use std::time::Duration;
use std::time::SystemTime;

use crate::api::task::BaseTask;

/// A task that tracks its execution time
///
/// This trait extends the base task with timing-related capabilities, allowing
/// receivers to monitor and analyze task performance.
pub trait TimedTask<Id: Debug + Send + Sync + 'static, T: Send + 'static>: BaseTask<Id> {
    // This trait inherits all timing-related methods from BaseTask
    // No additional methods needed as BaseTask already defines the timing interface
}
