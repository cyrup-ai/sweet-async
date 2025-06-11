#[macro_export]
macro_rules! async_task {
    // Direct block syntax
    ({ $($body:tt)* }) => {{
        $crate::task::AsyncTaskBuilder::builder()
            $($body)*
            .run(|| {})
    }};

    // Traditional closure syntax with no args
    (|| { $($body:tt)* }) => {{
        $crate::task::AsyncTaskBuilder::builder()
            $($body)*
            .run(|| {})
    }};

    // Traditional async closure syntax
    (|| async { $($body:tt)* }) => {{
        $crate::task::AsyncTaskBuilder::builder().run(|| async {
            $($body)*
        })
    }};
}
