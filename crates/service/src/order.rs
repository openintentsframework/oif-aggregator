//! Order service
//!
//! Service for submitting and retrieving orders.

use std::collections::HashMap;
use std::sync::Arc;

use crate::integrity::IntegrityService;
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::chrono::Utc;
use oif_types::{Order, OrdersRequest, Quote, Solver, SolverRuntimeConfig};
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
	#[error("solver not found: {0}")]
	SolverNotFound(String),
	#[error("adapter not found for solver: {0}")]
	AdapterNotFound(String),
	#[error("adapter error: {0}")]
	Adapter(String),
	#[error("quote integrity verification failed")]
	IntegrityVerificationFailed,
}

#[derive(Clone)]
pub struct OrderService {
	storage: Arc<dyn Storage>,
	adapter_registry: Arc<AdapterRegistry>,
	solvers: HashMap<String, Solver>,
	integrity_service: Arc<IntegrityService>,
}

impl OrderService {
	pub fn new(
		storage: Arc<dyn Storage>,
		adapter_registry: Arc<AdapterRegistry>,
		solvers: HashMap<String, Solver>,
		integrity_service: Arc<IntegrityService>,
	) -> Self {
		Self {
			storage,
			adapter_registry,
			solvers,
			integrity_service,
		}
	}

	/// Validate, persist and return the created order
	pub async fn submit_order(&self, request: &OrdersRequest) -> Result<Order, OrderServiceError> {
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
		let is_valid = self
			.integrity_service
			.verify_checksum(&quote_domain, &request.quote_response.integrity_checksum)
			.map_err(|_| OrderServiceError::IntegrityVerificationFailed)?;

		if !is_valid {
			return Err(OrderServiceError::IntegrityVerificationFailed);
		}

		// 3. Find the solver that generated this quote
		let solver = self
			.solvers
			.get(&request.quote_response.solver_id)
			.ok_or_else(|| {
				OrderServiceError::SolverNotFound(request.quote_response.solver_id.clone())
			})?;

		// 4. Get the adapter for this solver
		let adapter = self
			.adapter_registry
			.get(&solver.adapter_id)
			.ok_or_else(|| {
				OrderServiceError::AdapterNotFound(format!(
					"No adapter found for solver {} (adapter_id: {})",
					solver.solver_id, solver.adapter_id
				))
			})?;

		// 5. Create the order object
		let mut order = Order::new(request.user_address.clone());
		order.quote_id = Some(request.quote_response.quote_id.clone());
		order.signature = request.signature.clone();

		// 6. Submit the order to the solver via its adapter
		let config = SolverRuntimeConfig::from(solver);
		let _transaction_hash = adapter
			.submit_order(&order, &config)
			.await
			.map_err(|e| OrderServiceError::Adapter(e.to_string()))?;

		// 7. Update order status and save to storage
		order.status = oif_types::OrderStatus::Submitted;
		order.updated_at = Utc::now();

		self.storage
			.create_order(order.clone())
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
