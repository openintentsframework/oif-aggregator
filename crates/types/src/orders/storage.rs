//! Order storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Order, OrderError, OrderStatus};
use crate::adapters::{AssetAmount, Settlement};

/// Storage representation of an intent
///
/// This model is optimized for storage and persistence.
/// It can be converted to/from the domain Intent model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStorage {
	pub order_id: String,
	pub quote_id: Option<String>,
	pub solver_id: String,
	pub status: OrderStatusStorage,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub input_amount: AssetAmount,
	pub output_amount: AssetAmount,
	pub settlement: Settlement,
	pub fill_transaction: Option<serde_json::Value>,
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
}

impl OrderStorage {
	/// Update status and timestamp
	pub fn update_status(&mut self, status: OrderStatusStorage) {
		self.status = status;
		self.updated_at = Utc::now();
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
		}
	}
}

/// Conversion traits
impl From<Order> for OrderStorage {
	fn from(order: Order) -> Self {
		Self {
			order_id: order.order_id,
			quote_id: order.quote_id,
			solver_id: order.solver_id,
			status: OrderStatusStorage::from(order.status),
			created_at: order.created_at,
			updated_at: order.updated_at,
			input_amount: order.input_amount,
			output_amount: order.output_amount,
			settlement: order.settlement,
			fill_transaction: order.fill_transaction,
		}
	}
}

impl TryFrom<OrderStorage> for Order {
	type Error = OrderError;

	fn try_from(storage: OrderStorage) -> Result<Self, Self::Error> {
		Ok(Order {
			order_id: storage.order_id,
			quote_id: storage.quote_id,
			solver_id: storage.solver_id,
			status: OrderStatus::from(storage.status),
			created_at: storage.created_at,
			updated_at: storage.updated_at,
			input_amount: storage.input_amount,
			output_amount: storage.output_amount,
			settlement: storage.settlement,
			fill_transaction: storage.fill_transaction,
		})
	}
}
