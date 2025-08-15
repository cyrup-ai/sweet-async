//! Tokio implementation of sequence types

use sweet_async_api::task::emit::sequence::ComparatorSequence;

/// Tokio implementation of Sequence trait
#[derive(Debug)]
pub struct TokioSequence<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TokioSequence<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> ComparatorSequence<T> for TokioSequence<T>
where
    T: Send + 'static,
{
    fn compare(&self, a: &T, b: &T) -> std::cmp::Ordering {
        // TODO: Implement comparison logic
        std::cmp::Ordering::Equal
    }
}
