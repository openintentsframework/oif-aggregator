//! OIF v1 adapter implementation for HTTP-based solvers
//!
//! This adapter uses an optimized client cache for connection pooling and keep-alive.

use reqwest::{
	header::{HeaderMap, HeaderValue},
	Client,
};
use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use oif_types::adapters::models::{SubmitOrderRequest, SubmitOrderResponse};
use oif_types::adapters::GetOrderResponse;
use oif_types::{
	Adapter, Asset, AssetRoute, GetQuoteRequest, GetQuoteResponse, Network, SolverRuntimeConfig,
};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::client_cache::{ClientCache, ClientConfig};

/// OIF tokens response models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OifTokensResponse {
	networks: HashMap<String, OifNetwork>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OifNetwork {
	chain_id: u64,
	input_settler: String,
	output_settler: String,
	tokens: Vec<OifToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OifToken {
	address: String,
	symbol: String,
	decimals: u8,
}

/// Client strategy for the OIF adapter
#[derive(Debug)]
enum ClientStrategy {
	/// Use optimized client cache for connection pooling and reuse
	Cached(ClientCache),
	/// Create clients on-demand with no caching
	OnDemand,
}

/// OIF v1 adapter for HTTP-based solvers
#[derive(Debug)]
pub struct OifAdapter {
	config: Adapter,
	client_strategy: ClientStrategy,
}

impl OifAdapter {
	/// Create a new OIF adapter with optimized client caching (recommended)
	///
	/// This constructor provides optimal performance with connection pooling,
	/// keep-alive optimization, and automatic TTL management.
	pub fn new(config: Adapter) -> AdapterResult<Self> {
		Self::with_cache(config, ClientCache::for_adapter())
	}

	/// Create OIF adapter with custom client cache
	///
	/// Allows using a custom cache configuration for specific performance requirements
	/// or testing scenarios.
	pub fn with_cache(config: Adapter, cache: ClientCache) -> AdapterResult<Self> {
		Ok(Self {
			config,
			client_strategy: ClientStrategy::Cached(cache),
		})
	}

	/// Create OIF adapter without client caching
	///
	/// Creates clients on-demand for each request. Simpler but less efficient
	/// than the cached approach.
	pub fn without_cache(config: Adapter) -> AdapterResult<Self> {
		Ok(Self {
			config,
			client_strategy: ClientStrategy::OnDemand,
		})
	}

	/// Create a new HTTP client with OIF headers and specified timeout
	fn create_client(solver_config: &SolverRuntimeConfig) -> AdapterResult<Arc<reqwest::Client>> {
		let mut headers = HeaderMap::new();
		headers.insert("Content-Type", HeaderValue::from_static("application/json"));
		headers.insert("User-Agent", HeaderValue::from_static("OIF-Aggregator/1.0"));
		headers.insert("X-Adapter-Type", HeaderValue::from_static("OIF-v1"));

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

	/// Create default OIF adapter instance with optimization
	pub fn with_default_config() -> AdapterResult<Self> {
		let config = Adapter::new(
			"oif-v1".to_string(),
			"OIF v1 Protocol".to_string(),
			"OIF v1 Adapter".to_string(),
			"1.0.0".to_string(),
		);

		Self::new(config)
	}

	/// Fetch assets from OIF API (private helper method)
	async fn fetch_assets_from_api(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Vec<Asset>> {
		let client = self.get_client(config)?;
		let tokens_url = format!("{}/tokens", config.endpoint);

		debug!(
			"Fetching supported assets from OIF adapter at {} (solver: {})",
			tokens_url, config.solver_id
		);

		// Make the tokens request
		let response = client
			.get(&tokens_url)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			return Err(AdapterError::InvalidResponse {
				reason: format!("OIF tokens endpoint returned status {}", response.status()),
			});
		}

		// Get response body as text first so we can print it
		let body = response.text().await.unwrap_or_default();
		debug!("OIF tokens endpoint response body: {}", body);

		// Parse the OIF tokens response
		let oif_response: OifTokensResponse =
			serde_json::from_str(&body).map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to parse OIF tokens response: {}", e),
			})?;

		// Transform OIF response to internal asset format
		let mut assets = Vec::new();
		let networks_count = oif_response.networks.len();

		for (chain_id_str, network_data) in oif_response.networks {
			let chain_id = chain_id_str.parse::<u64>().unwrap_or(network_data.chain_id);

			for token in network_data.tokens {
				let asset = Asset::new(
					token.address,
					token.symbol,
					"".to_string(), // OIF doesn't provide token names
					token.decimals,
					chain_id,
				);
				assets.push(asset);
			}
		}

		info!(
			"OIF adapter found {} supported assets across {} networks",
			assets.len(),
			networks_count
		);

		Ok(assets)
	}

	/// Generate cross-chain routes from supported assets
	///
	/// This creates routes for all cross-chain asset pairs, providing good
	/// compatibility coverage until native route support is available.
	fn generate_routes_from_assets(&self, assets: Vec<Asset>) -> Vec<AssetRoute> {
		use oif_types::models::InteropAddress;

		let mut routes = Vec::new();
		let assets_count = assets.len();

		debug!(
			"Generating cross-chain routes from {} assets for OIF adapter",
			assets_count
		);

		// Create cross-chain routes only (avoid same-chain noise)
		for origin in &assets {
			for destination in &assets {
				// Skip same-asset pairs and same-chain pairs
				if origin.chain_id != destination.chain_id && origin != destination {
					match (
						InteropAddress::from_chain_and_address(origin.chain_id, &origin.address),
						InteropAddress::from_chain_and_address(
							destination.chain_id,
							&destination.address,
						),
					) {
						(Ok(origin_addr), Ok(dest_addr)) => {
							let metadata = serde_json::json!({
								"source": "generated-from-assets",
								"originChainId": origin.chain_id,
								"destinationChainId": destination.chain_id,
								"generatedAt": chrono::Utc::now().timestamp()
							});

							routes.push(AssetRoute::with_symbols_and_metadata(
								origin_addr,
								origin.symbol.clone(),
								dest_addr,
								destination.symbol.clone(),
								metadata,
							));
						},
						(Err(e), _) | (_, Err(e)) => {
							warn!(
								"Failed to create InteropAddress for route generation: {}",
								e
							);
							continue;
						},
					}
				}
			}
		}

		info!(
			"Generated {} cross-chain routes from {} assets for OIF adapter",
			routes.len(),
			assets_count
		);

		routes
	}
}

#[async_trait]
impl SolverAdapter for OifAdapter {
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

		let quote_url = format!("{}/quotes", config.endpoint);
		let client = self.get_client(config)?;

		let response = client
			.post(quote_url)
			.json(&request)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			return Err(AdapterError::InvalidResponse {
				reason: format!("OIF quote endpoint returned status {}", response.status()),
			});
		}

		// Get response body as text first so we can print it
		let body = response.text().await.unwrap_or_default();
		debug!("OIF quote endpoint response body: {}", body);

		// Parse the response body manually since we already consumed it
		let quote_response: GetQuoteResponse =
			serde_json::from_str(&body).map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to parse OIF quote response: {}", e),
			})?;

		Ok(quote_response)
	}

	async fn submit_order(
		&self,
		order: &SubmitOrderRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<SubmitOrderResponse> {
		debug!(
			"Submitting order {} to OIF adapter {} via solver {}",
			order.order, self.config.adapter_id, config.solver_id
		);

		let orders_url = format!("{}/orders", config.endpoint);
		let client = self.get_client(config)?;

		debug!(
			"Submitting order to OIF adapter {} via solver {}",
			self.config.adapter_id, config.solver_id
		);

		let response = client
			.post(orders_url)
			.json(&order)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			return Err(AdapterError::InvalidResponse {
				reason: format!(
					"OIF order endpoint returned status {} with body {}",
					response.status(),
					response.text().await.unwrap_or_default()
				),
			});
		}

		// Get response body as text first so we can print it
		let body = response.text().await.unwrap_or_default();
		debug!("OIF order endpoint response body: {}", body);

		// Parse the response body manually since we already consumed it
		let order_response: SubmitOrderResponse =
			serde_json::from_str(&body).map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to parse OIF order response: {}", e),
			})?;

		// Check if response is invalid: bad status AND no order_id
		if !matches!(order_response.status.as_str(), "success" | "received")
			&& order_response.order_id.is_none()
		{
			return Err(AdapterError::InvalidResponse {
				reason: format!(
					"OIF order endpoint returned status '{}' with no order_id",
					order_response.status
				),
			});
		}

		Ok(order_response)
	}

	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		let tokens_url = format!("{}/tokens", config.endpoint);
		let client = self.get_client(config)?;

		debug!(
			"Health checking OIF adapter at {} (solver: {}) via /tokens endpoint",
			tokens_url, config.solver_id
		);

		match client.get(&tokens_url).send().await {
			Ok(response) => {
				let is_healthy = response.status().is_success();
				if is_healthy {
					// Optionally validate the response format for more thorough health check
					let body = response.text().await.unwrap_or_default();
					debug!("OIF health check endpoint response body: {}", body);

					match serde_json::from_str::<OifTokensResponse>(&body) {
						Ok(_) => {
							debug!("OIF adapter {} is healthy - /tokens endpoint responded with valid JSON", self.config.adapter_id);
							Ok(true)
						},
						Err(e) => {
							warn!(
								"OIF adapter {} /tokens endpoint returned success but invalid JSON: {}",
								self.config.adapter_id, e
							);
							// Still consider it healthy if HTTP status was success,
							// as the service is responding (might just be format issue)
							Ok(true)
						},
					}
				} else {
					warn!(
						"OIF adapter {} health check failed - /tokens endpoint returned status {}",
						self.config.adapter_id,
						response.status()
					);
					Ok(false)
				}
			},
			Err(e) => {
				warn!(
					"OIF adapter {} health check failed - /tokens endpoint error: {}",
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

		let order_url = format!("{}/orders/{}", config.endpoint, order_id);
		let client = self.get_client(config)?;

		let response = client
			.get(order_url)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			return Err(AdapterError::InvalidResponse {
				reason: format!("OIF order endpoint returned status {}", response.status()),
			});
		}

		// Get response body as text first so we can print it
		let body = response.text().await.unwrap_or_default();
		debug!("OIF get order endpoint response body: {}", body);

		// Parse the response body manually since we already consumed it
		let order_response: GetOrderResponse =
			serde_json::from_str(&body).map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to parse OIF get order response: {}", e),
			})?;

		Ok(order_response)
	}

	async fn get_supported_networks(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Vec<Network>> {
		let client = self.get_client(config)?;
		let tokens_url = format!("{}/tokens", config.endpoint);

		debug!(
			"Fetching supported networks from OIF adapter at {} (solver: {})",
			tokens_url, config.solver_id
		);

		// Make the tokens request (for asset-to-route conversion)
		let response = client
			.get(&tokens_url)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			return Err(AdapterError::InvalidResponse {
				reason: format!("OIF tokens endpoint returned status {}", response.status()),
			});
		}

		// Get response body as text first so we can print it
		let body = response.text().await.unwrap_or_default();
		debug!("OIF networks endpoint response body: {}", body);

		// Parse the OIF tokens response
		let oif_response: OifTokensResponse =
			serde_json::from_str(&body).map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to parse OIF tokens response: {}", e),
			})?;

		// Extract networks from the response
		let mut networks = Vec::new();
		for (chain_id_str, network_data) in oif_response.networks {
			let chain_id = chain_id_str.parse::<u64>().unwrap_or(network_data.chain_id);

			let network = Network::new(chain_id, None, None);
			networks.push(network);
		}

		debug!("OIF adapter found {} supported networks", networks.len());

		Ok(networks)
	}

	async fn get_supported_routes(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Vec<AssetRoute>> {
		debug!(
			"OIF adapter getting supported routes via asset conversion for solver: {}",
			config.solver_id
		);

		// Generate routes from assets using existing tokens endpoint
		let assets = self.fetch_assets_from_api(config).await?;
		let routes = self.generate_routes_from_assets(assets);

		debug!(
			"OIF adapter generated {} routes from assets for solver {}",
			routes.len(),
			config.solver_id
		);

		Ok(routes)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn test_oif_adapter_construction_patterns() {
		let config = Adapter::new(
			"test-oif".to_string(),
			"Test OIF".to_string(),
			"Test OIF Adapter".to_string(),
			"1.0.0".to_string(),
		);

		// Test optimized constructor (default)
		let adapter_optimized = OifAdapter::new(config.clone()).unwrap();
		assert!(matches!(
			adapter_optimized.client_strategy,
			ClientStrategy::Cached(_)
		));

		// Test custom cache constructor
		let custom_cache = ClientCache::with_ttl(Duration::from_secs(60));
		let adapter_custom = OifAdapter::with_cache(config.clone(), custom_cache).unwrap();
		assert!(matches!(
			adapter_custom.client_strategy,
			ClientStrategy::Cached(_)
		));

		// Test on-demand constructor
		let adapter_on_demand = OifAdapter::without_cache(config.clone()).unwrap();
		assert!(matches!(
			adapter_on_demand.client_strategy,
			ClientStrategy::OnDemand
		));
	}

	#[test]
	fn test_oif_adapter_default_config() {
		let adapter = OifAdapter::with_default_config().unwrap();
		assert_eq!(adapter.id(), "oif-v1");
		assert_eq!(adapter.name(), "OIF v1 Adapter");
		assert!(matches!(adapter.client_strategy, ClientStrategy::Cached(_)));
	}

	#[test]
	fn test_oif_tokens_response_parsing() {
		let json_response = r#"{
			"networks": {
				"31338": {
					"chain_id": 31338,
					"input_settler": "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0",
					"output_settler": "0xcf7ed3acca5a467e9e704c703e8d87f634fb0fc9",
					"tokens": [
						{
							"address": "0x5fbdb2315678afecb367f032d93f642f64180aa3",
							"symbol": "TOKA",
							"decimals": 18
						},
						{
							"address": "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512",
							"symbol": "TOKB",
							"decimals": 18
						}
					]
				},
				"31337": {
					"chain_id": 31337,
					"input_settler": "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0",
					"output_settler": "0xcf7ed3acca5a467e9e704c703e8d87f634fb0fc9",
					"tokens": [
						{
							"address": "0x5fbdb2315678afecb367f032d93f642f64180aa3",
							"symbol": "TOKA",
							"decimals": 18
						}
					]
				}
			}
		}"#;

		// Test parsing
		let response: OifTokensResponse = serde_json::from_str(json_response).unwrap();

		assert_eq!(response.networks.len(), 2);
		assert!(response.networks.contains_key("31338"));
		assert!(response.networks.contains_key("31337"));

		let network_31338 = &response.networks["31338"];
		assert_eq!(network_31338.chain_id, 31338);
		assert_eq!(network_31338.tokens.len(), 2);
		assert_eq!(network_31338.tokens[0].symbol, "TOKA");
		assert_eq!(network_31338.tokens[0].decimals, 18);

		let network_31337 = &response.networks["31337"];
		assert_eq!(network_31337.chain_id, 31337);
		assert_eq!(network_31337.tokens.len(), 1);
	}

	#[test]
	fn test_oif_tokens_to_assets_conversion() {
		let oif_response = OifTokensResponse {
			networks: {
				let mut networks = HashMap::new();
				networks.insert(
					"1".to_string(),
					OifNetwork {
						chain_id: 1,
						input_settler: "0x123".to_string(),
						output_settler: "0x456".to_string(),
						tokens: vec![
							OifToken {
								address: "0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86".to_string(),
								symbol: "USDC".to_string(),
								decimals: 6,
							},
							OifToken {
								address: "0x0000000000000000000000000000000000000000".to_string(),
								symbol: "ETH".to_string(),
								decimals: 18,
							},
						],
					},
				);
				networks.insert(
					"137".to_string(),
					OifNetwork {
						chain_id: 137,
						input_settler: "0x789".to_string(),
						output_settler: "0xabc".to_string(),
						tokens: vec![OifToken {
							address: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(),
							symbol: "USDC".to_string(),
							decimals: 6,
						}],
					},
				);
				networks
			},
		};

		// Convert to assets
		let mut assets = Vec::new();
		for (chain_id_str, network) in oif_response.networks {
			let chain_id = chain_id_str.parse::<u64>().unwrap_or(network.chain_id);

			for token in network.tokens {
				let asset = Asset::new(
					token.address,
					token.symbol.clone(),
					token.symbol,
					token.decimals,
					chain_id,
				);
				assets.push(asset);
			}
		}

		// Verify conversion
		assert_eq!(assets.len(), 3);

		// Find USDC on Ethereum
		let usdc_eth = assets
			.iter()
			.find(|a| a.symbol == "USDC" && a.chain_id == 1)
			.unwrap();
		assert_eq!(
			usdc_eth.address,
			"0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86"
		);
		assert_eq!(usdc_eth.decimals, 6);
		assert_eq!(usdc_eth.name, "USDC");

		// Find ETH on Ethereum
		let eth = assets
			.iter()
			.find(|a| a.symbol == "ETH" && a.chain_id == 1)
			.unwrap();
		assert_eq!(eth.address, "0x0000000000000000000000000000000000000000");
		assert_eq!(eth.decimals, 18);

		// Find USDC on Polygon
		let usdc_poly = assets
			.iter()
			.find(|a| a.symbol == "USDC" && a.chain_id == 137)
			.unwrap();
		assert_eq!(
			usdc_poly.address,
			"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"
		);
		assert_eq!(usdc_poly.decimals, 6);
	}

	#[test]
	fn test_route_generation_from_assets() {
		let adapter = OifAdapter::with_default_config().unwrap();

		// Create test assets on different chains with different addresses
		let assets = vec![
			Asset::new(
				"0x5fbdb2315678afecb367f032d93f642f64180aa3".to_string(),
				"TOKA".to_string(),
				"Token A".to_string(),
				18,
				31338,
			),
			Asset::new(
				"0xe7f1725e7734ce288f8367e1bb143e90bb3f0512".to_string(),
				"TOKB".to_string(),
				"Token B".to_string(),
				18,
				31338,
			),
			Asset::new(
				"0xa0b86a33e6441e7c81f7c93451777f5f4de78e86".to_string(),
				"TOKA".to_string(),
				"Token A".to_string(),
				18,
				31337,
			),
		];

		let routes = adapter.generate_routes_from_assets(assets);

		// Should generate cross-chain routes only
		// 3 assets across 2 chains = 4 cross-chain routes:
		// 31338:TOKA -> 31337:TOKA
		// 31338:TOKB -> 31337:TOKA
		// 31337:TOKA -> 31338:TOKA
		// 31337:TOKA -> 31338:TOKB
		assert_eq!(routes.len(), 4);

		// Verify all routes are cross-chain
		for route in &routes {
			assert!(route.is_cross_chain());
			assert!(!route.is_same_chain());

			// Verify metadata
			assert!(route.metadata.is_some());
			let metadata = route.metadata.as_ref().unwrap();
			assert_eq!(metadata["source"], "generated-from-assets");
			assert!(metadata.get("generatedAt").is_some());
		}

		// Verify specific routes exist
		let has_toka_cross_route = routes.iter().any(|r| {
			r.origin_chain_id().unwrap_or(0) == 31338
				&& r.destination_chain_id().unwrap_or(0) == 31337
				&& r.origin_token_symbol == Some("TOKA".to_string())
				&& r.destination_token_symbol == Some("TOKA".to_string())
		});
		assert!(has_toka_cross_route, "Should have TOKA cross-chain route");
	}

	#[test]
	fn test_route_generation_edge_cases() {
		let adapter = OifAdapter::with_default_config().unwrap();

		// Test with empty assets
		let empty_routes = adapter.generate_routes_from_assets(vec![]);
		assert_eq!(empty_routes.len(), 0);

		// Test with single asset (no routes possible)
		let single_asset = vec![Asset::new(
			"0x123".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
			1,
		)];
		let single_routes = adapter.generate_routes_from_assets(single_asset);
		assert_eq!(single_routes.len(), 0);

		// Test with same-chain assets only (no cross-chain routes)
		let same_chain_assets = vec![
			Asset::new(
				"0x123".to_string(),
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
				1,
			),
			Asset::new(
				"0x456".to_string(),
				"WETH".to_string(),
				"Wrapped Ether".to_string(),
				18,
				1,
			),
		];
		let same_chain_routes = adapter.generate_routes_from_assets(same_chain_assets);
		assert_eq!(
			same_chain_routes.len(),
			0,
			"Should not generate same-chain routes"
		);
	}
}
