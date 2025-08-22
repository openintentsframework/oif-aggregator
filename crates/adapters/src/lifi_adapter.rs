//! LiFi adapter implementation
//!
//! This adapter uses an optimized client cache for connection pooling and keep-alive.

use async_trait::async_trait;
use oif_types::adapters::{
	models::{SubmitOrderRequest, SubmitOrderResponse},
	GetOrderResponse,
};
use oif_types::{Adapter, Asset, GetQuoteRequest, GetQuoteResponse, Network, SolverRuntimeConfig};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use tracing::debug;

use crate::client_cache::{ClientCache, ClientConfig};

/// LiFi adapter for cross-chain bridge quotes
/// This adapter supports optional client caching for optimal connection management
#[derive(Debug)]
pub struct LifiAdapter {
	config: Adapter,
	cache: Option<ClientCache>,
}

#[allow(dead_code)]
impl LifiAdapter {
	/// Create a new LiFi adapter with optimized client caching (recommended)
	///
	/// This constructor provides optimal performance with connection pooling,
	/// keep-alive optimization, and automatic TTL management.
	pub fn new(config: Adapter) -> AdapterResult<Self> {
		Self::with_cache(config, ClientCache::for_adapter())
	}

	/// Create LiFi adapter with custom client cache
	///
	/// Allows using a custom cache configuration for specific performance requirements
	/// or testing scenarios.
	pub fn with_cache(config: Adapter, cache: ClientCache) -> AdapterResult<Self> {
		Ok(Self {
			config,
			cache: Some(cache),
		})
	}

	/// Create LiFi adapter without client caching
	///
	/// This creates a basic adapter without optimization. Each request will create
	/// a new HTTP client. Only recommended for simple use cases or when you want
	/// to implement your own client management.
	pub fn without_cache(config: Adapter) -> AdapterResult<Self> {
		Ok(Self {
			config,
			cache: None,
		})
	}

	/// Get an HTTP client for the given solver configuration
	///
	/// Returns either an optimized cached client (if cache is enabled) or a basic client
	fn get_client(
		&self,
		solver_config: &SolverRuntimeConfig,
	) -> AdapterResult<std::sync::Arc<reqwest::Client>> {
		if let Some(cache) = &self.cache {
			// Optimized path: use cached client with configuration
			let mut client_config = ClientConfig::from(solver_config);

			// Add LiFi-specific headers
			client_config
				.headers
				.push(("X-Adapter-Type".to_string(), "LiFi-v1".to_string()));
			client_config
				.headers
				.push(("Accept".to_string(), "application/json".to_string()));

			cache.get_client(&client_config)
		} else {
			// Basic path: create simple client for each request
			debug!("Creating basic HTTP client for LiFi adapter (no cache)");

			use reqwest::{
				header::{HeaderMap, HeaderValue},
				Client,
			};
			use std::time::Duration;

			let mut headers = HeaderMap::new();
			headers.insert("Content-Type", HeaderValue::from_static("application/json"));
			headers.insert("User-Agent", HeaderValue::from_static("OIF-Aggregator/1.0"));
			headers.insert("X-Adapter-Type", HeaderValue::from_static("LiFi-v1"));
			headers.insert("Accept", HeaderValue::from_static("application/json"));

			let client = Client::builder()
				.default_headers(headers)
				.timeout(Duration::from_millis(solver_config.timeout_ms))
				.build()
				.map_err(AdapterError::HttpError)?;

			Ok(std::sync::Arc::new(client))
		}
	}

	/// Create default LiFi adapter instance with optimization
	pub fn with_default_config() -> AdapterResult<Self> {
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
		order: &SubmitOrderRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<SubmitOrderResponse> {
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

	async fn get_supported_networks(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Vec<Network>> {
		debug!(
			"LiFi adapter getting supported networks via solver: {}",
			config.solver_id
		);

		unimplemented!()
	}

	async fn get_supported_assets(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Vec<Asset>> {
		debug!(
			"LiFi adapter getting supported assets via solver: {}",
			config.solver_id
		);

		unimplemented!()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn test_lifi_adapter_construction_patterns() {
		let config = Adapter::new(
			"test-lifi".to_string(),
			"Test LiFi".to_string(),
			"Test LiFi Adapter".to_string(),
			"1.0.0".to_string(),
		);

		// Test optimized constructor (default)
		let adapter_optimized = LifiAdapter::new(config.clone()).unwrap();
		assert!(adapter_optimized.cache.is_some());

		// Test custom cache constructor
		let custom_cache = ClientCache::with_ttl(Duration::from_secs(60));
		let adapter_custom = LifiAdapter::with_cache(config.clone(), custom_cache).unwrap();
		assert!(adapter_custom.cache.is_some());

		// Test basic constructor (no cache)
		let adapter_basic = LifiAdapter::without_cache(config.clone()).unwrap();
		assert!(adapter_basic.cache.is_none());
	}

	#[test]
	fn test_lifi_adapter_default_config() {
		let adapter = LifiAdapter::with_default_config().unwrap();
		assert_eq!(adapter.adapter_id(), "lifi-v1");
		assert_eq!(adapter.adapter_name(), "LiFi v1 Adapter");
		assert!(adapter.cache.is_some()); // Should use optimized cache by default
	}
}
