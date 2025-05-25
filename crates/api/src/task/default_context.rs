/// Macro to implement ContextualizedTask with default system context
///
/// This macro provides a convenient way to add default ContextualizedTask
/// implementation to your task types when used with runtime crates that
/// provide DefaultTaskContext.
///
/// # Example
/// ```rust
/// use sweet_async_api::task::{ContextualizedTask, impl_default_context};
/// 
/// struct MyTask {
///     context: DefaultTaskContext,  // Provided by runtime crate
///     // ... other fields
/// }
/// 
/// impl_default_context!(MyTask);
/// ```
#[macro_export]
macro_rules! impl_default_context {
    ($task_type:ty) => {
        impl<T: Clone + Send + 'static, I: $crate::task::TaskId> $crate::task::ContextualizedTask<T, I> for $task_type {
            type RuntimeType = Box<dyn $crate::orchestra::Runtime<T, I>>;
            
            fn relationships(&self) -> &$crate::task::TaskRelationships<T, I> {
                &self.relationships
            }
            
            fn relationships_mut(&mut self) -> &mut $crate::task::TaskRelationships<T, I> {
                &mut self.relationships
            }
            
            fn runtime(&self) -> &Self::RuntimeType {
                unimplemented!("Runtime must be provided by concrete implementation")
            }
            
            fn cwd(&self) -> std::path::PathBuf {
                self.context.cwd.clone()
            }
        }
    };
    
    // Version for types that want to specify runtime type
    ($task_type:ty, $runtime_type:ty) => {
        impl<T: Clone + Send + 'static, I: $crate::task::TaskId> $crate::task::ContextualizedTask<T, I> for $task_type {
            type RuntimeType = $runtime_type;
            
            fn relationships(&self) -> &$crate::task::TaskRelationships<T, I> {
                &self.relationships
            }
            
            fn relationships_mut(&mut self) -> &mut $crate::task::TaskRelationships<T, I> {
                &mut self.relationships
            }
            
            fn runtime(&self) -> &Self::RuntimeType {
                &self.runtime
            }
            
            fn cwd(&self) -> std::path::PathBuf {
                self.context.cwd.clone()
            }
        }
    };
}