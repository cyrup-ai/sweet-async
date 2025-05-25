use sweet_async_tokio::task::metrics_aggregation::{MetricsAggregator, MetricsSnapshot};
use sweet_async_tokio::task::vector_clock::VectorClock;
use sweet_async_api::task::TaskId;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct TestTaskId(String);

impl TaskId for TestTaskId {
    fn to_string(&self) -> String {
        self.0.clone()
    }
    
    fn from_string(s: &str) -> Option<Self> {
        Some(TestTaskId(s.to_string()))
    }
}

#[test]
fn test_metrics_aggregation() {
    let mut aggregator = MetricsAggregator::new();
    
    // Add metrics from different tasks
    let task1 = TestTaskId("task1".to_string());
    let task2 = TestTaskId("task2".to_string());
    
    let mut clock1 = VectorClock::new();
    clock1.tick(&task1, "node1");
    
    let snapshot1 = MetricsSnapshot {
        task_id: task1.clone(),
        hostname: "node1".to_string(),
        vector_clock: clock1.clone(),
        cpu_time_nanos: 1000,
        memory_bytes: 1024,
        io_read_bytes: 100,
        io_write_bytes: 200,
        child_count: 0,
    };
    
    let mut clock2 = VectorClock::new();
    clock2.tick(&task2, "node2");
    
    let snapshot2 = MetricsSnapshot {
        task_id: task2.clone(),
        hostname: "node2".to_string(),
        vector_clock: clock2,
        cpu_time_nanos: 2000,
        memory_bytes: 2048,
        io_read_bytes: 300,
        io_write_bytes: 400,
        child_count: 0,
    };
    
    aggregator.add_snapshot(snapshot1);
    aggregator.add_snapshot(snapshot2);
    
    let total = aggregator.aggregate_all();
    assert_eq!(total.cpu_time_nanos, 3000);
    assert_eq!(total.memory_bytes, 2048); // Max memory
    assert_eq!(total.io_read_bytes, 400);
    assert_eq!(total.io_write_bytes, 600);
}