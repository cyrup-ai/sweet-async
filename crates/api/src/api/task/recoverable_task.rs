use std::fmt::Debug;

use crate::api::task::{AsyncTaskError, MetricsEnabledTask};


pub trait RecoverableTask<Id: Debug + Send + Sync + 'static, T: Send + 'static>: MetricsEnabledTask<Id, T> {
    fn recover(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError>;
    fn can_recover_from(&self, error: &AsyncTaskError) -> bool;
    fn fallback_value(&self) -> Option<T>;
}