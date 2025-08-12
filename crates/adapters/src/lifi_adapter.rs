//! LiFi adapter implementation

use async_trait::async_trait;
use oif_types::{
	AdapterConfig, Asset, Network, Order, OrderDetails, Quote, QuoteRequest, SolverRuntimeConfig,
};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use reqwest::Client;
use std::time::Duration;
use tracing::debug;

/// LiFi adapter for cross-chain bridge quotes
/// This adapter is stateless and receives solver configuration at runtime
#[derive(Debug)]
pub struct LifiAdapter {
	config: AdapterConfig,
	client: Client,
}

impl LifiAdapter {
	pub fn new(config: AdapterConfig) -> AdapterResult<Self> {
		let client = Client::builder().build().map_err(AdapterError::HttpError)?;

		Ok(Self { config, client })
	}

	/// Create default LiFi adapter instance
	pub fn default() -> AdapterResult<Self> {
		let config = AdapterConfig::new(
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
		"lifi-v1"
	}

	fn adapter_name(&self) -> &str {
		"LiFi v1 Protocol"
	}

	fn adapter_info(&self) -> &AdapterConfig {
		&self.config
	}

	async fn get_quote(
		&self,
		request: QuoteRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Quote> {
		debug!(
			"LiFi adapter getting quote for request: {:?} via solver: {}",
			request, config.solver_id
		);

		// TODO: Implement LiFi-specific quote logic using config.endpoint and config.timeout_ms
		// This is a placeholder implementation
		Err(AdapterError::NotImplemented(
			"LiFi adapter not yet implemented".to_string(),
		))
	}

	async fn submit_order(
		&self,
		order: &Order,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<String> {
		debug!(
			"LiFi adapter submitting order: {:?} via solver: {}",
			order, config.solver_id
		);

		// TODO: Implement LiFi-specific order submission logic using config.endpoint and config.timeout_ms
		Err(AdapterError::NotImplemented(
			"LiFi order submission not yet implemented".to_string(),
		))
	}

	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		debug!("LiFi adapter health check for solver: {}", config.solver_id);

		// TODO: Implement actual health check using config.endpoint and config.timeout_ms
		Ok(true)
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<OrderDetails> {
		debug!(
			"LiFi adapter getting order details for: {} via solver: {}",
			order_id, config.solver_id
		);

		// TODO: Implement LiFi-specific order details retrieval
		// For now, return a placeholder implementation
		Err(AdapterError::NotImplemented(
			"LiFi order details retrieval not yet implemented".to_string(),
		))
	}

	async fn get_supported_networks(&self) -> AdapterResult<Vec<Network>> {
		debug!("LiFi adapter getting supported networks");

		Err(AdapterError::NotImplemented(
			"LiFi order details retrieval not yet implemented".to_string(),
		))
	}

	async fn get_supported_assets(&self, network: &Network) -> AdapterResult<Vec<Asset>> {
		debug!(
			"LiFi adapter getting supported assets for network: {}",
			network.name
		);

		Err(AdapterError::NotImplemented(
			"LiFi adapter getting supported assets not yet implemented".to_string(),
		))
	}
}
