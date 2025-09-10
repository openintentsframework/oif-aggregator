//! LiFi adapter implementation
//!
//! This adapter uses an optimized client cache for connection pooling and keep-alive.

use async_trait::async_trait;
use oif_types::{Adapter, GetQuoteRequest, GetQuoteResponse, SolverRuntimeConfig};
use oif_types::{AdapterError, AdapterResult, SolverAdapter, SupportedAssetsData};
use reqwest::{
	header::{HeaderMap, HeaderValue},
	Client,
};
use std::{str::FromStr, sync::Arc};
use tracing::debug;

use crate::client_cache::{ClientCache, ClientConfig};

/// Client strategy for the LiFi adapter
#[derive(Debug)]
enum ClientStrategy {
	/// Use optimized client cache for connection pooling and reuse
	Cached(ClientCache),
	/// Create clients on-demand with no caching
	OnDemand,
}

/// LiFi adapter for cross-chain bridge quotes
#[derive(Debug)]
pub struct LifiAdapter {
	config: Adapter,
	client_strategy: ClientStrategy,
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
			client_strategy: ClientStrategy::Cached(cache),
		})
	}

	/// Create LiFi adapter without client caching
	///
	/// Creates clients on-demand for each request. Simpler but less efficient
	/// than the cached approach.
	pub fn without_cache(config: Adapter) -> AdapterResult<Self> {
		Ok(Self {
			config,
			client_strategy: ClientStrategy::OnDemand,
		})
	}

	/// Create a new HTTP client with LiFi headers and specified timeout
	fn create_client(solver_config: &SolverRuntimeConfig) -> AdapterResult<Arc<reqwest::Client>> {
		let mut headers = HeaderMap::new();
		headers.insert("Content-Type", HeaderValue::from_static("application/json"));
		headers.insert("User-Agent", HeaderValue::from_static("OIF-Aggregator/1.0"));
		headers.insert("X-Adapter-Type", HeaderValue::from_static("LiFi-v1"));
		headers.insert("Accept", HeaderValue::from_static("application/json"));

		// Add custom headers from the solver config
		if let Some(solver_headers) = &solver_config.headers {
			for (key, value) in solver_headers {
				if let (Ok(header_name), Ok(header_value)) = (
					reqwest::header::HeaderName::from_str(key),
					HeaderValue::from_str(value),
				) {
					headers.insert(header_name, header_value);
				}
			}
		}

		let client = Client::builder()
			.default_headers(headers)
			.build()
			.map_err(AdapterError::HttpError)?;

		Ok(Arc::new(client))
	}

	/// Get an HTTP client for the given solver configuration
	fn get_client(
		&self,
		solver_config: &SolverRuntimeConfig,
	) -> AdapterResult<Arc<reqwest::Client>> {
		match &self.client_strategy {
			ClientStrategy::Cached(cache) => {
				let client_config = ClientConfig::from(solver_config);
				cache.get_client(&client_config)
			},
			ClientStrategy::OnDemand => Self::create_client(solver_config),
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

	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		debug!("LiFi adapter health check for solver: {}", config.solver_id);

		unimplemented!()
	}

	async fn get_supported_assets(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<SupportedAssetsData> {
		debug!(
			"LiFi adapter getting supported assets for solver: {}",
			config.solver_id
		);

		// TODO: Implement LiFi asset/route fetching
		// For now, return empty assets mode
		Ok(SupportedAssetsData::Assets(Vec::new()))
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
		assert!(matches!(
			adapter_optimized.client_strategy,
			ClientStrategy::Cached(_)
		));

		// Test custom cache constructor
		let custom_cache = ClientCache::with_ttl(Duration::from_secs(60));
		let adapter_custom = LifiAdapter::with_cache(config.clone(), custom_cache).unwrap();
		assert!(matches!(
			adapter_custom.client_strategy,
			ClientStrategy::Cached(_)
		));

		// Test on-demand constructor
		let adapter_on_demand = LifiAdapter::without_cache(config.clone()).unwrap();
		assert!(matches!(
			adapter_on_demand.client_strategy,
			ClientStrategy::OnDemand
		));
	}

	#[test]
	fn test_lifi_adapter_default_config() {
		let adapter = LifiAdapter::with_default_config().unwrap();
		assert_eq!(adapter.id(), "lifi-v1");
		assert_eq!(adapter.name(), "LiFi v1 Adapter");
		assert!(matches!(adapter.client_strategy, ClientStrategy::Cached(_)));
	}
}
