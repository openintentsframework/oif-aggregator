//! OIF v1 adapter implementation for HTTP-based solvers

use crate::adapters::{AdapterError, AdapterResult, SolverAdapter};
use crate::models::adapters::AdapterConfig;
use crate::models::{Intent, Quote, QuoteRequest};
use crate::models::{IntentExecutionDetail, IntentResponse, IntentStatus};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// OIF v1 adapter for HTTP-based solvers
#[derive(Debug)]
pub struct OifAdapter {
    config: AdapterConfig,
    client: Client,
    endpoint: String,
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

/// OIF v1 intent request format
#[derive(Debug, Serialize)]
struct OifIntentRequest {
    pub quote_id: Option<String>,
    pub user_address: String,
    pub slippage_tolerance: f64,
    pub deadline: i64,
    pub signature: Option<String>,
}

/// OIF v1 intent response format
#[derive(Debug, Deserialize)]
struct OifIntentResponse {
    pub transaction_hash: Option<String>,
    pub status: String,
    pub message: Option<String>,
}

impl OifAdapter {
    /// Create a new OIF adapter
    pub fn new(config: AdapterConfig, endpoint: String, timeout_ms: u64) -> AdapterResult<Self> {
        let mut headers = HeaderMap::new();

        // Add default headers
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("User-Agent", HeaderValue::from_static("OIF-Aggregator/1.0"));

        // Add custom headers from configuration
        if let Some(custom_headers) = &config.configuration.get("headers") {
            if let Some(header_map) = custom_headers.as_object() {
                for (key, value) in header_map {
                    if let Some(value_str) = value.as_str() {
                        if let (Ok(header_name), Ok(header_value)) =
                            (HeaderName::try_from(key), HeaderValue::try_from(value_str))
                        {
                            headers.insert(header_name, header_value);
                        }
                    }
                }
            }
        }

        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .default_headers(headers)
            .build()
            .map_err(AdapterError::HttpError)?;

        Ok(Self {
            config,
            client,
            endpoint,
        })
    }

    /// Create from adapter configuration
    pub fn from_config(config: &AdapterConfig) -> AdapterResult<Self> {
        let endpoint = config
            .configuration
            .get("endpoint")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdapterError::ConfigError("Missing endpoint in adapter configuration".to_string())
            })?;

        let timeout_ms = config
            .configuration
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(5000); // Default 5 second timeout

        Self::new(config.clone(), endpoint.to_string(), timeout_ms)
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
        response_time_ms: u64,
    ) -> Quote {
        // Clone the raw response first before moving fields
        let raw_response = serde_json::to_value(&oif_response).unwrap_or(serde_json::Value::Null);

        // Create Quote directly using the new model structure
        let mut quote = Quote::new(
            self.config.adapter_id.clone(), // solver_id
            request.request_id.clone(),     // request_id
            request.token_in.clone(),       // token_in
            request.token_out.clone(),      // token_out
            request.amount_in.clone(),      // amount_in
            oif_response.amount_out,        // amount_out
            request.chain_id,               // chain_id
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
    fn adapter_info(&self) -> &AdapterConfig {
        &self.config
    }

    async fn get_quote(&self, request: QuoteRequest) -> AdapterResult<Quote> {
        let start_time = std::time::Instant::now();

        debug!(
            "Getting quote from OIF adapter {} for request {}",
            self.config.adapter_id, request.request_id
        );

        let oif_request = self.to_oif_quote_request(&request);
        let quote_url = format!("{}/v1/quote", self.endpoint);

        info!("Requesting quote from {}", quote_url);

        let response = self
            .client
            .post(&quote_url)
            .json(&oif_request)
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
            AdapterError::InvalidResponse(format!("JSON parsing failed: {}", e))
        })?;

        let quote = self.from_oif_quote_response(oif_response, &request, response_time_ms);

        info!(
            "Successfully got quote {} in {}ms",
            quote.quote_id, response_time_ms
        );

        Ok(quote)
    }

    async fn submit_intent(&self, intent: Intent) -> AdapterResult<IntentResponse> {
        debug!(
            "Submitting intent {} to OIF adapter {}",
            intent.intent_id, self.config.adapter_id
        );

        let oif_request = OifIntentRequest {
            quote_id: intent.quote_id.clone(),
            user_address: intent.user_address.clone(),
            slippage_tolerance: intent.slippage_tolerance,
            deadline: intent.deadline.timestamp(),
            signature: intent.signature.clone(),
        };

        let intent_url = format!("{}/v1/intent", self.endpoint);

        info!("Submitting intent to {}", intent_url);

        let response = self
            .client
            .post(&intent_url)
            .json(&oif_request)
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed to {}: {}", intent_url, e);
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

        let oif_response: OifIntentResponse = response.json().await.map_err(|e| {
            error!("Failed to parse OIF intent response: {}", e);
            AdapterError::InvalidResponse(format!("JSON parsing failed: {}", e))
        })?;

        let status = match oif_response.status.as_str() {
            "pending" => IntentStatus::Pending,
            "submitted" => IntentStatus::Submitted,
            "executing" => IntentStatus::Executing,
            "success" => IntentStatus::Success,
            "failed" => IntentStatus::Failed,
            "cancelled" => IntentStatus::Cancelled,
            _ => IntentStatus::Failed,
        };

        let intent_response = IntentResponse {
            intent_id: intent.intent_id,
            status,
            transaction_hash: oif_response.transaction_hash,
            block_number: None,
            gas_used: None,
            effective_gas_price: None,
            result: IntentExecutionDetail {
                amount_in: None,
                amount_out: None,
                actual_price_impact: None,
                gas_cost: None,
                execution_time_ms: None,
                error_message: oif_response.message,
                solver_used: Some(self.config.adapter_id.clone()),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        info!(
            "Successfully submitted intent {}",
            intent_response.intent_id
        );

        Ok(intent_response)
    }

    async fn health_check(&self) -> AdapterResult<bool> {
        let health_url = format!("{}/health", self.endpoint);

        debug!("Health checking OIF adapter at {}", health_url);

        match self.client.get(&health_url).send().await {
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
            }
            Err(e) => {
                warn!(
                    "OIF adapter {} health check failed: {}",
                    self.config.adapter_id, e
                );
                Ok(false)
            }
        }
    }

    fn supported_chains(&self) -> &[u64] {
        &self.config.supported_chains
    }
}

/// LiFi specific adapter implementation  
#[derive(Debug)]
pub struct LifiAdapter {
    config: AdapterConfig,
    client: Client,
    endpoint: String,
}

impl LifiAdapter {
    pub fn from_config(config: &AdapterConfig) -> AdapterResult<Self> {
        let endpoint = config
            .configuration
            .get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("https://li.quest/v1");

        let timeout_ms = config
            .configuration
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(5000);

        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(AdapterError::HttpError)?;

        Ok(Self {
            config: config.clone(),
            client,
            endpoint: endpoint.to_string(),
        })
    }
}

#[async_trait]
impl SolverAdapter for LifiAdapter {
    fn adapter_info(&self) -> &AdapterConfig {
        &self.config
    }

    async fn get_quote(&self, _request: QuoteRequest) -> AdapterResult<Quote> {
        // TODO: Implement LiFi-specific quote logic
        Err(AdapterError::ConfigError(
            "LiFi adapter not fully implemented".to_string(),
        ))
    }

    async fn submit_intent(&self, _intent: Intent) -> AdapterResult<IntentResponse> {
        // TODO: Implement LiFi-specific intent logic
        Err(AdapterError::ConfigError(
            "LiFi adapter not fully implemented".to_string(),
        ))
    }

    async fn health_check(&self) -> AdapterResult<bool> {
        let health_url = format!("{}/healthcheck", self.endpoint);

        match self.client.get(&health_url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    fn supported_chains(&self) -> &[u64] {
        &self.config.supported_chains
    }
}
