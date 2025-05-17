use crate::task::AsyncTaskError;

pub trait RecoverableTask<T: Send + 'static> {
    fn recover(&self, error: AsyncTaskError) -> Result<T, AsyncTaskError>;
    fn can_recover_from(&self, error: &AsyncTaskError) -> bool;
    fn fallback_value(&self) -> Option<T>;
}
