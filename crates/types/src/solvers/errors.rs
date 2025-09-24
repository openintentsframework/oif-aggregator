//! Error types for solver operations

use thiserror::Error;

/// Validation errors for solver configurations
#[derive(Error, Debug)]
pub enum SolverValidationError {
	#[error("Invalid solver ID: {solver_id}")]
	InvalidSolverId { solver_id: String },

	#[error("Invalid adapter ID: {adapter_id}")]
	InvalidAdapterId { adapter_id: String },

	#[error("Invalid endpoint URL: {endpoint} - {reason}")]
	InvalidEndpoint { endpoint: String, reason: String },

	#[error("Invalid timeout: {timeout_ms}ms (must be between {min}ms and {max}ms)")]
	InvalidTimeout { timeout_ms: u64, min: u64, max: u64 },

	#[error("Invalid chain ID: {chain_id}")]
	InvalidChainId { chain_id: u64 },

	#[error("Missing required field: {field}")]
	MissingRequiredField { field: String },

	#[error("Invalid configuration: {reason}")]
	InvalidConfiguration { reason: String },

	#[error("Unsupported adapter type: {adapter_type}")]
	UnsupportedAdapterType { adapter_type: String },

	#[error("Invalid retry count: {retries} (must be between 0 and {max})")]
	InvalidRetryCount { retries: u32, max: u32 },
}

/// Solver operation errors
#[derive(Error, Debug)]
pub enum SolverError {
	#[error("Solver validation failed: {0}")]
	Validation(#[from] SolverValidationError),

	#[error("Solver not found: {solver_id}")]
	NotFound { solver_id: String },

	#[error("Solver is disabled: {solver_id}")]
	Disabled { solver_id: String },

	#[error("Solver health check failed: {solver_id} - {reason}")]
	HealthCheckFailed { solver_id: String, reason: String },

	#[error("Solver timeout: {solver_id} after {timeout_ms}ms")]
	Timeout { solver_id: String, timeout_ms: u64 },

	#[error("Solver error: {solver_id} - {message}")]
	SolverError { solver_id: String, message: String },

	#[error("Configuration error: {0}")]
	Configuration(String),

	#[error("Storage error: {0}")]
	Storage(String),

	#[error("Adapter error: {0}")]
	Adapter(String),

	#[error("Network error: {0}")]
	Network(#[from] reqwest::Error),

	#[error("Serialization error: {0}")]
	Serialization(#[from] serde_json::Error),
}
