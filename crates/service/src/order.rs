//! Order service
//!
//! Service for submitting and retrieving orders.

use std::collections::HashMap;
use std::sync::Arc;

use oif_adapters::AdapterFactory;
use oif_storage::Storage;
use oif_types::chrono::Utc;
use oif_types::{Order, OrdersRequest, Solver, SolverRuntimeConfig};
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
}

#[derive(Clone)]
pub struct OrderService {
	storage: Arc<dyn Storage>,
	adapter_factory: Arc<AdapterFactory>,
	solvers: HashMap<String, Solver>,
}

impl OrderService {
	pub fn new(
		storage: Arc<dyn Storage>,
		adapter_factory: Arc<AdapterFactory>,
		solvers: HashMap<String, Solver>,
	) -> Self {
		Self {
			storage,
			adapter_factory,
			solvers,
		}
	}

	/// Validate, persist and return the created order
	pub async fn submit_order(&self, request: &OrdersRequest) -> Result<Order, OrderServiceError> {
		// 1. Get the quote to determine which solver to use
		let quote = if let Some(quote_id) = &request.quote_id {
			self.storage
				.get_quote(quote_id)
				.await
				.map_err(|e| OrderServiceError::Storage(e.to_string()))?
				.ok_or_else(|| OrderServiceError::QuoteNotFound(quote_id.clone()))?
		} else {
			return Err(OrderServiceError::Validation(
				"quote_id is required for order submission".to_string(),
			));
		};

		// 2. Find the solver that generated this quote
		let solver = self
			.solvers
			.get(&quote.solver_id)
			.ok_or_else(|| OrderServiceError::SolverNotFound(quote.solver_id.clone()))?;

		// 3. Get the adapter for this solver
		let adapter = self
			.adapter_factory
			.get(&solver.adapter_id)
			.ok_or_else(|| {
				OrderServiceError::AdapterNotFound(format!(
					"No adapter found for solver {} (adapter_id: {})",
					solver.solver_id, solver.adapter_id
				))
			})?;

		// 4. Create the order object
		let mut order = Order::new(request.user_address.clone());
		order.quote_id = Some(quote.quote_id.clone());
		order.signature = request.signature.clone();

		// 5. Submit the order to the solver via its adapter
		let config = SolverRuntimeConfig::from(solver);
		let _transaction_hash = adapter
			.submit_order(&order, &config)
			.await
			.map_err(|e| OrderServiceError::Adapter(e.to_string()))?;

		// 6. Update order status and save to storage
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

	/// Get detailed order information from the solver (includes transaction status, gas, etc.)
	pub async fn get_order_details(
		&self,
		order_id: &str,
	) -> Result<Option<oif_types::OrderDetails>, OrderServiceError> {
		// 1. Get the order from storage to determine which solver was used
		let order = match self.get_order(order_id).await? {
			Some(order) => order,
			None => return Ok(None),
		};

		// 2. Get the quote to find the solver
		let quote = if let Some(quote_id) = &order.quote_id {
			self.storage
				.get_quote(quote_id)
				.await
				.map_err(|e| OrderServiceError::Storage(e.to_string()))?
				.ok_or_else(|| OrderServiceError::QuoteNotFound(quote_id.clone()))?
		} else {
			return Err(OrderServiceError::Validation(
				"Order has no associated quote_id".to_string(),
			));
		};

		// 3. Find the solver that processed this order
		let solver = self
			.solvers
			.get(&quote.solver_id)
			.ok_or_else(|| OrderServiceError::SolverNotFound(quote.solver_id.clone()))?;

		// 4. Get the adapter for this solver
		let adapter = self
			.adapter_factory
			.get(&solver.adapter_id)
			.ok_or_else(|| {
				OrderServiceError::AdapterNotFound(format!(
					"No adapter found for solver {} (adapter_id: {})",
					solver.solver_id, solver.adapter_id
				))
			})?;

		// 5. Query the adapter for detailed order information
		let config = SolverRuntimeConfig::from(solver);
		let order_details = adapter
			.get_order_details(order_id, &config)
			.await
			.map_err(|e| OrderServiceError::Adapter(e.to_string()))?;

		Ok(Some(order_details))
	}
}
