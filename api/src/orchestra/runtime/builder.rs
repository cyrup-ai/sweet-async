use crate::task::TaskId;
use crate::orchestra::runtime::Runtime;

/// Trait for building and configuring runtime instances
/// 
/// Follows the same fluent API pattern as the AsyncTask builder
pub trait RuntimeBuilder<T: Clone + Send + 'static, I: TaskId>: Sized {
    /// Create a new runtime builder with default settings
    fn new() -> Self;
    
    /// Set the number of worker threads for the runtime
    fn worker_threads(self, count: usize) -> Self;
    
    /// Set the stack size for worker threads
    fn stack_size(self, size_bytes: usize) -> Self;
    
    /// Build and return a configured runtime
    fn build(self) -> impl Runtime<T, I>;
    
}