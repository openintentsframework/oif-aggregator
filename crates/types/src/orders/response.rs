//! Order response models for API layer

use chrono::Utc;
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;
use std::collections::HashMap;

use super::errors::OrderResult;
use super::{
	Order, OrderExecutionResult, OrderFees, OrderPriority, OrderStatus, OrderSystemHealth,
};

/// Response body for /v1/orders endpoint (order submission)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrdersResponse {
	/// Unique order identifier
	pub order_id: String,

	/// Current status
	pub status: String,

	/// Human-readable message
	pub message: String,

	/// Response timestamp
	pub timestamp: i64,

	/// Estimated execution time in milliseconds
	pub estimated_execution_time_ms: Option<u64>,

	/// Priority level
	pub priority: String,

	/// Associated quote ID
	pub quote_id: Option<String>,

	/// Fee information
	pub fees: OrderFeesResponse,

	/// Next steps for the user
	pub next_steps: Vec<String>,
}

/// Response body for /v1/intents/{id} endpoint (order status)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderStatusResponse {
	/// Order identifier
	pub order_id: String,

	/// Current status
	pub status: String,

	/// User's wallet address
	pub user_address: String,

	/// Associated quote ID
	pub quote_id: Option<String>,

	/// Transaction hash (if executed)
	pub transaction_hash: Option<String>,

	/// Block number (if confirmed)
	pub block_number: Option<u64>,

	/// Gas used
	pub gas_used: Option<u64>,

	/// Creation timestamp
	pub created_at: i64,

	/// Last update timestamp
	pub updated_at: i64,

	/// Execution result details
	pub result: Option<OrderResultResponse>,

	/// Execution history
	pub execution_history: Vec<OrderExecutionResponse>,

	/// Progress information
	pub progress: OrderProgressResponse,

	/// Fee breakdown
	pub fees: OrderFeesResponse,

	/// Metadata (filtered for public API)
	pub metadata: OrderMetadataResponse,
}

/// Order execution result in API format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderResultResponse {
	/// Input amount
	pub amount_in: Option<String>,

	/// Output amount received
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

	/// Transaction hash
	pub transaction_hash: Option<String>,

	/// Block number where transaction was included
	pub block_number: Option<u64>,

	/// Effective gas price
	pub effective_gas_price: Option<String>,
}

/// Order result API format (alias for backward compatibility)
pub type OrderResultApi = OrderResultResponse;

/// Order execution attempt in API format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderExecutionResponse {
	/// Whether this attempt was successful
	pub success: bool,

	/// Attempt timestamp
	pub timestamp: i64,

	/// Execution time for this attempt
	pub execution_time_ms: Option<u64>,

	/// Error message (if failed)
	pub error_message: Option<String>,

	/// Solver used for this attempt
	pub solver_used: Option<String>,

	/// Transaction hash (if submitted)
	pub transaction_hash: Option<String>,

	/// Retry count for this attempt
	pub retry_count: u32,
}

/// Order progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderProgressResponse {
	/// Overall progress percentage (0-100)
	pub progress_percent: u32,

	/// Current step description
	pub current_step: String,

	/// Estimated time remaining in seconds
	pub estimated_time_remaining_secs: Option<u64>,

	/// Whether the order can be cancelled
	pub can_cancel: bool,

	/// Whether the order can be retried
	pub can_retry: bool,

	/// Time until deadline in seconds
	pub seconds_until_deadline: i64,

	/// Retry count
	pub retry_count: u32,

	/// Maximum retries allowed
	pub max_retries: u32,
}

/// Fee information in API format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderFeesResponse {
	/// Platform fee rate (percentage)
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

	/// Fee breakdown
	pub breakdown: HashMap<String, String>,
}

/// Order metadata in API format (filtered for public consumption)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderMetadataResponse {
	/// Source of the intent
	pub source: String,

	/// Client platform
	pub platform: Option<String>,

	/// Session ID
	pub session_id: Option<String>,

	/// Geographic location (if permitted)
	pub location: Option<String>,

	/// Custom metadata (filtered)
	pub custom_data: HashMap<String, serde_json::Value>,
}

/// Collection of intents for admin/monitoring endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrdersCollectionResponse {
	/// List of intents
	pub intents: Vec<OrderStatusResponse>,

	/// Total number of intents
	pub total_count: usize,

	/// Number of intents per status
	pub status_counts: HashMap<String, usize>,

	/// Pagination information
	pub pagination: PaginationResponse,

	/// Summary statistics
	pub statistics: OrderStatisticsResponse,

	/// Response timestamp
	pub timestamp: i64,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PaginationResponse {
	/// Current page number
	pub page: u32,

	/// Number of items per page
	pub page_size: u32,

	/// Total number of pages
	pub total_pages: u32,

	/// Whether there are more pages
	pub has_next_page: bool,

	/// Whether there are previous pages
	pub has_previous_page: bool,
}

/// Order statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderStatisticsResponse {
	/// Success rate (0.0 to 1.0)
	pub success_rate: f64,

	/// Average execution time in milliseconds
	pub avg_execution_time_ms: f64,

	/// Total volume processed (in USD)
	pub total_volume_usd: f64,

	/// Number of unique users
	pub unique_users: u64,

	/// Most used solver
	pub top_solver: Option<String>,

	/// Average retry count
	pub avg_retry_count: f64,

	/// System health
	pub system_health: SystemHealthResponse,
}

/// System health for order processing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SystemHealthResponse {
	/// Overall health status
	pub status: String,

	/// Health score (0.0 to 1.0)
	pub health_score: f64,

	/// Number of pending intents
	pub pending_intents: u64,

	/// Number of executing intents
	pub executing_intents: u64,

	/// Success rate in last hour
	pub success_rate_last_hour: f64,

	/// Issues (if any)
	pub issues: Vec<String>,

	/// Last health check timestamp
	pub last_check: i64,
}

/// Detailed order analysis for admin endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderAnalysisResponse {
	/// Order ID
	pub order_id: String,

	/// Performance metrics
	pub performance: OrderPerformanceResponse,

	/// Risk assessment
	pub risk_assessment: OrderRiskResponse,

	/// Profitability analysis
	pub profitability: OrderProfitabilityResponse,

	/// Recommendations
	pub recommendations: Vec<String>,

	/// Analysis timestamp
	pub analyzed_at: i64,
}

/// Performance metrics for an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderPerformanceResponse {
	/// Execution score (0-100)
	pub execution_score: f64,

	/// Time to completion in seconds
	pub time_to_completion_secs: Option<u64>,

	/// Number of attempts
	pub attempt_count: u32,

	/// Gas efficiency score (0-100)
	pub gas_efficiency_score: f64,

	/// Price impact score (0-100)
	pub price_impact_score: f64,
}

/// Risk assessment for an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderRiskResponse {
	/// Overall risk level (low, medium, high)
	pub risk_level: String,

	/// MEV risk score (0-100)
	pub mev_risk_score: f64,

	/// Slippage risk
	pub slippage_risk: f64,

	/// Liquidity risk
	pub liquidity_risk: f64,

	/// Risk factors
	pub risk_factors: Vec<String>,
}

/// Profitability analysis for an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderProfitabilityResponse {
	/// Revenue generated (in USD)
	pub revenue_usd: f64,

	/// Costs incurred (in USD)
	pub costs_usd: f64,

	/// Net profit (in USD)
	pub profit_usd: f64,

	/// Profit margin percentage
	pub profit_margin_percent: f64,

	/// Cost breakdown
	pub cost_breakdown: HashMap<String, f64>,
}

impl OrdersResponse {
	/// Create response from domain intent
	pub fn from_domain(order: &Order) -> OrderResult<Self> {
		Ok(Self {
			order_id: order.order_id.clone(),
			status: order.status.to_string(),
			message: Self::generate_status_message(&order.status, order.priority),
			timestamp: Utc::now().timestamp(),
			estimated_execution_time_ms: order.estimated_execution_time_ms,
			priority: order.priority.to_string(),
			quote_id: order.quote_id.clone(),
			fees: OrderFeesResponse::from_domain(&order.fees),
			next_steps: Self::generate_next_steps(&order.status),
		})
	}

	fn generate_status_message(status: &OrderStatus, priority: OrderPriority) -> String {
		match status {
			OrderStatus::Pending => "Intent received and is being validated".to_string(),
			OrderStatus::Submitted => "Order has been submitted to solver".to_string(),
			OrderStatus::Queued => {
				format!("Order queued for execution with {} priority", priority)
			},
			OrderStatus::Executing => "Order is currently being executed".to_string(),
			OrderStatus::Success => "Order executed successfully".to_string(),
			OrderStatus::Failed => "Order execution failed".to_string(),
			OrderStatus::Cancelled => "Order was cancelled".to_string(),
			OrderStatus::Expired => "Order expired before execution".to_string(),
			OrderStatus::Simulating => "Order is being simulated".to_string(),
			OrderStatus::ReviewRequired => "Order requires manual review".to_string(),
		}
	}

	fn generate_next_steps(status: &OrderStatus) -> Vec<String> {
		match status {
			OrderStatus::Pending => vec![
				"Monitor order status for updates".to_string(),
				"Ensure sufficient balance in wallet".to_string(),
			],
			OrderStatus::Submitted => vec![
				"Intent is being processed by solver".to_string(),
				"Monitor for execution confirmation".to_string(),
			],
			OrderStatus::Queued => vec![
				"Intent will be executed automatically".to_string(),
				"Monitor for transaction hash".to_string(),
			],
			OrderStatus::Executing => vec![
				"Transaction is being processed".to_string(),
				"Wait for confirmation".to_string(),
			],
			OrderStatus::Success => vec![
				"Check your wallet for received tokens".to_string(),
				"View transaction on block explorer".to_string(),
			],
			OrderStatus::Failed => vec![
				"Check error details".to_string(),
				"Consider submitting a new order".to_string(),
			],
			OrderStatus::Cancelled => vec![
				"Intent processing stopped".to_string(),
				"Submit a new order if needed".to_string(),
			],
			OrderStatus::Expired => vec!["Submit a new order with fresh quote".to_string()],
			OrderStatus::Simulating => vec![
				"Simulation in progress".to_string(),
				"Results will determine execution".to_string(),
			],
			OrderStatus::ReviewRequired => vec![
				"Intent is under review".to_string(),
				"You will be notified of the outcome".to_string(),
			],
		}
	}
}

impl OrderStatusResponse {
	/// Create response from domain intent
	pub fn from_domain(order: &Order) -> OrderResult<Self> {
		let latest_result = order.latest_execution_result();

		Ok(Self {
			order_id: order.order_id.clone(),
			status: order.status.to_string(),
			user_address: order.user_address.clone(),
			quote_id: order.quote_id.clone(),
			transaction_hash: latest_result.and_then(|r| r.transaction_hash.clone()),
			block_number: latest_result.and_then(|r| r.block_number),
			gas_used: latest_result.and_then(|r| r.gas_used),
			created_at: order.created_at.timestamp(),
			updated_at: order.updated_at.timestamp(),
			result: latest_result.map(OrderResultResponse::from_domain),
			execution_history: order
				.execution_history
				.iter()
				.map(OrderExecutionResponse::from_domain)
				.collect(),
			progress: OrderProgressResponse::from_domain(order),
			fees: OrderFeesResponse::from_domain(&order.fees),
			metadata: OrderMetadataResponse::from_domain(&order.metadata),
		})
	}

	/// Create minimal response for public API
	pub fn minimal_from_domain(order: &Order) -> OrderResult<Self> {
		let mut response = Self::from_domain(order)?;

		// Hide sensitive information in public API
		response.user_address = format!(
			"{}...{}",
			&response.user_address[..6],
			&response.user_address[response.user_address.len() - 4..]
		);
		response.execution_history.clear(); // Hide execution details
		response.metadata.custom_data.clear(); // Hide custom metadata

		Ok(response)
	}
}

impl OrderResultResponse {
	fn from_domain(result: &OrderExecutionResult) -> Self {
		Self {
			amount_in: result.amount_in.clone(),
			amount_out: result.amount_out.clone(),
			actual_price_impact: result.actual_price_impact,
			gas_cost: result.gas_cost.clone(),
			execution_time_ms: result.execution_time_ms,
			error_message: result.error_message.clone(),
			solver_used: result.solver_used.clone(),
			transaction_hash: result.transaction_hash.clone(),
			block_number: result.block_number,
			effective_gas_price: result.effective_gas_price.clone(),
		}
	}
}

impl OrderExecutionResponse {
	fn from_domain(result: &OrderExecutionResult) -> Self {
		Self {
			success: result.success,
			timestamp: result.timestamp.timestamp(),
			execution_time_ms: result.execution_time_ms,
			error_message: result.error_message.clone(),
			solver_used: result.solver_used.clone(),
			transaction_hash: result.transaction_hash.clone(),
			retry_count: result.retry_count,
		}
	}
}

impl OrderProgressResponse {
	fn from_domain(order: &Order) -> Self {
		let progress_percent = match order.status {
			OrderStatus::Pending => 10,
			OrderStatus::Submitted => 20,
			OrderStatus::Queued => 25,
			OrderStatus::Simulating => 40,
			OrderStatus::Executing => 75,
			OrderStatus::Success => 100,
			OrderStatus::Failed | OrderStatus::Cancelled | OrderStatus::Expired => 0,
			OrderStatus::ReviewRequired => 50,
		};

		let current_step = match order.status {
			OrderStatus::Pending => "Validating order",
			OrderStatus::Submitted => "Submitted to solver",
			OrderStatus::Queued => "Waiting for execution",
			OrderStatus::Simulating => "Simulating transaction",
			OrderStatus::Executing => "Executing transaction",
			OrderStatus::Success => "Completed successfully",
			OrderStatus::Failed => "Execution failed",
			OrderStatus::Cancelled => "Cancelled",
			OrderStatus::Expired => "Expired",
			OrderStatus::ReviewRequired => "Under review",
		}
		.to_string();

		Self {
			progress_percent,
			current_step,
			estimated_time_remaining_secs: order.estimated_execution_time_ms.map(|ms| ms / 1000),
			can_cancel: !order.is_final_state(),
			can_retry: order.can_retry(),
			seconds_until_deadline: order.seconds_until_deadline(),
			retry_count: order.retry_count,
			max_retries: order.priority.retry_count(),
		}
	}
}

impl OrderFeesResponse {
	fn from_domain(fees: &OrderFees) -> Self {
		let mut breakdown = HashMap::new();

		if let Some(ref platform_fee) = fees.platform_fee_amount {
			breakdown.insert("platform".to_string(), platform_fee.clone());
		}
		if let Some(ref gas_fee) = fees.gas_fee {
			breakdown.insert("gas".to_string(), gas_fee.clone());
		}
		if let Some(ref solver_fee) = fees.solver_fee {
			breakdown.insert("solver".to_string(), solver_fee.clone());
		}

		Self {
			platform_fee_rate: fees.platform_fee_rate,
			platform_fee_amount: fees.platform_fee_amount.clone(),
			gas_fee: fees.gas_fee.clone(),
			solver_fee: fees.solver_fee.clone(),
			total_estimated_fee: fees.total_estimated_fee.clone(),
			fee_currency: fees.fee_currency.clone(),
			breakdown,
		}
	}
}

impl OrderMetadataResponse {
	fn from_domain(metadata: &super::OrderMetadata) -> Self {
		// Filter custom data to only include safe values
		let filtered_custom_data: HashMap<String, serde_json::Value> = metadata
			.custom_data
			.iter()
			.filter(|(key, _)| {
				// Only include non-sensitive metadata
				!matches!(key.as_str(), "ip_address" | "user_agent" | "session_secret")
			})
			.map(|(k, v)| (k.clone(), v.clone()))
			.collect();

		Self {
			source: metadata.source.clone(),
			platform: metadata
				.custom_data
				.get("platform")
				.and_then(|v| v.as_str())
				.map(|s| s.to_string()),
			session_id: metadata.session_id.clone(),
			location: metadata.location.clone(),
			custom_data: filtered_custom_data,
		}
	}
}

impl SystemHealthResponse {
	pub fn from_domain(health: &OrderSystemHealth) -> Self {
		Self {
			status: health.status().to_string(),
			health_score: if health.is_healthy {
				health.success_rate_last_hour
			} else {
				0.0
			},
			pending_intents: health.pending_orders,
			executing_intents: health.executing_orders,
			success_rate_last_hour: health.success_rate_last_hour,
			issues: health.issues.clone(),
			last_check: health.last_check.timestamp(),
		}
	}
}

impl ToString for OrderStatus {
	fn to_string(&self) -> String {
		match self {
			OrderStatus::Pending => "pending".to_string(),
			OrderStatus::Submitted => "submitted".to_string(),
			OrderStatus::Queued => "queued".to_string(),
			OrderStatus::Executing => "executing".to_string(),
			OrderStatus::Success => "success".to_string(),
			OrderStatus::Failed => "failed".to_string(),
			OrderStatus::Cancelled => "cancelled".to_string(),
			OrderStatus::Expired => "expired".to_string(),
			OrderStatus::Simulating => "simulating".to_string(),
			OrderStatus::ReviewRequired => "review_required".to_string(),
		}
	}
}

/// Convert domain Intent to API response
impl TryFrom<Order> for OrdersResponse {
	type Error = super::OrderError;

	fn try_from(order: Order) -> Result<Self, Self::Error> {
		OrdersResponse::from_domain(&order)
	}
}

/// Convert domain Intent to status response
impl TryFrom<Order> for OrderStatusResponse {
	type Error = super::OrderError;

	fn try_from(order: Order) -> Result<Self, Self::Error> {
		OrderStatusResponse::from_domain(&order)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::orders::{Order, OrderPriority};
	use chrono::{Duration, Utc};

	fn create_test_order() -> Order {
		Order::new(
			"0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e".to_string(),
			0.005,
			Utc::now() + Duration::hours(1),
		)
		.with_priority(OrderPriority::High)
		.with_quote_id("quote-123".to_string())
	}

	#[test]
	fn test_orders_response_from_domain() {
		let order = create_test_order();
		let response = OrdersResponse::from_domain(&order).unwrap();

		assert_eq!(response.order_id, order.order_id);
		assert_eq!(response.status, "pending");
		assert_eq!(response.priority, "high");
		assert_eq!(response.quote_id, Some("quote-123".to_string()));
		assert!(!response.next_steps.is_empty());
	}

	#[test]
	fn test_order_status_response_from_domain() {
		let mut order = create_test_order();

		// Add execution result
		let execution_result = super::OrderExecutionResult::success(
			"0x123".to_string(),
			12345,
			21000,
			"1000000".to_string(),
			"solver-1".to_string(),
			5000,
		);
		order.add_execution_result(execution_result);

		let response = OrderStatusResponse::from_domain(&order).unwrap();

		assert_eq!(response.order_id, order.order_id);
		assert_eq!(response.status, "success");
		assert_eq!(response.transaction_hash, Some("0x123".to_string()));
		assert_eq!(response.block_number, Some(12345));
		assert!(response.result.is_some());
		assert_eq!(response.execution_history.len(), 1);
	}

	#[test]
	fn test_minimal_status_response() {
		let order = create_test_order();
		let response = OrderStatusResponse::minimal_from_domain(&order).unwrap();

		// User address should be truncated
		assert!(response.user_address.contains("..."));
		assert!(response.user_address.len() < order.user_address.len());

		// Execution history should be empty
		assert!(response.execution_history.is_empty());

		// Custom metadata should be cleared
		assert!(response.metadata.custom_data.is_empty());
	}

	#[test]
	fn test_progress_response() {
		let mut order = create_test_order();

		// Test different status progress
		let progress = OrderProgressResponse::from_domain(&order);
		assert_eq!(progress.progress_percent, 10); // Pending
		assert_eq!(progress.current_step, "Validating order");

		order.update_status(super::OrderStatus::Executing);
		let progress = OrderProgressResponse::from_domain(&order);
		assert_eq!(progress.progress_percent, 75); // Executing
		assert_eq!(progress.current_step, "Executing transaction");

		order.update_status(super::OrderStatus::Success);
		let progress = OrderProgressResponse::from_domain(&order);
		assert_eq!(progress.progress_percent, 100); // Success
		assert_eq!(progress.current_step, "Completed successfully");
	}

	#[test]
	fn test_fees_response() {
		let mut fees = OrderFees::new(0.005, "USDC".to_string());
		fees.platform_fee_amount = Some("5.0".to_string());
		fees.gas_fee = Some("2.0".to_string());
		fees.solver_fee = Some("1.0".to_string());

		let response = OrderFeesResponse::from_domain(&fees);

		assert_eq!(response.platform_fee_rate, 0.005);
		assert_eq!(response.fee_currency, "USDC");
		assert_eq!(response.breakdown.len(), 3);
		assert!(response.breakdown.contains_key("platform"));
		assert!(response.breakdown.contains_key("gas"));
		assert!(response.breakdown.contains_key("solver"));
	}

	#[test]
	fn test_metadata_response_filtering() {
		let mut order = create_test_order();

		// Add sensitive and non-sensitive metadata
		order.metadata = order
			.metadata
			.with_ip_address("192.168.1.1".to_string())
			.with_custom_data(
				"ip_address".to_string(),
				serde_json::Value::String("192.168.1.1".to_string()),
			)
			.with_custom_data(
				"user_agent".to_string(),
				serde_json::Value::String("Chrome/90.0".to_string()),
			)
			.with_custom_data(
				"safe_data".to_string(),
				serde_json::Value::String("public_info".to_string()),
			);

		let response = OrderMetadataResponse::from_domain(&order.metadata);

		// Sensitive data should be filtered out
		assert!(!response.custom_data.contains_key("ip_address"));
		assert!(!response.custom_data.contains_key("user_agent"));

		// Safe data should be included
		assert!(response.custom_data.contains_key("safe_data"));
	}

	#[test]
	fn test_status_to_string() {
		assert_eq!(super::OrderStatus::Pending.to_string(), "pending");
		assert_eq!(super::OrderStatus::Executing.to_string(), "executing");
		assert_eq!(super::OrderStatus::Success.to_string(), "success");
		assert_eq!(super::OrderStatus::Failed.to_string(), "failed");
	}

	#[test]
	fn test_next_steps_generation() {
		let steps_pending = OrdersResponse::generate_next_steps(&super::OrderStatus::Pending);
		assert!(steps_pending.iter().any(|s| s.contains("Monitor")));

		let steps_success = OrdersResponse::generate_next_steps(&super::OrderStatus::Success);
		assert!(steps_success.iter().any(|s| s.contains("wallet")));

		let steps_failed = OrdersResponse::generate_next_steps(&super::OrderStatus::Failed);
		assert!(steps_failed.iter().any(|s| s.contains("error")));
	}

	#[test]
	fn test_execution_response() {
		let execution_result = super::OrderExecutionResult::success(
			"0x123".to_string(),
			12345,
			21000,
			"1000000".to_string(),
			"solver-1".to_string(),
			5000,
		);

		let response = OrderExecutionResponse::from_domain(&execution_result);

		assert!(response.success);
		assert_eq!(response.transaction_hash, Some("0x123".to_string()));
		assert_eq!(response.execution_time_ms, Some(5000));
		assert_eq!(response.solver_used, Some("solver-1".to_string()));
	}

	#[test]
	fn test_result_response() {
		let execution_result = super::OrderExecutionResult::success(
			"0x123".to_string(),
			12345,
			21000,
			"1000000".to_string(),
			"solver-1".to_string(),
			5000,
		)
		.with_amounts("500000".to_string(), "1000000".to_string())
		.with_price_impact(0.01);

		let response = OrderResultResponse::from_domain(&execution_result);

		assert_eq!(response.amount_in, Some("500000".to_string()));
		assert_eq!(response.amount_out, Some("1000000".to_string()));
		assert_eq!(response.actual_price_impact, Some(0.01));
		assert_eq!(response.transaction_hash, Some("0x123".to_string()));
	}
}
