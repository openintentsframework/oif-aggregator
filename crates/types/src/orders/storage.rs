//! Order storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Order, OrderError, OrderStatus};
use crate::{
	oif::common::{AssetAmount, Settlement},
	oif::OifGetOrderResponse,
	quotes::Quote,
};

/// Storage representation of an order using composition with OIF standard
///
/// This model mirrors the domain Order structure for consistency.
/// Uses composition to embed OIF GetOrderResponse while adding storage-specific fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStorage {
	// Domain-specific identifier (matches domain Order)
	pub order_id: String,
	// Which solver is handling this order
	pub solver_id: String,
	// Embedded OIF standard order (version-agnostic, version is encoded in wrapper)
	pub order: OifGetOrderResponse,
	// Associated quote details
	pub quote_details: Option<Quote>,
}

/// Storage-compatible order status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatusStorage {
	Created,
	Pending,
	Executed,
	Settled,
	Finalized,
	Failed,
	Executing,
	Settling,
	Refunded,
}

impl OrderStorage {
	/// Get the OIF version from embedded wrapper
	pub fn version(&self) -> &'static str {
		self.order.version()
	}

	/// Get the status from embedded OIF order
	pub fn status(&self) -> &crate::oif::common::OrderStatus {
		self.order.status()
	}

	/// Get other OIF fields via accessor methods (similar to domain Order)
	pub fn input_amount(&self) -> &AssetAmount {
		self.order
			.input_amounts()
			.first()
			.expect("Order should have at least one input amount")
	}

	pub fn output_amount(&self) -> &AssetAmount {
		self.order
			.output_amounts()
			.first()
			.expect("Order should have at least one output amount")
	}

	pub fn settlement(&self) -> &Settlement {
		self.order.settlement()
	}

	pub fn fill_transaction(&self) -> Option<&serde_json::Value> {
		self.order.fill_transaction()
	}

	pub fn oif_quote_id(&self) -> Option<&String> {
		self.order.quote_id()
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

impl From<OrderStatus> for OrderStatusStorage {
	fn from(status: OrderStatus) -> Self {
		match status {
			OrderStatus::Created => Self::Created,
			OrderStatus::Pending => Self::Pending,
			OrderStatus::Executed => Self::Executed,
			OrderStatus::Settled => Self::Settled,
			OrderStatus::Finalized => Self::Finalized,
			OrderStatus::Failed => Self::Failed,
			OrderStatus::Executing => Self::Executing,
			OrderStatus::Settling => Self::Settling,
			OrderStatus::Refunded => Self::Refunded,
		}
	}
}

impl From<OrderStatusStorage> for OrderStatus {
	fn from(storage: OrderStatusStorage) -> Self {
		match storage {
			OrderStatusStorage::Created => Self::Created,
			OrderStatusStorage::Pending => Self::Pending,
			OrderStatusStorage::Executed => Self::Executed,
			OrderStatusStorage::Settled => Self::Settled,
			OrderStatusStorage::Finalized => Self::Finalized,
			OrderStatusStorage::Failed => Self::Failed,
			OrderStatusStorage::Executing => Self::Executing,
			OrderStatusStorage::Settling => Self::Settling,
			OrderStatusStorage::Refunded => Self::Refunded,
		}
	}
}

/// Conversion traits
impl From<Order> for OrderStorage {
	fn from(order: Order) -> Self {
		Self {
			order_id: order.order_id,
			solver_id: order.solver_id,
			order: order.order,
			quote_details: order.quote_details,
		}
	}
}

impl TryFrom<OrderStorage> for Order {
	type Error = OrderError;

	fn try_from(storage: OrderStorage) -> Result<Self, Self::Error> {
		// Simple conversion since both structures use composition now
		Ok(Order {
			order_id: storage.order_id,
			solver_id: storage.solver_id,
			order: storage.order,
			quote_details: storage.quote_details,
		})
	}
}
