//! Order service
//!
//! Service for submitting and retrieving orders.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use oif_types::adapters::AssetAmount;

/// Tracing target for structured logging
const TRACING_TARGET: &str = "oif_service::order";

use crate::circuit_breaker::CircuitBreakerTrait;
use crate::integrity::IntegrityTrait;
use crate::jobs::scheduler::JobScheduler;
use crate::jobs::types::BackgroundJob;
use crate::solver_adapter::{SolverAdapterError, SolverAdapterService, SolverAdapterTrait};
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
	async fn refresh_order(&self, order_id: &str) -> Result<Option<Order>, OrderServiceError>;

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
	#[error("circuit breaker blocked solver '{0}' - high failure rate detected")]
	CircuitBreakerBlocked(String),
}

#[derive(Clone)]
pub struct OrderService {
	storage: Arc<dyn Storage>,
	adapter_registry: Arc<AdapterRegistry>,
	integrity_service: Arc<dyn IntegrityTrait>,
	job_scheduler: Arc<dyn JobScheduler>,
	circuit_breaker: Arc<dyn CircuitBreakerTrait>,
}

impl OrderService {
	pub fn new(
		storage: Arc<dyn Storage>,
		adapter_registry: Arc<AdapterRegistry>,
		integrity_service: Arc<dyn IntegrityTrait>,
		job_scheduler: Arc<dyn JobScheduler>,
		circuit_breaker: Arc<dyn CircuitBreakerTrait>,
	) -> Self {
		Self {
			storage,
			adapter_registry,
			integrity_service,
			job_scheduler,
			circuit_breaker,
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

		// 4. Check circuit breaker before submitting order
		// Get solver info for circuit breaker check
		let solver = self
			.storage
			.get_solver(&request.quote_response.solver_id)
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))?
			.ok_or_else(|| {
				OrderServiceError::Validation(format!(
					"Solver '{}' not found",
					request.quote_response.solver_id
				))
			})?;

		if !self.circuit_breaker.should_allow_request(&solver).await {
			tracing::warn!(
				target: TRACING_TARGET,
				solver_id = %request.quote_response.solver_id,
				"Circuit breaker blocked order submission - solver has high failure rate"
			);
			return Err(OrderServiceError::CircuitBreakerBlocked(
				request.quote_response.solver_id.clone(),
			));
		}

		// 5. Create solver adapter service and submit the order
		let solver_adapter = SolverAdapterService::new(
			&request.quote_response.solver_id,
			self.adapter_registry.clone(),
			self.storage.clone(),
			Some(self.job_scheduler.clone()),
		)
		.await?;

		let order_response = solver_adapter.submit_order(&submit_order_request).await?;

		let order_id = order_response
			.order_id
			.ok_or(OrderServiceError::Validation(
				"Order ID is required".to_string(),
			))?;

		// Create initial order object from submission response and quote details
		// Per-order monitoring jobs will fetch and update the full order details
		let now = chrono::Utc::now();

		// Use empty amounts for initial order creation - real amounts will be populated
		// when the order monitoring fetches the complete order details from the solver
		let input_amount = AssetAmount::default();
		let output_amount = AssetAmount::default();

		let order = Order {
			order_id: order_id.clone(),
			quote_id: Some(quote_domain.quote_id.clone()),
			solver_id: quote_domain.solver_id.clone(),
			status: oif_types::OrderStatus::Created, // Initial status
			created_at: now,
			updated_at: now,
			input_amount,
			output_amount,
			settlement: oif_types::adapters::Settlement {
				settlement_type: oif_types::adapters::SettlementType::Escrow,
				data: serde_json::json!({}),
			}, // Default settlement, will be updated by order monitoring
			fill_transaction: None, // Will be populated by order monitoring
			quote_details: Some(quote_domain),
		};

		// 6. Save the order to storage
		self.storage
			.create_order(order.clone())
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))?;

		// 7. Start order status monitoring
		let monitoring_delay = Duration::from_secs(5); // First check in 5 seconds
		let job_id = format!("order-monitor-{}", order_id);

		match self
			.job_scheduler
			.schedule_with_delay(
				BackgroundJob::OrderStatusMonitor {
					order_id: order_id.clone(),
					attempt: 0,
				},
				monitoring_delay,
				Some(job_id),
			)
			.await
		{
			Ok(scheduled_id) => {
				tracing::info!(
					"Started monitoring for order '{}' (job ID: {})",
					order_id,
					scheduled_id
				);
			},
			Err(e) => {
				tracing::warn!("Failed to start monitoring for order '{}': {}", order_id, e);
				// Don't fail the order submission if monitoring fails to start
			},
		}

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
	async fn refresh_order(&self, order_id: &str) -> Result<Option<Order>, OrderServiceError> {
		tracing::debug!(
			target: TRACING_TARGET,
			order_id = %order_id,
			"Starting order refresh from solver"
		);

		// 1. Get current order from storage
		let mut current_order = match self
			.storage
			.get_order(order_id)
			.await
			.map_err(|e| OrderServiceError::Storage(e.to_string()))?
		{
			Some(order) => order,
			None => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					"Order not found in storage"
				);
				return Ok(None);
			},
		};

		// 2. If order is already in final state, return as-is
		if Self::is_final_status(&current_order.status) {
			tracing::debug!(
				target: TRACING_TARGET,
				order_id = %order_id,
				status = ?current_order.status,
				"Order already in final status, skipping refresh"
			);
			return Ok(Some(current_order));
		}

		tracing::debug!(
			target: TRACING_TARGET,
			order_id = %order_id,
			current_status = ?current_order.status,
			solver_id = %current_order.solver_id,
			has_empty_amounts = %(current_order.input_amount.asset.is_empty() || current_order.output_amount.asset.is_empty()),
			"Order needs refresh, fetching details from solver"
		);

		// 3. Create solver adapter service for this order's solver
		let solver_adapter = match SolverAdapterService::new(
			&current_order.solver_id,
			self.adapter_registry.clone(),
			self.storage.clone(),
			Some(self.job_scheduler.clone()),
		)
		.await
		{
			Ok(adapter) => adapter,
			Err(e) => {
				tracing::warn!(
					target: TRACING_TARGET,
					order_id = %order_id,
					solver_id = %current_order.solver_id,
					error = %e,
					"Failed to create solver adapter, returning current order"
				);
				return Ok(Some(current_order));
			},
		};

		// 4. Get updated order details from the solver with circuit breaker protection
		let updated_order_response = match self
			.get_order_details_with_protection(&solver_adapter, order_id, &current_order)
			.await
		{
			Ok(response) => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					solver_id = %current_order.solver_id,
					"Successfully fetched order details from solver"
				);
				response
			},
			Err(e) => {
				// Log the error but don't fail - return current order from storage
				tracing::warn!(
					target: TRACING_TARGET,
					order_id = %order_id,
					solver_id = %current_order.solver_id,
					error = %e,
					"Failed to get order details from solver, returning current order"
				);
				return Ok(Some(current_order));
			},
		};

		// 5. Convert adapter response to domain order
		let quote_details = match current_order.quote_details.clone() {
			Some(quote) => quote,
			None => {
				tracing::warn!(
					target: TRACING_TARGET,
					order_id = %order_id,
					"Order has no quote details, cannot convert updated response, returning current order"
				);
				return Ok(Some(current_order));
			},
		};

		let updated_order: Order = Order::try_from((updated_order_response.order, quote_details))
			.map_err(|e| {
			tracing::error!(
				target: TRACING_TARGET,
				order_id = %order_id,
				error = %e,
				"Failed to convert GetOrderResponse to Order"
			);
			OrderServiceError::Validation(format!(
				"Failed to convert GetOrderResponse to Order: {}",
				e
			))
		})?;

		// 6. Check if anything changed and update storage with complete order details
		let status_changed = updated_order.status != current_order.status;

		// Check if fill transaction was added
		let fill_transaction_added =
			updated_order.fill_transaction.is_some() && current_order.fill_transaction.is_none();

		// Check various change conditions
		let is_first_fetch = current_order.input_amount.asset.is_empty()
			|| current_order.output_amount.asset.is_empty();
		let timestamp_changed = updated_order.updated_at != current_order.updated_at;

		// Always update if this is the first fetch (empty asset names) or if anything changed
		let should_update =
			is_first_fetch || status_changed || fill_transaction_added || timestamp_changed;

		tracing::debug!(
			target: TRACING_TARGET,
			order_id = %order_id,
			is_first_fetch = %is_first_fetch,
			status_changed = %status_changed,
			fill_transaction_added = %fill_transaction_added,
			timestamp_changed = %timestamp_changed,
			should_update = %should_update,
			current_status = ?current_order.status,
			updated_status = ?updated_order.status,
			"Evaluated update conditions"
		);

		if should_update {
			// Update the order in storage with complete details from solver
			current_order.status = updated_order.status.clone();
			current_order.updated_at = updated_order.updated_at;
			current_order.input_amount = updated_order.input_amount.clone();
			current_order.output_amount = updated_order.output_amount.clone();
			current_order.settlement = updated_order.settlement.clone();
			current_order.fill_transaction = updated_order.fill_transaction.clone();

			self.storage
				.update_order(current_order.clone())
				.await
				.map_err(|e| OrderServiceError::Storage(e.to_string()))?;

			if status_changed {
				tracing::info!(
					target: TRACING_TARGET,
					order_id = %order_id,
					old_status = ?current_order.status,
					new_status = ?updated_order.status,
					is_first_fetch = %is_first_fetch,
					fill_transaction_added = %fill_transaction_added,
					"Order status updated"
				);
			} else {
				tracing::info!(
					target: TRACING_TARGET,
					order_id = %order_id,
					status = ?current_order.status,
					is_first_fetch = %is_first_fetch,
					fill_transaction_added = %fill_transaction_added,
					"Order updated with complete details from solver"
				);
			}
		} else {
			tracing::debug!(
				target: TRACING_TARGET,
				order_id = %order_id,
				current_status = ?current_order.status,
				"No changes detected, skipping storage update"
			);
		}

		tracing::debug!(
			target: TRACING_TARGET,
			order_id = %order_id,
			final_status = ?current_order.status,
			"Order refresh completed successfully"
		);

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

	/// Get order details with circuit breaker protection and emergency override
	async fn get_order_details_with_protection(
		&self,
		solver_adapter: &SolverAdapterService,
		order_id: &str,
		current_order: &Order,
	) -> Result<oif_types::adapters::GetOrderResponse, SolverAdapterError> {
		// Get solver info for circuit breaker check
		let solver_result = self
			.storage
			.get_solver(&current_order.solver_id)
			.await
			.map_err(|e| SolverAdapterError::Storage(e.to_string()));

		if let Ok(Some(solver)) = solver_result {
			// Check if circuit breaker allows this request
			if !self.circuit_breaker.should_allow_request(&solver).await {
				tracing::warn!(
					target: TRACING_TARGET,
					order_id = %order_id,
					solver_id = %current_order.solver_id,
					"Circuit breaker would block order status check - attempting emergency override for user visibility"
				);

				// Emergency override: Allow status check with warning
				// Users need to see their order status even if solver is having issues
				tracing::info!(
					target: TRACING_TARGET,
					order_id = %order_id,
					solver_id = %current_order.solver_id,
					"Proceeding with order status check despite circuit breaker (emergency override)"
				);
			}
		}

		// Proceed with the order details call (either allowed by circuit breaker or emergency override)
		solver_adapter.get_order_details(order_id).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;

	#[tokio::test]
	async fn test_mock_order_service_trait() {
		let mut mock = MockOrderServiceTrait::new();

		// Setup simple expectations to verify the mock trait works
		mock.expect_get_order().returning(|_| Ok(None));

		mock.expect_refresh_order().returning(|_| Ok(None));

		mock.expect_cleanup_old_orders().returning(|_| Ok(0));

		// Test the mock methods work as expected
		let retrieved_order = mock.get_order("test-order").await.unwrap();
		assert!(retrieved_order.is_none());

		let refreshed_order = mock.refresh_order("test-order").await.unwrap();
		assert!(refreshed_order.is_none());

		let cleanup_result = mock.cleanup_old_orders(10).await.unwrap();
		assert_eq!(cleanup_result, 0);
	}

	#[tokio::test]
	async fn test_order_service_with_circuit_breaker() {
		// Test that OrderService can be configured with circuit breaker
		use crate::integrity::MockIntegrityTrait;
		use crate::jobs::scheduler::{JobScheduler, MockJobScheduler};
		use oif_adapters::AdapterRegistry;
		use oif_storage::MemoryStore;

		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::new());
		let integrity_service: Arc<dyn IntegrityTrait> = Arc::new(MockIntegrityTrait::new());
		let job_scheduler: Arc<dyn JobScheduler> = Arc::new(MockJobScheduler::new());
		let circuit_breaker: Arc<dyn CircuitBreakerTrait> =
			Arc::new(crate::MockCircuitBreakerTrait::new());

		// Test that we can create OrderService with circuit breaker (now required)
		let _order_service = OrderService::new(
			storage,
			adapter_registry,
			integrity_service,
			job_scheduler,
			circuit_breaker,
		);

		// If we get here without compiler errors, the integration works
		assert!(
			true,
			"OrderService with circuit breaker created successfully"
		);
	}

	#[tokio::test]
	async fn test_circuit_breaker_error_type() {
		// Test that our new error type works correctly
		let error = OrderServiceError::CircuitBreakerBlocked("test-solver".to_string());

		assert!(error
			.to_string()
			.contains("circuit breaker blocked solver 'test-solver'"));
		assert!(error.to_string().contains("high failure rate detected"));
	}
}
