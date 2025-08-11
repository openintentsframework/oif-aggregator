//! LiFi adapter implementation

use async_trait::async_trait;
use oif_types::{AdapterConfig, Asset, Network, Order, OrderDetails, Quote, QuoteRequest};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use reqwest::Client;
use std::time::Duration;
use tracing::debug;

/// LiFi adapter for cross-chain bridge quotes
#[derive(Debug)]
pub struct LifiAdapter {
	config: AdapterConfig,
	client: Client,
	endpoint: String,
	timeout: Duration,
}

impl LifiAdapter {
	pub fn new(config: AdapterConfig, endpoint: String, timeout_ms: u64) -> AdapterResult<Self> {
		let timeout = Duration::from_millis(timeout_ms);
		let client = Client::builder()
			.timeout(timeout)
			.build()
			.map_err(AdapterError::HttpError)?;

		Ok(Self {
			config,
			client,
			endpoint,
			timeout,
		})
	}
}

#[async_trait]
impl SolverAdapter for LifiAdapter {
	async fn get_quote(&self, request: QuoteRequest) -> AdapterResult<Quote> {
		debug!("LiFi adapter getting quote for request: {:?}", request);

		// TODO: Implement LiFi-specific quote logic
		// This is a placeholder implementation
		Err(AdapterError::NotImplemented(
			"LiFi adapter not yet implemented".to_string(),
		))
	}

	async fn submit_order(&self, order: &Order) -> AdapterResult<String> {
		debug!("LiFi adapter submitting order: {:?}", order);

		// TODO: Implement LiFi-specific order submission logic
		Err(AdapterError::NotImplemented(
			"LiFi order submission not yet implemented".to_string(),
		))
	}

	async fn health_check(&self) -> AdapterResult<bool> {
		debug!("LiFi adapter health check");

		// TODO: Implement actual health check
		Ok(true)
	}

	fn adapter_info(&self) -> &AdapterConfig {
		&self.config
	}

	async fn get_order_details(&self, order_id: &str) -> AdapterResult<OrderDetails> {
		debug!("LiFi adapter getting order details for: {}", order_id);

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
