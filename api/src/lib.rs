mod macros;
pub mod orchestra;
pub mod task;

// ── NEW extension points (syntax-sugar traits & enums) ──────────────────────
pub mod syntax_sugar;
pub mod enums;

// Re-export so macro-expanded code can do `$crate::syntax_sugar::*`
pub use syntax_sugar::*;
pub use enums::*;

pub use orchestra::*;
pub use task::*;
