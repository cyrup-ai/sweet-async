#[doc(hidden)]
#[macro_export]
macro_rules! __builder_spawn_internal {
    // Direct block syntax
    ({ $($body:tt)* }) => {{
        |_| { $($body)* }
    }};

    // Traditional closure syntax with no args
    (|| { $($body:tt)* }) => {{
        |_| { $($body)* }
    }};

    // Traditional async closure syntax
    (|| async { $($body:tt)* }) => {{
        |_| async { $($body)* }
    }};
}
