//! CPU usage tracking for tasks

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sweet_async_api::task::CpuUsage;

/// CPU usage tracker for tasks
#[derive(Debug)]
pub struct TokioCpuUsageTracker {
    cpu_time_ns: Arc<AtomicU64>,
    start_time: Option<Instant>,
    samples: Arc<std::sync::Mutex<Vec<CpuSample>>>,
}

#[derive(Debug, Clone)]
struct CpuSample {
    timestamp: Instant,
    cpu_percent: f64,
}

impl TokioCpuUsageTracker {
    /// Create a new CPU usage tracker
    pub fn new() -> Self {
        Self {
            cpu_time_ns: Arc::new(AtomicU64::new(0)),
            start_time: None,
            samples: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Start tracking CPU usage
    pub fn start_tracking(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Record CPU usage sample
    pub fn record_sample(&self, cpu_percent: f64) {
        let sample = CpuSample {
            timestamp: Instant::now(),
            cpu_percent,
        };
        
        if let Ok(mut samples) = self.samples.lock() {
            samples.push(sample);
            // Keep only last 100 samples to prevent memory growth
            if samples.len() > 100 {
                samples.remove(0);
            }
        }
    }

    /// Get average CPU usage
    pub fn average_cpu_usage(&self) -> f64 {
        if let Ok(samples) = self.samples.lock() {
            if samples.is_empty() {
                return 0.0;
            }
            
            let sum: f64 = samples.iter().map(|s| s.cpu_percent).sum();
            sum / samples.len() as f64
        } else {
            0.0
        }
    }

    /// Get peak CPU usage
    pub fn peak_cpu_usage(&self) -> f64 {
        if let Ok(samples) = self.samples.lock() {
            samples.iter().map(|s| s.cpu_percent).fold(0.0, f64::max)
        } else {
            0.0
        }
    }

    /// Get current CPU time in nanoseconds
    pub fn cpu_time_ns(&self) -> u64 {
        self.cpu_time_ns.load(Ordering::SeqCst)
    }

    /// Add CPU time
    pub fn add_cpu_time(&self, ns: u64) {
        self.cpu_time_ns.fetch_add(ns, Ordering::SeqCst);
    }
}

impl Default for TokioCpuUsageTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuUsage for TokioCpuUsageTracker {
    fn cpu_time(&self) -> Duration {
        Duration::from_nanos(self.cpu_time_ns())
    }

    fn utilization(&self) -> f64 {
        self.average_cpu_usage() / 100.0  // Convert percentage to fraction
    }

    fn user_time(&self) -> Duration {
        // For now, assume all CPU time is user time
        // In a real implementation, this would track user vs system time separately
        Duration::from_nanos(self.cpu_time_ns())
    }

    fn system_time(&self) -> Duration {
        // For now, return zero system time
        // In a real implementation, this would track system/kernel time
        Duration::ZERO
    }
}