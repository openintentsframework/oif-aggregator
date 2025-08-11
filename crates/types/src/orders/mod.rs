//! Core Order domain model and business logic

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod errors;
pub mod request;
pub mod response;
pub mod storage;

pub use errors::{OrderError, OrderValidationError};
pub use request::OrdersRequest;
pub use response::OrdersResponse;
pub use storage::OrderStorage;

/// Result types for order operations
pub type OrderResult<T> = Result<T, OrderError>;
pub type OrderValidationResult<T> = Result<T, OrderValidationError>;

/// Core Order domain model
///
/// This represents an order in the domain layer with business logic.
/// It should be converted from OrdersRequest and to OrderStorage/OrdersResponse.
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
	pub order_id: String,
	pub user_address: String,
	pub quote_id: Option<String>,
	pub signature: Option<String>,
	pub status: OrderStatus,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
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
}

impl Order {
	pub fn new(user_address: String) -> Self {
		let now = Utc::now();
		Self {
			order_id: uuid::Uuid::new_v4().to_string(),
			user_address,
			quote_id: None,
			signature: None,
			status: OrderStatus::Pending,
			created_at: now,
			updated_at: now,
		}
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

	/// When the response was created
	pub created_at: DateTime<Utc>,

	/// When the response was last updated
	pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
	use super::*;
}
