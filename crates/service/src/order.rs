use std::sync::Arc;

use oif_storage::Storage;
use oif_types::{Order, OrdersRequest};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrderServiceError {
	#[error("validation error: {0}")]
	Validation(String),
	#[error("quote not found: {0}")]
	QuoteNotFound(String),
	#[error("quote expired: {0}")]
	QuoteExpired(String),
	#[error("storage error: {0}")]
	Storage(String),
}

#[derive(Clone)]
pub struct OrderService {
	storage: Arc<dyn Storage>,
}

impl OrderService {
	pub fn new(storage: Arc<dyn Storage>) -> Self {
		Self { storage }
	}

	/// Validate, persist and return the created order
	pub async fn submit_order(
		&self,
		request: &OrdersRequest,
		ip_address: Option<String>,
	) -> Result<Order, OrderServiceError> {
		unimplemented!()
	}

	/// Retrieve an existing order by id
	pub async fn get_order(&self, order_id: &str) -> Result<Option<Order>, OrderServiceError> {
		self.storage
			.get_order(order_id)
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))
	}
}
