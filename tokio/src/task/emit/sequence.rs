use sweet_async_api::task::emit::sequence::{ComparatorSequence, IdSequence, TimestampSequence};
use std::cmp::Ordering;
use std::time::SystemTime;
use std::marker::PhantomData;

/// Tokio implementation of ComparatorSequence
pub struct TokioComparatorSequence<T, F> {
    comparator: F,
    _phantom: PhantomData<T>,
}

impl<T, F> TokioComparatorSequence<T, F>
where
    F: Fn(&T, &T) -> Ordering + Send + Sync + 'static,
{
    pub fn new(comparator: F) -> Self {
        Self {
            comparator,
            _phantom: PhantomData,
        }
    }
}

impl<T, F> ComparatorSequence<T> for TokioComparatorSequence<T, F>
where
    F: Fn(&T, &T) -> Ordering + Send + Sync + 'static,
{
    fn compare(&self, a: &T, b: &T) -> Ordering {
        (self.comparator)(a, b)
    }
}

/// Tokio implementation of TimestampSequence
pub struct TokioTimestampSequence {
    timestamp: SystemTime,
}

impl TokioTimestampSequence {
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now(),
        }
    }
    
    pub fn with_timestamp(timestamp: SystemTime) -> Self {
        Self { timestamp }
    }
}

impl Default for TokioTimestampSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl TimestampSequence for TokioTimestampSequence {
    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
}

/// Tokio implementation of IdSequence
pub struct TokioIdSequence<T>
where
    T: Ord + Copy + Clone,
{
    id: T,
}

impl<T> TokioIdSequence<T>
where
    T: Ord + Copy + Clone,
{
    pub fn new(id: T) -> Self {
        Self { id }
    }
}

impl<T> IdSequence<T> for TokioIdSequence<T>
where
    T: Ord + Copy + Clone,
{
    fn sequence_id(&self) -> T {
        self.id
    }
}