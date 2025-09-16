//! OIF v1 adapter implementation for HTTP-based solvers
//!
//! This adapter uses an optimized client cache for connection pooling and keep-alive.

use reqwest::{
	header::{HeaderMap, HeaderValue},
	Client,
};
use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use oif_types::adapters::models::{SubmitOrderRequest, SubmitOrderResponse};
use oif_types::adapters::GetOrderResponse;
use oif_types::{
	Adapter, Asset, GetQuoteRequest, GetQuoteResponse, SecretString, SolverRuntimeConfig,
};
use oif_types::{AdapterError, AdapterResult, SolverAdapter, SupportedAssetsData};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::client_cache::ClientCache;

/// JWT token expiry buffer in minutes - tokens are considered invalid this many minutes before actual expiry
/// This prevents race conditions where tokens expire between validation and actual use
pub const JWT_TOKEN_EXPIRY_BUFFER_MINUTES: i64 = 5;

/// Request structure for OIF order submission API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OifOrderRequest {
	/// Quote ID from the original quote request
	pub quote_id: String,
	/// User's signature for authorization
	pub signature: String,
	/// Order data (optional, from metadata)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub order: Option<String>,
	/// User's wallet address (optional, from metadata)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sponsor: Option<String>,
}

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

/// JWT authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthConfig {
	/// Whether authentication is enabled
	auth_enabled: Option<bool>,
	/// Client name for registration (defaults to "OIF Aggregator - {solver_id}")
	client_name: Option<String>,
	/// Requested scopes (defaults to ["read-orders", "create-orders"])
	scopes: Option<Vec<String>>,
	/// Token expiry in hours (defaults to 24)
	expiry_hours: Option<u32>,
}

/// JWT register request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtRegisterRequest {
	/// Client identifier (e.g., application name, user email)
	pub client_id: String,
	/// Optional client name for display purposes
	pub client_name: Option<String>,
	/// Requested scopes (if not provided, defaults to basic read permissions)
	pub scopes: Option<Vec<String>>,
}

/// Register response from OIF auth endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisterResponse {
	/// The generated access token
	pub access_token: String,
	/// The generated refresh token
	pub refresh_token: String,
	/// Client identifier
	pub client_id: String,
	/// Access token expiry time in Unix timestamp
	pub access_token_expires_at: i64,
	/// Refresh token expiry time in Unix timestamp
	pub refresh_token_expires_at: i64,
	/// Granted scopes
	pub scopes: Vec<String>,
	/// Token type (always "Bearer")
	pub token_type: String,
}

/// Refresh token request
#[derive(Debug, Serialize, Deserialize)]
struct RefreshTokenRequest {
	/// The refresh token
	pub refresh_token: String,
}

/// Refresh token response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshTokenResponse {
	/// The new access token
	pub access_token: String,
	/// The new refresh token (optional, may be same as original)
	pub refresh_token: Option<String>,
	/// Access token expiry time in Unix timestamp
	pub access_token_expires_at: i64,
	/// Refresh token expiry time in Unix timestamp (optional)
	pub refresh_token_expires_at: Option<i64>,
	/// Token type (always "Bearer")
	pub token_type: String,
	/// Scopes
	pub scopes: Vec<String>,
}

/// Cached JWT token information
#[derive(Debug, Clone)]
struct JwtTokenInfo {
	/// The access token
	access_token: SecretString,
	/// The refresh token
	refresh_token: SecretString,
	/// When the access token expires
	access_token_expires_at: Option<DateTime<Utc>>,
	/// When the refresh token expires
	refresh_token_expires_at: Option<DateTime<Utc>>,
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
	/// JWT token cache per solver
	jwt_tokens: Arc<DashMap<String, JwtTokenInfo>>,
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
			jwt_tokens: Arc::new(DashMap::new()),
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
			jwt_tokens: Arc::new(DashMap::new()),
		})
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

	/// Parse authentication configuration from adapter metadata
	fn parse_auth_config(&self, config: &SolverRuntimeConfig) -> Option<AuthConfig> {
		config.adapter_metadata.as_ref().and_then(|metadata| {
			// Look for auth config under the "auth" key within metadata
			metadata
				.get("auth")
				.and_then(|auth_value| serde_json::from_value(auth_value.clone()).ok())
		})
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

	/// Register with OIF auth endpoint to get JWT token
	async fn register_jwt(&self, config: &SolverRuntimeConfig) -> AdapterResult<JwtTokenInfo> {
		let auth_config = self.parse_auth_config(config);
		let register_url = self.build_url(&config.endpoint, "auth/register")?;

		// Create a basic HTTP client for auth requests (no cached headers to avoid circular dependency)
		let client = Client::new();

		debug!(
			"Registering with OIF auth endpoint {} for solver {}",
			register_url, config.solver_id
		);

		// Create register request using solver_id as client_id and configurable options
		let default_client_name = format!("OIF Aggregator - {}", config.solver_id);
		let default_scopes = vec!["read-orders".to_string(), "create-orders".to_string()];

		let register_request = JwtRegisterRequest {
			client_id: config.solver_id.clone(),
			client_name: Some(
				auth_config
					.as_ref()
					.and_then(|c| c.client_name.clone())
					.unwrap_or(default_client_name),
			),
			scopes: Some(
				auth_config
					.as_ref()
					.and_then(|c| c.scopes.clone())
					.unwrap_or(default_scopes),
			),
		};

		let response = client
			.post(&register_url)
			.json(&register_request)
			.send()
			.await
			.map_err(AdapterError::HttpError)?;

		if !response.status().is_success() {
			return Err(AdapterError::InvalidResponse {
				reason: format!(
					"OIF auth register endpoint returned status {}",
					response.status()
				),
			});
		}

		let body = response.text().await.unwrap_or_default();

		let register_response: RegisterResponse =
			serde_json::from_str(&body).map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to parse OIF auth register response: {}", e),
			})?;

		// Log non-sensitive parts of the response for debugging
		debug!(
			"OIF auth register successful for client_id: {}, access_token_expires_at: {}, refresh_token_expires_at: {}, scopes: {:?}, token_type: {}",
			register_response.client_id,
			register_response.access_token_expires_at,
			register_response.refresh_token_expires_at,
			register_response.scopes,
			register_response.token_type
		);

		// Convert Unix timestamps to DateTime
		let access_token_expires_at = if register_response.access_token_expires_at > 0 {
			Some(
				DateTime::<Utc>::from_timestamp(register_response.access_token_expires_at, 0)
					.ok_or_else(|| AdapterError::InvalidResponse {
						reason: format!(
							"Invalid access_token_expires_at timestamp: {}",
							register_response.access_token_expires_at
						),
					})?,
			)
		} else {
			None
		};

		let refresh_token_expires_at = if register_response.refresh_token_expires_at > 0 {
			Some(
				DateTime::<Utc>::from_timestamp(register_response.refresh_token_expires_at, 0)
					.ok_or_else(|| AdapterError::InvalidResponse {
						reason: format!(
							"Invalid refresh_token_expires_at timestamp: {}",
							register_response.refresh_token_expires_at
						),
					})?,
			)
		} else {
			None
		};

		let token_info = JwtTokenInfo {
			access_token: SecretString::new(register_response.access_token),
			refresh_token: SecretString::new(register_response.refresh_token),
			access_token_expires_at,
			refresh_token_expires_at,
		};

		// Cache the token
		self.jwt_tokens
			.insert(config.solver_id.clone(), token_info.clone());

		info!(
			"Successfully registered JWT tokens for solver {} (access expires: {:?}, refresh expires: {:?}, scopes: {:?})",
			config.solver_id, access_token_expires_at, refresh_token_expires_at, register_response.scopes
		);

		Ok(token_info)
	}

	/// Refresh JWT tokens using refresh token
	async fn refresh_jwt(
		&self,
		config: &SolverRuntimeConfig,
		token_info: &JwtTokenInfo,
	) -> AdapterResult<JwtTokenInfo> {
		let refresh_url = self.build_url(&config.endpoint, "auth/refresh")?;

		// Create a basic HTTP client for auth requests
		let client = Client::new();

		info!(
			"Starting token refresh for solver {} at endpoint {}",
			config.solver_id, refresh_url
		);

		let refresh_request = RefreshTokenRequest {
			refresh_token: token_info.refresh_token.expose_secret().to_string(),
		};

		let response = client
			.post(&refresh_url)
			.json(&refresh_request)
			.send()
			.await
			.map_err(|e| {
				error!(
					"HTTP error during token refresh for solver {}: {}",
					config.solver_id, e
				);
				AdapterError::HttpError(e)
			})?;

		let status = response.status();
		debug!("Token refresh HTTP response status: {}", status);

		if !status.is_success() {
			let error_body = response.text().await.unwrap_or_default();
			error!(
				"OIF auth refresh endpoint returned status {} for solver {}: {}",
				status, config.solver_id, error_body
			);
			return Err(AdapterError::InvalidResponse {
				reason: format!(
					"OIF auth refresh endpoint returned status {}: {}",
					status, error_body
				),
			});
		}

		let body = response.text().await.unwrap_or_default();

		let refresh_response: RefreshTokenResponse =
			serde_json::from_str(&body).map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to parse OIF auth refresh response: {}", e),
			})?;

		// Log non-sensitive parts of the response for debugging
		debug!(
			"OIF auth refresh successful for solver {}: access_token_expires_at: {}, token_type: {}, scopes: {:?}",
			config.solver_id,
			refresh_response.access_token_expires_at,
			refresh_response.token_type,
			refresh_response.scopes
		);

		// Convert Unix timestamp to DateTime
		let access_token_expires_at = if refresh_response.access_token_expires_at > 0 {
			let parsed_datetime =
				DateTime::<Utc>::from_timestamp(refresh_response.access_token_expires_at, 0)
					.ok_or_else(|| AdapterError::InvalidResponse {
						reason: format!(
							"Invalid access_token_expires_at timestamp: {}",
							refresh_response.access_token_expires_at
						),
					})?;
			Some(parsed_datetime)
		} else {
			None
		};

		let refresh_token_expires_at =
			if let Some(expires_at) = refresh_response.refresh_token_expires_at {
				if expires_at > 0 {
					Some(
						DateTime::<Utc>::from_timestamp(expires_at, 0).ok_or_else(|| {
							AdapterError::InvalidResponse {
								reason: format!(
									"Invalid refresh_token_expires_at timestamp: {}",
									expires_at
								),
							}
						})?,
					)
				} else {
					None
				}
			} else {
				// If not provided, keep the original refresh token expiry
				token_info.refresh_token_expires_at
			};

		let new_token_info = JwtTokenInfo {
			access_token: SecretString::new(refresh_response.access_token),
			refresh_token: if let Some(new_refresh_token) = refresh_response.refresh_token {
				SecretString::new(new_refresh_token)
			} else {
				// Keep the original refresh token if none provided
				token_info.refresh_token.clone()
			},
			access_token_expires_at,
			refresh_token_expires_at,
		};

		// Cache the new token
		self.jwt_tokens
			.insert(config.solver_id.clone(), new_token_info.clone());

		info!(
			"Successfully refreshed JWT tokens for solver {} (access expires: {:?}, refresh expires: {:?})",
			config.solver_id, access_token_expires_at, refresh_token_expires_at
		);

		Ok(new_token_info)
	}

	/// Get or refresh JWT token for a solver
	async fn get_jwt_token(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Option<SecretString>> {
		// Check if auth is enabled
		if let Some(auth_config) = self.parse_auth_config(config) {
			if auth_config.auth_enabled.unwrap_or(false) {
				// Check if we have tokens cached (clone to avoid holding lock during refresh)
				let cached_token_info = self
					.jwt_tokens
					.get(&config.solver_id)
					.map(|token| token.clone());
				if let Some(token_info) = cached_token_info {
					let now = Utc::now();

					// Check if access token is still valid
					let access_token_valid =
						token_info
							.access_token_expires_at
							.map_or(true, |expires_at| {
								let is_valid = now
									< expires_at
										- Duration::minutes(JWT_TOKEN_EXPIRY_BUFFER_MINUTES);
								debug!(
								"Access token for solver {}: expires_at={:?}, now={:?}, buffer={}min, valid={}",
								config.solver_id, expires_at, now, JWT_TOKEN_EXPIRY_BUFFER_MINUTES, is_valid
							);
								is_valid
							});

					if access_token_valid {
						// Access token is still valid, return it
						return Ok(Some(token_info.access_token.clone()));
					} else {
						debug!(
							"Access token expired for solver {}, will attempt refresh",
							config.solver_id
						);
					}

					// Access token expired, check if refresh token is still valid
					let refresh_token_valid =
						token_info
							.refresh_token_expires_at
							.map_or(true, |expires_at| {
								now < expires_at
									- Duration::minutes(JWT_TOKEN_EXPIRY_BUFFER_MINUTES)
							});

					if refresh_token_valid {
						// Try to refresh the access token
						info!(
							"Access token expired for solver {}, attempting to refresh using refresh token",
							config.solver_id
						);
						match self.refresh_jwt(config, &token_info).await {
							Ok(new_token_info) => {
								info!(
									"Successfully refreshed access token for solver {}",
									config.solver_id
								);
								return Ok(Some(new_token_info.access_token));
							},
							Err(e) => {
								error!(
									"Failed to refresh token for solver {}: {}. Falling back to registration",
									config.solver_id, e
								);
								// Fall through to registration
							},
						}
					} else {
						warn!(
							"Both access and refresh tokens expired for solver {}, re-registering",
							config.solver_id
						);
					}
				}

				// No cached tokens, or refresh failed, or tokens expired - register new tokens
				debug!("Registering new JWT tokens for solver {}", config.solver_id);
				let token_info = self.register_jwt(config).await?;
				Ok(Some(token_info.access_token))
			} else {
				Ok(None) // Auth not enabled
			}
		} else {
			Ok(None) // No auth config
		}
	}

	/// Get a configured HTTP client (with or without authentication based on solver settings)
	async fn get_configured_client(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Arc<reqwest::Client>> {
		// Get JWT token if auth is enabled
		let jwt_token = self.get_jwt_token(config).await?;

		// Use generic authentication configuration
		let auth_config = crate::client_cache::AuthConfig::jwt(jwt_token.as_ref());

		// Use client cache with auth-aware support for connection pooling and reuse
		match &self.client_strategy {
			ClientStrategy::Cached(cache) => cache.get_client_with_auth(config, &auth_config),
			ClientStrategy::OnDemand => {
				// For on-demand strategy, create a basic client
				self.create_basic_client_with_auth(config, jwt_token.as_ref())
					.await
			},
		}
	}

	/// Create a basic HTTP client with optional auth headers (for OnDemand strategy)
	async fn create_basic_client_with_auth(
		&self,
		config: &SolverRuntimeConfig,
		jwt_token: Option<&SecretString>,
	) -> AdapterResult<Arc<reqwest::Client>> {
		let mut headers = HeaderMap::new();
		headers.insert("Content-Type", HeaderValue::from_static("application/json"));
		headers.insert("User-Agent", HeaderValue::from_static("OIF-Aggregator/1.0"));
		headers.insert("X-Adapter-Type", HeaderValue::from_static("OIF-v1"));

		// Add JWT token if provided
		if let Some(token) = jwt_token {
			let auth_header_value = format!("Bearer {}", token.expose_secret());
			headers.insert(
				"Authorization",
				HeaderValue::from_str(&auth_header_value).map_err(|_| {
					AdapterError::InvalidResponse {
						reason: "Failed to create Authorization header".to_string(),
					}
				})?,
			);
		}

		// Add custom headers from the solver config
		if let Some(solver_headers) = &config.headers {
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

	/// Fetch assets from OIF API (private helper method)
	async fn fetch_assets_from_api(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Vec<Asset>> {
		let client = self.get_configured_client(config).await?;
		let tokens_url = self.build_url(&config.endpoint, "tokens")?;

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
				let asset = Asset::from_chain_and_address(
					chain_id,
					token.address,
					token.symbol,
					"".to_string(), // OIF doesn't provide token names
					token.decimals,
				)
				.map_err(|e| AdapterError::InvalidResponse {
					reason: format!("Invalid asset from OIF API: {}", e),
				})?;
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

	/// Check if an access token exists and is still valid (for testing)
	#[cfg(test)]
	pub fn is_token_valid(&self, solver_id: &str) -> bool {
		if let Some(token_info) = self.jwt_tokens.get(solver_id) {
			if let Some(expires_at) = token_info.access_token_expires_at {
				// Consider token invalid if it expires within the buffer time
				Utc::now() < expires_at - Duration::minutes(JWT_TOKEN_EXPIRY_BUFFER_MINUTES)
			} else {
				// If no expiration info, assume token is valid
				true
			}
		} else {
			false
		}
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

		let quote_url = self.build_url(&config.endpoint, "quotes")?;
		let client = self.get_configured_client(config).await?;

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
		debug!(
			"OIF quote endpoint responded successfully with {} bytes",
			body.len()
		);

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
			"Submitting order with quote_id {} to OIF adapter {} via solver {}",
			order.quote_response.quote_id, self.config.adapter_id, config.solver_id
		);

		let orders_url = self.build_url(&config.endpoint, "orders")?;
		let client = self.get_configured_client(config).await?;

		debug!(
			"Submitting order to OIF adapter {} via solver {}",
			self.config.adapter_id, config.solver_id
		);

		// Create OIF-specific order request from the generic SubmitOrderRequest
		let oif_request = OifOrderRequest {
			quote_id: order.quote_response.quote_id.clone(),
			signature: order.signature.clone(),
			// Extract optional fields from metadata
			order: order
				.metadata
				.as_ref()
				.and_then(|m| m.get("order"))
				.and_then(|v| v.as_str())
				.map(|s| s.to_string()),
			sponsor: order
				.metadata
				.as_ref()
				.and_then(|m| m.get("sponsor"))
				.and_then(|v| v.as_str())
				.map(|s| s.to_string()),
		};

		let response = client
			.post(orders_url)
			.json(&oif_request)
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
		debug!(
			"OIF order endpoint responded successfully with {} bytes",
			body.len()
		);

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
		let tokens_url = self.build_url(&config.endpoint, "tokens")?;
		let client = self.get_configured_client(config).await?;

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
					debug!(
						"OIF health check endpoint responded with {} bytes",
						body.len()
					);

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

		let order_path = format!("orders/{}", order_id);
		let order_url = self.build_url(&config.endpoint, &order_path)?;
		let client = self.get_configured_client(config).await?;

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
		debug!(
			"OIF get order endpoint responded successfully with {} bytes",
			body.len()
		);

		// Parse the response body manually since we already consumed it
		let order_response: GetOrderResponse =
			serde_json::from_str(&body).map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Failed to parse OIF get order response: {}", e),
			})?;

		Ok(order_response)
	}

	async fn get_supported_assets(
		&self,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<SupportedAssetsData> {
		debug!(
			"OIF adapter getting supported assets for solver: {}",
			config.solver_id
		);

		// Fetch assets directly from tokens endpoint
		let assets = self.fetch_assets_from_api(config).await?;

		debug!(
			"OIF adapter found {} supported assets for solver {} (using assets mode)",
			assets.len(),
			config.solver_id
		);

		Ok(SupportedAssetsData::Assets(assets))
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
	fn test_oif_tokens_to_assets_conversion() -> Result<(), Box<dyn std::error::Error>> {
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
				let asset = Asset::from_chain_and_address(
					chain_id,
					token.address,
					token.symbol.clone(),
					token.symbol,
					token.decimals,
				)?;
				assets.push(asset);
			}
		}

		// Verify conversion
		assert_eq!(assets.len(), 3);

		// Find USDC on Ethereum
		let usdc_eth = assets
			.iter()
			.find(|a| a.symbol == "USDC" && a.chain_id().unwrap_or(0) == 1)
			.unwrap();
		assert_eq!(
			usdc_eth.plain_address(),
			"0xa0b86a33e6441e7c81f7c93451777f5f4de78e86"
		);
		assert_eq!(usdc_eth.decimals, 6);
		assert_eq!(usdc_eth.name, "USDC");

		// Find ETH on Ethereum
		let eth = assets
			.iter()
			.find(|a| a.symbol == "ETH" && a.chain_id().unwrap_or(0) == 1)
			.unwrap();
		assert_eq!(
			eth.plain_address(),
			"0x0000000000000000000000000000000000000000"
		);
		assert_eq!(eth.decimals, 18);

		// Find USDC on Polygon
		let usdc_poly = assets
			.iter()
			.find(|a| a.symbol == "USDC" && a.chain_id().unwrap_or(0) == 137)
			.unwrap();
		assert_eq!(
			usdc_poly.plain_address(),
			"0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
		);
		assert_eq!(usdc_poly.decimals, 6);
		Ok(())
	}

	#[tokio::test]
	async fn test_assets_mode_return_type() {
		let adapter = OifAdapter::with_default_config().unwrap();
		let _config = SolverRuntimeConfig::new(
			"test-solver".to_string(),
			"http://localhost:3000/api".to_string(),
		);

		// Note: This test would need a mock server to actually work
		// For now, we'll just verify the adapter is configured correctly
		assert_eq!(adapter.id(), "oif-v1");
		assert_eq!(adapter.name(), "OIF v1 Adapter");

		// The get_supported_assets method should return Assets mode
		// when it successfully fetches from an OIF API endpoint
		// (This would require a mock server to test the actual return type)
	}

	#[test]
	fn test_assets_mode_behavior() {
		let adapter = OifAdapter::with_default_config().unwrap();

		// Test adapter configuration
		assert_eq!(adapter.id(), "oif-v1");
		assert_eq!(adapter.name(), "OIF v1 Adapter");

		// In assets mode, the adapter should support any-to-any conversions
		// within its asset list (including same-chain swaps)
		// This behavior is tested at the domain level (Solver tests)
		// rather than the adapter level since adapters just return data
	}

	#[test]
	fn test_url_construction() {
		let adapter = OifAdapter::new(Adapter::new(
			"test-adapter".to_string(),
			"Test adapter".to_string(),
			"OIF v1".to_string(),
			"1.0.0".to_string(),
		))
		.unwrap();

		// Test basic URL construction
		let base_url = "https://api.example.com";
		let result = adapter.build_url(base_url, "tokens").unwrap();
		assert_eq!(result, "https://api.example.com/tokens");

		// Test with trailing slash - should handle gracefully
		let base_with_slash = "https://api.example.com/";
		let result = adapter.build_url(base_with_slash, "tokens").unwrap();
		assert_eq!(result, "https://api.example.com/tokens");

		// Test with leading slash in path - should handle gracefully
		let result = adapter.build_url(base_url, "/tokens").unwrap();
		assert_eq!(result, "https://api.example.com/tokens");

		// Test with both trailing and leading slashes
		let result = adapter.build_url(base_with_slash, "/tokens").unwrap();
		assert_eq!(result, "https://api.example.com/tokens");

		// Test with complex path
		let result = adapter.build_url(base_url, "orders/123").unwrap();
		assert_eq!(result, "https://api.example.com/orders/123");

		// Test with path in base URL (the problematic case)
		let base_with_path = "http://127.0.0.1:3000/api";
		let result = adapter.build_url(base_with_path, "tokens").unwrap();
		assert_eq!(result, "http://127.0.0.1:3000/api/tokens");

		// Test with path in base URL and trailing slash
		let base_with_path_slash = "http://127.0.0.1:3000/api/";
		let result = adapter.build_url(base_with_path_slash, "tokens").unwrap();
		assert_eq!(result, "http://127.0.0.1:3000/api/tokens");

		// Test invalid URL
		let result = adapter.build_url("invalid://::url", "tokens");
		assert!(result.is_err());
	}

	#[test]
	fn test_auth_config_parsing() {
		let adapter = OifAdapter::with_default_config().unwrap();

		// Test config with full auth configuration
		let auth_metadata = serde_json::json!({
			"auth": {
				"auth_enabled": true,
				"client_name": "Custom OIF Client",
				"scopes": ["read", "write", "admin"],
				"expiry_hours": 48
			}
		});
		let config = SolverRuntimeConfig::new(
			"test-solver".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_adapter_metadata(auth_metadata);

		let auth_config = adapter.parse_auth_config(&config);
		assert!(auth_config.is_some());
		let auth_config = auth_config.unwrap();
		assert_eq!(auth_config.auth_enabled, Some(true));
		assert_eq!(
			auth_config.client_name,
			Some("Custom OIF Client".to_string())
		);
		assert_eq!(
			auth_config.scopes,
			Some(vec![
				"read".to_string(),
				"write".to_string(),
				"admin".to_string()
			])
		);
		assert_eq!(auth_config.expiry_hours, Some(48));

		// Test config with minimal auth configuration
		let minimal_auth_metadata = serde_json::json!({
			"auth": {
				"auth_enabled": true
			}
		});
		let config_minimal = SolverRuntimeConfig::new(
			"test-solver".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_adapter_metadata(minimal_auth_metadata);

		let auth_config_minimal = adapter.parse_auth_config(&config_minimal);
		assert!(auth_config_minimal.is_some());
		let auth_config_minimal = auth_config_minimal.unwrap();
		assert_eq!(auth_config_minimal.auth_enabled, Some(true));
		assert_eq!(auth_config_minimal.client_name, None); // Should use default
		assert_eq!(auth_config_minimal.scopes, None); // Should use default
		assert_eq!(auth_config_minimal.expiry_hours, None); // Should use default

		// Test config with auth disabled
		let no_auth_metadata = serde_json::json!({
			"auth": {
				"auth_enabled": false
			}
		});
		let config_no_auth = SolverRuntimeConfig::new(
			"test-solver".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_adapter_metadata(no_auth_metadata);

		let auth_config_no_auth = adapter.parse_auth_config(&config_no_auth);
		assert!(auth_config_no_auth.is_some());
		let auth_config_no_auth = auth_config_no_auth.unwrap();
		assert_eq!(auth_config_no_auth.auth_enabled, Some(false));

		// Test config with no metadata
		let config_no_metadata = SolverRuntimeConfig::new(
			"test-solver".to_string(),
			"https://api.example.com".to_string(),
		);
		let auth_config_empty = adapter.parse_auth_config(&config_no_metadata);
		assert!(auth_config_empty.is_none());

		// Test config with metadata but no auth key
		let non_auth_metadata = serde_json::json!({
			"timeout_ms": 5000,
			"retry_attempts": 3,
			"other_config": "value"
		});
		let config_no_auth_key = SolverRuntimeConfig::new(
			"test-solver".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_adapter_metadata(non_auth_metadata);
		let auth_config_no_auth_key = adapter.parse_auth_config(&config_no_auth_key);
		assert!(auth_config_no_auth_key.is_none());

		// Test config with auth alongside other metadata
		let mixed_metadata = serde_json::json!({
			"timeout_ms": 5000,
			"retry_attempts": 3,
			"auth": {
				"auth_enabled": true,
				"expiry_hours": 12
			},
			"other_config": "value"
		});
		let config_mixed = SolverRuntimeConfig::new(
			"test-solver".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_adapter_metadata(mixed_metadata);
		let auth_config_mixed = adapter.parse_auth_config(&config_mixed);
		assert!(auth_config_mixed.is_some());
		let auth_config_mixed = auth_config_mixed.unwrap();
		assert_eq!(auth_config_mixed.auth_enabled, Some(true));
		assert_eq!(auth_config_mixed.expiry_hours, Some(12));
	}

	#[test]
	fn test_jwt_token_validation() {
		let adapter = OifAdapter::with_default_config().unwrap();

		// Test invalid token (no token exists)
		assert!(!adapter.is_token_valid("nonexistent-solver"));

		// Test token with future expiration
		let future_expiry = Utc::now() + chrono::Duration::hours(1);
		let token_info = JwtTokenInfo {
			access_token: SecretString::from("valid-access-token"),
			refresh_token: SecretString::from("valid-refresh-token"),
			access_token_expires_at: Some(future_expiry),
			refresh_token_expires_at: Some(future_expiry + chrono::Duration::days(7)),
		};
		adapter
			.jwt_tokens
			.insert("test-solver".to_string(), token_info);
		assert!(adapter.is_token_valid("test-solver"));

		// Test token that expires soon (within 60 seconds)
		let soon_expiry = Utc::now() + chrono::Duration::seconds(30);
		let expiring_token_info = JwtTokenInfo {
			access_token: SecretString::from("expiring-access-token"),
			refresh_token: SecretString::from("expiring-refresh-token"),
			access_token_expires_at: Some(soon_expiry),
			refresh_token_expires_at: Some(soon_expiry + chrono::Duration::days(7)),
		};
		adapter
			.jwt_tokens
			.insert("expiring-solver".to_string(), expiring_token_info);
		assert!(!adapter.is_token_valid("expiring-solver"));

		// Test token with no expiration (should be valid)
		let no_expiry_token_info = JwtTokenInfo {
			access_token: SecretString::from("no-expiry-access-token"),
			refresh_token: SecretString::from("no-expiry-refresh-token"),
			access_token_expires_at: None,
			refresh_token_expires_at: None,
		};
		adapter
			.jwt_tokens
			.insert("no-expiry-solver".to_string(), no_expiry_token_info);
		assert!(adapter.is_token_valid("no-expiry-solver"));
	}
}
