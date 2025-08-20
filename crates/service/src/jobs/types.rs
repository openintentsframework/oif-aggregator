//! Background job types and definitions

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during job processing
#[derive(Debug, Error)]
pub enum JobError {
	#[error("Job processing failed: {message}")]
	ProcessingFailed { message: String },

	#[error("Job queue is full")]
	QueueFull,

	#[error("Job with ID '{id}' is already active or queued")]
	Duplicate { id: String },

	#[error("Job processor is shutting down")]
	ShuttingDown,

	#[error("Storage error: {0}")]
	Storage(String),

	#[error("Adapter error: {0}")]
	Adapter(String),

	#[error("Invalid job configuration: {0}")]
	InvalidConfig(String),
}

/// Result type for job operations
pub type JobResult<T = ()> = Result<T, JobError>;

/// Background job types that can be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackgroundJob {
	/// Perform health check on a specific solver
	SolverHealthCheck { solver_id: String },

	/// Fetch and update supported assets/networks for a solver
	FetchSolverAssets { solver_id: String },

	/// Perform health checks on all active solvers
	AllSolversHealthCheck,

	/// Fetch assets for all solvers that need it
	AllSolversFetchAssets,
}

impl BackgroundJob {
	/// Get a human-readable description of the job
	pub fn description(&self) -> String {
		match self {
			BackgroundJob::SolverHealthCheck { solver_id } => {
				format!("Health check for solver '{}'", solver_id)
			},
			BackgroundJob::FetchSolverAssets { solver_id } => {
				format!("Fetch assets for solver '{}'", solver_id)
			},
			BackgroundJob::AllSolversHealthCheck => "Health check for all solvers".to_string(),
			BackgroundJob::AllSolversFetchAssets => "Fetch assets for all solvers".to_string(),
		}
	}

	/// Get the solver ID if this job is solver-specific
	pub fn solver_id(&self) -> Option<&str> {
		match self {
			BackgroundJob::SolverHealthCheck { solver_id }
			| BackgroundJob::FetchSolverAssets { solver_id } => Some(solver_id),
			BackgroundJob::AllSolversHealthCheck | BackgroundJob::AllSolversFetchAssets => None,
		}
	}
}
