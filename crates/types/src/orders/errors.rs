//! Error types for order operations

use thiserror::Error;

/// Validation errors for order submissions
#[derive(Error, Debug)]
pub enum OrderValidationError {
	#[error("Invalid order ID: {order_id}")]
	InvalidOrderId { order_id: String },

	#[error("Invalid user address: {address}")]
	InvalidUserAddress { address: String },

	#[error("Invalid quote ID: {quote_id}")]
	InvalidQuoteId { quote_id: String },

	#[error("Invalid slippage tolerance: {slippage} (must be between 0.0 and 1.0)")]
	InvalidSlippage { slippage: f64 },

	#[error("Invalid deadline: {reason}")]
	InvalidDeadline { reason: String },

	#[error("Deadline must be in the future")]
	DeadlineInPast,

	#[error("Deadline too far in the future (max {max_hours} hours)")]
	DeadlineTooFar { max_hours: u64 },

	#[error("Missing required field: {field}")]
	MissingRequiredField { field: String },

	#[error("Invalid signature: {reason}")]
	InvalidSignature { reason: String },

	#[error("Quote not found: {quote_id}")]
	QuoteNotFound { quote_id: String },

	#[error("Quote expired: {quote_id}")]
	QuoteExpired { quote_id: String },

	#[error("Quote and quote_response cannot both be provided")]
	ConflictingQuoteData,

	#[error("Either quote_id or quote_response must be provided")]
	MissingQuoteData,

	#[error("Invalid metadata: {reason}")]
	InvalidMetadata { reason: String },

	#[error("Order amount too small: {amount} (minimum {min_amount})")]
	AmountTooSmall { amount: String, min_amount: String },

	#[error("Order amount too large: {amount} (maximum {max_amount})")]
	AmountTooLarge { amount: String, max_amount: String },

	#[error("Unsupported chain: {chain_id}")]
	UnsupportedChain { chain_id: u64 },

	#[error("Token not supported: {token_address} on chain {chain_id}")]
	UnsupportedToken {
		token_address: String,
		chain_id: u64,
	},

	#[error("Invalid configuration: {reason}")]
	InvalidConfiguration { reason: String },
}

/// Order operation errors
#[derive(Error, Debug)]
pub enum OrderError {
	#[error("Order validation failed: {0}")]
	Validation(#[from] OrderValidationError),

	#[error("Order not found: {order_id}")]
	NotFound { order_id: String },

	#[error("Order already exists: {order_id}")]
	AlreadyExists { order_id: String },

	#[error("Order is in invalid state: {order_id} - current state: {current_state}")]
	InvalidState {
		order_id: String,
		current_state: String,
	},

	#[error("Order submission failed: {order_id} - {reason}")]
	SubmissionFailed { order_id: String, reason: String },

	#[error("Order execution failed: {order_id} - {reason}")]
	ExecutionFailed { order_id: String, reason: String },

	#[error("Order timeout: {order_id} after {timeout_ms}ms")]
	Timeout { order_id: String, timeout_ms: u64 },

	#[error("Order cancelled: {order_id} - {reason}")]
	Cancelled { order_id: String, reason: String },

	#[error("Quote validation failed: {quote_id} - {reason}")]
	QuoteValidationFailed { quote_id: String, reason: String },

	#[error("Solver error: {solver_id} - {message}")]
	SolverError { solver_id: String, message: String },

	#[error("Transaction failed: {transaction_hash} - {reason}")]
	TransactionFailed {
		transaction_hash: String,
		reason: String,
	},

	#[error("Insufficient balance: required {required}, available {available}")]
	InsufficientBalance { required: String, available: String },

	#[error("Price impact too high: {price_impact}% (max allowed: {max_allowed}%)")]
	PriceImpactTooHigh { price_impact: f64, max_allowed: f64 },

	#[error("Network error: {0}")]
	Network(#[from] reqwest::Error),

	#[error("Serialization error: {0}")]
	Serialization(#[from] serde_json::Error),

	#[error("Storage error: {0}")]
	Storage(String),

	#[error("Authentication error: {0}")]
	Authentication(String),

	#[error("Rate limit exceeded for user: {user_address}")]
	RateLimitExceeded { user_address: String },

	#[error("Service unavailable: {reason}")]
	ServiceUnavailable { reason: String },

	#[error("Internal error: {0}")]
	Internal(String),
}
