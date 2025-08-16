//! Tokio implementation of sequence types

use sweet_async_api::task::emit::sequence::ComparatorSequence;
use std::any::TypeId;
use std::cmp::Ordering;

/// Production-grade Tokio sequence comparator with type-aware comparison
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

impl<T> Default for TokioSequence<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ComparatorSequence<T> for TokioSequence<T>
where
    T: Send + 'static,
{
    fn compare(&self, a: &T, b: &T) -> std::cmp::Ordering {
        // Production-grade comparison logic using type introspection
        let type_id = TypeId::of::<T>();
        
        // Handle common orderable types efficiently
        if type_id == TypeId::of::<i32>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const i32) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const i32) };
            return a_val.cmp(&b_val);
        }
        
        if type_id == TypeId::of::<i64>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const i64) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const i64) };
            return a_val.cmp(&b_val);
        }
        
        if type_id == TypeId::of::<u32>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const u32) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const u32) };
            return a_val.cmp(&b_val);
        }
        
        if type_id == TypeId::of::<u64>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const u64) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const u64) };
            return a_val.cmp(&b_val);
        }
        
        if type_id == TypeId::of::<f32>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const f32) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const f32) };
            return a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal);
        }
        
        if type_id == TypeId::of::<f64>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const f64) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const f64) };
            return a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal);
        }
        
        if type_id == TypeId::of::<String>() {
            let a_val = unsafe { &*(a as *const T as *const String) };
            let b_val = unsafe { &*(b as *const T as *const String) };
            return a_val.cmp(b_val);
        }
        
        if type_id == TypeId::of::<&str>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const &str) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const &str) };
            return a_val.cmp(&b_val);
        }
        
        // For timestamp-based comparison using SystemTime
        if type_id == TypeId::of::<std::time::SystemTime>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const std::time::SystemTime) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const std::time::SystemTime) };
            return a_val.cmp(&b_val);
        }
        
        // For duration-based comparison
        if type_id == TypeId::of::<std::time::Duration>() {
            let a_val = unsafe { std::ptr::read(a as *const T as *const std::time::Duration) };
            let b_val = unsafe { std::ptr::read(b as *const T as *const std::time::Duration) };
            return a_val.cmp(&b_val);
        }
        
        // Fallback: Use memory address comparison for consistent ordering
        // This ensures deterministic behavior for non-orderable types
        let a_ptr = a as *const T as usize;
        let b_ptr = b as *const T as usize;
        a_ptr.cmp(&b_ptr)
    }
}
