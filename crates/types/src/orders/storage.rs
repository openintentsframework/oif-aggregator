//! Order storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Order, OrderError, OrderStatus};

/// Storage representation of an intent
///
/// This model is optimized for storage and persistence.
/// It can be converted to/from the domain Intent model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStorage {
	pub order_id: String,
	pub quote_id: Option<String>,
	pub user_address: String,
	pub signature: Option<String>,
	pub status: OrderStatusStorage,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

/// Storage-compatible order status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatusStorage {
	Pending,
	Submitted,
	Queued,
	Executing,
	Success,
	Failed,
	Cancelled,
	Expired,
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
			OrderStatus::Pending => Self::Pending,
			OrderStatus::Submitted => Self::Submitted,
			OrderStatus::Queued => Self::Queued,
			OrderStatus::Executing => Self::Executing,
			OrderStatus::Success => Self::Success,
			OrderStatus::Failed => Self::Failed,
			OrderStatus::Cancelled => Self::Cancelled,
			OrderStatus::Expired => Self::Expired,
		}
	}
}

impl From<OrderStatusStorage> for OrderStatus {
	fn from(storage: OrderStatusStorage) -> Self {
		match storage {
			OrderStatusStorage::Pending => Self::Pending,
			OrderStatusStorage::Submitted => Self::Submitted,
			OrderStatusStorage::Queued => Self::Queued,
			OrderStatusStorage::Executing => Self::Executing,
			OrderStatusStorage::Success => Self::Success,
			OrderStatusStorage::Failed => Self::Failed,
			OrderStatusStorage::Cancelled => Self::Cancelled,
			OrderStatusStorage::Expired => Self::Expired,
		}
	}
}

/// Conversion traits
impl From<Order> for OrderStorage {
	fn from(order: Order) -> Self {
		Self {
			order_id: order.order_id,
			quote_id: order.quote_id,
			user_address: order.user_address,
			signature: order.signature,
			status: OrderStatusStorage::from(order.status),
			created_at: order.created_at,
			updated_at: order.updated_at,
		}
	}
}

impl TryFrom<OrderStorage> for Order {
	type Error = OrderError;

	fn try_from(storage: OrderStorage) -> Result<Self, Self::Error> {
		Ok(Order {
			order_id: storage.order_id,
			user_address: storage.user_address,
			quote_id: storage.quote_id,
			signature: storage.signature,
			status: OrderStatus::from(storage.status),
			created_at: storage.created_at,
			updated_at: storage.updated_at,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
}
