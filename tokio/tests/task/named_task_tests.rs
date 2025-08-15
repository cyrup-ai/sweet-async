//! Tests for TokioNamedTask implementation
//!
//! This module contains all tests for the named task functionality,
//! ensuring proper task identification, tagging, and display behavior.

use sweet_async_tokio::task::TokioNamedTask;
use sweet_async_api::task::NamedTask;

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

#[test]
fn test_named_task_trait_implementation() {
    let mut task = TokioNamedTask::new("initial_name");
    
    // Test NamedTask trait methods
    assert_eq!(task.name(), Some("initial_name"));
    
    task.set_name("updated_name".to_string());
    assert_eq!(task.name(), Some("updated_name"));
}

#[test]
fn test_named_task_default() {
    let task = TokioNamedTask::default();
    assert_eq!(task.name(), Some("unnamed_task"));
    assert_eq!(task.description(), None);
    assert!(task.tags().is_empty());
}

#[test]
fn test_named_task_display_formatting() {
    let task_without_tags = TokioNamedTask::new("simple_task");
    assert_eq!(format!("{}", task_without_tags), "simple_task");
    
    let task_with_tags = TokioNamedTask::new("complex_task")
        .with_tag("urgent")
        .with_tag("batch");
    assert_eq!(format!("{}", task_with_tags), "complex_task [urgent, batch]");
}

#[test]
fn test_named_task_with_tags_bulk() {
    let tags = vec!["test".to_string(), "integration".to_string(), "automated".to_string()];
    let task = TokioNamedTask::new("bulk_task")
        .with_tags(tags.clone());
    
    assert_eq!(task.tags(), tags);
    assert!(task.has_tag("test"));
    assert!(task.has_tag("integration"));
    assert!(task.has_tag("automated"));
    assert!(!task.has_tag("manual"));
}

#[test]
fn test_named_task_description_handling() {
    let task_with_desc = TokioNamedTask::new("described_task")
        .with_description("This task has a description");
    
    assert_eq!(task_with_desc.description(), Some("This task has a description"));
    
    let task_without_desc = TokioNamedTask::new("no_desc_task");
    assert_eq!(task_without_desc.description(), None);
}