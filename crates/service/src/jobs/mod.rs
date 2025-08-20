//! Background job processing system
//!
//! This module provides a flexible job processing system that can handle
//! multiple job types with different handlers. The implementation is hidden
//! to allow for future backend changes.

pub mod handlers;
pub mod processor;
pub mod types;

pub use handlers::BackgroundJobHandler;
pub use processor::{
	JobInfo, JobInfoStats, JobProcessor, JobProcessorConfig, JobStatus, RetryPolicy, ScheduledJob,
};
pub use types::{BackgroundJob, JobError, JobResult};

#[cfg(test)]
mod tests;
