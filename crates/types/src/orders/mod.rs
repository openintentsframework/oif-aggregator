//! Core Order domain model and business logic

use chrono::{DateTime, Utc};
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

use crate::oif::common::{AssetAmount, Settlement};
use crate::oif::{v0::GetOrderResponse, OifGetOrderResponse};
use crate::quotes::Quote;

/// Result types for order operations
pub type OrderResult<T> = Result<T, OrderError>;
pub type OrderValidationResult<T> = Result<T, OrderValidationError>;

/// Core Order domain model using composition with OIF standard
///
/// This represents an order in the domain layer with business logic.
/// Uses composition to embed OIF GetOrderResponse while adding domain-specific fields.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Order {
	// Domain-specific identifier (may differ from OIF id for aggregator use)
	pub order_id: String,
	// Which solver is handling this order
	pub solver_id: String,
	// Embedded OIF standard order (version-agnostic, version is encoded in the wrapper)
	pub order: OifGetOrderResponse,
	// Associated quote details (domain concept)
	pub quote_details: Option<Quote>,
}

impl Order {
	/// Create a new Order using composition with OIF GetOrderResponse
	pub fn new(
		solver_id: String,
		order_response: OifGetOrderResponse,
		quote_details: Option<Quote>,
	) -> Self {
		// Use OIF id as domain order_id (can be different if needed)
		let order_id = order_response.id().to_string();

		Self {
			order_id,
			solver_id,
			order: order_response, // Use the version-agnostic wrapper directly
			quote_details,
		}
	}

	// Accessor methods for embedded OIF fields

	/// Get the OIF version from embedded wrapper
	pub fn version(&self) -> &'static str {
		self.order.version()
	}

	/// Get the order status from embedded OIF order
	pub fn status(&self) -> &crate::oif::common::OrderStatus {
		self.order.status()
	}

	/// Get the OIF quote ID (different from domain quote_details)
	pub fn oif_quote_id(&self) -> Option<&String> {
		self.order.quote_id()
	}

	/// Get input amount from embedded OIF order
	pub fn input_amount(&self) -> &AssetAmount {
		self.order
			.input_amounts()
			.first()
			.expect("Order should have at least one input amount")
	}

	/// Get output amount from embedded OIF order
	pub fn output_amount(&self) -> &AssetAmount {
		self.order
			.output_amounts()
			.first()
			.expect("Order should have at least one output amount")
	}

	/// Get settlement information from embedded OIF order
	pub fn settlement(&self) -> &Settlement {
		self.order.settlement()
	}

	/// Get fill transaction from embedded OIF order
	pub fn fill_transaction(&self) -> Option<&serde_json::Value> {
		self.order.fill_transaction()
	}

	/// Get the OIF internal ID (may differ from domain order_id)
	pub fn oif_id(&self) -> &str {
		self.order.id()
	}

	/// Get OIF created timestamp as u64
	pub fn oif_created_at(&self) -> u64 {
		self.order.created_at_timestamp()
	}

	/// Get OIF updated timestamp as u64
	pub fn oif_updated_at(&self) -> u64 {
		self.order.updated_at_timestamp()
	}

	/// Get created timestamp as DateTime<Utc> (convenience accessor for solver timestamp)
	pub fn created_at(&self) -> DateTime<Utc> {
		self.order.created_at()
	}

	/// Get updated timestamp as DateTime<Utc> (convenience accessor for solver timestamp)  
	pub fn updated_at(&self) -> DateTime<Utc> {
		self.order.updated_at()
	}
}

impl TryFrom<(GetOrderResponse, Quote)> for Order {
	type Error = OrderError;

	fn try_from((src, quote_details): (GetOrderResponse, Quote)) -> Result<Self, Self::Error> {
		Ok(Order::new(
			quote_details.solver_id.clone(),
			OifGetOrderResponse::new(src), // Wrap version-specific type in version-agnostic wrapper
			Some(quote_details),
		))
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
	/// Order is currently executing - prepare confirmed, fill transaction in progress.
	Executing,
	/// Order has been executed.
	Executed,
	/// Order has been settled and is ready to be claimed.
	Settled,
	/// Order is settling and is ready to be claimed.
	Settling,
	/// Order is finalized and complete (after claim confirmation).
	Finalized,
	/// Order execution failed with specific transaction type.
	Failed,
	/// Order has been refunded.
	Refunded,
}

impl From<crate::oif::common::OrderStatus> for OrderStatus {
	fn from(s: crate::oif::common::OrderStatus) -> Self {
		match s {
			crate::oif::common::OrderStatus::Created => Self::Created,
			crate::oif::common::OrderStatus::Pending => Self::Pending,
			crate::oif::common::OrderStatus::Executing => Self::Executing,
			crate::oif::common::OrderStatus::Executed => Self::Executed,
			crate::oif::common::OrderStatus::Settled => Self::Settled,
			crate::oif::common::OrderStatus::Settling => Self::Settling,
			crate::oif::common::OrderStatus::Finalized => Self::Finalized,
			crate::oif::common::OrderStatus::Failed => Self::Failed,
			crate::oif::common::OrderStatus::Refunded => Self::Refunded,
		}
	}
}

impl From<OrderStatus> for crate::oif::common::OrderStatus {
	fn from(s: OrderStatus) -> Self {
		match s {
			OrderStatus::Created => Self::Created,
			OrderStatus::Pending => Self::Pending,
			OrderStatus::Executing => Self::Executing,
			OrderStatus::Executed => Self::Executed,
			OrderStatus::Settled => Self::Settled,
			OrderStatus::Settling => Self::Settling,
			OrderStatus::Finalized => Self::Finalized,
			OrderStatus::Failed => Self::Failed,
			OrderStatus::Refunded => Self::Refunded,
		}
	}
}
