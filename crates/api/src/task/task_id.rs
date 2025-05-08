use std::fmt::Debug;

/// Trait for task identifiers
///
/// Defines the requirements for types that can be used as task identifiers.
/// Task IDs must be copyable, comparable, and support serialization.
///
/// # Purpose
///
/// The TaskId trait provides a standardized way to identify, track, and
/// reference tasks within the async task system. A good TaskId implementation
/// should:
///
/// - Be unique within the task system's scope
/// - Be efficiently comparable for quick lookups
/// - Support string serialization for logging and diagnostics
/// - Be small enough to be passed by copy
///
/// # Implementation Examples
///
/// Common implementations might include:
///
/// ```rust,no_run
/// // UUID-based TaskId
/// use uuid::Uuid;
/// 
/// #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// struct UuidTaskId(Uuid);
///
/// impl TaskId for UuidTaskId {
///     fn to_string(&self) -> String {
///         self.0.to_string()
///     }
///     
///     fn from_string(s: &str) -> Option<Self> {
///         Uuid::parse_str(s).ok().map(UuidTaskId)
///     }
/// }
/// ```
///
/// Or a simpler sequential ID:
///
/// ```rust,no_run
/// #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// struct SimpleTaskId(u64);
///
/// impl TaskId for SimpleTaskId {
///     fn to_string(&self) -> String {
///         self.0.to_string()
///     }
///     
///     fn from_string(s: &str) -> Option<Self> {
///         s.parse::<u64>().ok().map(SimpleTaskId)
///     }
/// }
/// ```
pub trait TaskId: Debug + Copy + Eq + Ord + Send + Sync + 'static {
    /// Get the string representation of this task ID
    fn to_string(&self) -> String;
    
    /// Parse a task ID from a string representation
    fn from_string(s: &str) -> Option<Self> where Self: Sized;
}

// Note: Specific TaskId implementations should be provided by the library receiver