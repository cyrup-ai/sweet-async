#[macro_export]
macro_rules! to {
    // Type parameter syntax for single type (using default orchestrator)
    (<$T:ty>) => {
        $crate::task::SpawningTaskBuilder::<$T, $crate::task::DefaultTaskId>::new()
    };

    // With explicit task ID and return type
    (<$Id:ty, $T:ty>) => {
        $crate::task::SpawningTaskBuilder::<$T, $Id>::new()
    };
}
