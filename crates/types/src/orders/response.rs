//! Order response models for API layer

use chrono::Utc;
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use super::errors::OrderResult;
use super::{Order, OrderStatus};

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

	/// Associated quote ID
	pub quote_id: Option<String>,
}

impl OrdersResponse {
	/// Create response from domain intent
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
			OrderStatus::Pending => "Intent received and is being validated".to_string(),
			OrderStatus::Submitted => "Order has been submitted to solver".to_string(),
			OrderStatus::Queued => "Order queued for execution".to_string(),
			OrderStatus::Executing => "Order is currently being executed".to_string(),
			OrderStatus::Success => "Order executed successfully".to_string(),
			OrderStatus::Failed => "Order execution failed".to_string(),
			OrderStatus::Cancelled => "Order was cancelled".to_string(),
			OrderStatus::Expired => "Order expired before execution".to_string(),
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

#[cfg(test)]
mod tests {
	use super::*;
}
