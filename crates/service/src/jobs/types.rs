//! Background job types and definitions

use chrono::{DateTime, Utc};
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

/// Metrics data collected during solver interactions for updating time-series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverMetricsUpdate {
	/// Response time for this specific request/interaction
	pub response_time_ms: u64,
	/// Whether the request was successful
	pub was_successful: bool,
	/// Whether the request timed out
	pub was_timeout: bool,
	/// Timestamp when the interaction occurred
	pub timestamp: DateTime<Utc>,
	/// Optional error message if the request failed
	pub error_message: Option<String>,
}

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

	/// Clean up old orders in final status
	OrdersCleanup,

	/// Monitor and update status for a specific order with exponential backoff
	OrderStatusMonitor { order_id: String, attempt: u32 },

	/// Update metrics for multiple solvers from a single aggregation
	AggregationMetricsUpdate {
		aggregation_id: String,
		solver_metrics: Vec<(String, SolverMetricsUpdate)>,
		aggregation_timestamp: DateTime<Utc>,
	},

	/// Clean up old time-series metrics data
	MetricsCleanup,
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
			BackgroundJob::OrdersCleanup => "Clean up old orders in final status".to_string(),
			BackgroundJob::OrderStatusMonitor { order_id, attempt } => {
				format!(
					"Monitor status for order '{}' (attempt {})",
					order_id, attempt
				)
			},
			BackgroundJob::AggregationMetricsUpdate {
				aggregation_id,
				solver_metrics,
				..
			} => {
				format!(
					"Update metrics for {} solvers from aggregation '{}'",
					solver_metrics.len(),
					aggregation_id
				)
			},
			BackgroundJob::MetricsCleanup => "Clean up old time-series metrics data".to_string(),
		}
	}

	/// Get the solver ID if this job is solver-specific
	pub fn solver_id(&self) -> Option<&str> {
		match self {
			BackgroundJob::SolverHealthCheck { solver_id }
			| BackgroundJob::FetchSolverAssets { solver_id } => Some(solver_id),
			BackgroundJob::AllSolversHealthCheck
			| BackgroundJob::AllSolversFetchAssets
			| BackgroundJob::OrdersCleanup
			| BackgroundJob::OrderStatusMonitor { .. }
			| BackgroundJob::AggregationMetricsUpdate { .. }
			| BackgroundJob::MetricsCleanup => None,
		}
	}
}
