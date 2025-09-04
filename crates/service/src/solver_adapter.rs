//! Solver adapter service for connecting to specific solvers
//!
//! This service represents a connection to a specific solver and its adapter.
//! It encapsulates the solver configuration and provides a clean interface
//! for interacting with that particular solver.

use std::sync::Arc;

use async_trait::async_trait;
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::adapters::models::{SubmitOrderRequest, SubmitOrderResponse};
use oif_types::adapters::{GetOrderResponse, GetQuoteResponse};
use oif_types::{GetQuoteRequest, Solver, SolverAdapter, SolverRuntimeConfig};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolverAdapterError {
	#[error("adapter error: {0}")]
	Adapter(String),
	#[error("solver not found: {0}")]
	SolverNotFound(String),
	#[error("adapter not found for solver: {0}")]
	AdapterNotFound(String),
	#[error("storage error: {0}")]
	Storage(String),
}

/// Trait for solver adapter operations - enables easy mocking in tests
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait SolverAdapterTrait: Send + Sync {
	/// Get quotes from this solver
	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
	) -> Result<GetQuoteResponse, SolverAdapterError>;

	/// Submit an order to this solver
	async fn submit_order(
		&self,
		request: &SubmitOrderRequest,
	) -> Result<SubmitOrderResponse, SolverAdapterError>;

	/// Get order details from this solver
	async fn get_order_details(
		&self,
		order_id: &str,
	) -> Result<GetOrderResponse, SolverAdapterError>;

	/// Perform health check on this solver
	async fn health_check(&self) -> Result<bool, SolverAdapterError>;

	/// Get the solver ID this service is connected to
	fn solver_id(&self) -> &str;

	/// Get what assets/routes this solver supports
	async fn get_supported_assets(
		&self,
	) -> Result<oif_types::SupportedAssetsData, SolverAdapterError>;
}

/// Service for interacting with a specific solver through its adapter
#[derive(Clone)]
pub struct SolverAdapterService {
	solver: Solver,
	config: SolverRuntimeConfig,
	solver_adapter: Arc<dyn SolverAdapter>,
}

impl SolverAdapterService {
	/// Create a new solver adapter service for a specific solver by ID
	pub async fn new(
		solver_id: &str,
		adapter_registry: Arc<AdapterRegistry>,
		storage: Arc<dyn Storage>,
	) -> Result<Self, SolverAdapterError> {
		// 1. Find the solver in storage
		let solver = storage
			.get_solver(solver_id)
			.await
			.map_err(|e| SolverAdapterError::Storage(e.to_string()))?
			.ok_or_else(|| SolverAdapterError::SolverNotFound(solver_id.to_string()))?;

		// 2. Verify the adapter exists
		let solver_adapter = adapter_registry.get(&solver.adapter_id).ok_or_else(|| {
			SolverAdapterError::AdapterNotFound(format!(
				"No adapter found for solver {} (adapter_id: {})",
				solver.solver_id, solver.adapter_id
			))
		})?;

		// 3. Create the service
		let config = SolverRuntimeConfig::from(&solver);
		Ok(Self {
			solver,
			config,
			solver_adapter,
		})
	}

	/// Create a solver adapter service from an existing solver
	pub fn from_solver(
		solver: Solver,
		adapter_registry: Arc<AdapterRegistry>,
	) -> Result<Self, SolverAdapterError> {
		// Verify the adapter exists
		let solver_adapter = adapter_registry.get(&solver.adapter_id).ok_or_else(|| {
			SolverAdapterError::AdapterNotFound(format!(
				"No adapter found for solver {} (adapter_id: {})",
				solver.solver_id, solver.adapter_id
			))
		})?;

		let config = SolverRuntimeConfig::from(&solver);
		Ok(Self {
			solver,
			config,
			solver_adapter,
		})
	}

	/// Get the adapter for this service's solver
	///
	/// This method is guaranteed to succeed because the adapter existence
	/// is verified during construction in both `new()` and `from_solver()`.
	/// We can safely unwrap here to avoid redundant error handling.
	fn get_adapter(&self) -> &dyn SolverAdapter {
		self.solver_adapter.as_ref()
	}
}

#[async_trait]
impl SolverAdapterTrait for SolverAdapterService {
	/// Get quotes from this solver
	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
	) -> Result<GetQuoteResponse, SolverAdapterError> {
		let adapter = self.get_adapter();
		adapter
			.get_quotes(request, &self.config)
			.await
			.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
	}

	/// Submit an order to this solver
	async fn submit_order(
		&self,
		request: &SubmitOrderRequest,
	) -> Result<SubmitOrderResponse, SolverAdapterError> {
		let adapter = self.get_adapter();
		adapter
			.submit_order(request, &self.config)
			.await
			.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
	}

	/// Get order details from this solver
	async fn get_order_details(
		&self,
		order_id: &str,
	) -> Result<GetOrderResponse, SolverAdapterError> {
		let adapter = self.get_adapter();
		adapter
			.get_order_details(order_id, &self.config)
			.await
			.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
	}

	/// Perform health check on this solver
	async fn health_check(&self) -> Result<bool, SolverAdapterError> {
		let adapter = self.get_adapter();
		adapter
			.health_check(&self.config)
			.await
			.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
	}

	/// Get what assets/routes this solver supports
	async fn get_supported_assets(
		&self,
	) -> Result<oif_types::SupportedAssetsData, SolverAdapterError> {
		let adapter = self.get_adapter();
		adapter
			.get_supported_assets(&self.config)
			.await
			.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
	}

	/// Get the solver ID this service is connected to
	fn solver_id(&self) -> &str {
		&self.solver.solver_id
	}
}
