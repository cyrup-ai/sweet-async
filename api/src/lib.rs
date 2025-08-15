#![feature(impl_trait_in_assoc_type)]
#![feature(impl_trait_in_fn_trait_return)]

mod macros;
pub mod orchestra;
pub mod task;

// ── NEW extension points (syntax-sugar traits & enums) ──────────────────────
pub mod enums;
pub mod size_ext;
pub mod syntax_sugar;
pub mod time_ext;

// Re-export so macro-expanded code can do `$crate::syntax_sugar::*`
pub use size_ext::*;
pub use syntax_sugar::*;
pub use time_ext::*;

// Export enums but avoid conflicts with task module
pub use enums::{
    Browser, BrowserProfile, Delimiter, EventOrder, EventType, FeedType, Offset, PartitionStrategy,
    PostOrder, PublicSentiment, Region, ResultOrder, TaskOrder, TimeRange,
};
// Note: DateRange, TaskStatus, RetryStrategy are handled by task module to avoid conflicts

// Macros are exported via #[macro_export] in their respective files

pub use orchestra::*;
pub use task::*;
