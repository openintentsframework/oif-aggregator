//! Order service
//!
//! Service for submitting and retrieving orders.

use std::sync::Arc;

use crate::integrity::IntegrityTrait;
use crate::solver_adapter_service::{SolverAdapterError, SolverAdapterService, SolverAdapterTrait};
use async_trait::async_trait;
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::adapters::models::SubmitOrderRequest;
use oif_types::{IntegrityPayload, Order, OrderRequest, Quote};
use thiserror::Error;

/// Trait for order service operations
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait OrderServiceTrait: Send + Sync {
	/// Validate, persist and return the created order
	async fn submit_order(&self, request: &OrderRequest) -> Result<Order, OrderServiceError>;

	/// Retrieve an existing order by id
	async fn get_order(&self, order_id: &str) -> Result<Option<Order>, OrderServiceError>;

	/// Refresh order status from the solver and update storage if status changed
	async fn refresh_order_status(
		&self,
		order_id: &str,
	) -> Result<Option<Order>, OrderServiceError>;
}

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
	#[error("solver adapter error: {0}")]
	SolverAdapter(#[from] SolverAdapterError),
	#[error("quote integrity verification failed")]
	IntegrityVerificationFailed,
	#[error("order not found: {0}")]
	OrderNotFound(String),
}

#[derive(Clone)]
pub struct OrderService {
	storage: Arc<dyn Storage>,
	adapter_registry: Arc<AdapterRegistry>,
	integrity_service: Arc<dyn IntegrityTrait>,
}

impl OrderService {
	pub fn new(
		storage: Arc<dyn Storage>,
		adapter_registry: Arc<AdapterRegistry>,
		integrity_service: Arc<dyn IntegrityTrait>,
	) -> Self {
		Self {
			storage,
			adapter_registry,
			integrity_service,
		}
	}
}

#[async_trait]
impl OrderServiceTrait for OrderService {
	/// Validate, persist and return the created order
	async fn submit_order(&self, request: &OrderRequest) -> Result<Order, OrderServiceError> {
		// 1. Verify quote integrity checksum
		if request.quote_response.integrity_checksum.is_empty() {
			return Err(OrderServiceError::Validation(
				"Quote integrity checksum is required".to_string(),
			));
		}

		let quote_domain: Quote = Quote::try_from(request.quote_response.clone()).map_err(|e| {
			OrderServiceError::Validation(format!(
				"Failed to convert QuoteResponse to Quote: {}",
				e
			))
		})?;

		// 2. Verify quote integrity using QuoteResponse directly
		let payload = quote_domain.to_integrity_payload();
		let is_valid = self
			.integrity_service
			.verify_checksum_from_payload(&payload, &request.quote_response.integrity_checksum)
			.map_err(|_| OrderServiceError::IntegrityVerificationFailed)?;

		if !is_valid {
			return Err(OrderServiceError::IntegrityVerificationFailed);
		}

		// 3. Prepare the submit order request
		let submit_order_request = SubmitOrderRequest::try_from(request.clone()).map_err(|e| {
			OrderServiceError::Validation(format!(
				"Failed to convert OrderRequest to SubmitOrderRequest: {}",
				e
			))
		})?;

		// 4. Create solver adapter service and submit the order
		let solver_adapter = SolverAdapterService::new(
			&request.quote_response.solver_id,
			self.adapter_registry.clone(),
			self.storage.clone(),
		)
		.await?;

		let order_response = solver_adapter.submit_order(&submit_order_request).await?;

		let order: Order = Order::try_from((
			order_response.order,
			request.quote_response.solver_id.clone(),
		))
		.map_err(|e| {
			OrderServiceError::Validation(format!(
				"Failed to convert GetOrderResponse to Order: {}",
				e
			))
		})?;

		// 5. Save the order to storage
		self.storage
			.create_order(order.clone())
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))?;

		Ok(order)
	}

	/// Retrieve an existing order by id
	async fn get_order(&self, order_id: &str) -> Result<Option<Order>, OrderServiceError> {
		self.storage
			.get_order(order_id)
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))
	}

	/// Refresh order status from the solver and update storage if status changed
	async fn refresh_order_status(
		&self,
		order_id: &str,
	) -> Result<Option<Order>, OrderServiceError> {
		// 1. Get current order from storage
		let mut current_order = match self
			.storage
			.get_order(order_id)
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))?
		{
			Some(order) => order,
			None => return Ok(None),
		};

		// 2. If order is already in final state, return as-is
		if Self::is_final_status(&current_order.status) {
			return Ok(Some(current_order));
		}

		// 3. Create solver adapter service for this order's solver
		let solver_adapter = match SolverAdapterService::new(
			&current_order.solver_id,
			self.adapter_registry.clone(),
			self.storage.clone(),
		)
		.await
		{
			Ok(adapter) => adapter,
			Err(e) => {
				tracing::warn!(
					"Failed to create solver adapter for solver {}: {}",
					current_order.solver_id,
					e
				);
				return Ok(Some(current_order));
			},
		};

		// 4. Get updated order details from the solver
		let updated_order_response = match solver_adapter.get_order_details(order_id).await {
			Ok(response) => response,
			Err(e) => {
				// Log the error but don't fail - return current order from storage
				tracing::warn!(
					"Failed to get order details from solver {} for order {}: {}",
					current_order.solver_id,
					order_id,
					e
				);
				return Ok(Some(current_order));
			},
		};

		// 5. Convert adapter response to domain order
		let updated_order: Order = Order::try_from((
			updated_order_response.order,
			current_order.solver_id.clone(),
		))
		.map_err(|e| {
			OrderServiceError::Validation(format!(
				"Failed to convert GetOrderResponse to Order: {}",
				e
			))
		})?;

		// 6. Check if status actually changed
		if updated_order.status != current_order.status {
			// Update the order in storage with new status and timestamp
			current_order.status = updated_order.status.clone();
			current_order.updated_at = updated_order.updated_at;
			current_order.fill_transaction = updated_order.fill_transaction.clone();

			self.storage
				.update_order(current_order.clone())
				.await
				.map_err(|e| OrderServiceError::Storage(e.to_string()))?;

			tracing::info!(
				"Updated order {} status from {:?} to {:?}",
				order_id,
				current_order.status,
				updated_order.status
			);
		}

		Ok(Some(current_order))
	}
}

impl OrderService {
	/// Check if an order status is in a final state
	fn is_final_status(status: &oif_types::OrderStatus) -> bool {
		matches!(
			status,
			oif_types::OrderStatus::Finalized | oif_types::OrderStatus::Failed
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_mock_order_service_trait() {
		let mut mock = MockOrderServiceTrait::new();

		// Setup simple expectations to verify the mock trait works
		mock.expect_get_order().returning(|_| Ok(None));

		mock.expect_refresh_order_status().returning(|_| Ok(None));

		// Test the mock methods work as expected
		let retrieved_order = mock.get_order("test-order").await.unwrap();
		assert!(retrieved_order.is_none());

		let refreshed_order = mock.refresh_order_status("test-order").await.unwrap();
		assert!(refreshed_order.is_none());
	}
}
