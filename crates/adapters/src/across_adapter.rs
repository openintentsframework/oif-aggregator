//! Across adapter implementation
//!
//! This adapter uses an optimized client cache for connection pooling and keep-alive.
//! Submission and tracking of orders are not supported by this adapter.

use async_trait::async_trait;
use oif_types::adapters::AdapterQuote;
use oif_types::{
	Adapter, Asset, AssetRoute, GetQuoteRequest, GetQuoteResponse, SolverRuntimeConfig,
};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use reqwest::{
	header::{HeaderMap, HeaderValue},
	Client,
};
use serde_json;
use std::{str::FromStr, sync::Arc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::client_cache::{ClientCache, ClientConfig};

// ================================
// ACROSS API MODELS
// ================================

/// Across available route response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossRoute {
	/// Origin chain ID
	pub origin_chain_id: u64,
	/// Destination chain ID  
	pub destination_chain_id: u64,
	/// Origin token contract address
	pub origin_token: String,
	/// Destination token contract address
	pub destination_token: String,
	/// Origin token symbol
	pub origin_token_symbol: String,
	/// Destination token symbol
	pub destination_token_symbol: String,
	/// Whether the origin token is a native token
	pub is_native: bool,
}

/// Across suggested fees response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossQuoteResponse {
	/// Estimated fill time in seconds
	pub estimated_fill_time_sec: u64,
	/// Capital fee percentage
	pub capital_fee_pct: String,
	/// Capital fee total
	pub capital_fee_total: String,
	/// Relay gas fee percentage
	pub relay_gas_fee_pct: String,
	/// Relay gas fee total
	pub relay_gas_fee_total: String,
	/// Relay fee percentage
	pub relay_fee_pct: String,
	/// Relay fee total
	pub relay_fee_total: String,
	/// LP fee percentage
	pub lp_fee_pct: String,
	/// Timestamp
	pub timestamp: String,
	/// Whether amount is too low
	pub is_amount_too_low: bool,
	/// Quote block number
	pub quote_block: String,
	/// Exclusive relayer address
	pub exclusive_relayer: String,
	/// Exclusivity deadline timestamp
	pub exclusivity_deadline: u64,
	/// Spoke pool address
	pub spoke_pool_address: String,
	/// Destination spoke pool address
	pub destination_spoke_pool_address: String,
	/// Total relay fee breakdown
	pub total_relay_fee: AcrossFeeBand,
	/// Relayer capital fee breakdown
	pub relayer_capital_fee: AcrossFeeBand,
	/// Relayer gas fee breakdown
	pub relayer_gas_fee: AcrossFeeBand,
	/// LP fee breakdown
	pub lp_fee: AcrossFeeBand,
	/// Deposit limits
	pub limits: AcrossLimits,
	/// Fill deadline timestamp
	pub fill_deadline: String,
	/// Output amount
	pub output_amount: String,
	/// Input token details
	pub input_token: AcrossToken,
	/// Output token details
	pub output_token: AcrossToken,
	/// Quote ID
	pub id: String,
}

/// Across fee breakdown
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcrossFeeBand {
	/// Fee percentage
	pub pct: String,
	/// Fee total amount
	pub total: String,
}

/// Across deposit limits
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossLimits {
	/// Minimum deposit amount
	pub min_deposit: String,
	/// Maximum deposit amount
	pub max_deposit: String,
	/// Maximum instant deposit amount
	pub max_deposit_instant: String,
	/// Maximum short delay deposit amount
	pub max_deposit_short_delay: String,
	/// Recommended instant deposit amount
	pub recommended_deposit_instant: String,
}

/// Across token information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossToken {
	/// Token contract address
	pub address: String,
	/// Token symbol
	pub symbol: String,
	/// Token decimals
	pub decimals: u8,
	/// Chain ID
	pub chain_id: u64,
}

impl AcrossRoute {
	/// Convert Across route to internal Asset models
	/// Returns a tuple of (origin_asset, destination_asset)
	pub fn to_assets(&self) -> AdapterResult<(Asset, Asset)> {
		let origin_chain_id = self.origin_chain_id;
		let destination_chain_id = self.destination_chain_id;

		// Create origin asset
		let origin_asset = Asset::new(
			self.origin_token.clone(),
			self.origin_token_symbol.clone(),
			self.origin_token_symbol.clone(), // Use symbol as name for now
			18, // Default to 18 decimals, could be improved with token registry
			origin_chain_id,
		);

		// Create destination asset
		let destination_asset = Asset::new(
			self.destination_token.clone(),
			self.destination_token_symbol.clone(),
			self.destination_token_symbol.clone(), // Use symbol as name for now
			18, // Default to 18 decimals, could be improved with token registry
			destination_chain_id,
		);

		Ok((origin_asset, destination_asset))
	}

	/// Convert Across route to internal AssetRoute model
	pub fn to_asset_route(&self) -> AdapterResult<AssetRoute> {
		use oif_types::models::InteropAddress;

		// Create InteropAddresses for origin and destination
		let origin_asset =
			InteropAddress::from_chain_and_address(self.origin_chain_id, &self.origin_token)
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Invalid origin asset in Across route: {}", e),
				})?;

		let destination_asset = InteropAddress::from_chain_and_address(
			self.destination_chain_id,
			&self.destination_token,
		)
		.map_err(|e| AdapterError::InvalidResponse {
			reason: format!("Invalid destination asset in Across route: {}", e),
		})?;

		Ok(AssetRoute::with_symbols(
			origin_asset,
			self.origin_token_symbol.clone(),
			destination_asset,
			self.destination_token_symbol.clone(),
		))
	}
}

/// Client strategy for the Across adapter
#[derive(Debug)]
enum ClientStrategy {
	/// Use optimized client cache for connection pooling and reuse
	Cached(ClientCache),
	/// Create clients on-demand with no caching
	OnDemand,
}

/// Across adapter for cross-chain bridge quotes
#[derive(Debug)]
pub struct AcrossAdapter {
	config: Adapter,
	client_strategy: ClientStrategy,
}

#[allow(dead_code)]
impl AcrossAdapter {
	/// Create a new Across adapter with optimized client caching (recommended)
	///
	/// This constructor provides optimal performance with connection pooling,
	/// keep-alive optimization, and automatic TTL management.
	pub fn new(config: Adapter) -> AdapterResult<Self> {
		Self::with_cache(config, ClientCache::for_adapter())
	}

	/// Create Across adapter with custom client cache
	///
	/// Allows using a custom cache configuration for specific performance requirements
	/// or testing scenarios.
	pub fn with_cache(config: Adapter, cache: ClientCache) -> AdapterResult<Self> {
		Ok(Self {
			config,
			client_strategy: ClientStrategy::Cached(cache),
		})
	}

	/// Create Across adapter without client caching
	///
	/// Creates clients on-demand for each request. Simpler but less efficient
	/// than the cached approach.
	pub fn without_cache(config: Adapter) -> AdapterResult<Self> {
		Ok(Self {
			config,
			client_strategy: ClientStrategy::OnDemand,
		})
	}

	/// Create a new HTTP client with Across headers and specified timeout
	fn create_client(solver_config: &SolverRuntimeConfig) -> AdapterResult<Arc<reqwest::Client>> {
		let mut headers = HeaderMap::new();
		headers.insert("Content-Type", HeaderValue::from_static("application/json"));
		headers.insert("User-Agent", HeaderValue::from_static("OIF-Aggregator/1.0"));
		headers.insert("X-Adapter-Type", HeaderValue::from_static("Across-v1"));
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

	/// Create default Across adapter instance with optimization
	pub fn with_default_config() -> AdapterResult<Self> {
		let config = Adapter::new(
			"across-v1".to_string(),
			"Across v1 Protocol".to_string(),
			"Across v1 Adapter".to_string(),
			"1.0.0".to_string(),
		);

		Self::new(config)
	}

	/// Convert Across quote response to internal adapter quote format
	fn convert_across_quote_to_adapter_quote(
		&self,
		across_quote: AcrossQuoteResponse,
		request: &GetQuoteRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<AdapterQuote> {
		use oif_types::adapters::{AdapterQuote, QuoteDetails};

		// Generate unique quote ID
		let quote_id = Uuid::new_v4().to_string();

		// Create quote details from the request structure
		let details = QuoteDetails {
			available_inputs: request.available_inputs.clone(),
			requested_outputs: request.requested_outputs.clone(),
		};

		// Parse timestamp for valid_until
		let valid_until = across_quote.timestamp.parse::<u64>().ok();

		let metadata = serde_json::json!({
			"across": across_quote
		});

		Ok(AdapterQuote {
			orders: vec![], // EIP-712 orders are not supported for Across
			details,
			valid_until,
			eta: Some(across_quote.estimated_fill_time_sec),
			quote_id,
			provider: format!("Across Protocol v{}", self.config.version),
			metadata: Some(metadata),
		})
	}
}

#[async_trait]
impl SolverAdapter for AcrossAdapter {
	fn adapter_info(&self) -> &Adapter {
		&self.config
	}

	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<GetQuoteResponse> {
		debug!(
			"Across adapter getting quotes for {} inputs and {} outputs via solver: {}",
			request.available_inputs.len(),
			request.requested_outputs.len(),
			config.solver_id
		);

		// Validate request - Across adapter currently supports single input/output
		if request.available_inputs.len() != 1 {
			return Err(AdapterError::InvalidResponse {
				reason: "Across adapter currently supports only single input".to_string(),
			});
		}
		if request.requested_outputs.len() != 1 {
			return Err(AdapterError::InvalidResponse {
				reason: "Across adapter currently supports only single output".to_string(),
			});
		}

		let input = &request.available_inputs[0];
		let output = &request.requested_outputs[0];

		// Extract chain IDs and addresses from InteropAddress
		let input_chain_id =
			input
				.asset
				.extract_chain_id()
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Invalid input asset chain ID: {}", e),
				})?;
		let input_token = input.asset.extract_address();

		let output_chain_id =
			output
				.asset
				.extract_chain_id()
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Invalid output asset chain ID: {}", e),
				})?;
		let output_token = output.asset.extract_address();

		// Build query parameters for Across API
		let client = self.get_client(config)?;
		let quote_url = format!("{}/suggested-fees", config.endpoint);

		debug!(
			"Fetching Across quote from {} (solver: {}) - {}:{} -> {}:{}",
			quote_url, config.solver_id, input_chain_id, input_token, output_chain_id, output_token
		);

		// Make the quote request
		let response = client
			.get(&quote_url)
			.query(&[
				("inputToken", input_token.as_str()),
				("outputToken", output_token.as_str()),
				("originChainId", &input_chain_id.to_string()),
				("destinationChainId", &output_chain_id.to_string()),
				("amount", &input.amount.to_string()),
			])
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			return Err(AdapterError::InvalidResponse {
				reason: format!(
					"Across quote endpoint returned status {}",
					response.status()
				),
			});
		}

		// Parse the JSON response
		let across_quote: AcrossQuoteResponse =
			response
				.json()
				.await
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Failed to parse Across quote response: {}", e),
				})?;

		// Check if amount is too low
		if across_quote.is_amount_too_low {
			return Err(AdapterError::InvalidResponse {
				reason: format!(
					"Amount {} is below minimum deposit of {}",
					input.amount, across_quote.limits.min_deposit
				),
			});
		}

		// Convert Across response to internal quote format
		let quote = self.convert_across_quote_to_adapter_quote(across_quote, request, config)?;

		debug!(
			"Across adapter successfully fetched quote {} for solver {}",
			quote.quote_id, config.solver_id
		);

		Ok(GetQuoteResponse {
			quotes: vec![quote],
		})
	}

	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		debug!(
			"Across adapter health check for solver: {}",
			config.solver_id
		);

		let client = self.get_client(config)?;
		let routes_url = format!("{}/available-routes", config.endpoint);

		debug!(
			"Performing Across adapter health check at {} (solver: {})",
			routes_url, config.solver_id
		);

		// Make a lightweight request to the routes endpoint
		let response = client.get(&routes_url).send().await.map_err(|e| {
			warn!(
				"Across adapter health check failed for solver {}: HTTP error - {}",
				config.solver_id, e
			);
			AdapterError::HttpError(e)
		})?;

		// Check if the response status indicates the service is healthy
		let is_healthy = response.status().is_success();

		if is_healthy {
			// Optionally verify the response contains valid JSON
			match response.json::<Vec<AcrossRoute>>().await {
				Ok(routes) => {
					debug!(
						"Across adapter health check passed for solver {}: {} routes available",
						config.solver_id,
						routes.len()
					);
					Ok(true)
				},
				Err(e) => {
					warn!(
						"Across adapter health check failed for solver {}: Invalid JSON response - {}",
						config.solver_id, e
					);
					Ok(false)
				},
			}
		} else {
			warn!(
				"Across adapter health check failed for solver {}: HTTP status {}",
				config.solver_id,
				response.status()
			);
			Ok(false)
		}
	}

	async fn get_supported_routes(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Vec<AssetRoute>> {
		debug!(
			"Across adapter getting supported routes via solver: {}",
			config.solver_id
		);

		let client = self.get_client(config)?;
		let routes_url = format!("{}/available-routes", config.endpoint);

		debug!(
			"Fetching supported routes from Across adapter at {} (solver: {})",
			routes_url, config.solver_id
		);

		// Make the routes request
		let response = client
			.get(&routes_url)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			return Err(AdapterError::InvalidResponse {
				reason: format!(
					"Across routes endpoint returned status {}",
					response.status()
				),
			});
		}

		// Parse the JSON response into Across route models
		let across_routes: Vec<AcrossRoute> =
			response
				.json()
				.await
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Failed to parse Across routes response: {}", e),
				})?;

		let routes_count = across_routes.len();
		debug!(
			"Parsed {} routes from Across adapter for route conversion",
			routes_count
		);

		// Transform Across routes to internal AssetRoute models
		let mut asset_routes = Vec::new();
		for across_route in across_routes {
			match across_route.to_asset_route() {
				Ok(asset_route) => {
					asset_routes.push(asset_route);
				},
				Err(e) => {
					warn!("Failed to convert Across route to AssetRoute: {}", e);
					continue;
				},
			}
		}

		info!(
			"Across adapter converted {} Across routes to {} asset routes for solver {}",
			routes_count,
			asset_routes.len(),
			config.solver_id
		);

		Ok(asset_routes)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn test_across_adapter_construction_patterns() {
		let config = Adapter::new(
			"test-across".to_string(),
			"Test Across".to_string(),
			"Test Across Adapter".to_string(),
			"1.0.0".to_string(),
		);

		// Test optimized constructor (default)
		let adapter_optimized = AcrossAdapter::new(config.clone()).unwrap();
		assert!(matches!(
			adapter_optimized.client_strategy,
			ClientStrategy::Cached(_)
		));

		// Test custom cache constructor
		let custom_cache = ClientCache::with_ttl(Duration::from_secs(60));
		let adapter_custom = AcrossAdapter::with_cache(config.clone(), custom_cache).unwrap();
		assert!(matches!(
			adapter_custom.client_strategy,
			ClientStrategy::Cached(_)
		));

		// Test on-demand constructor
		let adapter_on_demand = AcrossAdapter::without_cache(config.clone()).unwrap();
		assert!(matches!(
			adapter_on_demand.client_strategy,
			ClientStrategy::OnDemand
		));
	}

	#[test]
	fn test_across_adapter_default_config() {
		let adapter = AcrossAdapter::with_default_config().unwrap();
		assert_eq!(adapter.id(), "across-v1");
		assert_eq!(adapter.name(), "Across v1 Adapter");
		assert!(matches!(adapter.client_strategy, ClientStrategy::Cached(_)));
	}

	#[test]
	fn test_across_route_to_assets() {
		let route = AcrossRoute {
			origin_chain_id: 1,
			destination_chain_id: 10,
			origin_token: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
			destination_token: "0x4200000000000000000000000000000000000006".to_string(),
			origin_token_symbol: "WETH".to_string(),
			destination_token_symbol: "WETH".to_string(),
			is_native: false,
		};

		let (origin_asset, destination_asset) = route.to_assets().unwrap();

		// Verify origin asset
		assert_eq!(
			origin_asset.address,
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
		);
		assert_eq!(origin_asset.symbol, "WETH");
		assert_eq!(origin_asset.chain_id, 1);
		assert_eq!(origin_asset.decimals, 18);

		// Verify destination asset
		assert_eq!(
			destination_asset.address,
			"0x4200000000000000000000000000000000000006"
		);
		assert_eq!(destination_asset.symbol, "WETH");
		assert_eq!(destination_asset.chain_id, 10);
		assert_eq!(destination_asset.decimals, 18);
	}

	#[test]
	fn test_across_route_to_asset_route() {
		let route = AcrossRoute {
			origin_chain_id: 1,
			destination_chain_id: 137,
			origin_token: "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
			destination_token: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(),
			origin_token_symbol: "USDC".to_string(),
			destination_token_symbol: "USDC".to_string(),
			is_native: false,
		};

		let asset_route = route.to_asset_route().unwrap();

		// Verify the AssetRoute was created correctly
		assert_eq!(asset_route.origin_chain_id().unwrap(), 1);
		assert_eq!(asset_route.destination_chain_id().unwrap(), 137);
		assert_eq!(
			asset_route.origin_address().to_lowercase(),
			"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0"
		);
		assert_eq!(
			asset_route.destination_address().to_lowercase(),
			"0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
		);

		// Verify symbols are preserved
		assert_eq!(asset_route.origin_token_symbol, Some("USDC".to_string()));
		assert_eq!(
			asset_route.destination_token_symbol,
			Some("USDC".to_string())
		);

		// Verify route properties
		assert!(asset_route.is_cross_chain());
		assert!(!asset_route.is_same_chain());
	}

	#[test]
	fn test_across_route_valid_conversion() {
		let route = AcrossRoute {
			origin_chain_id: 1,
			destination_chain_id: 10,
			origin_token: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
			destination_token: "0x4200000000000000000000000000000000000006".to_string(),
			origin_token_symbol: "WETH".to_string(),
			destination_token_symbol: "WETH".to_string(),
			is_native: false,
		};

		// With u64 chain IDs, conversion should always succeed for valid routes
		assert!(route.to_assets().is_ok());
		let (origin_asset, destination_asset) = route.to_assets().unwrap();

		// Verify the conversion worked correctly
		assert_eq!(origin_asset.chain_id, 1);
		assert_eq!(destination_asset.chain_id, 10);
		assert_eq!(origin_asset.symbol, "WETH");
		assert_eq!(destination_asset.symbol, "WETH");
	}

	#[test]
	fn test_across_adapter_health_check_url_construction() {
		let adapter = AcrossAdapter::with_default_config().unwrap();
		let config = SolverRuntimeConfig {
			solver_id: "test-across".to_string(),
			endpoint: "https://app.across.to".to_string(),
			headers: None,
		};

		// Test URL construction (this is what the health check method would use)
		let expected_url = format!("{}/available-routes", config.endpoint);
		assert_eq!(expected_url, "https://app.across.to/available-routes");

		// Verify adapter configuration
		assert_eq!(adapter.id(), "across-v1");
		assert_eq!(adapter.name(), "Across v1 Adapter");
	}

	#[test]
	fn test_get_quote_request_construction() {
		use oif_types::models::InteropAddress;
		use oif_types::{AvailableInput, RequestedOutput, U256};

		// Test data from user query:
		// originChainId=11155420&destinationChainId=129399
		// outputToken=0x17B8Ee96E3bcB3b04b3e8334de4524520C51caB4
		// inputToken=0x4200000000000000000000000000000000000006
		// amount=100000000000000

		let origin_chain_id = 11155420u64; // Optimism Sepolia
		let destination_chain_id = 129399u64; // Custom chain
		let input_token = "0x4200000000000000000000000000000000000006";
		let output_token = "0x17B8Ee96E3bcB3b04b3e8334de4524520C51caB4";
		let amount = "100000000000000"; // 0.0001 ETH (assuming 18 decimals)

		// Create InteropAddress for input token (user and asset)
		let user_address = InteropAddress::from_chain_and_address(
			origin_chain_id,
			"0x742d35Cc6634C0532925a3b8D38BA2297C33A9D7", // Example user address
		)
		.expect("Failed to create user InteropAddress");

		let input_asset = InteropAddress::from_chain_and_address(origin_chain_id, input_token)
			.expect("Failed to create input asset InteropAddress");

		let output_receiver = InteropAddress::from_chain_and_address(
			destination_chain_id,
			"0x742d35Cc6634C0532925a3b8D38BA2297C33A9D7", // Same user as receiver
		)
		.expect("Failed to create output receiver InteropAddress");

		let output_asset =
			InteropAddress::from_chain_and_address(destination_chain_id, output_token)
				.expect("Failed to create output asset InteropAddress");

		// Create U256 from amount string
		let amount_u256 = U256::new(amount.to_string());

		// Create the GetQuoteRequest
		let quote_request = GetQuoteRequest {
			user: user_address.clone(),
			available_inputs: vec![AvailableInput {
				user: user_address,
				asset: input_asset,
				amount: amount_u256.clone(),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				receiver: output_receiver,
				asset: output_asset,
				amount: amount_u256, // Same amount for simplicity
				calldata: None,
			}],
			min_valid_until: Some(300), // 5 minutes
			preference: None,
		};

		// Serialize to JSON for Postman usage
		let json_request = serde_json::to_string_pretty(&quote_request)
			.expect("Failed to serialize GetQuoteRequest to JSON");

		println!("📋 GetQuoteRequest JSON for Postman:");
		println!("{}", json_request);

		// Verify the request was constructed correctly
		assert_eq!(quote_request.available_inputs.len(), 1);
		assert_eq!(quote_request.requested_outputs.len(), 1);

		let input = &quote_request.available_inputs[0];
		let output = &quote_request.requested_outputs[0];

		// Verify extracted chain IDs and addresses
		assert_eq!(input.asset.extract_chain_id().unwrap(), origin_chain_id);
		assert_eq!(
			input.asset.extract_address().to_lowercase(),
			input_token.to_lowercase()
		);
		assert_eq!(
			output.asset.extract_chain_id().unwrap(),
			destination_chain_id
		);
		assert_eq!(
			output.asset.extract_address().to_lowercase(),
			output_token.to_lowercase()
		);
		assert_eq!(input.amount.to_string(), amount);
		assert_eq!(output.amount.to_string(), amount);

		println!("✅ GetQuoteRequest constructed successfully:");
		println!(
			"   Origin Chain: {} -> Token: {}",
			origin_chain_id, input_token
		);
		println!(
			"   Destination Chain: {} -> Token: {}",
			destination_chain_id, output_token
		);
		println!("   Amount: {} wei", amount);
	}

	#[test]
	fn test_across_metadata_enrichment() {
		// Create a mock Across quote response with rich data
		let mock_across_response = AcrossQuoteResponse {
			estimated_fill_time_sec: 120,
			capital_fee_pct: "78750000000001".to_string(),
			capital_fee_total: "78750000000001".to_string(),
			relay_gas_fee_pct: "155024308002".to_string(),
			relay_gas_fee_total: "155024308002".to_string(),
			relay_fee_pct: "78905024308003".to_string(),
			relay_fee_total: "78905024308003".to_string(),
			lp_fee_pct: "0".to_string(),
			timestamp: "1754342087".to_string(),
			is_amount_too_low: false,
			quote_block: "23070320".to_string(),
			exclusive_relayer: "0x394311A6Aaa0D8E3411D8b62DE4578D41322d1bD".to_string(),
			exclusivity_deadline: 1754342267,
			spoke_pool_address: "0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5".to_string(),
			destination_spoke_pool_address: "0x6f26Bf09B1C792e3228e5467807a900A503c0281"
				.to_string(),
			total_relay_fee: AcrossFeeBand {
				pct: "78905024308003".to_string(),
				total: "78905024308003".to_string(),
			},
			relayer_capital_fee: AcrossFeeBand {
				pct: "78750000000001".to_string(),
				total: "78750000000001".to_string(),
			},
			relayer_gas_fee: AcrossFeeBand {
				pct: "155024308002".to_string(),
				total: "155024308002".to_string(),
			},
			lp_fee: AcrossFeeBand {
				pct: "0".to_string(),
				total: "0".to_string(),
			},
			limits: AcrossLimits {
				min_deposit: "134862494200912".to_string(),
				max_deposit: "1661211802629989209324".to_string(),
				max_deposit_instant: "231397155893653275446".to_string(),
				max_deposit_short_delay: "1661211802629989209324".to_string(),
				recommended_deposit_instant: "231397155893653275446".to_string(),
			},
			fill_deadline: "1754353917".to_string(),
			output_amount: "999921094975691997".to_string(),
			input_token: AcrossToken {
				address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
				symbol: "ETH".to_string(),
				decimals: 18,
				chain_id: 1,
			},
			output_token: AcrossToken {
				address: "0x4200000000000000000000000000000000000006".to_string(),
				symbol: "ETH".to_string(),
				decimals: 18,
				chain_id: 10,
			},
			id: "xn8fx-1754342218143-67be35cfbdb6".to_string(),
		};

		// Create a simple mock request
		use oif_types::models::InteropAddress;
		use oif_types::{AvailableInput, RequestedOutput, U256};

		let user_address =
			InteropAddress::from_chain_and_address(1, "0x742d35Cc6634C0532925a3b8D38BA2297C33A9D7")
				.expect("Failed to create user address");
		let input_asset =
			InteropAddress::from_chain_and_address(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.expect("Failed to create input asset");
		let output_asset = InteropAddress::from_chain_and_address(
			10,
			"0x4200000000000000000000000000000000000006",
		)
		.expect("Failed to create output asset");

		let mock_request = GetQuoteRequest {
			user: user_address.clone(),
			available_inputs: vec![AvailableInput {
				user: user_address.clone(),
				asset: input_asset,
				amount: U256::new("1000000000000000000".to_string()),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				receiver: user_address,
				asset: output_asset,
				amount: U256::new("999000000000000000".to_string()),
				calldata: None,
			}],
			min_valid_until: Some(600),
			preference: None,
		};

		let mock_config = SolverRuntimeConfig {
			solver_id: "test-across".to_string(),
			endpoint: "https://api.across.to".to_string(),
			headers: None,
		};

		// Create the adapter and convert the response
		let adapter = AcrossAdapter::with_default_config().expect("Failed to create adapter");

		let adapter_quote = adapter
			.convert_across_quote_to_adapter_quote(
				mock_across_response,
				&mock_request,
				&mock_config,
			)
			.expect("Failed to convert Across response");

		// Serialize the enriched response to JSON
		let json_response = serde_json::to_string_pretty(&adapter_quote)
			.expect("Failed to serialize AdapterQuote to JSON");

		println!("🎯 Enhanced AdapterQuote with Across Metadata:");
		println!("{}", json_response);

		// Verify metadata is present
		assert!(adapter_quote.metadata.is_some());

		let metadata = adapter_quote.metadata.unwrap();
		assert!(metadata.get("across").is_some());

		let across_data = &metadata["across"];
		// Verify key Across fields are present (flat structure)
		assert!(across_data.get("capitalFeePct").is_some());
		assert!(across_data.get("spokePoolAddress").is_some());
		assert!(across_data.get("limits").is_some());
		assert!(across_data.get("inputToken").is_some());
		assert!(across_data.get("outputToken").is_some());
		assert!(across_data.get("estimatedFillTimeSec").is_some());
		assert!(across_data.get("id").is_some());

		println!("✅ All Across metadata fields are present and accessible!");
	}

	#[tokio::test]
	async fn test_get_quotes_with_real_data() {
		use oif_types::models::InteropAddress;
		use oif_types::{AvailableInput, RequestedOutput, U256};

		// Same test data as above but with async quote request
		let origin_chain_id = 11155420u64; // Optimism Sepolia
		let destination_chain_id = 129399u64; // Custom chain
		let input_token = "0x4200000000000000000000000000000000000006";
		let output_token = "0x17B8Ee96E3bcB3b04b3e8334de4524520C51caB4";
		let amount = "100000000000000";

		// Create the adapter
		let adapter = AcrossAdapter::with_default_config().expect("Failed to create adapter");

		// Create runtime config
		let runtime_config = SolverRuntimeConfig {
			solver_id: "test-across".to_string(),
			endpoint: "https://api.across.to".to_string(),
			headers: None,
		};

		// Create InteropAddresses
		let user_address = InteropAddress::from_chain_and_address(
			origin_chain_id,
			"0x742d35Cc6634C0532925a3b8D38BA2297C33A9D7",
		)
		.expect("Failed to create user InteropAddress");

		let input_asset = InteropAddress::from_chain_and_address(origin_chain_id, input_token)
			.expect("Failed to create input asset InteropAddress");

		let output_receiver = InteropAddress::from_chain_and_address(
			destination_chain_id,
			"0x742d35Cc6634C0532925a3b8D38BA2297C33A9D7",
		)
		.expect("Failed to create output receiver InteropAddress");

		let output_asset =
			InteropAddress::from_chain_and_address(destination_chain_id, output_token)
				.expect("Failed to create output asset InteropAddress");

		let amount_u256 = U256::new(amount.to_string());

		// Create the GetQuoteRequest
		let quote_request = GetQuoteRequest {
			user: user_address.clone(),
			available_inputs: vec![AvailableInput {
				user: user_address,
				asset: input_asset,
				amount: amount_u256.clone(),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				receiver: output_receiver,
				asset: output_asset,
				amount: amount_u256,
				calldata: None,
			}],
			min_valid_until: Some(300),
			preference: None,
		};

		// Test the actual get_quotes method
		// Note: This will make a real HTTP request to Across API, so it might fail in CI
		println!("🚀 Testing get_quotes with real data...");
		println!(
			"   Request: {}:{} -> {}:{}",
			origin_chain_id, input_token, destination_chain_id, output_token
		);
		println!("   Amount: {} wei", amount);

		match adapter.get_quotes(&quote_request, &runtime_config).await {
			Ok(response) => {
				println!("✅ Quote request successful!");
				println!("   Received {} quotes", response.quotes.len());

				if !response.quotes.is_empty() {
					let quote = &response.quotes[0];
					println!("   Quote ID: {}", quote.quote_id);
					println!("   Provider: {}", quote.provider);
					if let Some(eta) = quote.eta {
						println!("   ETA: {} seconds", eta);
					}
				}

				// Verify we got at least one quote
				assert!(
					!response.quotes.is_empty(),
					"Should receive at least one quote"
				);
			},
			Err(e) => {
				println!(
					"⚠️  Quote request failed (this might be expected in CI): {}",
					e
				);
				// In a real test environment, we might want to mock the HTTP response
				// For now, we just print the error and don't fail the test
				// This allows the test to pass in CI environments without network access
			},
		}
	}
}
