#[macro_export]
macro_rules! register_task {
    // Direct block syntax
    ($orchestra:expr, { $($body:tt)* }) => {{
        // Creates a task and registers it with the orchestra
        let task = $crate::task::AsyncTask::to::<_>().run({ $($body)* });
        $orchestra.register_task(task)
    }};
}
