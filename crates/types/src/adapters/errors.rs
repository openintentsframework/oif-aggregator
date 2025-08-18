//! Error types for adapter operations

use thiserror::Error;

/// Validation errors for adapter configurations
#[derive(Error, Debug)]
pub enum AdapterValidationError {
	#[error("Invalid adapter ID: {adapter_id}")]
	InvalidAdapterId { adapter_id: String },

	#[error("Invalid adapter name: {name}")]
	InvalidAdapterName { name: String },

	#[error("Invalid version format: {version}")]
	InvalidVersion { version: String },

	#[error("Missing required field: {field}")]
	MissingRequiredField { field: String },

	#[error("Invalid configuration: {reason}")]
	InvalidConfiguration { reason: String },

	#[error("Unsupported adapter type: {adapter_type}")]
	UnsupportedAdapterType { adapter_type: String },

	#[error("Incompatible adapter version: expected {expected}, got {actual}")]
	IncompatibleVersion { expected: String, actual: String },

	#[error("Configuration schema validation failed: {reason}")]
	SchemaValidationFailed { reason: String },
}

/// Adapter operation errors
#[derive(Error, Debug)]
pub enum AdapterError {
	#[error("Adapter validation failed: {0}")]
	Validation(#[from] AdapterValidationError),

	#[error("Adapter not found: {adapter_id}")]
	NotFound { adapter_id: String },

	#[error("Adapter is disabled: {adapter_id}")]
	Disabled { adapter_id: String },

	#[error("Adapter initialization failed: {adapter_id} - {reason}")]
	InitializationFailed { adapter_id: String, reason: String },

	#[error("HTTP request failed: {0}")]
	HttpError(#[from] reqwest::Error),

	#[error("Timeout occurred after {timeout_ms}ms")]
	Timeout { timeout_ms: u64 },

	#[error("Invalid response format: {reason}")]
	InvalidResponse { reason: String },

	#[error("Solver returned error: {code} - {message}")]
	SolverError { code: String, message: String },

	#[error("Configuration error: {reason}")]
	ConfigError { reason: String },

	#[error("Unsupported operation: {operation} for adapter {adapter_id}")]
	UnsupportedOperation {
		operation: String,
		adapter_id: String,
	},

	#[error("Chain not supported: {chain_id} by adapter {adapter_id}")]
	ChainNotSupported { chain_id: u64, adapter_id: String },

	#[error("Rate limit exceeded for adapter {adapter_id}")]
	RateLimitExceeded { adapter_id: String },

	#[error("Authentication failed for adapter {adapter_id}")]
	AuthenticationFailed { adapter_id: String },

	#[error("Adapter is unhealthy: {adapter_id} - {reason}")]
	Unhealthy { adapter_id: String, reason: String },

	#[error("Protocol version mismatch: {adapter_id} expects {expected}, got {actual}")]
	ProtocolMismatch {
		adapter_id: String,
		expected: String,
		actual: String,
	},

	#[error("Connection error: {0}")]
	Connection(String),

	#[error("Serialization error: {0}")]
	Serialization(#[from] serde_json::Error),

	#[error("Storage error: {0}")]
	Storage(String),

	#[error("Network error: {0}")]
	Network(String),

	#[error("Not implemented: {0}")]
	NotImplemented(String),

	#[error("Unsupported adapter: {0}")]
	UnsupportedAdapter(String),
}

/// Factory-specific errors
#[derive(Error, Debug)]
pub enum AdapterFactoryError {
	#[error("Failed to create adapter: {adapter_type} - {reason}")]
	CreationFailed {
		adapter_type: String,
		reason: String,
	},

	#[error("Adapter already registered: {adapter_id}")]
	AlreadyRegistered { adapter_id: String },

	#[error("Adapter not registered: {adapter_id}")]
	NotRegistered { adapter_id: String },

	#[error("Factory configuration error: {reason}")]
	ConfigurationError { reason: String },

	#[error("Dependency injection failed: {dependency} for {adapter_id}")]
	DependencyInjectionFailed {
		dependency: String,
		adapter_id: String,
	},

	#[error("Maximum adapters limit reached: {limit}")]
	LimitReached { limit: usize },

	#[error("Factory is locked and cannot be modified")]
	FactoryLocked,
}
