//! LiFi adapter implementation
//! TODO: Implement LiFi adapter

use async_trait::async_trait;
use oif_types::adapters::GetOrderResponse;
use oif_types::{
	Adapter, Asset, GetQuoteRequest, GetQuoteResponse, Network, Order, SolverRuntimeConfig,
};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use reqwest::Client;
use tracing::debug;

/// LiFi adapter for cross-chain bridge quotes
/// This adapter is stateless and receives solver configuration at runtime
#[derive(Debug)]
pub struct LifiAdapter {
	config: Adapter,
	#[allow(dead_code)]
	client: Client,
}

impl LifiAdapter {
	pub fn new(config: Adapter) -> AdapterResult<Self> {
		let client = Client::builder().build().map_err(AdapterError::HttpError)?;

		Ok(Self { config, client })
	}

	/// Create default LiFi adapter instance
	pub fn default() -> AdapterResult<Self> {
		let config = Adapter::new(
			"lifi-v1".to_string(),
			"LiFi v1 Protocol".to_string(),
			"LiFi v1 Adapter".to_string(),
			"1.0.0".to_string(),
		);

		Self::new(config)
	}
}

#[async_trait]
impl SolverAdapter for LifiAdapter {
	fn adapter_id(&self) -> &str {
		&self.config.adapter_id
	}

	fn adapter_name(&self) -> &str {
		&self.config.name
	}

	fn adapter_info(&self) -> &Adapter {
		&self.config
	}

	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetQuoteResponse> {
		debug!(
			"LiFi adapter getting quotes for {} inputs and {} outputs via solver: {}",
			request.available_inputs.len(),
			request.requested_outputs.len(),
			config.solver_id
		);

		unimplemented!()
	}

	async fn submit_order(
		&self,
		order: &Order,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		debug!(
			"LiFi adapter submitting order: {:?} via solver: {}",
			order, config.solver_id
		);
		unimplemented!()
	}

	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		debug!("LiFi adapter health check for solver: {}", config.solver_id);

		unimplemented!()
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		debug!(
			"LiFi adapter getting order details for: {} via solver: {}",
			order_id, config.solver_id
		);

		unimplemented!()
	}

	async fn get_supported_networks(&self) -> AdapterResult<Vec<Network>> {
		debug!("LiFi adapter getting supported networks");

		unimplemented!()
	}

	async fn get_supported_assets(&self, network: &Network) -> AdapterResult<Vec<Asset>> {
		debug!(
			"LiFi adapter getting supported assets for network: {}",
			network.name
		);

		unimplemented!()
	}
}
