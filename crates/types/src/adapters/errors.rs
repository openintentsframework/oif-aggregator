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

	#[error("HTTP {status_code}: {reason}")]
	HttpStatusError { status_code: u16, reason: String },

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

impl AdapterError {
	/// Extract HTTP status code from the error if available
	pub fn status_code(&self) -> Option<u16> {
		match self {
			AdapterError::HttpStatusError { status_code, .. } => Some(*status_code),
			AdapterError::HttpError(reqwest_error) => {
				// Extract status code from reqwest error if available
				reqwest_error.status().map(|status| status.as_u16())
			},
			_ => None,
		}
	}

	/// Create an HTTP failure error with the given status code and reason
	pub fn http_failure(status_code: u16, reason: impl Into<String>) -> Self {
		Self::HttpStatusError {
			status_code,
			reason: reason.into(),
		}
	}

	/// Create an HTTP failure error from response status with default reason
	pub fn from_http_failure(status_code: u16) -> Self {
		let reason = match status_code {
			400 => "Bad Request".to_string(),
			401 => "Unauthorized".to_string(),
			403 => "Forbidden".to_string(),
			404 => "Not Found".to_string(),
			408 => "Request Timeout".to_string(),
			429 => "Too Many Requests".to_string(),
			500 => "Internal Server Error".to_string(),
			502 => "Bad Gateway".to_string(),
			503 => "Service Unavailable".to_string(),
			504 => "Gateway Timeout".to_string(),
			_ => format!("HTTP Error {}", status_code),
		};

		Self::HttpStatusError {
			status_code,
			reason,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_adapter_error_status_code_extraction() {
		// Test direct HttpStatusError
		let error = AdapterError::HttpStatusError {
			status_code: 404,
			reason: "Not Found".to_string(),
		};
		assert_eq!(error.status_code(), Some(404));

		// Test http_failure constructor
		let error = AdapterError::http_failure(500, "Internal Server Error");
		assert_eq!(error.status_code(), Some(500));

		// Test from_http_failure constructor
		let error = AdapterError::from_http_failure(429);
		assert_eq!(error.status_code(), Some(429));

		// Test other error types return None
		let error = AdapterError::InvalidResponse {
			reason: "Bad response".to_string(),
		};
		assert_eq!(error.status_code(), None);
	}

	#[test]
	fn test_http_failure_status_message_mapping() {
		let error = AdapterError::from_http_failure(404);
		assert!(error.to_string().contains("404"));
		assert!(error.to_string().contains("Not Found"));

		let error = AdapterError::from_http_failure(408);
		assert!(error.to_string().contains("408"));
		assert!(error.to_string().contains("Request Timeout"));

		let error = AdapterError::from_http_failure(500);
		assert!(error.to_string().contains("500"));
		assert!(error.to_string().contains("Internal Server Error"));

		let error = AdapterError::from_http_failure(429);
		assert!(error.to_string().contains("429"));
		assert!(error.to_string().contains("Too Many Requests"));
	}
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
