//! Core adapter traits for user implementations

use super::{AdapterResult, SolverRuntimeConfig};
use crate::adapters::{
	models::{SubmitOrderRequest, SubmitOrderResponse},
	AdapterError, GetOrderResponse,
};
use crate::models::SupportedAssetsData;
use crate::{Adapter, GetQuoteRequest, GetQuoteResponse};
use async_trait::async_trait;
use std::fmt::Debug;

/// Core trait for solver adapter implementations
///
/// This trait defines the interface that all solver adapters must implement.
/// Users can create custom adapters by implementing this trait.
#[async_trait]
pub trait SolverAdapter: Send + Sync + Debug {
	/// Get adapter configuration information
	/// This is the only required method - others have default implementations
	fn adapter_info(&self) -> &Adapter;

	/// Get adapter ID (for registration and solver matching)
	fn id(&self) -> &str {
		&self.adapter_info().adapter_id
	}

	/// Get quotes from the solver using runtime configuration
	/// Adapters can choose to handle multi-input/output or process simple swaps
	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetQuoteResponse>;

	/// Submit an order to the solver using runtime configuration
	///
	/// Default implementation returns UnsupportedOperation error.
	/// Override this method if your adapter supports order submission.
	async fn submit_order(
		&self,
		_order: &SubmitOrderRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<SubmitOrderResponse> {
		Err(AdapterError::UnsupportedOperation {
			operation: "submit_order".to_string(),
			adapter_id: self.id().to_string(),
		})
	}

	/// Health check for the solver using runtime configuration
	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool>;

	/// Get detailed order information from the solver using runtime configuration
	///
	/// This method retrieves comprehensive information about an order including
	/// transaction status, gas usage, fees, and any additional metadata from the solver.
	///
	/// Default implementation returns UnsupportedOperation error.
	/// Override this method if your adapter supports order tracking.
	async fn get_order_details(
		&self,
		_order_id: &str,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		Err(AdapterError::UnsupportedOperation {
			operation: "get_order_details".to_string(),
			adapter_id: self.id().to_string(),
		})
	}

	/// Get what assets/routes this adapter supports
	///
	/// Each adapter chooses its own mode:
	/// - Assets mode: Simple any-to-any within asset list (including same-chain)
	/// - Routes mode: Precise origin->destination pairs
	///
	/// All adapters must implement this method to provide support information.
	/// The service layer will automatically set the source as AutoDiscovered.
	async fn get_supported_assets(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<SupportedAssetsData>;

	/// Get human-readable name for this adapter
	fn name(&self) -> &str {
		&self.adapter_info().name
	}

	/// Get adapter version
	fn version(&self) -> &str {
		&self.adapter_info().version
	}
}
