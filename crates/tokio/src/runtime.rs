//! Minimal stub for TokioRuntime and safe_blocking to unblock build

pub struct TokioRuntime;

impl TokioRuntime {
    pub fn new() -> Self {
        unimplemented!("TokioRuntime::new is not yet implemented")
    }
    pub fn with_config(_workers: usize) -> Self {
        unimplemented!("TokioRuntime::with_config is not yet implemented")
    }
}

pub fn safe_blocking<F, R>(_f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    unimplemented!("safe_blocking is not yet implemented")
}
