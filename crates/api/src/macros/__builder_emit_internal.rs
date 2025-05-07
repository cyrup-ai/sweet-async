#[doc(hidden)]
#[macro_export]
macro_rules! __builder_emit_internal {
    // Direct block syntax
    ({ $($body:tt)* }) => {{
        { $($body)* }
    }};
} 