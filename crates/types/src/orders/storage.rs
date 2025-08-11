//! Order storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::errors::OrderResult;
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
	/// Create storage order from domain intent
	pub fn from_domain(order: Order) -> Self {
		Self {
			order_id: order.order_id,
			quote_id: order.quote_id,
			user_address: order.user_address,
			signature: order.signature,
			status: OrderStatusStorage::from_domain(order.status),
			created_at: order.created_at,
			updated_at: order.updated_at,
		}
	}

	/// Convert storage order to domain intent
	pub fn to_domain(self) -> OrderResult<Order> {
		Ok(Order {
			order_id: self.order_id,
			user_address: self.user_address,
			quote_id: self.quote_id,
			signature: self.signature,
			status: self.status.to_domain(),
			created_at: self.created_at,
			updated_at: self.updated_at,
		})
	}

	/// Update status and timestamp
	pub fn update_status(&mut self, status: OrderStatusStorage) {
		self.status = status;
		self.updated_at = Utc::now();
	}
}

impl OrderStatusStorage {
	fn from_domain(status: OrderStatus) -> Self {
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

	fn to_domain(self) -> OrderStatus {
		match self {
			Self::Pending => OrderStatus::Pending,
			Self::Submitted => OrderStatus::Submitted,
			Self::Queued => OrderStatus::Queued,
			Self::Executing => OrderStatus::Executing,
			Self::Success => OrderStatus::Success,
			Self::Failed => OrderStatus::Failed,
			Self::Cancelled => OrderStatus::Cancelled,
			Self::Expired => OrderStatus::Expired,
		}
	}
}

/// Conversion traits
impl From<Order> for OrderStorage {
	fn from(order: Order) -> Self {
		Self::from_domain(order)
	}
}

impl TryFrom<OrderStorage> for Order {
	type Error = OrderError;

	fn try_from(storage: OrderStorage) -> Result<Self, Self::Error> {
		storage.to_domain()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
}
