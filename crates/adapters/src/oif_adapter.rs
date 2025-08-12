//! OIF v1 adapter implementation for HTTP-based solvers

use async_trait::async_trait;
use oif_types::{
	AdapterConfig, Asset, Network, Order, OrderDetails, OrderStatus, Quote, QuoteRequest,
	SolverRuntimeConfig,
};
use oif_types::{AdapterError, AdapterResult, SolverAdapter};
use reqwest::{
	header::{HeaderMap, HeaderName, HeaderValue},
	Client,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// OIF v1 adapter for HTTP-based solvers
/// This adapter is stateless and receives solver configuration at runtime
#[derive(Debug)]
pub struct OifAdapter {
	config: AdapterConfig,
	client: Client,
}

/// OIF v1 quote request format
#[derive(Debug, Serialize)]
struct OifQuoteRequest {
	pub token_in: String,
	pub token_out: String,
	pub amount_in: String,
	pub chain_id: u64,
	pub slippage_tolerance: Option<f64>,
	pub recipient: Option<String>,
}

/// OIF v1 quote response format
#[derive(Debug, Serialize, Deserialize)]
struct OifQuoteResponse {
	pub amount_out: String,
	pub estimated_gas: Option<u64>,
	pub price_impact: Option<f64>,
	pub route: Option<serde_json::Value>,
	pub fees: Option<serde_json::Value>,
}

/// OIF v1 order request format
#[derive(Debug, Serialize)]
struct OifOrderRequest {
	pub quote_id: Option<String>,
	pub user_address: String,
	pub signature: Option<String>,
}

/// OIF v1 order response format
#[derive(Debug, Deserialize)]
struct OifOrderResponse {
	pub transaction_hash: Option<String>,
	pub status: String,
	pub message: Option<String>,
}

impl OifAdapter {
	/// Create a new OIF adapter (stateless, no hardcoded endpoint/timeout)
	pub fn new(config: AdapterConfig) -> AdapterResult<Self> {
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
	pub fn default() -> AdapterResult<Self> {
		let config = AdapterConfig::new(
			"oif-v1".to_string(),
			"OIF v1 Protocol".to_string(),
			"OIF v1 Adapter".to_string(),
			"1.0.0".to_string(),
		);

		Self::new(config)
	}

	/// Convert our QuoteRequest to OIF format
	fn to_oif_quote_request(&self, request: &QuoteRequest) -> OifQuoteRequest {
		OifQuoteRequest {
			token_in: request.token_in.clone(),
			token_out: request.token_out.clone(),
			amount_in: request.amount_in.clone(),
			chain_id: request.chain_id,
			slippage_tolerance: request.slippage_tolerance,
			recipient: request.recipient.clone(),
		}
	}

	/// Convert OIF response to our Quote format
	fn from_oif_quote_response(
		&self,
		oif_response: OifQuoteResponse,
		request: &QuoteRequest,
		config: &SolverRuntimeConfig,
		response_time_ms: u64,
	) -> Quote {
		// Clone the raw response first before moving fields
		let raw_response = serde_json::to_value(&oif_response).unwrap_or(serde_json::Value::Null);

		// Create Quote directly using the new model structure
		let mut quote = Quote::new(
			config.solver_id.clone(),   // solver_id (from runtime config)
			request.request_id.clone(), // request_id
			request.token_in.clone(),   // token_in
			request.token_out.clone(),  // token_out
			request.amount_in.clone(),  // amount_in
			oif_response.amount_out,    // amount_out
			request.chain_id,           // chain_id
		);

		// Set additional optional fields using builder pattern
		if let Some(gas) = oif_response.estimated_gas {
			quote = quote.with_estimated_gas(gas);
		}

		if let Some(impact) = oif_response.price_impact {
			quote = quote.with_price_impact(impact);
		}

		// Set response time and confidence
		quote = quote
			.with_response_time(response_time_ms)
			.with_confidence_score(0.8); // Default confidence for OIF adapters

		// Set raw response for debugging
		quote.raw_response = raw_response;

		quote
	}
}

#[async_trait]
impl SolverAdapter for OifAdapter {
	fn adapter_id(&self) -> &str {
		"oif-v1"
	}

	fn adapter_name(&self) -> &str {
		"OIF v1 Protocol"
	}

	fn adapter_info(&self) -> &AdapterConfig {
		&self.config
	}

	async fn get_quote(
		&self,
		request: QuoteRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Quote> {
		let start_time = std::time::Instant::now();

		debug!(
			"Getting quote from OIF adapter {} for solver {} (request {})",
			self.config.adapter_id, config.solver_id, request.request_id
		);

		let oif_request = self.to_oif_quote_request(&request);
		let quote_url = format!("{}/v1/quote", config.endpoint);

		info!(
			"Requesting quote from {} (solver: {})",
			quote_url, config.solver_id
		);

		let response = self
			.client
			.post(&quote_url)
			.json(&oif_request)
			.timeout(Duration::from_millis(config.timeout_ms))
			.send()
			.await
			.map_err(|e| {
				error!("HTTP request failed to {}: {}", quote_url, e);
				AdapterError::HttpError(e)
			})?;

		let response_time_ms = start_time.elapsed().as_millis() as u64;

		if !response.status().is_success() {
			let status = response.status();
			let error_text = response.text().await.unwrap_or_default();
			error!("OIF adapter returned error {}: {}", status, error_text);
			return Err(AdapterError::SolverError {
				code: status.to_string(),
				message: error_text,
			});
		}

		let oif_response: OifQuoteResponse = response.json().await.map_err(|e| {
			error!("Failed to parse OIF response: {}", e);
			AdapterError::InvalidResponse {
				reason: format!("JSON parsing failed: {}", e),
			}
		})?;

		let quote = self.from_oif_quote_response(oif_response, &request, config, response_time_ms);

		info!(
			"Successfully got quote {} in {}ms",
			quote.quote_id, response_time_ms
		);

		Ok(quote)
	}

	async fn submit_order(
		&self,
		order: &Order,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<String> {
		debug!(
			"Submitting order {} to OIF adapter {} via solver {}",
			order.order_id, self.config.adapter_id, config.solver_id
		);

		let oif_request = OifOrderRequest {
			quote_id: order.quote_id.clone(),
			user_address: order.user_address.clone(),
			signature: order.signature.clone(),
		};

		let order_url = format!("{}/v1/order", config.endpoint);

		info!(
			"Submitting order to {} (solver: {})",
			order_url, config.solver_id
		);

		let response = self
			.client
			.post(&order_url)
			.json(&oif_request)
			.timeout(Duration::from_millis(config.timeout_ms))
			.send()
			.await
			.map_err(|e| {
				error!("HTTP request failed to {}: {}", order_url, e);
				AdapterError::HttpError(e)
			})?;

		if !response.status().is_success() {
			let status = response.status();
			let error_text = response.text().await.unwrap_or_default();
			error!("OIF adapter returned error {}: {}", status, error_text);
			return Err(AdapterError::SolverError {
				code: status.to_string(),
				message: error_text,
			});
		}

		let oif_response: OifOrderResponse = response.json().await.map_err(|e| {
			error!("Failed to parse OIF order response: {}", e);
			AdapterError::InvalidResponse {
				reason: format!("JSON parsing failed: {}", e),
			}
		})?;

		let _status = match oif_response.status.as_str() {
			"pending" => OrderStatus::Pending,
			"submitted" => OrderStatus::Submitted,
			"executing" => OrderStatus::Executing,
			"success" => OrderStatus::Success,
			"failed" => OrderStatus::Failed,
			"cancelled" => OrderStatus::Cancelled,
			_ => OrderStatus::Failed,
		};

		info!("Successfully submitted order {}", order.order_id);

		Ok(order.order_id.clone())
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
	) -> AdapterResult<OrderDetails> {
		let order_url = format!("{}/v1/order/{}", config.endpoint, order_id);

		debug!(
			"Getting order details for {} from {} (solver: {})",
			order_id, order_url, config.solver_id
		);

		let response = self
			.client
			.get(&order_url)
			.timeout(Duration::from_millis(config.timeout_ms))
			.send()
			.await
			.map_err(|e| {
				error!("HTTP request failed to {}: {}", order_url, e);
				AdapterError::HttpError(e)
			})?;

		if !response.status().is_success() {
			let status = response.status();
			let error_text = response.text().await.unwrap_or_default();
			error!("OIF adapter returned error {}: {}", status, error_text);
			return Err(AdapterError::SolverError {
				code: status.to_string(),
				message: error_text,
			});
		}

		let order_response: serde_json::Value = response.json().await.map_err(|e| {
			error!("Failed to parse OIF order details response: {}", e);
			AdapterError::InvalidResponse {
				reason: format!("JSON parsing failed: {}", e),
			}
		})?;

		// Parse the response into OrderDetails
		let mut order_details = OrderDetails::new(
			order_id.to_string(),
			order_response
				.get("status")
				.and_then(|v| v.as_str())
				.unwrap_or("unknown")
				.to_string(),
		);

		// Extract additional fields from response
		if let Some(tx_hash) = order_response
			.get("transaction_hash")
			.and_then(|v| v.as_str())
		{
			order_details.transaction_hash = Some(tx_hash.to_string());
		}

		if let Some(gas_used) = order_response.get("gas_used").and_then(|v| v.as_u64()) {
			order_details.gas_used = Some(gas_used);
		}

		if let Some(gas_price) = order_response.get("gas_price").and_then(|v| v.as_str()) {
			order_details.gas_price = Some(gas_price.to_string());
		}

		if let Some(fee) = order_response
			.get("transaction_fee")
			.and_then(|v| v.as_str())
		{
			order_details.transaction_fee = Some(fee.to_string());
		}

		if let Some(block_num) = order_response.get("block_number").and_then(|v| v.as_u64()) {
			order_details.block_number = Some(block_num);
		}

		// Store any additional metadata
		if let Some(metadata) = order_response.get("metadata").and_then(|v| v.as_object()) {
			for (key, value) in metadata {
				order_details.metadata.insert(key.clone(), value.clone());
			}
		}

		info!("Successfully retrieved order details for {}", order_id);
		Ok(order_details)
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
