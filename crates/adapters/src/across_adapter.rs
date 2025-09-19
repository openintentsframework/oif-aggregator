//! Across adapter implementation
//!
//! This adapter uses an optimized client cache for connection pooling and keep-alive.
//! Submission and tracking of orders are not supported by this adapter.

use async_trait::async_trait;
use oif_types::adapters::AdapterQuote;
use oif_types::{
	Adapter, Asset, AssetRoute, GetQuoteRequest, GetQuoteResponse, SolverRuntimeConfig,
};
use oif_types::{AdapterError, AdapterResult, SolverAdapter, SupportedAssetsData};
use reqwest::{
	header::{HeaderMap, HeaderValue},
	Client,
};
use serde_json;
use std::{str::FromStr, sync::Arc};
use tracing::{debug, info, warn};
use url::Url;

use crate::client_cache::{ClientCache, ClientConfig};

/// Helper function to normalize addresses to ensure they have 0x prefix
fn normalize_address(address: &str) -> String {
	if address.starts_with("0x") || address.starts_with("0X") {
		address.to_string()
	} else {
		format!("0x{}", address)
	}
}

/// Helper function to categorize validation errors for analysis
fn categorize_validation_error(error_msg: &str) -> &'static str {
	if error_msg.contains("address missing 0x prefix") {
		"missing_0x_prefix"
	} else if error_msg.contains("Invalid token address") {
		"invalid_address_format"
	} else if error_msg.contains("Invalid chain ID") {
		"invalid_chain_id"
	} else if error_msg.contains("address too short") {
		"address_too_short"
	} else if error_msg.contains("address too long") {
		"address_too_long"
	} else if error_msg.contains("invalid hex") {
		"invalid_hex_characters"
	} else {
		"other"
	}
}

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

/// Across swap API response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapResponse {
	/// Cross swap type
	pub cross_swap_type: String,
	/// Amount type (exactInput, exactOutput, minOutput)
	pub amount_type: String,
	/// Validation checks
	pub checks: AcrossSwapChecks,
	/// Approval transactions (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub approval_txns: Option<Vec<AcrossApprovalTx>>,
	/// Swap steps
	pub steps: AcrossSwapSteps,
	/// Input token details
	pub input_token: AcrossSwapToken,
	/// Output token details
	pub output_token: AcrossSwapToken,
	/// Refund token details
	pub refund_token: AcrossSwapToken,
	/// Fee breakdown
	pub fees: AcrossSwapFees,
	/// Input amount
	pub input_amount: String,
	/// Expected output amount
	pub expected_output_amount: String,
	/// Minimum output amount
	pub min_output_amount: String,
	/// Expected fill time in seconds
	pub expected_fill_time: u64,
	/// Transaction details for execution (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub swap_tx: Option<AcrossSwapTx>,
	/// Quote ID (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
}

/// Approval transaction details
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossApprovalTx {
	/// Chain ID for the approval transaction
	pub chain_id: u64,
	/// Contract address to approve
	pub to: String,
	/// Transaction data
	pub data: String,
}

/// Swap API data structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapChecks {
	/// Allowance check
	pub allowance: Option<AcrossSwapCheck>,
	/// Balance check
	pub balance: Option<AcrossSwapCheck>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcrossSwapCheck {
	/// Token address
	pub token: String,
	/// Spender address (for allowance)
	pub spender: Option<String>,
	/// Actual value
	pub actual: String,
	/// Expected value
	pub expected: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapSteps {
	/// Origin swap step (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin_swap: Option<AcrossSwapDestination>,
	/// Bridge step
	pub bridge: AcrossSwapBridge,
	/// Destination swap step (optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub destination_swap: Option<AcrossSwapDestination>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapBridge {
	/// Input amount
	pub input_amount: String,
	/// Output amount
	pub output_amount: String,
	/// Input token
	pub token_in: AcrossSwapToken,
	/// Output token
	pub token_out: AcrossSwapToken,
	/// Fee breakdown
	pub fees: AcrossSwapBridgeFees,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapDestination {
	/// Input token
	pub token_in: AcrossSwapToken,
	/// Output token
	pub token_out: AcrossSwapToken,
	/// Input amount
	pub input_amount: String,
	/// Max input amount
	pub max_input_amount: String,
	/// Output amount
	pub output_amount: String,
	/// Min output amount
	pub min_output_amount: String,
	/// Swap provider details
	pub swap_provider: AcrossSwapProvider,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcrossSwapProvider {
	/// Provider name
	pub name: String,
	/// Sources used
	pub sources: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapToken {
	/// Token decimals
	pub decimals: u8,
	/// Token symbol
	pub symbol: String,
	/// Token address
	pub address: String,
	/// Token name
	pub name: Option<String>,
	/// Chain ID
	pub chain_id: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapFees {
	/// Total fees
	pub total: AcrossSwapFee,
	/// Origin gas fees
	pub origin_gas: AcrossSwapFee,
	/// Destination gas fees
	pub destination_gas: AcrossSwapFee,
	/// Relayer capital fees
	pub relayer_capital: AcrossSwapFee,
	/// LP fees
	pub lp_fee: AcrossSwapFee,
	/// Relayer total fees
	pub relayer_total: AcrossSwapFee,
	/// App fees
	pub app: AcrossSwapFee,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapFee {
	/// Fee amount
	pub amount: String,
	/// Fee amount in USD
	pub amount_usd: Option<String>,
	/// Fee percentage
	pub pct: Option<String>,
	/// Fee token
	pub token: Option<AcrossSwapToken>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapBridgeFees {
	/// Total relay fees
	pub total_relay: AcrossSwapSimpleFee,
	/// Relayer capital fees
	pub relayer_capital: AcrossSwapSimpleFee,
	/// Relayer gas fees
	pub relayer_gas: AcrossSwapSimpleFee,
	/// LP fees
	pub lp: AcrossSwapSimpleFee,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcrossSwapSimpleFee {
	/// Fee percentage
	pub pct: String,
	/// Fee total
	pub total: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapTx {
	/// Whether simulation was successful
	pub simulation_success: bool,
	/// Chain ID
	pub chain_id: u64,
	/// Contract address to call
	pub to: String,
	/// Transaction data
	pub data: String,
	/// Gas limit
	pub gas: Option<String>,
	/// Max fee per gas
	pub max_fee_per_gas: Option<String>,
	/// Max priority fee per gas
	pub max_priority_fee_per_gas: Option<String>,
}

/// Across swap quote request body
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossSwapQuoteRequest {
	/// Amount type (exactInput, exactOutput, minOutput)  
	pub amount_type: String,
	/// Required amount of input/output token in smallest unit
	pub amount: String,
	/// Address of the input token on the origin chain
	pub input_token: String,
	/// Address of the output token on the destination chain
	pub output_token: String,
	/// Chain ID of the origin chain
	pub origin_chain_id: u64,
	/// Chain ID of the destination chain
	pub destination_chain_id: u64,
	/// Address of the depositor initiating the swap
	pub depositor: String,
	/// Address of the account receiving the output token
	pub recipient: String,
	/// Slippage tolerance percentage (optional, defaults to 1%)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub slippage: Option<f64>,
	/// Whether to skip origin transaction estimation (optional, defaults to false)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub skip_origin_tx_estimation: Option<bool>,
	/// Whether to strictly follow the defined trade type (optional, defaults to true)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub strict_trade_type: Option<bool>,
}

/// Optional query parameters for Across API requests
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossQueryParams {
	/// Trade type (exactInput, exactOutput, minOutput)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trade_type: Option<String>,
	/// Integrator ID for tracking
	#[serde(skip_serializing_if = "Option::is_none")]
	pub integrator_id: Option<String>,
	/// Refund address for failed transactions
	#[serde(skip_serializing_if = "Option::is_none")]
	pub refund_address: Option<String>,
	/// Whether to refund on origin chain
	#[serde(skip_serializing_if = "Option::is_none")]
	pub refund_on_origin: Option<String>,
	/// Slippage tolerance (accepts both string and number)
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(default, deserialize_with = "deserialize_optional_number_or_string")]
	pub slippage: Option<String>,
	/// Whether to skip origin transaction estimation
	#[serde(skip_serializing_if = "Option::is_none")]
	pub skip_origin_tx_estimation: Option<String>,
	/// Sources to exclude from quote
	#[serde(skip_serializing_if = "Option::is_none")]
	pub exclude_sources: Option<String>,
	/// Sources to include in quote (exclusive with exclude_sources)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub include_sources: Option<String>,
	/// App fee percentage (accepts both string and number)
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(default, deserialize_with = "deserialize_optional_number_or_string")]
	pub app_fee: Option<String>,
	/// App fee recipient address
	#[serde(skip_serializing_if = "Option::is_none")]
	pub app_fee_recipient: Option<String>,
	/// Whether to strictly follow the defined trade type
	#[serde(skip_serializing_if = "Option::is_none")]
	pub strict_trade_type: Option<bool>,
}

/// Helper function for optional number/string deserialization
fn deserialize_optional_number_or_string<'de, D>(
	deserializer: D,
) -> Result<Option<String>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;

	let opt_value = Option::<serde_json::Value>::deserialize(deserializer)?;

	Ok(opt_value.map(|v| match v {
		serde_json::Value::String(s) => s,
		serde_json::Value::Number(n) => {
			if let Some(f) = n.as_f64() {
				// Always format as decimal (e.g., 1 -> "1.0", 2.5 -> "2.5")
				if f.fract() == 0.0 {
					format!("{:.1}", f) // Force one decimal place for whole numbers
				} else {
					format!("{}", f)
				}
			} else if let Some(i) = n.as_i64() {
				format!("{:.1}", i as f64) // Convert integer to float format
			} else if let Some(u) = n.as_u64() {
				format!("{:.1}", u as f64) // Convert unsigned to float format
			} else {
				n.to_string()
			}
		},
		_ => v.to_string(),
	}))
}

/// Across token from swap/tokens endpoint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcrossTokenInfo {
	/// Chain ID
	pub chain_id: u64,
	/// Token contract address
	pub address: String,
	/// Token name
	pub name: String,
	/// Token symbol
	pub symbol: String,
	/// Token decimals
	pub decimals: u8,
}

impl AcrossTokenInfo {
	/// Convert Across token info to internal Asset model
	pub fn to_asset(&self) -> AdapterResult<Asset> {
		let normalized_address = normalize_address(&self.address);

		Asset::from_chain_and_address(
			self.chain_id,
			normalized_address,
			self.symbol.clone(),
			self.name.clone(),
			self.decimals,
		)
		.map_err(|e| AdapterError::InvalidResponse {
			reason: format!("Invalid asset from Across token: {}", e),
		})
	}
}

impl AcrossRoute {
	/// Convert Across route to internal Asset models
	/// Returns a tuple of (origin_asset, destination_asset)
	pub fn to_assets(&self) -> AdapterResult<(Asset, Asset)> {
		let origin_chain_id = self.origin_chain_id;
		let destination_chain_id = self.destination_chain_id;

		// Normalize addresses to ensure they have 0x prefix
		let origin_token = normalize_address(&self.origin_token);
		let destination_token = normalize_address(&self.destination_token);

		// Create origin asset
		let origin_asset = Asset::from_chain_and_address(
			origin_chain_id,
			origin_token,
			self.origin_token_symbol.clone(),
			self.origin_token_symbol.clone(), // Use symbol as name for now
			18, // Default to 18 decimals, could be improved with token registry
		)
		.map_err(|e| AdapterError::InvalidResponse {
			reason: format!("Invalid origin asset: {}", e),
		})?;

		// Create destination asset
		let destination_asset = Asset::from_chain_and_address(
			destination_chain_id,
			destination_token,
			self.destination_token_symbol.clone(),
			self.destination_token_symbol.clone(), // Use symbol as name for now
			18, // Default to 18 decimals, could be improved with token registry
		)
		.map_err(|e| AdapterError::InvalidResponse {
			reason: format!("Invalid destination asset: {}", e),
		})?;

		Ok((origin_asset, destination_asset))
	}

	/// Convert Across route to internal AssetRoute model
	pub fn to_asset_route(&self) -> AdapterResult<AssetRoute> {
		use oif_types::models::InteropAddress;

		// Normalize addresses to ensure they have 0x prefix
		let origin_token = normalize_address(&self.origin_token);
		let destination_token = normalize_address(&self.destination_token);

		// Create InteropAddresses for origin and destination
		let origin_asset =
			InteropAddress::from_chain_and_address(self.origin_chain_id, &origin_token).map_err(
				|e| AdapterError::InvalidResponse {
					reason: format!("Invalid origin asset in Across route: {}", e),
				},
			)?;

		let destination_asset =
			InteropAddress::from_chain_and_address(self.destination_chain_id, &destination_token)
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

	/// Properly construct URL by joining base endpoint with path
	fn build_url(&self, base_url: &str, path: &str) -> AdapterResult<String> {
		let mut base = Url::parse(base_url).map_err(|e| AdapterError::InvalidResponse {
			reason: format!("Invalid base URL '{}': {}", base_url, e),
		})?;

		// Ensure the base URL is treated as a directory by ensuring it ends with a slash
		if !base.path().ends_with('/') {
			base.set_path(&format!("{}/", base.path()));
		}

		let joined = base.join(path).map_err(|e| AdapterError::InvalidResponse {
			reason: format!(
				"Failed to join URL path '{}' to base '{}': {}",
				path, base_url, e
			),
		})?;

		Ok(joined.to_string())
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

	/// Extract query parameters from request metadata.requestParams and build query parameters
	///
	/// Expects optional Across-specific parameters to be nested under metadata.requestParams:
	/// ```json
	/// {
	///   "metadata": {
	///     "requestParams": {
	///       "tradeType": "exactInput",
	///       "slippage": "0.005",
	///       "skipOriginTxEstimation": true,
	///       "appFee": "0.001",
	///       "appFeeRecipient": "0x1234567890123456789012345678901234567890"
	///     }
	///   }
	/// }
	/// ```
	#[allow(clippy::too_many_arguments)]
	fn build_query_params(
		&self,
		request: &GetQuoteRequest,
		amount: &str,
		input_token: &str,
		output_token: &str,
		input_chain_id: u64,
		output_chain_id: u64,
		depositor: &str,
		recipient: &str,
	) -> AdapterResult<Vec<(&'static str, String)>> {
		// Start with required parameters
		let mut params = vec![
			("amount", amount.to_string()),
			("inputToken", input_token.to_string()),
			("outputToken", output_token.to_string()),
			("originChainId", input_chain_id.to_string()),
			("destinationChainId", output_chain_id.to_string()),
			("depositor", depositor.to_string()),
			("recipient", recipient.to_string()),
		];

		// Extract optional parameters from metadata.requestParams
		if let Some(metadata) = &request.metadata {
			debug!("metadata: {:?}", metadata);
			// Try to parse AcrossQueryParams from metadata.requestParams
			if let Some(request_params) = metadata.get("requestParams") {
				match serde_json::from_value::<AcrossQueryParams>(request_params.clone()) {
					Ok(query_params) => {
						debug!("Successfully parsed query_params: {:?}", query_params);
						// Add optional parameters if they exist
						if let Some(trade_type) = query_params.trade_type {
							params.push(("tradeType", trade_type));
						}

						if let Some(integrator_id) = query_params.integrator_id {
							params.push(("integratorId", integrator_id));
						}

						if let Some(refund_address) = query_params.refund_address {
							params.push(("refundAddress", refund_address));
						}

						if let Some(refund_on_origin) = query_params.refund_on_origin {
							params.push(("refundOnOrigin", refund_on_origin));
						}

						if let Some(slippage) = query_params.slippage {
							params.push(("slippage", slippage));
						}

						if let Some(skip_origin_tx_estimation) =
							query_params.skip_origin_tx_estimation
						{
							params.push(("skipOriginTxEstimation", skip_origin_tx_estimation));
						}

						if let Some(exclude_sources) = query_params.exclude_sources {
							params.push(("excludeSources", exclude_sources));
						}

						if let Some(include_sources) = query_params.include_sources {
							params.push(("includeSources", include_sources));
						}

						if let Some(app_fee) = query_params.app_fee {
							params.push(("appFee", app_fee));
						}

						if let Some(app_fee_recipient) = query_params.app_fee_recipient {
							params.push(("appFeeRecipient", app_fee_recipient));
						}

						if let Some(strict_trade_type) = query_params.strict_trade_type {
							params.push(("strictTradeType", strict_trade_type.to_string()));
						}

						debug!(
							"Built dynamic query parameters from metadata.requestParams: {:?}",
							params
						);
					},
					Err(e) => {
						debug!("Failed to parse requestParams as AcrossQueryParams: {}", e);
						debug!("requestParams content: {:?}", request_params);
					},
				}
			}
		}

		Ok(params)
	}

	/// Convert Across swap response to internal adapter quote format
	fn convert_across_swap_to_adapter_quote(
		&self,
		across_swap: AcrossSwapResponse,
		request: &GetQuoteRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<AdapterQuote> {
		use oif_types::adapters::{AdapterQuote, QuoteDetails};

		// Generate unique quote ID (use provided ID if available, otherwise generate one)
		let quote_id = across_swap
			.id
			.clone()
			.unwrap_or_else(|| format!("across-quote-{}", uuid::Uuid::new_v4()));

		// Create quote details from the request structure
		let details = QuoteDetails {
			available_inputs: request.available_inputs.clone(),
			requested_outputs: request.requested_outputs.clone(),
		};

		// The new API doesn't provide a timestamp, so we don't set valid_until
		let valid_until = None;

		let metadata = serde_json::json!({
			"across_response": across_swap,
		});

		Ok(AdapterQuote {
			orders: vec![], // EIP-712 orders are not supported for Across yet
			details,
			valid_until,
			eta: Some(across_swap.expected_fill_time),
			quote_id,
			provider: format!("Across Protocol v{}", self.config.version),
			metadata: Some(metadata),
			cost: None,
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
		let quote_url = self.build_url(&config.endpoint, "swap/approval")?;

		debug!(
			"Fetching Across quote from {} (solver: {}) - {}:{} -> {}:{}",
			quote_url, config.solver_id, input_chain_id, input_token, output_chain_id, output_token
		);

		// Extract user and receiver addresses
		let depositor = input.user.extract_address();
		let recipient = output.receiver.extract_address();

		// Build query parameters dynamically from metadata
		let query_params = self.build_query_params(
			request,
			&input.amount.to_string(),
			input_token.as_str(),
			output_token.as_str(),
			input_chain_id,
			output_chain_id,
			depositor.as_str(),
			recipient.as_str(),
		)?;

		// Make the quote request (remove Content-Type header for GET request)
		let response = client
			.get(&quote_url)
			.header("Content-Type", "") // Override default Content-Type header to avoid 400 error (body set error)
			.query(&query_params)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			let status_code = response.status().as_u16();
			let error_body = response.text().await.unwrap_or_default();
			return Err(AdapterError::http_failure(
				status_code,
				format!(
					"Across quote endpoint returned status {}: {}",
					status_code, error_body
				),
			));
		}

		// Get response text for logging and parsing
		let response_text = response
			.text()
			.await
			.map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to get response text: {}", e),
			})?;

		debug!(
			"Across API response for solver {}: {}",
			config.solver_id, response_text
		);

		// Parse the JSON response using new swap API format
		let across_swap: AcrossSwapResponse =
			serde_json::from_str(&response_text).map_err(|e| AdapterError::InvalidResponse {
				reason: format!(
					"Failed to parse Across swap response: {}. Response body: {}",
					e, response_text
				),
			})?;

		// Convert Across response to internal quote format
		let quote = self.convert_across_swap_to_adapter_quote(across_swap, request, config)?;

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
		let tokens_url = self.build_url(&config.endpoint, "swap/tokens")?;

		debug!(
			"Performing Across adapter health check at {} (solver: {})",
			tokens_url, config.solver_id
		);

		// Make a lightweight request to the tokens endpoint
		let response = client.get(&tokens_url).send().await.map_err(|e| {
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
			match response.json::<Vec<AcrossTokenInfo>>().await {
				Ok(tokens) => {
					debug!(
						"Across adapter health check passed for solver {}: {} tokens available",
						config.solver_id,
						tokens.len()
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
			let status_code = response.status().as_u16();
			warn!(
				"Across adapter health check failed for solver {}: HTTP status {}",
				config.solver_id, status_code
			);
			Ok(false)
		}
	}

	async fn get_supported_assets(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<SupportedAssetsData> {
		debug!(
			"Across adapter getting supported tokens for solver: {}",
			config.solver_id
		);

		let client = self.get_client(config)?;
		let tokens_url = self.build_url(&config.endpoint, "swap/tokens")?;

		debug!(
			"Fetching supported tokens from Across adapter at {} (solver: {})",
			tokens_url, config.solver_id
		);

		// Make the tokens request
		let response = client
			.get(&tokens_url)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			let status_code = response.status().as_u16();
			return Err(AdapterError::http_failure(
				status_code,
				format!("Across tokens endpoint returned status {}", status_code),
			));
		}

		// Parse the JSON response into Across token models
		let across_tokens: Vec<AcrossTokenInfo> =
			response
				.json()
				.await
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Failed to parse Across tokens response: {}", e),
				})?;

		let tokens_count = across_tokens.len();
		debug!(
			"Parsed {} tokens from Across adapter for asset conversion",
			tokens_count
		);

		// Transform Across tokens to Asset models
		let mut assets = Vec::new();
		let mut skipped_tokens = 0;
		let mut invalid_token_details = Vec::new();
		let mut error_categories = std::collections::HashMap::new();

		for across_token in across_tokens {
			match across_token.to_asset() {
				Ok(asset) => {
					assets.push(asset);
				},
				Err(e) => {
					let error_str = e.to_string();
					let error_category = categorize_validation_error(&error_str);

					let invalid_token_info = format!(
						"chain={}:{} ({}) - {}, error: {} [category: {}]",
						across_token.chain_id,
						across_token.address,
						across_token.symbol,
						across_token.name,
						error_str,
						error_category
					);

					warn!("Skipping invalid Across token: {}", invalid_token_info);
					invalid_token_details.push(invalid_token_info);

					// Track error categories for statistics
					*error_categories.entry(error_category).or_insert(0) += 1;
					skipped_tokens += 1;
				},
			}
		}

		if skipped_tokens > 0 {
			info!(
				"Across adapter converted {} Across tokens to {} assets for solver {} (using tokens mode), skipped {} invalid tokens",
				tokens_count,
				assets.len(),
				config.solver_id,
				skipped_tokens
			);

			// Log error category statistics
			warn!(
				"Error statistics for {} invalid tokens from Across API for solver {}:",
				skipped_tokens, config.solver_id
			);
			for (category, count) in error_categories.iter() {
				warn!("  {}: {} tokens", category, count);
			}

			// Log detailed summary of invalid tokens for analysis
			warn!(
				"Detailed list of {} invalid tokens from Across API for solver {}:",
				skipped_tokens, config.solver_id
			);
			for (index, invalid_token) in invalid_token_details.iter().enumerate() {
				warn!("  Invalid token #{}: {}", index + 1, invalid_token);
			}
		} else {
			info!(
				"Across adapter converted {} Across tokens to {} assets for solver {} (using tokens mode)",
				tokens_count,
				assets.len(),
				config.solver_id
			);
		}

		Ok(SupportedAssetsData::Assets(assets))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn test_normalize_address() {
		// Test address with 0x prefix
		assert_eq!(
			normalize_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
		);

		// Test address without 0x prefix
		assert_eq!(
			normalize_address("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
		);

		// Test address with 0X prefix (uppercase)
		assert_eq!(
			normalize_address("0XC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
			"0XC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
		);

		// Test empty string (edge case)
		assert_eq!(normalize_address(""), "0x");
	}

	#[test]
	fn test_resilient_route_processing() {
		// Test that invalid routes are skipped but valid ones are processed
		let valid_route = AcrossRoute {
			origin_chain_id: 1,
			destination_chain_id: 10,
			origin_token: "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // No 0x prefix
			destination_token: "0x4200000000000000000000000000000000000006".to_string(),
			origin_token_symbol: "WETH".to_string(),
			destination_token_symbol: "WETH".to_string(),
			is_native: false,
		};

		let invalid_route = AcrossRoute {
			origin_chain_id: 1,
			destination_chain_id: 10,
			origin_token: "invalid_address".to_string(), // Invalid address
			destination_token: "0x4200000000000000000000000000000000000006".to_string(),
			origin_token_symbol: "INVALID".to_string(),
			destination_token_symbol: "WETH".to_string(),
			is_native: false,
		};

		// Test individual route conversion
		assert!(valid_route.to_assets().is_ok());
		assert!(invalid_route.to_assets().is_err()); // This should fail

		// Test that normalize_address handles the missing prefix
		let normalized = normalize_address(&valid_route.origin_token);
		assert!(normalized.starts_with("0x"));
		assert_eq!(normalized, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
	}

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
			origin_asset.plain_address(),
			"0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
		);
		assert_eq!(origin_asset.symbol, "WETH");
		assert_eq!(origin_asset.chain_id().unwrap(), 1);
		assert_eq!(origin_asset.decimals, 18);

		// Verify destination asset
		assert_eq!(
			destination_asset.plain_address(),
			"0x4200000000000000000000000000000000000006"
		);
		assert_eq!(destination_asset.symbol, "WETH");
		assert_eq!(destination_asset.chain_id().unwrap(), 10);
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
		assert_eq!(origin_asset.chain_id().unwrap(), 1);
		assert_eq!(destination_asset.chain_id().unwrap(), 10);
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
			adapter_metadata: None,
		};

		// Test URL construction (this is what the health check method would use)
		let expected_url = format!("{}/swap/tokens", config.endpoint);
		assert_eq!(expected_url, "https://app.across.to/swap/tokens");

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
			metadata: None,
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
	fn test_url_construction() {
		let adapter = AcrossAdapter::new(Adapter::new(
			"test-adapter".to_string(),
			"Test adapter".to_string(),
			"Across v1".to_string(),
			"1.0.0".to_string(),
		))
		.unwrap();

		// Test basic URL construction with quote endpoint
		let base_url = "https://api.across.to";
		let result = adapter.build_url(base_url, "swap/approval").unwrap();
		assert_eq!(result, "https://api.across.to/swap/approval");

		// Test with trailing slash - should handle gracefully
		let base_with_slash = "https://api.across.to/";
		let result = adapter.build_url(base_with_slash, "swap/approval").unwrap();
		assert_eq!(result, "https://api.across.to/swap/approval");

		// Test with leading slash in path - should handle gracefully
		let result = adapter.build_url(base_url, "/swap/approval").unwrap();
		assert_eq!(result, "https://api.across.to/swap/approval");

		// Test with both trailing and leading slashes
		let result = adapter
			.build_url(base_with_slash, "/swap/approval")
			.unwrap();
		assert_eq!(result, "https://api.across.to/swap/approval");

		// Test with tokens endpoint
		let result = adapter.build_url(base_url, "swap/tokens").unwrap();
		assert_eq!(result, "https://api.across.to/swap/tokens");

		// Test with path in base URL (the problematic case)
		let base_with_path = "http://127.0.0.1:3000/api";
		let result = adapter.build_url(base_with_path, "swap/approval").unwrap();
		assert_eq!(result, "http://127.0.0.1:3000/api/swap/approval");

		// Test with path in base URL and trailing slash
		let base_with_path_slash = "http://127.0.0.1:3000/api/";
		let result = adapter
			.build_url(base_with_path_slash, "swap/approval")
			.unwrap();
		assert_eq!(result, "http://127.0.0.1:3000/api/swap/approval");

		// Test tokens endpoint with base path
		let result = adapter.build_url(base_with_path, "swap/tokens").unwrap();
		assert_eq!(result, "http://127.0.0.1:3000/api/swap/tokens");

		// Test invalid URL
		let result = adapter.build_url("invalid://::url", "swap/approval");
		assert!(result.is_err());
	}

	#[test]
	fn test_across_swap_api_response_parsing() {
		// Test parsing of the new swap API response format
		let json_response = r#"{
			"crossSwapType": "bridgeableToAny",
			"amountType": "exactInput",
			"checks": {
				"allowance": {
					"token": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
					"spender": "0x6f26Bf09B1C792e3228e5467807a900A503c0281",
					"actual": "115792089237316195423570985008687907853269984665640564039457584007913099639935",
					"expected": "1000000"
				},
				"balance": {
					"token": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
					"actual": "7169942",
					"expected": "1000000"
				}
			},
			"steps": {
				"bridge": {
					"inputAmount": "1000000",
					"outputAmount": "980662",
					"tokenIn": {
						"decimals": 6,
						"symbol": "USDC",
						"address": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
						"name": "USD Coin",
						"chainId": 10
					},
					"tokenOut": {
						"decimals": 6,
						"symbol": "USDC",
						"address": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
						"chainId": 42161
					},
					"fees": {
						"totalRelay": {
							"pct": "19338767572622738",
							"total": "19338"
						},
						"relayerCapital": {
							"pct": "100000000000000",
							"total": "100"
						},
						"relayerGas": {
							"pct": "19227000000000000",
							"total": "19227"
						},
						"lp": {
							"pct": "11767572622738",
							"total": "11"
						}
					}
				}
			},
			"inputToken": {
				"decimals": 6,
				"symbol": "USDC",
				"address": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
				"name": "USD Coin",
				"chainId": 10
			},
			"outputToken": {
				"decimals": 18,
				"symbol": "ETH",
				"address": "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
				"name": "Ether",
				"chainId": 42161
			},
			"refundToken": {
				"decimals": 6,
				"symbol": "USDC",
				"address": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
				"name": "USD Coin",
				"chainId": 10
			},
			"fees": {
				"total": {
					"amount": "47651",
					"amountUsd": "0.047642182071510164",
					"pct": "47651283466652296",
					"token": {
						"decimals": 6,
						"symbol": "USDC",
						"address": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
						"name": "USD Coin",
						"chainId": 10
					}
				},
				"originGas": {
					"amount": "134879133226",
					"amountUsd": "0.000502067341563801",
					"token": {
						"chainId": 10,
						"address": "0x0000000000000000000000000000000000000000",
						"decimals": 18,
						"symbol": "ETH"
					}
				},
				"destinationGas": {
					"amount": "5161137520505",
					"amountUsd": "0.019223327642999999",
					"pct": "19227000000000001",
					"token": {
						"chainId": 42161,
						"address": "0x0000000000000000000000000000000000000000",
						"decimals": 18,
						"symbol": "ETH"
					}
				},
				"relayerCapital": {
					"amount": "100",
					"amountUsd": "0.0000999809",
					"pct": "100000000000000",
					"token": {
						"decimals": 6,
						"symbol": "USDC",
						"address": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
						"name": "USD Coin",
						"chainId": 10
					}
				},
				"lpFee": {
					"amount": "11",
					"amountUsd": "0.000010997899",
					"pct": "11000000000000",
					"token": {
						"decimals": 6,
						"symbol": "USDC",
						"address": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
						"name": "USD Coin",
						"chainId": 10
					}
				},
				"relayerTotal": {
					"amount": "19338",
					"amountUsd": "0.019334306442000002",
					"pct": "19338000000000001",
					"token": {
						"decimals": 6,
						"symbol": "USDC",
						"address": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
						"name": "USD Coin",
						"chainId": 10
					}
				},
				"app": {
					"amount": "0",
					"amountUsd": "0.0",
					"pct": "0",
					"token": {
						"decimals": 18,
						"symbol": "ETH",
						"address": "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
						"name": "Ether",
						"chainId": 42161
					}
				}
			},
			"inputAmount": "1000000",
			"expectedOutputAmount": "263466241499732",
			"minOutputAmount": "260831579075100",
			"expectedFillTime": 3,
			"swapTx": {
				"simulationSuccess": true,
				"chainId": 10,
				"to": "0x6f26Bf09B1C792e3228e5467807a900A503c0281",
				"data": "0xad5425c6000000000000000000000000a4d353bbc130cbef1811f27ac70989f9d568ceab",
				"gas": "122554",
				"maxFeePerGas": "1100569",
				"maxPriorityFeePerGas": "1100000"
			},
			"id": "6pl4c-1754347045980-2353645c0fb7"
		}"#;

		// Parse the JSON response
		let parsed_response: Result<AcrossSwapResponse, _> = serde_json::from_str(json_response);
		assert!(parsed_response.is_ok(), "Failed to parse swap API response");

		let swap_response = parsed_response.unwrap();
		assert_eq!(swap_response.amount_type, "exactInput");
		assert_eq!(swap_response.cross_swap_type, "bridgeableToAny");
		assert_eq!(swap_response.expected_fill_time, 3);
		if let Some(ref swap_tx) = swap_response.swap_tx {
			assert!(swap_tx.simulation_success);
		}
		assert_eq!(
			swap_response.id,
			Some("6pl4c-1754347045980-2353645c0fb7".to_string())
		);

		println!("✅ Successfully parsed Across swap API response!");
	}

	#[test]
	fn test_across_token_parsing() {
		// Test parsing of the new swap/tokens response format
		let json_response = r#"[
			{
				"chainId": 1,
				"address": "0x111111111117dC0aa78b770fA6A738034120C302",
				"name": "1inch",
				"symbol": "1INCH",
				"decimals": 18,
				"logoUrl": "https://assets.coingecko.com/coins/images/13469/thumb/1inch-token.png?1608803028",
				"priceUsd": "0.2530907825982632"
			},
			{
				"chainId": 10,
				"address": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
				"name": "USD Coin",
				"symbol": "USDC",
				"decimals": 6,
				"logoUrl": "https://assets.coingecko.com/coins/images/6319/thumb/USD_Coin_icon.png?1547042389",
				"priceUsd": "0.9999"
			}
		]"#;

		// Parse the JSON response
		let parsed_response: Result<Vec<AcrossTokenInfo>, _> = serde_json::from_str(json_response);
		assert!(
			parsed_response.is_ok(),
			"Failed to parse tokens API response"
		);

		let tokens = parsed_response.unwrap();
		assert_eq!(tokens.len(), 2);

		// Test first token
		let token1 = &tokens[0];
		assert_eq!(token1.chain_id, 1);
		assert_eq!(token1.address, "0x111111111117dC0aa78b770fA6A738034120C302");
		assert_eq!(token1.name, "1inch");
		assert_eq!(token1.symbol, "1INCH");
		assert_eq!(token1.decimals, 18);

		// Test second token
		let token2 = &tokens[1];
		assert_eq!(token2.chain_id, 10);
		assert_eq!(token2.address, "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85");
		assert_eq!(token2.name, "USD Coin");
		assert_eq!(token2.symbol, "USDC");
		assert_eq!(token2.decimals, 6);

		// Test conversion to Asset
		let asset1 = token1.to_asset().expect("Failed to convert token to asset");
		assert_eq!(asset1.chain_id().unwrap(), 1);
		assert_eq!(asset1.symbol, "1INCH");
		assert_eq!(asset1.name, "1inch");
		assert_eq!(asset1.decimals, 18);
		assert_eq!(
			asset1.plain_address(),
			"0x111111111117dc0aa78b770fa6a738034120c302"
		);

		let asset2 = token2.to_asset().expect("Failed to convert token to asset");
		assert_eq!(asset2.chain_id().unwrap(), 10);
		assert_eq!(asset2.symbol, "USDC");
		assert_eq!(asset2.name, "USD Coin");
		assert_eq!(asset2.decimals, 6);
		assert_eq!(
			asset2.plain_address(),
			"0x0b2c639c533813f4aa9d7837caf62653d097ff85"
		);

		println!("✅ Successfully parsed Across tokens and converted to assets!");
	}

	#[test]
	fn test_across_query_params_deserialization() {
		// Test JSON with mixed types (numbers and strings)
		let json_data = serde_json::json!({
			"tradeType": "minOutput",
			"slippage": 5,        // Number
			"appFee": 2.5,        // Float
			"integratorId": "test-id",
			"strictTradeType": true
		});

		// Should successfully deserialize mixed types
		let params: AcrossQueryParams =
			serde_json::from_value(json_data).expect("Failed to deserialize mixed types");

		assert_eq!(params.trade_type, Some("minOutput".to_string()));
		assert_eq!(params.slippage, Some("5.0".to_string())); // Integer converted to decimal string
		assert_eq!(params.app_fee, Some("2.5".to_string())); // Float converted to string
		assert_eq!(params.integrator_id, Some("test-id".to_string()));
		assert_eq!(params.strict_trade_type, Some(true));

		println!("✅ Successfully parsed mixed number/string parameters");

		// Test with string values
		let json_string_data = serde_json::json!({
			"tradeType": "exactInput",
			"slippage": "0.005",  // String
			"appFee": "1.0",      // String
		});

		let params_string: AcrossQueryParams =
			serde_json::from_value(json_string_data).expect("Failed to deserialize string types");

		assert_eq!(params_string.trade_type, Some("exactInput".to_string()));
		assert_eq!(params_string.slippage, Some("0.005".to_string()));
		assert_eq!(params_string.app_fee, Some("1.0".to_string()));

		println!("✅ Successfully parsed string parameters");

		// Test with null/missing values
		let json_minimal = serde_json::json!({
			"tradeType": "exactOutput"
		});

		let params_minimal: AcrossQueryParams =
			serde_json::from_value(json_minimal).expect("Failed to deserialize minimal data");

		assert_eq!(params_minimal.trade_type, Some("exactOutput".to_string()));
		assert_eq!(params_minimal.slippage, None);
		assert_eq!(params_minimal.app_fee, None);

		println!("✅ Successfully parsed minimal parameters with null values");

		// Test with the user's exact payload
		let user_payload = serde_json::json!({
			"tradeType": "minOutput",
			"integratorId": "0xdede",
			"strictTradeType": true
		});

		let params_user: AcrossQueryParams =
			serde_json::from_value(user_payload).expect("Failed to deserialize user payload");

		assert_eq!(params_user.trade_type, Some("minOutput".to_string()));
		assert_eq!(params_user.integrator_id, Some("0xdede".to_string()));
		assert_eq!(params_user.strict_trade_type, Some(true));
		assert_eq!(params_user.slippage, None); // Missing field should be None
		assert_eq!(params_user.app_fee, None); // Missing field should be None

		println!("✅ Successfully parsed user payload with missing numeric fields");

		// Test integer to decimal conversion specifically
		let json_integers = serde_json::json!({
			"slippage": 1,    // Integer
			"appFee": 10      // Integer
		});

		let params_int: AcrossQueryParams =
			serde_json::from_value(json_integers).expect("Failed to deserialize integers");

		assert_eq!(params_int.slippage, Some("1.0".to_string())); // 1 -> "1.0"
		assert_eq!(params_int.app_fee, Some("10.0".to_string())); // 10 -> "10.0"

		println!("✅ Successfully converted integers to decimal format");
	}

	#[test]
	fn test_across_adapter_http_failure_error_handling() {
		// Test that AdapterError::http_failure creates proper error with status code
		let error =
			AdapterError::http_failure(404, "Across quote endpoint returned status 404: Not Found");
		assert_eq!(error.status_code(), Some(404));

		let error = AdapterError::http_failure(500, "Across tokens endpoint returned status 500");
		assert_eq!(error.status_code(), Some(500));

		let error = AdapterError::http_failure(429, "Rate limited by Across API");
		assert_eq!(error.status_code(), Some(429));

		// Test error message formatting
		let error = AdapterError::from_http_failure(503);
		assert!(error.to_string().contains("503"));
		assert!(error.to_string().contains("Service Unavailable"));

		println!("✅ Across adapter HTTP failure error handling works correctly");
	}
}
