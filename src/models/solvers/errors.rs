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

    #[error("Solver is inactive: {solver_id}")]
    Inactive { solver_id: String },

    #[error("Solver is in maintenance mode: {solver_id}")]
    Maintenance { solver_id: String },

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

/// Adapter-specific errors
#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Timeout occurred after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Invalid response format: {reason}")]
    InvalidResponse { reason: String },

    #[error("Solver returned error: {code} - {message}")]
    SolverError { code: String, message: String },

    #[error("Adapter not found: {adapter_id}")]
    AdapterNotFound { adapter_id: String },

    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    #[error("Unsupported operation: {operation}")]
    UnsupportedOperation { operation: String },

    #[error("Chain not supported: {chain_id}")]
    ChainNotSupported { chain_id: u64 },

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result types for solver operations
pub type SolverResult<T> = Result<T, SolverError>;
pub type SolverValidationResult<T> = Result<T, SolverValidationError>;
pub type AdapterResult<T> = Result<T, AdapterError>;

/// Health check result with detailed information
#[derive(Debug, Clone, PartialEq)]
pub struct HealthCheckResult {
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub consecutive_failures: u32,
}

impl HealthCheckResult {
    pub fn healthy(response_time_ms: u64) -> Self {
        Self {
            is_healthy: true,
            response_time_ms,
            error_message: None,
            last_check: chrono::Utc::now(),
            consecutive_failures: 0,
        }
    }

    pub fn unhealthy(error_message: String, consecutive_failures: u32) -> Self {
        Self {
            is_healthy: false,
            response_time_ms: 0,
            error_message: Some(error_message),
            last_check: chrono::Utc::now(),
            consecutive_failures,
        }
    }
}
