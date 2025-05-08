use crate::api::task::{AsyncTaskError, MetricsEnabledTask};


pub trait RecoverableTask<T: Send + 'static>: MetricsEnabledTask<T> {
    fn recover(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError>;
    fn can_recover_from(&self, error: &AsyncTaskError) -> bool;
    fn fallback_value(&self) -> Option<T>;
}