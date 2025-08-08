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

/// Connection health check result
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterHealthResult {
	pub is_healthy: bool,
	pub response_time_ms: u64,
	pub error_message: Option<String>,
	pub last_check: chrono::DateTime<chrono::Utc>,
	pub consecutive_failures: u32,
	pub health_score: f64, // 0.0 to 1.0
	pub capabilities: Vec<String>,
}

impl AdapterHealthResult {
	pub fn healthy(response_time_ms: u64) -> Self {
		Self {
			is_healthy: true,
			response_time_ms,
			error_message: None,
			last_check: chrono::Utc::now(),
			consecutive_failures: 0,
			health_score: 1.0,
			capabilities: Vec::new(),
		}
	}

	pub fn unhealthy(error_message: String, consecutive_failures: u32) -> Self {
		let health_score = if consecutive_failures == 0 {
			0.5
		} else {
			(1.0 / (consecutive_failures as f64 + 1.0)).max(0.1)
		};

		Self {
			is_healthy: false,
			response_time_ms: 0,
			error_message: Some(error_message),
			last_check: chrono::Utc::now(),
			consecutive_failures,
			health_score,
			capabilities: Vec::new(),
		}
	}

	pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
		self.capabilities = capabilities;
		self
	}

	/// Get overall health status as string
	pub fn status(&self) -> &'static str {
		if self.is_healthy {
			if self.health_score > 0.9 {
				"excellent"
			} else if self.health_score > 0.7 {
				"good"
			} else {
				"fair"
			}
		} else if self.consecutive_failures > 5 {
			"critical"
		} else if self.consecutive_failures > 2 {
			"poor"
		} else {
			"degraded"
		}
	}
}

/// Capability check result
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterCapability {
	pub name: String,
	pub available: bool,
	pub version: Option<String>,
	pub description: Option<String>,
	pub required_config: Vec<String>,
}

impl AdapterCapability {
	pub fn new(name: String, available: bool) -> Self {
		Self {
			name,
			available,
			version: None,
			description: None,
			required_config: Vec::new(),
		}
	}

	pub fn with_version(mut self, version: String) -> Self {
		self.version = Some(version);
		self
	}

	pub fn with_description(mut self, description: String) -> Self {
		self.description = Some(description);
		self
	}

	pub fn with_required_config(mut self, config: Vec<String>) -> Self {
		self.required_config = config;
		self
	}
}

/// Result types for adapter operations
pub type AdapterResult<T> = Result<T, AdapterError>;
pub type AdapterValidationResult<T> = Result<T, AdapterValidationError>;
pub type AdapterFactoryResult<T> = Result<T, AdapterFactoryError>;

/// Performance metrics for adapter operations
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterPerformanceMetrics {
	pub total_requests: u64,
	pub successful_requests: u64,
	pub failed_requests: u64,
	pub timeout_requests: u64,
	pub avg_response_time_ms: f64,
	pub min_response_time_ms: u64,
	pub max_response_time_ms: u64,
	pub p95_response_time_ms: u64,
	pub success_rate: f64,
	pub error_rate: f64,
	pub last_reset: chrono::DateTime<chrono::Utc>,
}

impl AdapterPerformanceMetrics {
	pub fn new() -> Self {
		Self {
			total_requests: 0,
			successful_requests: 0,
			failed_requests: 0,
			timeout_requests: 0,
			avg_response_time_ms: 0.0,
			min_response_time_ms: 0,
			max_response_time_ms: 0,
			p95_response_time_ms: 0,
			success_rate: 0.0,
			error_rate: 0.0,
			last_reset: chrono::Utc::now(),
		}
	}

	pub fn record_success(&mut self, response_time_ms: u64) {
		self.total_requests += 1;
		self.successful_requests += 1;

		// Update response time metrics
		if self.min_response_time_ms == 0 || response_time_ms < self.min_response_time_ms {
			self.min_response_time_ms = response_time_ms;
		}
		if response_time_ms > self.max_response_time_ms {
			self.max_response_time_ms = response_time_ms;
		}

		// Update average
		let total_time = self.avg_response_time_ms * (self.total_requests - 1) as f64;
		self.avg_response_time_ms =
			(total_time + response_time_ms as f64) / self.total_requests as f64;

		// Update rates
		self.success_rate = self.successful_requests as f64 / self.total_requests as f64;
		self.error_rate = self.failed_requests as f64 / self.total_requests as f64;

		// Estimate P95 (simplified)
		self.p95_response_time_ms =
			((self.avg_response_time_ms * 1.5) as u64).min(self.max_response_time_ms);
	}

	pub fn record_failure(&mut self, is_timeout: bool) {
		self.total_requests += 1;
		self.failed_requests += 1;

		if is_timeout {
			self.timeout_requests += 1;
		}

		// Update rates
		self.success_rate = self.successful_requests as f64 / self.total_requests as f64;
		self.error_rate = self.failed_requests as f64 / self.total_requests as f64;
	}

	pub fn reset(&mut self) {
		*self = Self::new();
	}

	pub fn is_performing_well(&self) -> bool {
		self.success_rate > 0.95 && self.avg_response_time_ms < 1000.0
	}
}

impl Default for AdapterPerformanceMetrics {
	fn default() -> Self {
		Self::new()
	}
}
