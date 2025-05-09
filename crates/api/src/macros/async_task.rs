#[macro_export]
macro_rules! async_task {
    // Direct block syntax
    ({ $($body:tt)* }) => {{
        $crate::task::AsyncTaskBuilder::builder().spawn(|| {
            $($body)*
        })
    }};
    
    // Traditional closure syntax with no args
    (|| { $($body:tt)* }) => {{
        $crate::task::AsyncTaskBuilder::builder().spawn(|| {
            $($body)*
        })
    }};
    
    // Traditional async closure syntax
    (|| async { $($body:tt)* }) => {{
        $crate::task::AsyncTaskBuilder::builder().spawn(|| async {
            $($body)*
        })
    }};
} 