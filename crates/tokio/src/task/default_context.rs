use std::env;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use sweet_async_api::task::{AsyncTask, ContextualizedTask, TaskId, TaskRelationships};
use sweet_async_api::orchestra::runtime::Runtime;
use crate::UuidTaskId;

/// Default implementation of task context
///
/// Provides automatic population of hostname, IP address, and current working directory
/// for tasks that don't need custom implementations.
#[derive(Clone, Debug)]
pub struct DefaultTaskContext {
    hostname: String,
    ipaddr: IpAddr,
    cwd: PathBuf,
}

impl DefaultTaskContext {
    /// Create a new default task context with system information
    pub fn new() -> Self {
        Self {
            hostname: Self::get_hostname(),
            ipaddr: Self::get_primary_ip(),
            cwd: Self::get_cwd(),
        }
    }

    /// Get the system hostname
    fn get_hostname() -> String {
        hostname::get()
            .ok()
            .and_then(|name| name.into_string().ok())
            .unwrap_or_else(|| "localhost".to_string())
    }

    /// Get the primary IP address of the system
    /// 
    /// This attempts to find the first non-loopback IP address.
    /// Falls back to localhost if no suitable address is found.
    fn get_primary_ip() -> IpAddr {
        use std::net::UdpSocket;
        
        // Try to connect to a public DNS server to determine our IP
        // This doesn't actually send any data
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    return addr.ip();
                }
            }
        }
        
        // Fallback to localhost
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    }

    /// Get the current working directory
    fn get_cwd() -> PathBuf {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    }
}

impl Default for DefaultTaskContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Example task type that uses DefaultTaskContext
pub struct TaskWithDefaultContext<T: Clone + Send + 'static> {
    context: DefaultTaskContext,
    relationships: TaskRelationships<T, UuidTaskId>,
    runtime: Option<Box<dyn Runtime<T, UuidTaskId, SpawnedTask = Box<dyn AsyncTask<T, UuidTaskId>>>>>,
}

impl<T: Clone + Send + 'static> TaskWithDefaultContext<T> {
    pub fn new() -> Self {
        Self {
            context: DefaultTaskContext::new(),
            relationships: TaskRelationships::default(),
            runtime: None,
        }
    }
}

impl<T: Clone + Send + 'static> ContextualizedTask<T, UuidTaskId> for TaskWithDefaultContext<T> {
    type RuntimeType = Box<dyn Runtime<T, UuidTaskId>>;
    
    fn relationships(&self) -> &TaskRelationships<T, UuidTaskId> {
        &self.relationships
    }
    
    fn relationships_mut(&mut self) -> &mut TaskRelationships<T, UuidTaskId> {
        &mut self.relationships
    }
    
    fn runtime(&self) -> &Self::RuntimeType {
        self.runtime.as_ref().expect("Runtime not set")
    }
    
    fn cwd(&self) -> PathBuf {
        self.context.cwd.clone()
    }
    
}