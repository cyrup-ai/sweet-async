/// Trait for tasks that have names
pub trait NamedTask {
    /// Get the task's name
    fn name(&self) -> Option<&str>;

    /// Set the task's name
    fn set_name(&mut self, name: String);
}
