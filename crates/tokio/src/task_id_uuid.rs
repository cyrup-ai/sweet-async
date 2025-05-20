//! TaskId implementation for `uuid::Uuid` so that Sweet Async can use UUIDs
//! as task identifiers out‐of‐the‐box.

use sweet_async_api::task::TaskId;

impl TaskId for uuid::Uuid {
    fn to_string(&self) -> String {
        self.to_string()
    }

    fn from_string(s: &str) -> Option<Self> {
        uuid::Uuid::parse_str(s).ok()
    }
}
