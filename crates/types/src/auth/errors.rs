//! Authentication and authorization error types

use thiserror::Error;

/// Authentication and authorization errors
#[derive(Error, Debug, Clone)]
pub enum AuthError {
	#[error("Authentication failed: {0}")]
	AuthenticationFailed(String),

	#[error("Authorization denied: {0}")]
	AuthorizationDenied(String),

	#[error("Invalid token: {0}")]
	InvalidToken(String),

	#[error("Token expired")]
	TokenExpired,

	#[error("Invalid credentials")]
	InvalidCredentials,

	#[error("Auth service unavailable: {0}")]
	ServiceUnavailable(String),

	#[error("Configuration error: {0}")]
	ConfigurationError(String),
}

/// Rate limiting errors
#[derive(Error, Debug, Clone)]
pub enum RateLimitError {
	#[error("Rate limit exceeded")]
	RateLimitExceeded,

	#[error("Invalid rate limit configuration: {0}")]
	InvalidConfiguration(String),

	#[error("Rate limiter storage error: {0}")]
	StorageError(String),

	#[error("Rate limiter unavailable: {0}")]
	ServiceUnavailable(String),
}
