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

/// Order execution result with detailed status
#[derive(Debug, Clone, PartialEq)]
pub struct OrderExecutionResult {
	pub success: bool,
	pub transaction_hash: Option<String>,
	pub block_number: Option<u64>,
	pub gas_used: Option<u64>,
	pub effective_gas_price: Option<String>,
	pub amount_in: Option<String>,
	pub amount_out: Option<String>,
	pub actual_price_impact: Option<f64>,
	pub gas_cost: Option<String>,
	pub execution_time_ms: Option<u64>,
	pub error_message: Option<String>,
	pub solver_used: Option<String>,
	pub retry_count: u32,
	pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl OrderExecutionResult {
	pub fn success(
		transaction_hash: String,
		block_number: u64,
		gas_used: u64,
		amount_out: String,
		solver_used: String,
		execution_time_ms: u64,
	) -> Self {
		Self {
			success: true,
			transaction_hash: Some(transaction_hash),
			block_number: Some(block_number),
			gas_used: Some(gas_used),
			effective_gas_price: None,
			amount_in: None,
			amount_out: Some(amount_out),
			actual_price_impact: None,
			gas_cost: None,
			execution_time_ms: Some(execution_time_ms),
			error_message: None,
			solver_used: Some(solver_used),
			retry_count: 0,
			timestamp: chrono::Utc::now(),
		}
	}

	pub fn failure(error_message: String, retry_count: u32) -> Self {
		Self {
			success: false,
			transaction_hash: None,
			block_number: None,
			gas_used: None,
			effective_gas_price: None,
			amount_in: None,
			amount_out: None,
			actual_price_impact: None,
			gas_cost: None,
			execution_time_ms: None,
			error_message: Some(error_message),
			solver_used: None,
			retry_count,
			timestamp: chrono::Utc::now(),
		}
	}

	pub fn with_amounts(mut self, amount_in: String, amount_out: String) -> Self {
		self.amount_in = Some(amount_in);
		self.amount_out = Some(amount_out);
		self
	}

	pub fn with_price_impact(mut self, price_impact: f64) -> Self {
		self.actual_price_impact = Some(price_impact);
		self
	}

	pub fn with_gas_cost(mut self, gas_cost: String, effective_gas_price: String) -> Self {
		self.gas_cost = Some(gas_cost);
		self.effective_gas_price = Some(effective_gas_price);
		self
	}
}

/// Order simulation result
#[derive(Debug, Clone, PartialEq)]
pub struct OrderSimulationResult {
	pub can_execute: bool,
	pub estimated_gas: Option<u64>,
	pub estimated_amount_out: Option<String>,
	pub estimated_price_impact: Option<f64>,
	pub estimated_execution_time_ms: Option<u64>,
	pub warnings: Vec<String>,
	pub errors: Vec<String>,
	pub simulated_at: chrono::DateTime<chrono::Utc>,
}

impl OrderSimulationResult {
	pub fn success(
		estimated_gas: u64,
		estimated_amount_out: String,
		estimated_price_impact: f64,
	) -> Self {
		Self {
			can_execute: true,
			estimated_gas: Some(estimated_gas),
			estimated_amount_out: Some(estimated_amount_out),
			estimated_price_impact: Some(estimated_price_impact),
			estimated_execution_time_ms: Some(5000), // Default 5s
			warnings: Vec::new(),
			errors: Vec::new(),
			simulated_at: chrono::Utc::now(),
		}
	}

	pub fn failure(errors: Vec<String>) -> Self {
		Self {
			can_execute: false,
			estimated_gas: None,
			estimated_amount_out: None,
			estimated_price_impact: None,
			estimated_execution_time_ms: None,
			warnings: Vec::new(),
			errors,
			simulated_at: chrono::Utc::now(),
		}
	}

	pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
		self.warnings = warnings;
		self
	}

	pub fn add_warning(&mut self, warning: String) {
		self.warnings.push(warning);
	}

	pub fn add_error(&mut self, error: String) {
		self.errors.push(error);
		self.can_execute = false;
	}
}

/// Order health check for system monitoring
#[derive(Debug, Clone, PartialEq)]
pub struct OrderSystemHealth {
	pub is_healthy: bool,
	pub pending_orders: u64,
	pub executing_orders: u64,
	pub failed_orders_last_hour: u64,
	pub success_rate_last_hour: f64,
	pub avg_execution_time_ms: f64,
	pub solver_availability: f64,
	pub last_check: chrono::DateTime<chrono::Utc>,
	pub issues: Vec<String>,
}

impl OrderSystemHealth {
	pub fn healthy(
		pending_orders: u64,
		executing_orders: u64,
		success_rate: f64,
		avg_execution_time_ms: f64,
	) -> Self {
		Self {
			is_healthy: true,
			pending_orders,
			executing_orders,
			failed_orders_last_hour: 0,
			success_rate_last_hour: success_rate,
			avg_execution_time_ms,
			solver_availability: 1.0,
			last_check: chrono::Utc::now(),
			issues: Vec::new(),
		}
	}

	pub fn unhealthy(issues: Vec<String>) -> Self {
		Self {
			is_healthy: false,
			pending_orders: 0,
			executing_orders: 0,
			failed_orders_last_hour: 0,
			success_rate_last_hour: 0.0,
			avg_execution_time_ms: 0.0,
			solver_availability: 0.0,
			last_check: chrono::Utc::now(),
			issues,
		}
	}

	pub fn status(&self) -> &'static str {
		if self.is_healthy {
			if self.success_rate_last_hour > 0.95 && self.avg_execution_time_ms < 10000.0 {
				"excellent"
			} else if self.success_rate_last_hour > 0.85 {
				"good"
			} else {
				"degraded"
			}
		} else {
			"unhealthy"
		}
	}
}

/// Result types for order operations
pub type OrderResult<T> = Result<T, OrderError>;
pub type OrderValidationResult<T> = Result<T, OrderValidationError>;

/// Order priority levels for execution ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrderPriority {
	Low = 1,
	Normal = 2,
	High = 3,
	Critical = 4,
}

impl OrderPriority {
	pub fn from_amount(amount_usd: f64) -> Self {
		if amount_usd > 100000.0 {
			Self::Critical
		} else if amount_usd > 10000.0 {
			Self::High
		} else if amount_usd > 1000.0 {
			Self::Normal
		} else {
			Self::Low
		}
	}

	pub fn execution_timeout_ms(&self) -> u64 {
		match self {
			Self::Critical => 30000, // 30 seconds
			Self::High => 60000,     // 1 minute
			Self::Normal => 120000,  // 2 minutes
			Self::Low => 300000,     // 5 minutes
		}
	}

	pub fn retry_count(&self) -> u32 {
		match self {
			Self::Critical => 5,
			Self::High => 3,
			Self::Normal => 2,
			Self::Low => 1,
		}
	}
}

impl std::fmt::Display for OrderPriority {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Low => write!(f, "low"),
			Self::Normal => write!(f, "normal"),
			Self::High => write!(f, "high"),
			Self::Critical => write!(f, "critical"),
		}
	}
}

/// Order rate limiting information
#[derive(Debug, Clone, PartialEq)]
pub struct OrderRateLimit {
	pub user_address: String,
	pub current_count: u32,
	pub limit: u32,
	pub window_start: chrono::DateTime<chrono::Utc>,
	pub window_duration_secs: u64,
	pub reset_at: chrono::DateTime<chrono::Utc>,
}

impl OrderRateLimit {
	pub fn new(user_address: String, limit: u32, window_duration_secs: u64) -> Self {
		let now = chrono::Utc::now();
		Self {
			user_address,
			current_count: 0,
			limit,
			window_start: now,
			window_duration_secs,
			reset_at: now + chrono::Duration::seconds(window_duration_secs as i64),
		}
	}

	pub fn is_exceeded(&self) -> bool {
		self.current_count >= self.limit
	}

	pub fn remaining(&self) -> u32 {
		self.limit.saturating_sub(self.current_count)
	}

	pub fn time_until_reset(&self) -> chrono::Duration {
		self.reset_at - chrono::Utc::now()
	}

	pub fn should_reset(&self) -> bool {
		chrono::Utc::now() >= self.reset_at
	}

	pub fn reset(&mut self) {
		let now = chrono::Utc::now();
		self.current_count = 0;
		self.window_start = now;
		self.reset_at = now + chrono::Duration::seconds(self.window_duration_secs as i64);
	}

	pub fn increment(&mut self) -> bool {
		if self.should_reset() {
			self.reset();
		}

		if self.is_exceeded() {
			false
		} else {
			self.current_count += 1;
			true
		}
	}
}
