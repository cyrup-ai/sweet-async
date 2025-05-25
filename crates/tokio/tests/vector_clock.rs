use sweet_async_tokio::task::vector_clock::VectorClock;
use sweet_async_tokio::task::task_envelope::{TaskEnvelopeExt, EnvelopeBuilder};
use sweet_async_tokio::task::distributed_comm::create_distributed_channel_pair;
use sweet_async_api::task::{TaskId, TaskMessage, TaskStatus};
use std::time::Duration;
use tokio::time::sleep;
    
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
    
    #[tokio::test]
    async fn test_distributed_causality() {
        // Create parent and child tasks with communication channels
        let parent_id = TestTaskId("parent".to_string());
        let child_id = TestTaskId("child".to_string());
        
        let (mut parent_comm, mut child_comm) = create_distributed_channel_pair::<String, TestTaskId>(
            parent_id.clone(),
            "node1".to_string(),
            child_id.clone(),
            "node2".to_string(),
        );
        
        // Parent sends first message
        parent_comm.broadcast_to_children(TaskMessage::Data("Hello from parent".to_string()))
            .await
            .unwrap();
            
        // Child receives and its clock updates
        let msg1 = child_comm.receive().await.unwrap();
        assert_eq!(child_comm.vector_clock.get_time(&parent_id, "node1"), 1);
        assert_eq!(child_comm.vector_clock.get_time(&child_id, "node2"), 1);
        
        // Child sends response
        child_comm.send_to_parent(TaskMessage::Data("Hello from child".to_string()))
            .await
            .unwrap();
            
        // Parent receives
        let msg2 = parent_comm.receive().await.unwrap();
        assert_eq!(parent_comm.vector_clock.get_time(&parent_id, "node1"), 2);
        assert_eq!(parent_comm.vector_clock.get_time(&child_id, "node2"), 2);
        
        // Verify causality
        assert!(msg1.happened_before(&msg2));
        assert!(!msg2.happened_before(&msg1));
    }
    
    #[tokio::test]
    async fn test_concurrent_messages() {
        let task1 = TestTaskId("task1".to_string());
        let task2 = TestTaskId("task2".to_string());
        
        // Create independent vector clocks
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();
        
        // Both tasks tick independently
        clock1.tick(&task1, "node1");
        clock2.tick(&task2, "node2");
        
        // Create envelopes
        let env1 = EnvelopeBuilder::new(task1.clone(), clock1.clone())
            .build(TaskMessage::StatusUpdate(TaskStatus::Running));
            
        let env2 = EnvelopeBuilder::new(task2.clone(), clock2.clone())
            .build(TaskMessage::StatusUpdate(TaskStatus::Running));
            
        // These messages are concurrent
        assert!(env1.is_concurrent(&env2));
        assert!(env2.is_concurrent(&env1));
        
        // Neither happened before the other
        assert!(!env1.happened_before(&env2));
        assert!(!env2.happened_before(&env1));
    }
    
    #[tokio::test]
    async fn test_causal_chain_ordering() {
        let task_id = TestTaskId("test".to_string());
        let mut clock = VectorClock::new();
        let mut envelopes = Vec::new();
        
        // Create a series of causally related messages
        for i in 0..5 {
            clock.tick(&task_id, "node1");
            let env = EnvelopeBuilder::new(task_id.clone(), clock.clone())
                .build(TaskMessage::Data(format!("Message {}", i)));
            envelopes.push(env);
        }
        
        // Shuffle them
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        envelopes.shuffle(&mut rng);
        
        // Build causal chain should restore order
        let comm = sweet_async_tokio::task::distributed_comm::DistributedTaskComm::<String, TestTaskId>::new(
            task_id,
            "node1".to_string(),
            tokio::sync::mpsc::unbounded_channel().1,
            Default::default(),
        );
        
        comm.build_causal_chain(&mut envelopes);
        
        // Verify they're back in causal order
        for i in 0..envelopes.len() - 1 {
            assert!(envelopes[i].happened_before(&envelopes[i + 1]));
        }
    }
    
    #[tokio::test]
    async fn test_vector_clock_pruning() {
        let mut clock = VectorClock::<TestTaskId>::new();
        
        // Add some entries
        for i in 0..10 {
            let task_id = TestTaskId(format!("task{}", i));
            for j in 0..i {
                clock.tick(&task_id, "node1");
            }
        }
        
        // Prune entries below threshold
        clock.prune_below_threshold(5);
        
        // Check that low-value entries are gone
        for i in 0..5 {
            let task_id = TestTaskId(format!("task{}", i));
            assert_eq!(clock.get_time(&task_id, "node1"), 0);
        }
        
        // Higher entries remain
        for i in 6..10 {
            let task_id = TestTaskId(format!("task{}", i));
            assert!(clock.get_time(&task_id, "node1") >= 5);
        }
    }