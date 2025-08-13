//! Core adapter traits for user implementations

use super::{AdapterResult, SolverRuntimeConfig};
use crate::{
	adapters::{models::SubmitOrderRequest, GetOrderResponse},
	models::{Asset, Network},
};
use crate::{Adapter, GetQuoteRequest, GetQuoteResponse, Order};
use async_trait::async_trait;
use std::fmt::Debug;

/// Core trait for solver adapter implementations
///
/// This trait defines the interface that all solver adapters must implement.
/// Users can create custom adapters by implementing this trait.
#[async_trait]
pub trait SolverAdapter: Send + Sync + Debug {
	/// Get adapter ID (for registration and solver matching)
	fn adapter_id(&self) -> &str;

	/// Get adapter name (human-readable)
	fn adapter_name(&self) -> &str;

	/// Get adapter configuration information
	fn adapter_info(&self) -> &Adapter;

	/// Get quotes from the solver using runtime configuration
	/// Adapters can choose to handle multi-input/output or process simple swaps
	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetQuoteResponse>;

	/// Submit an order to the solver using runtime configuration
	async fn submit_order(
		&self,
		order: &SubmitOrderRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse>;

	/// Health check for the solver using runtime configuration
	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool>;

	/// Get detailed order information from the solver using runtime configuration
	///
	/// This method retrieves comprehensive information about an order including
	/// transaction status, gas usage, fees, and any additional metadata from the solver.
	async fn get_order_details(
		&self,
		order_id: &str,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse>;

	/// Get the list of blockchain networks supported by this adapter
	///
	/// Returns the networks (chains) that this adapter can process transactions on.
	/// Each network includes chain ID, name, and testnet status.
	async fn get_supported_networks(&self) -> AdapterResult<Vec<Network>>;

	/// Get the list of assets supported on a specific network
	///
	/// Returns the tokens/assets that this adapter can handle on the given network.
	/// Each asset includes contract address, symbol, name, decimals, and chain ID.
	async fn get_supported_assets(&self, network: &Network) -> AdapterResult<Vec<Asset>>;

	/// Get human-readable name for this adapter
	fn name(&self) -> &str {
		&self.adapter_info().name
	}

	/// Get adapter version
	fn version(&self) -> &str {
		&self.adapter_info().version
	}
}
