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
		// Validate and convert to domain order
		let order = request
			.to_domain(ip_address)
			.map_err(|e| OrderServiceError::Validation(e.to_string()))?;

		// If quote_id is provided, verify existence and expiry
		if let Some(ref quote_id) = order.quote_id {
			match self
				.storage
				.get_quote(quote_id)
				.await
				.map_err(|e| OrderServiceError::Storage(e.to_string()))?
			{
				Some(quote) => {
					if quote.is_expired() {
						return Err(OrderServiceError::QuoteExpired(quote_id.clone()));
					}
				},
				None => {
					return Err(OrderServiceError::QuoteNotFound(quote_id.clone()));
				},
			}
		}

		// Persist order
		self.storage
			.add_order(order.clone())
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))?;

		Ok(order)
	}

	/// Retrieve an existing order by id
	pub async fn get_order(&self, order_id: &str) -> Result<Option<Order>, OrderServiceError> {
		self.storage
			.get_order(order_id)
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))
	}
}
