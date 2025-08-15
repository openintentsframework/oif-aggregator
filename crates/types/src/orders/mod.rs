//! Core Order domain model and business logic

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

pub mod errors;
pub mod request;
pub mod response;
pub mod storage;

pub use errors::{OrderError, OrderValidationError};
pub use request::OrderRequest;
pub use response::{OrderResponse, OrdersResponse};
pub use storage::OrderStorage;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::adapters::OrderResponse as AdapterOrderResponse;
use crate::adapters::{AssetAmount, Settlement};

/// Result types for order operations
pub type OrderResult<T> = Result<T, OrderError>;
pub type OrderValidationResult<T> = Result<T, OrderValidationError>;

/// Core Order domain model
///
/// This represents an order in the domain layer with business logic.
/// It should be converted from  and to OrderStorage/OrderResponse.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Order {
	// Order ID
	pub order_id: String,
	// Quote ID
	pub quote_id: Option<String>,
	// Solver ID
	pub solver_id: String,
	// Order status
	pub status: OrderStatus,
	// When the order was created
	pub created_at: DateTime<Utc>,
	// When the order was last updated
	pub updated_at: DateTime<Utc>,
	// Input amount
	pub input_amount: AssetAmount,
	// Output amount
	pub output_amount: AssetAmount,
	// Settlement information
	pub settlement: Settlement,
	// Fill transaction information
	pub fill_transaction: Option<serde_json::Value>,
}

impl TryFrom<(AdapterOrderResponse, String)> for Order {
	type Error = OrderError;

	fn try_from((src, solver_id): (AdapterOrderResponse, String)) -> Result<Self, Self::Error> {
		Ok(Order {
			order_id: src.id,
			quote_id: src.quote_id,
			solver_id,
			status: src.status.into(),
			created_at: chrono::Utc
				.timestamp_opt(src.created_at as i64, 0)
				.single()
				.unwrap_or_default(),
			updated_at: chrono::Utc
				.timestamp_opt(src.updated_at as i64, 0)
				.single()
				.unwrap_or_default(),
			input_amount: src.input_amount,
			output_amount: src.output_amount,
			settlement: src.settlement,
			fill_transaction: src.fill_transaction,
		})
	}
}

/// Order execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum OrderStatus {
	/// Order has been created but not yet prepared.
	Created,
	/// Order is pending execution.
	Pending,
	/// Order has been executed.
	Executed,
	/// Order has been settled and is ready to be claimed.
	Settled,
	/// Order is finalized and complete (after claim confirmation).
	Finalized,
	/// Order execution failed with specific transaction type.
	Failed,
}

impl From<crate::adapters::OrderStatus> for OrderStatus {
	fn from(s: crate::adapters::OrderStatus) -> Self {
		match s {
			crate::adapters::OrderStatus::Created => Self::Created,
			crate::adapters::OrderStatus::Pending => Self::Pending,
			crate::adapters::OrderStatus::Executed => Self::Executed,
			crate::adapters::OrderStatus::Settled => Self::Settled,
			crate::adapters::OrderStatus::Finalized => Self::Finalized,
			crate::adapters::OrderStatus::Failed(_) => Self::Failed,
		}
	}
}
#[cfg(test)]
mod tests {
	use super::*;
}
