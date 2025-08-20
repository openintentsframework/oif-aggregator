//! Background job processing system
//!
//! This module provides a flexible job processing system that can handle
//! multiple job types with different handlers. The implementation is hidden
//! to allow for future backend changes.

use async_trait::async_trait;

pub mod generic_handler;
pub mod handlers;
pub mod processor;
pub mod types;

pub use handlers::BackgroundJobHandler;
pub use processor::{
	JobInfo, JobInfoStats, JobProcessor, JobProcessorConfig, JobStatus, RetryPolicy, ScheduledJob,
};
pub use types::{BackgroundJob, JobError, JobResult};

// Re-export generic handler components for the new approach
pub use generic_handler::{
	BulkFetchAssetsParams, BulkHealthCheckParams, GenericJobHandler, SolverFetchAssetsParams,
	SolverHealthCheckParams, UniversalJobHandler,
};

/// Trait for handling specific job types with typed parameters
///
/// This allows users to implement their own handlers for custom job processing
/// while maintaining type safety and consistency.
#[async_trait]
pub trait SpecificJobHandler<T>: Send + Sync {
	/// Handle a specific job with typed input
	async fn handle(&self, job_data: T) -> JobResult<()>;
}

/// Trait for solver-specific job handlers that take a solver ID
#[async_trait]
pub trait SolverJobHandler: Send + Sync {
	/// Handle a job for a specific solver
	async fn handle(&self, solver_id: &str) -> JobResult<()>;
}

#[cfg(test)]
mod tests;
