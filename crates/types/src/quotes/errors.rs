//! Error types for quote operations

use thiserror::Error;

/// Validation errors for quote requests
#[derive(Error, Debug)]
pub enum QuoteValidationError {
	#[error("Invalid token address: {field}")]
	InvalidTokenAddress { field: String },

	#[error("Invalid amount: {field} - {reason}")]
	InvalidAmount { field: String, reason: String },

	#[error("Invalid chain ID: {chain_id}")]
	InvalidChainId { chain_id: u64 },

	#[error("Invalid slippage tolerance: {value} (must be between 0 and 1)")]
	InvalidSlippageTolerance { value: f64 },

	#[error("Invalid deadline: {reason}")]
	InvalidDeadline { reason: String },

	#[error("Missing required field: {field}")]
	MissingRequiredField { field: String },

	#[error("Unsupported chain: {chain_id}")]
	UnsupportedChain { chain_id: u64 },
}

/// General quote-related errors
#[derive(Error, Debug)]
pub enum QuoteError {
	#[error("Quote validation failed: {0}")]
	Validation(#[from] QuoteValidationError),

	#[error("Quote has expired")]
	Expired,

	#[error("Quote not found: {quote_id}")]
	NotFound { quote_id: String },

	#[error("Quote processing failed: {reason}")]
	ProcessingFailed { reason: String },

	#[error("Storage error: {0}")]
	Storage(String),

	#[error("Serialization error: {0}")]
	Serialization(#[from] serde_json::Error),
}
