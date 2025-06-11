//! Placeholder enums referenced in README examples.  Flesh out later.

#![allow(dead_code)]

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Delimiter       { NewLine }
pub enum Browser         { Chrome }
pub enum BrowserProfile  { Primary }
pub enum FeedType        { Replies }
pub enum PublicSentiment { Negative }
pub enum Offset          { Latest }
pub enum PartitionStrategy { RoundRobin }
pub enum TimeRange       { LastHour }
pub enum ResultOrder     { Influence }
pub enum EventOrder      { StartTime }
pub enum TaskOrder       { Priority }
pub enum PostOrder       { Relevance }
pub enum EventType       { Meeting }
pub enum TaskStatus      { Uncompleted }
pub enum Region          { UsEast1 }

// "wrapper" types used as semantic markers
pub struct DateRange;
pub struct RetryStrategy;