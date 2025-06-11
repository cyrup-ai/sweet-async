#[macro_export]
macro_rules! await_result {
    // Block syntax only (matches README.md)
    ($builder:expr, { $($body:tt)* }) => {{
        $builder.await_result(|| { $($body)* })
    }};
    // Block + handler closure (matches README.md)
    ($builder:expr, { $($body:tt)* }, $handler:expr) => {{
        $builder.result_handler(|| { $($body)* }, $handler)
    }};
    // When called on an existing task (less common)
    ($task:expr) => {{
        $task.await_result()
    }};
}
