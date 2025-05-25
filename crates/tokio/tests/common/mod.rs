//! Common test utilities and helpers

use sweet_async_api::task::TaskId;
use sweet_async_tokio::UuidTaskId;

/// Test task ID implementation for testing
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct TestTaskId(pub String);

impl TaskId for TestTaskId {
    fn to_string(&self) -> String {
        self.0.clone()
    }
    
    fn from_string(s: &str) -> Option<Self> {
        Some(TestTaskId(s.to_string()))
    }
}

/// Create a new test task ID with a given name
pub fn test_id(name: &str) -> TestTaskId {
    TestTaskId(name.to_string())
}

/// Create a new UUID-based task ID for testing
pub fn uuid_id() -> UuidTaskId {
    UuidTaskId(uuid::Uuid::new_v4())
}

/// Initialize test logging if needed
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("sweet_async=debug")
        .try_init();
}