use std::cmp::Ordering;
use std::time::SystemTime;

/// Trait for custom comparator-based sequence ordering
pub trait ComparatorSequence<T> {
    fn compare(&self, a: &T, b: &T) -> Ordering;
}

/// Trait for timestamp-based sequence ordering
pub trait TimestampSequence {
    fn timestamp(&self) -> SystemTime;
}

/// Trait for ID-based sequence ordering with any orderable numeric type
pub trait IdSequence<T>
where
    T: Ord + Copy + Clone,
{
    fn sequence_id(&self) -> T;
} 