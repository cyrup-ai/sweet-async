#[macro_export]
macro_rules! fallback_value {
    // Block syntax leveraging the internal builder
    ($builder:expr, { $($body:tt)* }) => {
        $builder.fallback_value(|| { $($body)* })
    };
}
