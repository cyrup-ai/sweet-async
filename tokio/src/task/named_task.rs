//! Named task implementation for task identification

use std::sync::Arc;
use sweet_async_api::task::NamedTask;

/// Tokio-specific named task implementation
#[derive(Debug, Clone)]
pub struct TokioNamedTask {
    name: Arc<String>,
    description: Option<Arc<String>>,
    tags: Arc<Vec<String>>,
}

impl TokioNamedTask {
    /// Create a new named task
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Arc::new(name.into()),
            description: None,
            tags: Arc::new(Vec::new()),
        }
    }

    /// Set task description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(Arc::new(description.into()));
        self
    }

    /// Add tags to the task
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Arc::new(tags);
        self
    }

    /// Add a single tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        let mut tags = (*self.tags).clone();
        tags.push(tag.into());
        self.tags = Arc::new(tags);
        self
    }

    /// Get task description
    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|d| d.as_str())
    }

    /// Get task tags
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Check if task has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }

    /// Get a display name that includes tags if available
    pub fn display_name(&self) -> String {
        if self.tags.is_empty() {
            self.name.to_string()
        } else {
            format!("{} [{}]", self.name, self.tags.join(", "))
        }
    }
}

impl Default for TokioNamedTask {
    fn default() -> Self {
        Self::new("unnamed_task")
    }
}

impl NamedTask for TokioNamedTask {
    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }

    fn set_name(&mut self, name: String) {
        self.name = Arc::new(name);
    }
}

impl std::fmt::Display for TokioNamedTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_task_creation() {
        let task = TokioNamedTask::new("test_task")
            .with_description("A test task")
            .with_tag("testing")
            .with_tag("example");
        
        assert_eq!(task.name(), Some("test_task"));
        assert_eq!(task.description(), Some("A test task"));
        assert!(task.has_tag("testing"));
        assert!(task.has_tag("example"));
        assert!(!task.has_tag("nonexistent"));
    }

    #[test]
    fn test_display_name() {
        let task = TokioNamedTask::new("test")
            .with_tag("tag1")
            .with_tag("tag2");
        
        assert_eq!(task.display_name(), "test [tag1, tag2]");
    }

    #[test]
    fn test_display_name_without_tags() {
        let task = TokioNamedTask::new("test");
        assert_eq!(task.display_name(), "test");
    }
}