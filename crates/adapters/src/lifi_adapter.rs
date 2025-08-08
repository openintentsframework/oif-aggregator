//! LiFi adapter implementation

use crate::{AdapterError, AdapterResult, SolverAdapter};
use async_trait::async_trait;
use oif_types::{AdapterConfig, Order, Quote, QuoteRequest};
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
	async fn get_quote(&self, request: &QuoteRequest) -> AdapterResult<Quote> {
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

	fn supported_chains(&self) -> &[u64] {
		// Return a static reference based on adapter type
		match self.config.adapter_type {
			oif_types::AdapterType::OifV1 => &[1], // Ethereum mainnet
			oif_types::AdapterType::LifiV1 => &[1, 137, 56], // Ethereum, Polygon, BSC
		}
	}
}
