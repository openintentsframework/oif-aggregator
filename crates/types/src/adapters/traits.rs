//! Core adapter traits for user implementations

use super::{AdapterResult, SolverRuntimeConfig};
use crate::{
	adapters::{
		models::{SubmitOrderRequest, SubmitOrderResponse},
		AdapterError, GetOrderResponse,
	},
	models::AssetRoute,
};
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

	/// Get the list of supported asset routes (origin -> destination pairs)
	///
	/// Returns the specific asset conversion routes that this adapter can handle.
	/// Routes provide precise compatibility checking for cross-chain operations.
	/// For example, an adapter might support Base USDC → Ethereum USDC but not the reverse.
	///
	/// All adapters must implement this method to provide route information.
	/// For adapters that only expose asset lists (like some legacy APIs),
	/// generate cross-chain routes from the available assets.
	async fn get_supported_routes(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Vec<AssetRoute>>;

	/// Get human-readable name for this adapter
	fn name(&self) -> &str {
		&self.adapter_info().name
	}

	/// Get adapter version
	fn version(&self) -> &str {
		&self.adapter_info().version
	}
}
