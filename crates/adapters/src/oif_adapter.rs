//! OIF v1 adapter implementation for HTTP-based solvers
//! TODO: Implement OIF adapter

use async_trait::async_trait;
use oif_types::adapters::models::SubmitOrderRequest;
use oif_types::adapters::GetOrderResponse;
use oif_types::{Adapter, Asset, GetQuoteRequest, GetQuoteResponse, Network, SolverRuntimeConfig};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use reqwest::{
	header::{HeaderMap, HeaderValue},
	Client,
};
use std::time::Duration;
use tracing::{debug, warn};

/// OIF v1 adapter for HTTP-based solvers
/// This adapter is stateless and receives solver configuration at runtime
#[derive(Debug)]
pub struct OifAdapter {
	config: Adapter,
	client: Client,
}

impl OifAdapter {
	/// Create a new OIF adapter (stateless, no hardcoded endpoint/timeout)
	pub fn new(config: Adapter) -> AdapterResult<Self> {
		let mut headers = HeaderMap::new();

		// Add default headers
		headers.insert("Content-Type", HeaderValue::from_static("application/json"));
		headers.insert("User-Agent", HeaderValue::from_static("OIF-Aggregator/1.0"));

		let client = Client::builder()
			.default_headers(headers)
			.build()
			.map_err(AdapterError::HttpError)?;

		Ok(Self { config, client })
	}

	/// Create default OIF adapter instance
	pub fn with_default_config() -> AdapterResult<Self> {
		let config = Adapter::new(
			"oif-v1".to_string(),
			"OIF v1 Protocol".to_string(),
			"OIF v1 Adapter".to_string(),
			"1.0.0".to_string(),
		);

		Self::new(config)
	}
}

#[async_trait]
impl SolverAdapter for OifAdapter {
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
			"Getting quotes from OIF adapter {} for solver {} with {} inputs and {} outputs",
			self.config.adapter_id,
			config.solver_id,
			request.available_inputs.len(),
			request.requested_outputs.len()
		);

		unimplemented!()
	}

	async fn submit_order(
		&self,
		order: &SubmitOrderRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		debug!(
			"Submitting order {} to OIF adapter {} via solver {}",
			order.order, self.config.adapter_id, config.solver_id
		);

		unimplemented!()
	}

	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		let health_url = format!("{}/health", config.endpoint);

		debug!(
			"Health checking OIF adapter at {} (solver: {})",
			health_url, config.solver_id
		);

		match self
			.client
			.get(&health_url)
			.timeout(Duration::from_millis(config.timeout_ms))
			.send()
			.await
		{
			Ok(response) => {
				let is_healthy = response.status().is_success();
				if is_healthy {
					debug!("OIF adapter {} is healthy", self.config.adapter_id);
				} else {
					warn!(
						"OIF adapter {} health check failed with status {}",
						self.config.adapter_id,
						response.status()
					);
				}
				Ok(is_healthy)
			},
			Err(e) => {
				warn!(
					"OIF adapter {} health check failed: {}",
					self.config.adapter_id, e
				);
				Ok(false)
			},
		}
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		debug!(
			"Getting order details for {} from {} (solver: {})",
			order_id, config.endpoint, config.solver_id
		);

		unimplemented!()
	}

	async fn get_supported_networks(&self) -> AdapterResult<Vec<Network>> {
		Err(AdapterError::NotImplemented(
			"OIF adapter getting supported networks not yet implemented".to_string(),
		))
	}

	async fn get_supported_assets(&self, _network: &Network) -> AdapterResult<Vec<Asset>> {
		Err(AdapterError::NotImplemented(
			"OIF adapter getting supported networks not yet implemented".to_string(),
		))
	}
}
