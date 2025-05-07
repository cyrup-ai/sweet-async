#[macro_export]
macro_rules! register_task {
    // Direct block syntax
    ($orchestra:expr, { $($body:tt)* }) => {{
        // Creates a plain AsyncTask and registers it with the orchestra
        let task = $crate::AsyncTaskBuilder::builder().build();
        $orchestra.register_task(task)
    }};
} 