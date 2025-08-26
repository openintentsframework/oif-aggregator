//! Order service
//!
//! Service for submitting and retrieving orders.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::integrity::IntegrityTrait;
use crate::solver_adapter::{SolverAdapterError, SolverAdapterService, SolverAdapterTrait};
use async_trait::async_trait;
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::adapters::models::{GetOrderResponse, SubmitOrderRequest};
use oif_types::{IntegrityPayload, Order, OrderRequest, Quote};
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, warn};

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

	/// Clean up old orders in final status (Finalized or Failed) older than the specified retention period
	async fn cleanup_old_orders(&self, retention_days: u32) -> Result<usize, OrderServiceError>;
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

	/// Retry getting order details with exponential backoff
	/// Orders might not be immediately available after submission due to eventual consistency
	async fn get_order_details_with_retry(
		&self,
		order_id: &str,
		solver_adapter: &SolverAdapterService,
	) -> Result<GetOrderResponse, SolverAdapterError> {
		const MAX_RETRIES: u32 = 5;
		const INITIAL_DELAY_MS: u64 = 100; // Start with 100ms
		const MAX_DELAY_MS: u64 = 6000; // Cap at 6 seconds

		let mut delay_ms = INITIAL_DELAY_MS;

		for attempt in 1..=MAX_RETRIES {
			debug!(
				"Attempting to get order details for order {} (attempt {}/{})",
				order_id, attempt, MAX_RETRIES
			);

			match solver_adapter.get_order_details(order_id).await {
				Ok(order) => {
					debug!(
						"Successfully retrieved order {} on attempt {}",
						order_id, attempt
					);
					return Ok(order);
				},
				Err(e) => {
					if attempt == MAX_RETRIES {
						warn!(
							"Failed to get order {} after {} attempts: {}",
							order_id, MAX_RETRIES, e
						);
						return Err(e);
					}

					// Check if this is a retryable error (not found, temporary failure)
					let should_retry =
						match &e {
							SolverAdapterError::Adapter(reason) => {
								// Retry on HTTP errors that might be temporary or not found errors
								reason.contains("404")
									|| reason.contains("not found")
									|| reason.contains("Not Found")
									|| reason.contains("503") || reason.contains("502")
									|| reason.contains("500") || reason.contains("HttpError")
									|| reason.contains("Invalid response")
							},
							SolverAdapterError::SolverNotFound(_) => false, // Don't retry solver not found
							SolverAdapterError::AdapterNotFound(_) => false, // Don't retry adapter not found
							SolverAdapterError::Storage(_) => false,        // Don't retry storage errors
						};

					if should_retry {
						warn!(
							"Order {} not available yet (attempt {}), retrying in {}ms: {}",
							order_id, attempt, delay_ms, e
						);
						sleep(Duration::from_millis(delay_ms)).await;
						// Exponential backoff with jitter
						delay_ms = (delay_ms * 2).min(MAX_DELAY_MS);
					} else {
						// Non-retryable error, fail immediately
						warn!("Non-retryable error getting order {}: {}", order_id, e);
						return Err(e);
					}
				},
			}
		}

		unreachable!("Loop should have returned or errored by now")
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

		let order_id = order_response
			.order_id
			.ok_or(OrderServiceError::Validation(
				"Order ID is required".to_string(),
			))?;

		// TODO: Remove fetching order details and retry logic once solver starts to return order details immediately
		// Use retry logic since orders might not be immediately available
		debug!(
			"Retrieving order details for order {} with retry logic",
			order_id
		);
		let order = self
			.get_order_details_with_retry(&order_id, &solver_adapter)
			.await?;

		let order: Order = Order::try_from((order.order, quote_domain)).map_err(|e| {
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
			current_order.quote_details.clone().unwrap(),
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

	/// Clean up old orders in final status (Finalized or Failed) older than the specified retention period
	async fn cleanup_old_orders(&self, retention_days: u32) -> Result<usize, OrderServiceError> {
		let cutoff_date = Utc::now() - chrono::Duration::days(retention_days as i64);
		let mut deleted_count = 0usize;

		// Get all orders with final status
		let final_statuses = [
			oif_types::OrderStatus::Finalized,
			oif_types::OrderStatus::Failed,
		];

		for status in final_statuses {
			let orders = self
				.storage
				.get_orders_by_status(status)
				.await
				.map_err(|e| OrderServiceError::Storage(e.to_string()))?;

			// Filter by age and delete old orders
			for order in orders {
				if order.updated_at < cutoff_date {
					let deleted = self
						.storage
						.delete_order(&order.order_id)
						.await
						.map_err(|e| OrderServiceError::Storage(e.to_string()))?;

					if deleted {
						deleted_count += 1;
						tracing::info!(
							"Deleted old order {} with status {:?}, last updated: {}",
							order.order_id,
							order.status,
							order.updated_at
						);
					}
				}
			}
		}

		tracing::info!(
			"Order cleanup completed: deleted {} orders older than {} days",
			deleted_count,
			retention_days
		);

		Ok(deleted_count)
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

		mock.expect_cleanup_old_orders().returning(|_| Ok(0));

		// Test the mock methods work as expected
		let retrieved_order = mock.get_order("test-order").await.unwrap();
		assert!(retrieved_order.is_none());

		let refreshed_order = mock.refresh_order_status("test-order").await.unwrap();
		assert!(refreshed_order.is_none());

		let cleanup_result = mock.cleanup_old_orders(10).await.unwrap();
		assert_eq!(cleanup_result, 0);
	}
}
