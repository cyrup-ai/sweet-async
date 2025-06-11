use std::fmt::Debug;

use crate::task::MetricsEnabledTask;

/// Trait defining the behavior of a task priority system
///
/// This allows different priority implementations to be used interchangeably.
/// The RankableByPriority trait provides an abstraction over priority systems,
/// enabling custom priority implementations while maintaining a consistent interface.
///
/// # Customization Possibilities
///
/// This trait enables creating specialized priority systems such as:
///
/// - **Domain-Specific Priorities**: Create custom priorities for specific use cases
/// - **Numeric Range Priorities**: Use a wider range of values (e.g., 0-100) for finer control
/// - **Dynamic Priorities**: Implement priorities that can change based on execution context
/// - **Multi-Factor Priorities**: Combine multiple factors (urgency, importance, etc.)
///
/// # Implementation Requirements
///
/// When implementing this trait, ensure:
///
/// 1. **Consistent Ordering**: Higher priority should always be "greater than" lower priority
/// 2. **Deterministic Comparison**: The same priorities should always compare the same way
/// 3. **Default Provided**: Every implementation needs a sensible default
/// 4. **Efficient Operations**: Priority comparisons should be very fast (O(1))
///
/// # Custom Priority Example
///
/// ```rust,no_run
/// /// A percentage-based priority system (0% = lowest, 100% = highest)
/// #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// struct PercentagePriority(u8); // 0-100
///
/// impl RankableByPriority for PercentagePriority {
///     fn as_u8(&self) -> u8 {
///         // For compatibility with standard range, map 0-100 to 0-4 range
///         match self.0 {
///             0..=20 => 4,  // 0-20% = Background
///             21..=40 => 3, // 21-40% = Low
///             41..=60 => 2, // 41-60% = Normal
///             61..=80 => 1, // 61-80% = High
///             _ => 0,       // 81-100% = Critical
///         }
///     }
///
///     fn from_u8(value: u8) -> Self {
///         // Map standard range back to percentage
///         match value {
///             0 => PercentagePriority(90), // Critical
///             1 => PercentagePriority(70), // High
///             2 => PercentagePriority(50), // Normal
///             3 => PercentagePriority(30), // Low
///             _ => PercentagePriority(10), // Background
///         }
///     }
///
///     fn default_priority() -> Self {
///         PercentagePriority(50) // Normal
///     }
///
///     fn is_higher_than(&self, other: &Self) -> bool {
///         self.0 > other.0 // Higher percentage = higher priority
///     }
///
///     fn is_lower_than(&self, other: &Self) -> bool {
///         self.0 < other.0 // Lower percentage = lower priority
///     }
///
///     fn difference(&self, other: &Self) -> u8 {
///         if self.0 > other.0 {
///             self.0 - other.0
///         } else {
///             other.0 - self.0
///         }
///     }
///
///     fn highest() -> Self {
///         PercentagePriority(100)
///     }
///
///     fn lowest() -> Self {
///         PercentagePriority(0)
///     }
/// }
/// ```
pub trait RankableByPriority: Copy + Eq + Ord + Send + Sync + 'static {
    /// Get the numeric value of this priority
    fn as_u8(&self) -> u8;

    /// Convert a raw numeric value to a priority
    fn from_u8(value: u8) -> Self;

    /// Get the default priority level
    fn default_priority() -> Self;

    /// Check if this priority is higher than another
    fn is_higher_than(&self, other: &Self) -> bool;

    /// Check if this priority is lower than another
    fn is_lower_than(&self, other: &Self) -> bool;

    /// Calculate the difference between two priorities
    fn difference(&self, other: &Self) -> u8;

    /// Get the highest possible priority
    fn highest() -> Self;

    /// Get the lowest possible priority
    fn lowest() -> Self;
}

/// A task that has an assigned execution priority
///
/// This trait extends a metrics-enabled task with priority information,
/// allowing for intelligent scheduling and resource allocation.
///
/// # Priority-Based Task Management
///
/// The PrioritizedTask trait enables a sophisticated priority-based task management system:
///
/// - **Scheduling**: Tasks can be executed in order of priority
/// - **Resource Allocation**: Higher priority tasks can receive more resources
/// - **Preemption**: Higher priority tasks can interrupt lower priority ones
/// - **Quality of Service**: Different service levels based on priority
///
/// # Implementation Considerations
///
/// When implementing a priority-based task system:
///
/// 1. **Avoid Priority Inversion**: Ensure higher priority tasks aren't blocked by lower ones
/// 2. **Prevent Starvation**: Low priority tasks should eventually execute
/// 3. **Consider Aging**: Low priority tasks can gain priority over time
/// 4. **Priority Inheritance**: Tasks should inherit priority when holding resources needed by higher priority tasks
///
/// # Example Usage
///
/// ```rust,no_run
/// // Task scheduler that respects priorities
/// struct PriorityScheduler<Id, T> {
///     queued_tasks: Vec<Box<dyn PrioritizedTask<Id, T>>>,
/// }
///
/// impl<Id, T> PriorityScheduler<Id, T>
/// where
///     Id: TaskId,
///     T: Send + 'static,
/// {
///     // Add a task to the scheduler
///     fn schedule(&mut self, task: Box<dyn PrioritizedTask<Id, T>>) {
///         self.queued_tasks.push(task);
///     }
///     
///     // Get the next task to execute based on priority
///     fn next_task(&mut self) -> Option<Box<dyn PrioritizedTask<Id, T>>> {
///         if self.queued_tasks.is_empty() {
///             return None;
///         }
///         
///         // Find the highest priority task
///         let highest_idx = self.queued_tasks
///             .iter()
///             .enumerate()
///             .min_by_key(|(_, task)| task.priority().as_u8())
///             .map(|(idx, _)| idx)?;
///             
///         // Remove and return the highest priority task
///         Some(self.queued_tasks.swap_remove(highest_idx))
///     }
/// }
/// ```
pub trait PrioritizedTask<T: Send + 'static>: MetricsEnabledTask<T> {
    /// Get the priority of this task
    fn priority(&self) -> &impl RankableByPriority;
}

/// Standard task priority levels
///
/// Provides a common set of priority levels that can be used
/// across different task implementations.
///
/// # Priority System Design
///
/// The priority system follows these principles:
///
/// 1. **Lower Numbers = Higher Priority**: Critical (0) is highest, Background (4) is lowest
/// 2. **Default is Normal**: Most tasks should use the Normal (2) priority
/// 3. **Five Distinct Levels**: Provides enough granularity without overcomplicating scheduling
/// 4. **Extensible**: Custom priority implementations can be created with `RankableByPriority`
///
/// # Usage Guidelines
///
/// - **Critical (0)**: Reserved for system-critical operations like health checks or emergency shutdown
/// - **High (1)**: User-facing operations where responsiveness is essential
/// - **Normal (2)**: Standard operations with typical timing requirements
/// - **Low (3)**: Operations that can be delayed when the system is under load
/// - **Background (4)**: Maintenance, cleanup, or other non-essential tasks
///
/// # Examples
///
/// ```rust,no_run
/// use async_task::TaskPriority;
///
/// // Critical priority for system health checks
/// let health_check_task = task_manager.create_task("health_check")
///     .priority(TaskPriority::Critical)
///     .build();
///
/// // Normal priority for standard user requests
/// let user_request = task_manager.create_task("user_request")
///     .priority(TaskPriority::Normal) // This is the default
///     .build();
///
/// // Background priority for cleanup tasks
/// let cleanup_task = task_manager.create_task("cleanup")
///     .priority(TaskPriority::Background)
///     .build();
///
/// // Priority comparison
/// assert!(TaskPriority::Critical.is_higher_than(&TaskPriority::High));
/// assert!(TaskPriority::Background.is_lower_than(&TaskPriority::Low));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// Highest priority - critical system tasks
    Critical = 0,

    /// High priority - important user-facing operations
    High = 1,

    /// Default priority for most tasks
    Normal = 2,

    /// Lower priority - background operations
    Low = 3,

    /// Lowest priority - non-essential maintenance tasks
    Background = 4,
}

// Implement the RankableByPriority trait for the TaskPriority enum
impl RankableByPriority for TaskPriority {
    fn as_u8(&self) -> u8 {
        *self as u8
    }

    fn from_u8(value: u8) -> Self {
        match value {
            0 => TaskPriority::Critical,
            1 => TaskPriority::High,
            2 => TaskPriority::Normal,
            3 => TaskPriority::Low,
            _ => TaskPriority::Background,
        }
    }

    fn default_priority() -> Self {
        TaskPriority::Normal
    }

    fn is_higher_than(&self, other: &Self) -> bool {
        self.as_u8() < other.as_u8() // Lower number = higher priority
    }

    fn is_lower_than(&self, other: &Self) -> bool {
        self.as_u8() > other.as_u8() // Higher number = lower priority
    }

    /// Calculate the absolute difference between two priorities
    fn difference(&self, other: &Self) -> u8 {
        let self_val = self.as_u8();
        let other_val = other.as_u8();
        if self_val > other_val {
            self_val - other_val
        } else {
            other_val - self_val
        }
    }

    /// Get the highest possible priority
    fn highest() -> Self {
        TaskPriority::Critical
    }

    /// Get the lowest possible priority
    fn lowest() -> Self {
        TaskPriority::Background
    }
}
