#[macro_export]
macro_rules! await_final_event {
    ($task:expr) => {
        $task.await_final_event()
    };
}
