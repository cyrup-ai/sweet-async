#[macro_export]
macro_rules! spawn_task {
    // Block syntax for spawn - matches README.md
    ($builder:expr, { $($body:tt)* }) => {
        $builder.run(|| { $($body)* })
    };
}
