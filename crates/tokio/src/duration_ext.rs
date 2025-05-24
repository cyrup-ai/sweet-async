use std::time::Duration;

/// Extension trait to add duration creation methods to numeric types
pub trait DurationExt {
    /// Create a Duration from seconds
    fn seconds(self) -> Duration;
    
    /// Create a Duration from milliseconds  
    fn milliseconds(self) -> Duration;
    
    /// Create a Duration from minutes
    fn minutes(self) -> Duration;
}

impl DurationExt for u64 {
    fn seconds(self) -> Duration {
        Duration::from_secs(self)
    }
    
    fn milliseconds(self) -> Duration {
        Duration::from_millis(self)
    }
    
    fn minutes(self) -> Duration {
        Duration::from_secs(self * 60)
    }
}

impl DurationExt for u32 {
    fn seconds(self) -> Duration {
        Duration::from_secs(self as u64)
    }
    
    fn milliseconds(self) -> Duration {
        Duration::from_millis(self as u64)
    }
    
    fn minutes(self) -> Duration {
        Duration::from_secs((self * 60) as u64)
    }
}

impl DurationExt for i32 {
    fn seconds(self) -> Duration {
        Duration::from_secs(self as u64)
    }
    
    fn milliseconds(self) -> Duration {
        Duration::from_millis(self as u64)
    }
    
    fn minutes(self) -> Duration {
        Duration::from_secs((self * 60) as u64)
    }
}