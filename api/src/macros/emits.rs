#[macro_export]
macro_rules! emits {
    // For data-only streaming tasks (with default task ID)
    (<$T:ty>) => {
        $crate::task::EmittingTaskBuilder::<$T, $crate::task::DefaultTaskId>::new()
    };

    // For streaming tasks with ID and data type
    (<$Id:ty, $T:ty>) => {
        $crate::task::EmittingTaskBuilder::<$T, $Id>::new()
    };
}
