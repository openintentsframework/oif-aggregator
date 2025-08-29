//! Order response models for API layer

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use serde_json::json;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::{
	adapters::{AssetAmount, Settlement},
	orders::OrderResult,
};

use super::{Order, OrderStatus};

/// Response body for /v1/orders endpoint (order submission)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrderResponse {
	/// The order ID
	pub order_id: String,

	/// Current status of the order
	pub status: OrderStatus,

	/// Quote ID
	pub quote_id: Option<String>,

	/// When the response was created
	pub created_at: DateTime<Utc>,

	/// When the response was last updated
	pub updated_at: DateTime<Utc>,

	/// Input amount
	pub input_amount: AssetAmount,

	/// Output amount
	pub output_amount: AssetAmount,

	/// Settlement information
	pub settlement: Settlement,

	/// Fill transaction information
	pub fill_transaction: Option<serde_json::Value>,
}

/// Response body for /v1/orders endpoint (order submission)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "orderId": "ord_6a22e92f-3e5d-4f05-ab5f-007b01e58b21",
    "status": "pending",
    "message": "Intent received and is being validated",
    "timestamp": 1756457492,
    "quoteId": "6a22e92f-3e5d-4f05-ab5f-007b01e58b21"
})))]
#[serde(rename_all = "camelCase")]
pub struct OrdersResponse {
	/// Unique order identifier
	pub order_id: String,

	/// Current status
	pub status: String,

	/// Human-readable message
	pub message: String,

	/// Response timestamp
	pub timestamp: i64,

	/// Associated quote ID
	pub quote_id: Option<String>,
}

impl OrdersResponse {
	/// Create response from domain order
	pub fn from_domain(order: &Order) -> OrderResult<Self> {
		Ok(Self {
			order_id: order.order_id.clone(),
			status: order.status.to_string(),
			message: Self::generate_status_message(&order.status),
			timestamp: Utc::now().timestamp(),
			quote_id: order.quote_id.clone(),
		})
	}

	fn generate_status_message(status: &OrderStatus) -> String {
		match status {
			OrderStatus::Created => "Order created".to_string(),
			OrderStatus::Pending => "Intent received and is being validated".to_string(),
			OrderStatus::Executed => "Order executed".to_string(),
			OrderStatus::Settled => "Order settled".to_string(),
			OrderStatus::Finalized => "Order finalized".to_string(),
			OrderStatus::Failed => "Order execution failed".to_string(),
		}
	}
}

impl std::fmt::Display for OrderStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			OrderStatus::Created => write!(f, "created"),
			OrderStatus::Pending => write!(f, "pending"),
			OrderStatus::Executed => write!(f, "executed"),
			OrderStatus::Settled => write!(f, "settled"),
			OrderStatus::Finalized => write!(f, "finalized"),
			OrderStatus::Failed => write!(f, "failed"),
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

/// Convert domain Order to detailed API OrderResponse
impl TryFrom<&Order> for OrderResponse {
	type Error = super::OrderError;

	fn try_from(order: &Order) -> Result<Self, Self::Error> {
		Ok(OrderResponse {
			order_id: order.order_id.clone(),
			status: order.status.clone(),
			quote_id: order.quote_id.clone(),
			created_at: order.created_at,
			updated_at: order.updated_at,
			input_amount: order.input_amount.clone(),
			output_amount: order.output_amount.clone(),
			settlement: order.settlement.clone(),
			fill_transaction: order.fill_transaction.clone(),
		})
	}
}
