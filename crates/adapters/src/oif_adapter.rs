//! OIF v1 adapter implementation for HTTP-based solvers
//!
//! This adapter uses an optimized client cache for connection pooling and keep-alive.

use std::sync::Arc;

use async_trait::async_trait;
use oif_types::adapters::models::SubmitOrderRequest;
use oif_types::adapters::GetOrderResponse;
use oif_types::{Adapter, Asset, GetQuoteRequest, GetQuoteResponse, Network, SolverRuntimeConfig};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

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

/// OIF v1 adapter for HTTP-based solvers
/// This adapter supports optional client caching for optimal connection management
#[derive(Debug)]
pub struct OifAdapter {
	config: Adapter,
	cache: Option<ClientCache>,
}

impl OifAdapter {
	/// Convert chain ID to network name and detect if it's a testnet
	fn chain_id_to_network_info(chain_id: u64) -> (String, bool) {
		match chain_id {
			// Mainnets
			1 => ("Ethereum".to_string(), false),
			137 => ("Polygon".to_string(), false),
			56 => ("BSC".to_string(), false),
			42161 => ("Arbitrum One".to_string(), false),
			8453 => ("Base".to_string(), false),
			10 => ("Optimism".to_string(), false),
			43114 => ("Avalanche".to_string(), false),
			250 => ("Fantom".to_string(), false),

			// Testnets
			11155111 => ("Sepolia".to_string(), true),
			80001 => ("Mumbai".to_string(), true),
			5 => ("Goerli".to_string(), true),
			421613 => ("Arbitrum Goerli".to_string(), true),
			84531 => ("Base Goerli".to_string(), true),
			420 => ("Optimism Goerli".to_string(), true),

			// Local/dev networks (common dev chain IDs)
			31337 | 31338 | 1337 => (format!("Local Dev ({})", chain_id), true),

			// Unknown chain IDs - try to detect testnet patterns
			_ => {
				let is_testnet = chain_id > 1000000 || // Very high chain IDs often testnets
					chain_id.to_string().contains("000"); // Chain IDs with lots of zeros often testnets
				let name = format!("Chain {}", chain_id);
				(name, is_testnet)
			},
		}
	}
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
			cache: Some(cache),
		})
	}

	/// Create OIF adapter without client caching
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
	) -> AdapterResult<Arc<reqwest::Client>> {
		if let Some(cache) = &self.cache {
			// Optimized path: use cached client with configuration
			let mut client_config = ClientConfig::from(solver_config);

			// Add OIF-specific headers
			client_config
				.headers
				.push(("X-Adapter-Type".to_string(), "OIF-v1".to_string()));

			cache.get_client(&client_config)
		} else {
			// Basic path: create simple client for each request
			debug!("Creating basic HTTP client for OIF adapter (no cache)");

			use reqwest::{
				header::{HeaderMap, HeaderValue},
				Client,
			};
			use std::time::Duration;

			let mut headers = HeaderMap::new();
			headers.insert("Content-Type", HeaderValue::from_static("application/json"));
			headers.insert("User-Agent", HeaderValue::from_static("OIF-Aggregator/1.0"));
			headers.insert("X-Adapter-Type", HeaderValue::from_static("OIF-v1"));

			let client = Client::builder()
				.default_headers(headers)
				.timeout(Duration::from_millis(solver_config.timeout_ms))
				.build()
				.map_err(AdapterError::HttpError)?;

			Ok(Arc::new(client))
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
		let tokens_url = format!("{}/tokens", config.endpoint);
		let client = self.get_client(config)?;

		debug!(
			"Health checking OIF adapter at {} (solver: {}) via /tokens endpoint with optimized client",
			tokens_url, config.solver_id
		);

		match client
			.get(&tokens_url)
			.send() // Timeout is configured at client level
			.await
		{
			Ok(response) => {
				let is_healthy = response.status().is_success();
				if is_healthy {
					// Optionally validate the response format for more thorough health check
					match response.json::<OifTokensResponse>().await {
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

		unimplemented!()
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

		// Make the tokens request (same as get_supported_assets)
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

		// Parse the OIF tokens response
		let oif_response: OifTokensResponse =
			response
				.json()
				.await
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Failed to parse OIF tokens response: {}", e),
				})?;

		// Extract networks from the response
		let mut networks = Vec::new();
		for (chain_id_str, network_data) in oif_response.networks {
			let chain_id = chain_id_str.parse::<u64>().unwrap_or(network_data.chain_id);

			let (name, is_testnet) = Self::chain_id_to_network_info(chain_id);

			let network = Network::new(chain_id, name, is_testnet);
			networks.push(network);
		}

		debug!("OIF adapter found {} supported networks", networks.len());

		Ok(networks)
	}

	async fn get_supported_assets(
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

		// Parse the OIF tokens response
		let oif_response: OifTokensResponse =
			response
				.json()
				.await
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Failed to parse OIF tokens response: {}", e),
				})?;

		// Convert OIF tokens to Assets
		let mut assets = Vec::new();
		let networks_count = oif_response.networks.len();

		for (chain_id_str, network) in oif_response.networks {
			let chain_id = chain_id_str.parse::<u64>().unwrap_or(network.chain_id); // Use parsed chain_id or fallback to network.chain_id

			for token in network.tokens {
				let asset = Asset::new(
					token.address,
					token.symbol.clone(),
					token.symbol, // Use symbol as name for now (could be enhanced with a token registry)
					token.decimals,
					chain_id,
				);
				assets.push(asset);
			}
		}

		debug!(
			"OIF adapter found {} supported assets across {} networks",
			assets.len(),
			networks_count
		);

		Ok(assets)
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
		assert!(adapter_optimized.cache.is_some());

		// Test custom cache constructor
		let custom_cache = ClientCache::with_ttl(Duration::from_secs(60));
		let adapter_custom = OifAdapter::with_cache(config.clone(), custom_cache).unwrap();
		assert!(adapter_custom.cache.is_some());

		// Test basic constructor (no cache)
		let adapter_basic = OifAdapter::without_cache(config.clone()).unwrap();
		assert!(adapter_basic.cache.is_none());
	}

	#[test]
	fn test_oif_adapter_default_config() {
		let adapter = OifAdapter::with_default_config().unwrap();
		assert_eq!(adapter.adapter_id(), "oif-v1");
		assert_eq!(adapter.adapter_name(), "OIF v1 Adapter");
		assert!(adapter.cache.is_some()); // Should use optimized cache by default
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
	fn test_chain_id_to_network_info() {
		// Test mainnet networks
		assert_eq!(
			OifAdapter::chain_id_to_network_info(1),
			("Ethereum".to_string(), false)
		);
		assert_eq!(
			OifAdapter::chain_id_to_network_info(137),
			("Polygon".to_string(), false)
		);
		assert_eq!(
			OifAdapter::chain_id_to_network_info(8453),
			("Base".to_string(), false)
		);

		// Test testnet networks
		assert_eq!(
			OifAdapter::chain_id_to_network_info(11155111),
			("Sepolia".to_string(), true)
		);
		assert_eq!(
			OifAdapter::chain_id_to_network_info(80001),
			("Mumbai".to_string(), true)
		);

		// Test local dev networks
		assert_eq!(
			OifAdapter::chain_id_to_network_info(31337),
			("Local Dev (31337)".to_string(), true)
		);
		assert_eq!(
			OifAdapter::chain_id_to_network_info(31338),
			("Local Dev (31338)".to_string(), true)
		);

		// Test unknown chain IDs
		assert_eq!(
			OifAdapter::chain_id_to_network_info(999999),
			("Chain 999999".to_string(), false)
		);
		assert_eq!(
			OifAdapter::chain_id_to_network_info(9999999),
			("Chain 9999999".to_string(), true)
		); // High chain ID = testnet
	}

	#[test]
	fn test_oif_networks_extraction() {
		let oif_response = OifTokensResponse {
			networks: {
				let mut networks = HashMap::new();
				networks.insert(
					"1".to_string(),
					OifNetwork {
						chain_id: 1,
						input_settler: "0x123".to_string(),
						output_settler: "0x456".to_string(),
						tokens: vec![],
					},
				);
				networks.insert(
					"137".to_string(),
					OifNetwork {
						chain_id: 137,
						input_settler: "0x789".to_string(),
						output_settler: "0xabc".to_string(),
						tokens: vec![],
					},
				);
				networks.insert(
					"31337".to_string(),
					OifNetwork {
						chain_id: 31337,
						input_settler: "0xdef".to_string(),
						output_settler: "0x123".to_string(),
						tokens: vec![],
					},
				);
				networks
			},
		};

		// Extract networks (simulating the logic from get_supported_networks)
		let mut networks = Vec::new();
		for (chain_id_str, network_data) in oif_response.networks {
			let chain_id = chain_id_str.parse::<u64>().unwrap_or(network_data.chain_id);

			let (name, is_testnet) = OifAdapter::chain_id_to_network_info(chain_id);

			let network = Network::new(chain_id, name, is_testnet);
			networks.push(network);
		}

		// Verify extraction
		assert_eq!(networks.len(), 3);

		// Find Ethereum
		let ethereum = networks.iter().find(|n| n.chain_id == 1).unwrap();
		assert_eq!(ethereum.name, "Ethereum");
		assert!(!ethereum.is_testnet);

		// Find Polygon
		let polygon = networks.iter().find(|n| n.chain_id == 137).unwrap();
		assert_eq!(polygon.name, "Polygon");
		assert!(!polygon.is_testnet);

		// Find Local Dev
		let local_dev = networks.iter().find(|n| n.chain_id == 31337).unwrap();
		assert_eq!(local_dev.name, "Local Dev (31337)");
		assert!(local_dev.is_testnet);
	}
}
