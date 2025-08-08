//! Core Order domain model and business logic

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod errors;
pub mod request;
pub mod response;
pub mod storage;

pub use errors::{
	OrderError, OrderExecutionResult, OrderPriority, OrderRateLimit, OrderSimulationResult,
	OrderSystemHealth, OrderValidationError, OrderValidationResult,
};
pub use request::OrdersRequest;
pub use response::OrdersResponse;
pub use storage::OrderStorage;

/// Core Order domain model
///
/// This represents an order in the domain layer with business logic.
/// It should be converted from OrdersRequest and to OrderStorage/OrdersResponse.
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
	/// Unique identifier for the order
	pub order_id: String,

	/// Associated quote ID (if using a specific quote)
	pub quote_id: Option<String>,

	/// Quote data (if not using quote_id)
	pub quote_data: Option<OrderQuoteData>,

	/// User's wallet address
	pub user_address: String,

	/// Maximum acceptable slippage (0.0 to 1.0)
	pub slippage_tolerance: f64,

	/// Deadline for order execution
	pub deadline: DateTime<Utc>,

	/// User's signature for authorization
	pub signature: Option<String>,

	/// Order metadata and context
	pub metadata: OrderMetadata,

	/// Current execution status
	pub status: OrderStatus,

	/// Order priority for execution ordering
	pub priority: OrderPriority,

	/// Execution attempts and results
	pub execution_history: Vec<OrderExecutionResult>,

	/// Current retry count
	pub retry_count: u32,

	/// When the order was created
	pub created_at: DateTime<Utc>,

	/// Last time the order was updated
	pub updated_at: DateTime<Utc>,

	/// When the order was last processed
	pub last_processed_at: Option<DateTime<Utc>>,

	/// Estimated execution time
	pub estimated_execution_time_ms: Option<u64>,

	/// Associated solver for execution
	pub assigned_solver: Option<String>,

	/// Fee information
	pub fees: OrderFees,
}

/// Order execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
	/// Order has been received and is pending validation
	Pending,
	/// Order has been submitted to a solver
	Submitted,
	/// Order has been validated and is queued for execution
	Queued,
	/// Order is currently being processed
	Executing,
	/// Order has been successfully executed
	Success,
	/// Order execution failed
	Failed,
	/// Order was cancelled by user or system
	Cancelled,
	/// Order expired before execution
	Expired,
	/// Order is being simulated
	Simulating,
	/// Order requires manual review
	ReviewRequired,
}

/// Order metadata and context information
#[derive(Debug, Clone, PartialEq)]
pub struct OrderMetadata {
	/// Source of the order (web, mobile, api, etc.)
	pub source: String,

	/// User agent string
	pub user_agent: Option<String>,

	/// Referrer information
	pub referrer: Option<String>,

	/// Session identifier
	pub session_id: Option<String>,

	/// IP address (for rate limiting)
	pub ip_address: Option<String>,

	/// Additional custom metadata
	pub custom_data: HashMap<String, serde_json::Value>,

	/// Geographic location
	pub location: Option<String>,

	/// Client version
	pub client_version: Option<String>,
}

/// Quote data embedded in order (when not using quote_id)
#[derive(Debug, Clone, PartialEq)]
pub struct OrderQuoteData {
	/// Input token address
	pub token_in: String,

	/// Output token address
	pub token_out: String,

	/// Input amount
	pub amount_in: String,

	/// Expected output amount
	pub amount_out: String,

	/// Blockchain network
	pub chain_id: u64,

	/// Price impact estimation
	pub price_impact: Option<f64>,

	/// Gas estimation
	pub estimated_gas: Option<u64>,

	/// Route information
	pub route_info: Option<serde_json::Value>,

	/// When this quote expires
	pub expires_at: DateTime<Utc>,
}

/// Fee information for order execution
#[derive(Debug, Clone, PartialEq)]
pub struct OrderFees {
	/// Platform fee (percentage)
	pub platform_fee_rate: f64,

	/// Platform fee amount
	pub platform_fee_amount: Option<String>,

	/// Network gas fee
	pub gas_fee: Option<String>,

	/// Solver fee
	pub solver_fee: Option<String>,

	/// Total estimated fees
	pub total_estimated_fee: Option<String>,

	/// Fee currency
	pub fee_currency: String,
}

impl Order {
	/// Create a new order
	pub fn new(user_address: String, slippage_tolerance: f64, deadline: DateTime<Utc>) -> Self {
		let now = Utc::now();
		let order_id = uuid::Uuid::new_v4().to_string();

		Self {
			order_id,
			quote_id: None,
			quote_data: None,
			user_address,
			slippage_tolerance,
			deadline,
			signature: None,
			metadata: OrderMetadata::default(),
			status: OrderStatus::Pending,
			priority: OrderPriority::Normal,
			execution_history: Vec::new(),
			retry_count: 0,
			created_at: now,
			updated_at: now,
			last_processed_at: None,
			estimated_execution_time_ms: None,
			assigned_solver: None,
			fees: OrderFees::default(),
		}
	}

	/// Check if the order is executable
	pub fn is_executable(&self) -> bool {
		matches!(self.status, OrderStatus::Queued | OrderStatus::Pending)
			&& !self.is_expired()
			&& self.retry_count < self.priority.retry_count()
	}

	/// Check if the order has expired
	pub fn is_expired(&self) -> bool {
		Utc::now() > self.deadline
	}

	/// Check if the order is in a final state
	pub fn is_final_state(&self) -> bool {
		matches!(
			self.status,
			OrderStatus::Success
				| OrderStatus::Failed
				| OrderStatus::Cancelled
				| OrderStatus::Expired
		)
	}

	/// Check if the order can be retried
	pub fn can_retry(&self) -> bool {
		(matches!(
			self.status,
			OrderStatus::Failed | OrderStatus::Queued | OrderStatus::Submitted
		)) && self.retry_count < self.priority.retry_count()
			&& !self.is_expired()
	}

	/// Get time until deadline
	pub fn time_until_deadline(&self) -> Duration {
		self.deadline - Utc::now()
	}

	/// Get seconds until deadline
	pub fn seconds_until_deadline(&self) -> i64 {
		self.time_until_deadline().num_seconds()
	}

	/// Update order status
	pub fn update_status(&mut self, status: OrderStatus) {
		self.status = status;
		self.updated_at = Utc::now();

		// Auto-expire if deadline passed
		if self.is_expired() && !self.is_final_state() {
			self.status = OrderStatus::Expired;
		}
	}

	/// Mark order as processed
	pub fn mark_processed(&mut self) {
		self.last_processed_at = Some(Utc::now());
		self.updated_at = Utc::now();
	}

	/// Add execution result
	pub fn add_execution_result(&mut self, result: OrderExecutionResult) {
		self.execution_history.push(result.clone());

		if result.success {
			self.status = OrderStatus::Success;
		} else {
			self.retry_count += 1;
			// Check if we can retry based on retry count and expiration
			if self.retry_count < self.priority.retry_count() && !self.is_expired() {
				self.status = OrderStatus::Queued;
			} else {
				self.status = OrderStatus::Failed;
			}
		}

		self.updated_at = Utc::now();
		self.mark_processed();
	}

	/// Cancel the order
	pub fn cancel(&mut self, reason: String) {
		if !self.is_final_state() {
			self.status = OrderStatus::Cancelled;
			self.updated_at = Utc::now();

			// Add cancellation to execution history
			let cancel_result = OrderExecutionResult::failure(
				format!("Order cancelled: {}", reason),
				self.retry_count,
			);
			self.execution_history.push(cancel_result);
		}
	}

	/// Assign a solver for execution
	pub fn assign_solver(&mut self, solver_id: String) {
		self.assigned_solver = Some(solver_id);
		self.updated_at = Utc::now();
	}

	/// Set priority based on amount or other factors
	pub fn update_priority(&mut self, priority: OrderPriority) {
		self.priority = priority;
		self.updated_at = Utc::now();
	}

	/// Calculate priority from quote data
	pub fn calculate_priority_from_amount(&mut self, amount_usd: f64) {
		self.priority = OrderPriority::from_amount(amount_usd);
		self.updated_at = Utc::now();
	}

	/// Get the most recent execution result
	pub fn latest_execution_result(&self) -> Option<&OrderExecutionResult> {
		self.execution_history.last()
	}

	/// Get successful execution result
	pub fn successful_execution(&self) -> Option<&OrderExecutionResult> {
		self.execution_history.iter().find(|result| result.success)
	}

	/// Calculate execution score (for analytics)
	pub fn execution_score(&self) -> f64 {
		match self.status {
			OrderStatus::Success => {
				let base_score = 100.0;
				let retry_penalty = self.retry_count as f64 * 10.0;
				let time_bonus = if let Some(exec_time) = self.estimated_execution_time_ms {
					if exec_time < 5000 {
						10.0
					} else {
						0.0
					}
				} else {
					0.0
				};

				(base_score - retry_penalty + time_bonus).max(0.0)
			},
			OrderStatus::Failed | OrderStatus::Cancelled => 0.0,
			OrderStatus::Expired => 10.0,
			_ => 50.0, // Pending/executing states
		}
	}

	/// Estimate gas cost based on quote data
	pub fn estimate_gas_cost(&self) -> Option<String> {
		if let Some(ref quote_data) = self.quote_data {
			if let Some(estimated_gas) = quote_data.estimated_gas {
				// Rough estimation: gas * 20 gwei
				let gas_cost_wei = estimated_gas * 20_000_000_000;
				Some(gas_cost_wei.to_string())
			} else {
				None
			}
		} else {
			None
		}
	}

	/// Check if order is high value (for special handling)
	pub fn is_high_value(&self) -> bool {
		matches!(self.priority, OrderPriority::High | OrderPriority::Critical)
	}

	/// Get execution timeout based on priority
	pub fn execution_timeout_ms(&self) -> u64 {
		self.priority.execution_timeout_ms()
	}

	/// Builder methods for easy configuration
	pub fn with_quote_id(mut self, quote_id: String) -> Self {
		self.quote_id = Some(quote_id);
		self
	}

	pub fn with_quote_data(mut self, quote_data: OrderQuoteData) -> Self {
		self.quote_data = Some(quote_data);
		self
	}

	pub fn with_signature(mut self, signature: String) -> Self {
		self.signature = Some(signature);
		self
	}

	pub fn with_metadata(mut self, metadata: OrderMetadata) -> Self {
		self.metadata = metadata;
		self
	}

	pub fn with_priority(mut self, priority: OrderPriority) -> Self {
		self.priority = priority;
		self
	}

	pub fn with_fees(mut self, fees: OrderFees) -> Self {
		self.fees = fees;
		self
	}
}

impl OrderMetadata {
	pub fn new(source: String) -> Self {
		Self {
			source,
			user_agent: None,
			referrer: None,
			session_id: None,
			ip_address: None,
			custom_data: HashMap::new(),
			location: None,
			client_version: None,
		}
	}

	pub fn with_user_agent(mut self, user_agent: String) -> Self {
		self.user_agent = Some(user_agent);
		self
	}

	pub fn with_session_id(mut self, session_id: String) -> Self {
		self.session_id = Some(session_id);
		self
	}

	pub fn with_ip_address(mut self, ip_address: String) -> Self {
		self.ip_address = Some(ip_address);
		self
	}

	pub fn with_custom_data(mut self, key: String, value: serde_json::Value) -> Self {
		self.custom_data.insert(key, value);
		self
	}
}

impl Default for OrderMetadata {
	fn default() -> Self {
		Self::new("aggregator".to_string())
	}
}

impl OrderQuoteData {
	pub fn new(
		token_in: String,
		token_out: String,
		amount_in: String,
		amount_out: String,
		chain_id: u64,
		expires_at: DateTime<Utc>,
	) -> Self {
		Self {
			token_in,
			token_out,
			amount_in,
			amount_out,
			chain_id,
			price_impact: None,
			estimated_gas: None,
			route_info: None,
			expires_at,
		}
	}

	pub fn is_expired(&self) -> bool {
		Utc::now() > self.expires_at
	}

	pub fn with_price_impact(mut self, price_impact: f64) -> Self {
		self.price_impact = Some(price_impact);
		self
	}

	pub fn with_gas_estimate(mut self, estimated_gas: u64) -> Self {
		self.estimated_gas = Some(estimated_gas);
		self
	}

	pub fn with_route_info(mut self, route_info: serde_json::Value) -> Self {
		self.route_info = Some(route_info);
		self
	}
}

impl OrderFees {
	pub fn new(platform_fee_rate: f64, fee_currency: String) -> Self {
		Self {
			platform_fee_rate,
			platform_fee_amount: None,
			gas_fee: None,
			solver_fee: None,
			total_estimated_fee: None,
			fee_currency,
		}
	}

	pub fn calculate_platform_fee(&mut self, amount: &str) -> Result<(), OrderError> {
		let amount_val: f64 = amount
			.parse()
			.map_err(|_| OrderError::Internal("Invalid amount for fee calculation".to_string()))?;

		let fee_amount = amount_val * self.platform_fee_rate;
		self.platform_fee_amount = Some(fee_amount.to_string());

		Ok(())
	}

	pub fn estimate_total(&mut self) -> Option<String> {
		let mut total = 0.0;

		if let Some(ref platform_fee) = self.platform_fee_amount {
			if let Ok(fee) = platform_fee.parse::<f64>() {
				total += fee;
			}
		}

		if let Some(ref gas_fee) = self.gas_fee {
			if let Ok(fee) = gas_fee.parse::<f64>() {
				total += fee;
			}
		}

		if let Some(ref solver_fee) = self.solver_fee {
			if let Ok(fee) = solver_fee.parse::<f64>() {
				total += fee;
			}
		}

		self.total_estimated_fee = Some(total.to_string());
		self.total_estimated_fee.clone()
	}
}

impl Default for OrderFees {
	fn default() -> Self {
		Self::new(0.003, "USDC".to_string()) // 0.3% default fee
	}
}

/// Order execution response from a solver
///
/// This represents the response when submitting an order to a solver adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
	/// The order ID
	pub order_id: String,

	/// Current status of the order
	pub status: OrderStatus,

	/// Transaction hash (if executed)
	pub transaction_hash: Option<String>,

	/// Block number where transaction was included
	pub block_number: Option<u64>,

	/// Gas used for the transaction
	pub gas_used: Option<u64>,

	/// Effective gas price
	pub effective_gas_price: Option<String>,

	/// Execution result
	pub result: OrderExecutionDetail,

	/// When the response was created
	pub created_at: DateTime<Utc>,

	/// When the response was last updated
	pub updated_at: DateTime<Utc>,
}

/// Order execution result details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderExecutionDetail {
	/// Actual input amount
	pub amount_in: Option<String>,

	/// Actual output amount received
	pub amount_out: Option<String>,

	/// Actual price impact
	pub actual_price_impact: Option<f64>,

	/// Gas cost in native currency
	pub gas_cost: Option<String>,

	/// Execution time in milliseconds
	pub execution_time_ms: Option<u64>,

	/// Error message (if failed)
	pub error_message: Option<String>,

	/// Solver used for execution
	pub solver_used: Option<String>,
}

#[cfg(test)]
mod tests {
	use super::*;

	fn create_test_order() -> Order {
		Order::new(
			"0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e".to_string(),
			0.005, // 0.5% slippage
			Utc::now() + Duration::hours(1),
		)
	}

	#[test]
	fn test_order_creation() {
		let order = create_test_order();

		assert!(!order.order_id.is_empty());
		assert_eq!(order.slippage_tolerance, 0.005);
		assert_eq!(order.status, OrderStatus::Pending);
		assert_eq!(order.priority, OrderPriority::Normal);
		assert!(!order.is_expired());
		assert!(order.is_executable());
	}

	#[test]
	fn test_order_expiration() {
		let mut order = create_test_order();

		// Set deadline in the past
		order.deadline = Utc::now() - Duration::hours(1);

		assert!(order.is_expired());
		assert!(!order.is_executable());

		// Update status should auto-expire
		order.update_status(OrderStatus::Queued);
		assert_eq!(order.status, OrderStatus::Expired);
	}

	#[test]
	fn test_order_execution_results() {
		let mut order = create_test_order();

		// Add a failed execution
		let failed_result = OrderExecutionResult::failure("Test error".to_string(), 0);
		order.add_execution_result(failed_result);

		assert_eq!(order.retry_count, 1);
		assert_eq!(order.status, OrderStatus::Queued); // Should retry
		assert!(order.can_retry());

		// Add a successful execution
		let success_result = OrderExecutionResult::success(
			"0x123".to_string(),
			12345,
			21000,
			"1000000".to_string(),
			"solver-1".to_string(),
			5000,
		);
		order.add_execution_result(success_result);

		assert_eq!(order.status, OrderStatus::Success);
		assert!(!order.can_retry());
		assert!(order.successful_execution().is_some());
	}

	#[test]
	fn test_order_priority() {
		let mut order = create_test_order();

		// Test priority calculation from amount
		order.calculate_priority_from_amount(50000.0); // $50k
		assert_eq!(order.priority, OrderPriority::High);

		order.calculate_priority_from_amount(500.0); // $500
		assert_eq!(order.priority, OrderPriority::Low);

		order.calculate_priority_from_amount(200000.0); // $200k
		assert_eq!(order.priority, OrderPriority::Critical);
	}

	#[test]
	fn test_order_cancellation() {
		let mut order = create_test_order();

		order.cancel("User requested cancellation".to_string());

		assert_eq!(order.status, OrderStatus::Cancelled);
		assert!(order.is_final_state());
		assert!(!order.can_retry());
		assert!(!order.execution_history.is_empty());
	}

	#[test]
	fn test_order_timeouts() {
		let _order = create_test_order();

		// Test different priority timeouts
		assert_eq!(OrderPriority::Critical.execution_timeout_ms(), 30000);
		assert_eq!(OrderPriority::High.execution_timeout_ms(), 60000);
		assert_eq!(OrderPriority::Normal.execution_timeout_ms(), 120000);
		assert_eq!(OrderPriority::Low.execution_timeout_ms(), 300000);
	}

	#[test]
	fn test_order_fees() {
		let mut fees = OrderFees::new(0.005, "USDC".to_string());

		fees.calculate_platform_fee("1000").unwrap();
		assert_eq!(fees.platform_fee_amount, Some("5".to_string()));

		fees.gas_fee = Some("2".to_string());
		fees.solver_fee = Some("1".to_string());

		let total = fees.estimate_total();
		assert_eq!(total, Some("8".to_string()));
	}

	#[test]
	fn test_quote_data() {
		let quote_data = OrderQuoteData::new(
			"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
			"1000000".to_string(),
			"2000000000000000000".to_string(),
			1,
			Utc::now() + Duration::minutes(10),
		);

		assert!(!quote_data.is_expired());
	}

	#[test]
	fn test_order_builder_pattern() {
		let order = create_test_order()
			.with_quote_id("quote-123".to_string())
			.with_signature("0xsignature".to_string())
			.with_priority(OrderPriority::High);

		assert_eq!(order.quote_id, Some("quote-123".to_string()));
		assert_eq!(order.signature, Some("0xsignature".to_string()));
		assert_eq!(order.priority, OrderPriority::High);
	}

	#[test]
	fn test_execution_score() {
		let mut order = create_test_order();

		// Successful order should have high score
		order.status = OrderStatus::Success;
		assert!(order.execution_score() > 90.0);

		// Failed order should have low score
		order.status = OrderStatus::Failed;
		assert_eq!(order.execution_score(), 0.0);

		// Order with retries should have lower score
		order.status = OrderStatus::Success;
		order.retry_count = 2;
		assert!(order.execution_score() < 90.0);
	}

	#[test]
	fn test_order_metadata() {
		let metadata = OrderMetadata::new("web".to_string())
			.with_user_agent("Mozilla/5.0".to_string())
			.with_session_id("session-123".to_string())
			.with_custom_data("test_key".to_string(), serde_json::json!("test_value"));

		assert_eq!(metadata.source, "web");
		assert_eq!(metadata.user_agent, Some("Mozilla/5.0".to_string()));
		assert_eq!(metadata.session_id, Some("session-123".to_string()));
		assert!(metadata.custom_data.contains_key("test_key"));
	}
}
