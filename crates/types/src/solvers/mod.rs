//! Core Solver domain model and business logic

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::metrics::time_series::ErrorType;
use crate::models::{Asset, AssetRoute, InteropAddress};
use url::Url;

pub mod config;
pub mod errors;
pub mod response;
pub mod storage;

pub use config::{AdapterConfig, AdapterType, RouteConfig, SolverConfig};
pub use errors::{SolverError, SolverValidationError};
pub use response::SolverResponse;
pub use storage::SolverStorage;

/// Result types for solver operations
pub type SolverResult<T> = Result<T, SolverError>;
pub type SolverValidationResult<T> = Result<T, SolverValidationError>;

/// Core Solver domain model
///
/// This represents a solver in the domain layer with business logic.
/// It should be converted from SolverConfig and to SolverStorage/SolverResponse.
#[derive(Debug, Clone, PartialEq)]
pub struct Solver {
	/// Unique identifier for the solver
	pub solver_id: String,

	/// ID of the adapter used to communicate with this solver
	pub adapter_id: String,

	/// HTTP endpoint for the solver API
	pub endpoint: String,

	/// Current operational status
	pub status: SolverStatus,

	/// Additional metadata and configuration
	pub metadata: SolverMetadata,

	/// When the solver was registered
	pub created_at: DateTime<Utc>,

	/// Last time the solver was seen/health checked
	pub last_seen: Option<DateTime<Utc>>,

	/// Performance and health metrics
	pub metrics: SolverMetrics,

	/// Custom HTTP headers for requests
	pub headers: Option<HashMap<String, String>>,

	/// Adapter-specific metadata (JSON configuration for adapter customization)
	pub adapter_metadata: Option<serde_json::Value>,
}

/// Solver operational status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum SolverStatus {
	/// Solver is active and available for requests
	Active,
	/// Solver is disabled (not available for requests)
	Disabled,
}

/// 🆕 HYBRID: What assets/routes a solver supports
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SupportedAssets {
	/// Asset-based: supports any-to-any within asset list (including same-chain)
	Assets {
		assets: Vec<Asset>,
		source: AssetSource,
	},
	/// Route-based: supports specific origin->destination pairs
	Routes {
		routes: Vec<AssetRoute>,
		source: AssetSource,
	},
}

/// Source of the support data
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AssetSource {
	/// Data is manually defined in configuration
	Config,
	/// Data is auto-discovered from solver API
	AutoDiscovered,
}

/// Solver metadata and configuration
#[derive(Debug, Clone, PartialEq)]
pub struct SolverMetadata {
	/// Human-readable name
	pub name: Option<String>,

	/// Description of the solver
	pub description: Option<String>,

	/// Version of the solver API
	pub version: Option<String>,

	/// What assets/routes this solver supports
	pub supported_assets: SupportedAssets,

	/// Custom HTTP headers for requests
	pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolverMetrics {
	/// Total number of requests made (all types: quotes, health checks, etc.)
	/// Lifetime counter since solver creation - used for monitoring/debugging
	pub total_requests: u64,

	/// Number of successful requests (lifetime)
	/// Lifetime counter since solver creation - used for monitoring/debugging  
	pub successful_requests: u64,

	/// Service errors (5xx, 429, network issues, timeouts) - lifetime
	/// These errors typically indicate solver or infrastructure problems
	pub service_errors: u64,
	/// === WINDOWED METRICS (for accurate circuit breaker decisions) ===

	/// Total requests in current window - used for recent performance analysis
	pub recent_total_requests: u64,

	/// Successful requests in current window - used for recent success rate calculation
	pub recent_successful_requests: u64,

	/// Service errors in current window - used for recent error rate analysis
	/// Includes timeouts, 5xx errors, network issues, 429 rate limits
	pub recent_service_errors: u64,

	/// When the current metrics window started
	/// Used to determine when to reset window counters
	pub window_start: DateTime<Utc>,

	/// Consecutive request failures (for all request types, not just health checks)
	/// Real-time state that persists across window resets
	pub consecutive_failures: u32,

	/// Last time metrics were updated
	pub last_updated: DateTime<Utc>,
}

impl Solver {
	/// Create a new solver
	pub fn new(solver_id: String, adapter_id: String, endpoint: String) -> Self {
		let now = Utc::now();

		Self {
			solver_id,
			adapter_id,
			endpoint,
			status: SolverStatus::Active,
			metadata: SolverMetadata::default(),
			created_at: now,
			last_seen: None,
			metrics: SolverMetrics::new(),
			headers: None,
			adapter_metadata: None,
		}
	}

	/// Check if the solver is available for requests
	pub fn is_available(&self) -> bool {
		matches!(self.status, SolverStatus::Active)
	}

	/// Check if the solver is healthy based on recent metrics
	/// Health is determined by consecutive failures - simple and effective
	pub fn is_healthy(&self) -> bool {
		self.metrics.consecutive_failures < 3
	}

	/// Update solver status
	pub fn update_status(&mut self, status: SolverStatus) {
		self.status = status;
		self.last_seen = Some(Utc::now());
	}

	/// Mark solver as seen (update last_seen timestamp)
	pub fn mark_seen(&mut self) {
		self.last_seen = Some(Utc::now());
	}

	/// Validate solver parameters
	pub fn validate(&self) -> SolverValidationResult<()> {
		// Validate solver ID
		if self.solver_id.is_empty() {
			return Err(SolverValidationError::MissingRequiredField {
				field: "solver_id".to_string(),
			});
		}

		if !self
			.solver_id
			.chars()
			.all(|c| c.is_alphanumeric() || c == '-' || c == '_')
		{
			return Err(SolverValidationError::InvalidSolverId {
				solver_id: self.solver_id.clone(),
			});
		}

		// Validate adapter ID
		if self.adapter_id.is_empty() {
			return Err(SolverValidationError::MissingRequiredField {
				field: "adapter_id".to_string(),
			});
		}

		// Validate endpoint URL
		if self.endpoint.is_empty() {
			return Err(SolverValidationError::MissingRequiredField {
				field: "endpoint".to_string(),
			});
		}

		match Url::parse(&self.endpoint) {
			Ok(url) => {
				if !matches!(url.scheme(), "http" | "https") {
					return Err(SolverValidationError::InvalidEndpoint {
						endpoint: self.endpoint.clone(),
						reason: "Only HTTP and HTTPS schemes are supported".to_string(),
					});
				}
				if url.host().is_none() {
					return Err(SolverValidationError::InvalidEndpoint {
						endpoint: self.endpoint.clone(),
						reason: "URL must have a valid host".to_string(),
					});
				}
			},
			Err(e) => {
				return Err(SolverValidationError::InvalidEndpoint {
					endpoint: self.endpoint.clone(),
					reason: e.to_string(),
				});
			},
		}

		Ok(())
	}

	/// Record a successful request
	pub fn record_success(
		&mut self,
		window_duration_minutes: u32,
		max_window_age_minutes: u32,
		min_requests_for_rate_check: u64,
	) {
		self.metrics.record_success(
			window_duration_minutes,
			max_window_age_minutes,
			min_requests_for_rate_check,
		);
		self.mark_seen();
	}

	/// Record a failed request
	pub fn record_failure(
		&mut self,
		window_duration_minutes: u32,
		max_window_age_minutes: u32,
		min_requests_for_rate_check: u64,
	) {
		self.metrics.record_failure(
			Some(ErrorType::ServiceError),
			window_duration_minutes,
			max_window_age_minutes,
			min_requests_for_rate_check,
		);
		self.mark_seen();
	}

	/// Check if solver supports a specific chain
	pub fn supports_chain(&self, chain_id: u64) -> bool {
		match &self.metadata.supported_assets {
			SupportedAssets::Assets { assets, .. } => assets
				.iter()
				.any(|asset| asset.chain_id().unwrap_or(0) == chain_id),
			SupportedAssets::Routes { routes, .. } => routes.iter().any(|r| {
				r.origin_chain_id().unwrap_or(0) == chain_id
					|| r.destination_chain_id().unwrap_or(0) == chain_id
			}),
		}
	}

	/// Check if solver supports a specific asset by symbol
	pub fn supports_asset_symbol(&self, symbol: &str) -> bool {
		match &self.metadata.supported_assets {
			SupportedAssets::Assets { assets, .. } => assets
				.iter()
				.any(|asset| asset.symbol.eq_ignore_ascii_case(symbol)),
			SupportedAssets::Routes { routes, .. } => routes.iter().any(|r| {
				r.origin_token_symbol
					.as_ref()
					.is_some_and(|s| s.eq_ignore_ascii_case(symbol))
					|| r.destination_token_symbol
						.as_ref()
						.is_some_and(|s| s.eq_ignore_ascii_case(symbol))
			}),
		}
	}

	/// Check if solver supports a specific asset on a specific chain
	pub fn supports_asset_on_chain(&self, chain_id: u64, address: &str) -> bool {
		match &self.metadata.supported_assets {
			SupportedAssets::Assets { assets, .. } => assets.iter().any(|asset| {
				asset.chain_id().unwrap_or(0) == chain_id
					&& asset.plain_address().eq_ignore_ascii_case(address)
			}),
			SupportedAssets::Routes { routes, .. } => routes.iter().any(|r| {
				(r.origin_chain_id().unwrap_or(0) == chain_id
					&& r.origin_address().eq_ignore_ascii_case(address))
					|| (r.destination_chain_id().unwrap_or(0) == chain_id
						&& r.destination_address().eq_ignore_ascii_case(address))
			}),
		}
	}

	/// Check if solver supports a specific asset
	pub fn supports_asset(&self, asset: &Asset) -> bool {
		match &self.metadata.supported_assets {
			SupportedAssets::Assets { assets, .. } => assets
				.iter()
				.any(|supported_asset| supported_asset.address == asset.address),
			SupportedAssets::Routes { routes, .. } => routes.iter().any(|route| {
				route.origin_asset == asset.address || route.destination_asset == asset.address
			}),
		}
	}

	/// Check if solver supports a specific asset route
	pub fn supports_route(&self, origin: &InteropAddress, destination: &InteropAddress) -> bool {
		match &self.metadata.supported_assets {
			SupportedAssets::Assets { assets, .. } => {
				// For assets mode: check if both origin and destination assets are supported
				// by directly comparing InteropAddress values
				let origin_supported = assets.iter().any(|asset| asset.address == *origin);
				let dest_supported = assets.iter().any(|asset| asset.address == *destination);

				// Both assets must be supported (same-chain is allowed by default now)
				origin_supported && dest_supported
			},
			SupportedAssets::Routes { routes, .. } => routes
				.iter()
				.any(|route| route.matches(origin, destination)),
		}
	}

	/// Check if solver has route information available
	pub fn has_route_info(&self) -> bool {
		match &self.metadata.supported_assets {
			SupportedAssets::Assets { assets, .. } => !assets.is_empty(),
			SupportedAssets::Routes { routes, .. } => !routes.is_empty(),
		}
	}

	/// Get all routes for a specific origin asset
	pub fn routes_from_asset(&self, origin: &InteropAddress) -> Vec<&AssetRoute> {
		match &self.metadata.supported_assets {
			SupportedAssets::Assets { .. } => {
				// For assets mode, we don't have explicit routes
				// This method is mainly used for routes mode
				Vec::new()
			},
			SupportedAssets::Routes { routes, .. } => routes
				.iter()
				.filter(|route| route.origin_asset == *origin)
				.collect(),
		}
	}

	/// Get all routes to a specific destination asset
	pub fn routes_to_asset(&self, destination: &InteropAddress) -> Vec<&AssetRoute> {
		match &self.metadata.supported_assets {
			SupportedAssets::Assets { .. } => {
				// For assets mode, we don't have explicit routes
				Vec::new()
			},
			SupportedAssets::Routes { routes, .. } => routes
				.iter()
				.filter(|route| route.destination_asset == *destination)
				.collect(),
		}
	}

	/// Get solver priority score (higher is better)
	/// Basic scoring using immediate metrics only - for advanced scoring use MetricsTimeSeries
	pub fn priority_score(&self) -> f64 {
		if !self.is_available() {
			return 0.0;
		}

		let base_score = 100.0;

		// Use basic success rate calculation if we have requests
		let success_rate_bonus = if self.metrics.total_requests > 0 {
			(self.metrics.successful_requests as f64 / self.metrics.total_requests as f64) * 50.0
		} else {
			0.0
		};

		let failure_penalty = self.metrics.consecutive_failures as f64 * 10.0;

		(base_score + success_rate_bonus - failure_penalty).max(0.0)
	}

	/// Check if solver has been inactive for too long
	pub fn is_stale(&self, max_age: Duration) -> bool {
		if let Some(last_seen) = self.last_seen {
			Utc::now() - last_seen > max_age
		} else {
			Utc::now() - self.created_at > max_age
		}
	}

	/// Builder methods for easy configuration
	pub fn with_name(mut self, name: String) -> Self {
		self.metadata.name = Some(name);
		self
	}

	pub fn with_description(mut self, description: String) -> Self {
		self.metadata.description = Some(description);
		self
	}

	pub fn with_version(mut self, version: String) -> Self {
		self.metadata.version = Some(version);
		self
	}

	pub fn with_routes(mut self, routes: Vec<AssetRoute>) -> Self {
		self.metadata.supported_assets = SupportedAssets::Routes {
			routes,
			source: AssetSource::Config,
		};
		self
	}

	pub fn with_assets(mut self, assets: Vec<Asset>) -> Self {
		self.metadata.supported_assets = SupportedAssets::Assets {
			assets,
			source: AssetSource::Config,
		};
		self
	}

	pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
		self.metadata.headers = Some(headers);
		self
	}

	pub fn with_adapter_metadata(mut self, adapter_metadata: serde_json::Value) -> Self {
		self.adapter_metadata = Some(adapter_metadata);
		self
	}
}

impl Default for SolverMetadata {
	fn default() -> Self {
		Self {
			name: None,
			description: None,
			version: None,
			supported_assets: SupportedAssets::Routes {
				routes: Vec::new(),
				source: AssetSource::AutoDiscovered, // Default to auto-discovery
			},
			headers: None,
		}
	}
}

impl SolverMetrics {
	pub fn new() -> Self {
		let now = Utc::now();
		Self {
			// Lifetime counters
			total_requests: 0,
			successful_requests: 0,
			service_errors: 0,

			// Windowed counters (for circuit breaker)
			recent_total_requests: 0,
			recent_successful_requests: 0,
			recent_service_errors: 0,
			window_start: now,

			// State fields
			consecutive_failures: 0,
			last_updated: now,
		}
	}

	/// Check if the metrics window has expired and reset it if needed using dual thresholds
	/// This should be called before updating any windowed metrics
	///
	/// Uses a dual threshold approach:
	/// 1. Normal window duration (15 min) - reset if we have sufficient recent data
	/// 2. Maximum window age (60 min) - force reset regardless to prevent staleness
	/// 3. Preserve window if insufficient recent data AND not too old (adaptive for low-traffic)
	pub fn maybe_reset_window(
		&mut self,
		window_duration_minutes: u32,
		max_window_age_minutes: u32,
		min_requests_for_rate_check: u64,
	) {
		let now = Utc::now();
		let window_elapsed = now.signed_duration_since(self.window_start);
		let normal_window_expired = window_elapsed.num_minutes() >= window_duration_minutes as i64;
		let max_age_exceeded = window_elapsed.num_minutes() >= max_window_age_minutes as i64;

		if max_age_exceeded {
			// Force reset after maximum age - prevents indefinite staleness
			// Even low-traffic solvers need periodic refresh
			self.reset_window_counters(now);
		} else if normal_window_expired {
			// Normal window expiration - use intelligent logic
			if self.recent_total_requests >= min_requests_for_rate_check {
				// High-traffic: sufficient recent data, reset normally
				self.reset_window_counters(now);
			} else if self.total_requests >= min_requests_for_rate_check {
				// Low-traffic: preserve window to accumulate data, but we have lifetime fallback
				// Don't reset - let the window grow until it reaches min_circuit_breaker_requests
				// or max_window_age_minutes (handled above)
			} else {
				// Brand new solver: extend window until we have basic lifetime data as fallback
				self.window_start = now
					.checked_sub_signed(chrono::Duration::minutes(
						window_duration_minutes as i64 / 2,
					))
					.unwrap_or(now);
			}
		}
	}

	/// Reset windowed counters while preserving lifetime counters and state
	fn reset_window_counters(&mut self, now: DateTime<Utc>) {
		self.recent_total_requests = 0;
		self.recent_successful_requests = 0;
		self.recent_service_errors = 0;
		self.window_start = now;
		// Note: consecutive_failures and health_status are preserved across window resets
	}

	/// Record a successful request
	pub fn record_success(
		&mut self,
		window_duration_minutes: u32,
		max_window_age_minutes: u32,
		min_requests_for_rate_check: u64,
	) {
		self.maybe_reset_window(
			window_duration_minutes,
			max_window_age_minutes,
			min_requests_for_rate_check,
		);

		// Update lifetime counters
		self.total_requests += 1;
		self.successful_requests += 1;

		// Update windowed counters
		self.recent_total_requests += 1;
		self.recent_successful_requests += 1;

		// Reset consecutive failures on success
		self.consecutive_failures = 0;
		self.last_updated = Utc::now();
	}

	/// Record a failed request with error type categorization
	pub fn record_failure(
		&mut self,
		error_type: Option<ErrorType>,
		window_duration_minutes: u32,
		max_window_age_minutes: u32,
		min_requests_for_rate_check: u64,
	) {
		self.maybe_reset_window(
			window_duration_minutes,
			max_window_age_minutes,
			min_requests_for_rate_check,
		);

		// Update lifetime counters
		self.total_requests += 1;
		self.consecutive_failures += 1;

		// Update windowed counters
		self.recent_total_requests += 1;

		// Update error category counters based on the error type
		let is_service_error = if let Some(error_type) = error_type {
			match error_type {
				ErrorType::ServiceError => {
					self.service_errors += 1;
					true
				},
				ErrorType::ClientError => {
					// Client errors no longer tracked separately
					false
				},
				ErrorType::ApplicationError => {
					// Application errors are service-side issues that affect circuit breaker
					self.service_errors += 1;
					true
				},
				ErrorType::Unknown => {
					// Unknown errors are treated as service errors to be safe
					self.service_errors += 1;
					true
				},
			}
		} else {
			// If no error type provided, categorize as unknown service error
			self.service_errors += 1;
			true
		};

		// Update windowed service errors (only for service-side errors)
		if is_service_error {
			self.recent_service_errors += 1;
		}

		self.last_updated = Utc::now();
	}

	/// Reset metrics (useful for maintenance cycles)
	pub fn reset(&mut self) {
		*self = Self::new();
	}
}

impl Default for SolverMetrics {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn create_test_solver() -> Solver {
		Solver::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
		)
	}

	#[test]
	fn test_solver_creation() {
		let solver = create_test_solver();

		assert_eq!(solver.solver_id, "test-solver");
		assert_eq!(solver.adapter_id, "oif-v1");
		assert_eq!(solver.endpoint, "https://api.example.com");
		assert_eq!(solver.status, SolverStatus::Active);
		assert!(solver.is_available());
	}

	#[test]
	fn test_solver_availability() {
		let mut solver = create_test_solver();

		assert!(solver.is_available());

		solver.update_status(SolverStatus::Active);
		assert!(solver.is_available());

		solver.update_status(SolverStatus::Disabled);
		assert!(!solver.is_available());
	}

	#[test]
	fn test_metrics_recording() {
		let mut solver = create_test_solver();

		// Record some successes
		solver.record_success(15, 60, 5);
		solver.record_success(15, 60, 5);

		assert_eq!(solver.metrics.total_requests, 2);
		assert_eq!(solver.metrics.successful_requests, 2);
		assert_eq!(solver.metrics.consecutive_failures, 0);

		// Record a failure
		solver.record_failure(15, 60, 5);

		assert_eq!(solver.metrics.total_requests, 3);
		assert_eq!(solver.metrics.successful_requests, 2);
		// failed_requests is now calculated as: total_requests - successful_requests
		let failed_requests = solver.metrics.total_requests - solver.metrics.successful_requests;
		assert_eq!(failed_requests, 1);
		assert_eq!(solver.metrics.consecutive_failures, 1);
	}

	#[test]
	fn test_priority_score() {
		let mut solver = create_test_solver();
		solver.update_status(SolverStatus::Active);

		// Initial score should be positive
		let initial_score = solver.priority_score();
		assert!(initial_score > 0.0);

		// Record success should improve score
		solver.record_success(15, 60, 5);
		let improved_score = solver.priority_score();
		assert!(improved_score > initial_score);

		// Record failures should decrease score
		solver.record_failure(15, 60, 5);
		solver.record_failure(15, 60, 5);
		let decreased_score = solver.priority_score();
		assert!(decreased_score < improved_score);
	}

	#[test]
	fn test_builder_pattern() {
		let solver = create_test_solver()
			.with_name("Test Solver".to_string())
			.with_version("1.0.0".to_string())
			.with_routes(vec![
				AssetRoute::with_symbols(
					InteropAddress::from_text(
						"eip155:1:0x0000000000000000000000000000000000000000",
					)
					.unwrap(), // ETH on Ethereum
					"ETH".to_string(),
					InteropAddress::from_text(
						"eip155:10:0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
					)
					.unwrap(), // USDC on Optimism
					"USDC".to_string(),
				),
				AssetRoute::with_symbols(
					InteropAddress::from_text(
						"eip155:10:0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
					)
					.unwrap(), // USDC on Optimism
					"USDC".to_string(),
					InteropAddress::from_text(
						"eip155:1:0x0000000000000000000000000000000000000000",
					)
					.unwrap(), // ETH on Ethereum
					"ETH".to_string(),
				),
			]);

		assert_eq!(solver.metadata.name, Some("Test Solver".to_string()));
		assert_eq!(solver.metadata.version, Some("1.0.0".to_string()));
		assert!(solver.supports_chain(1)); // Should support chain 1 via cross-chain routes
		assert!(solver.supports_chain(10)); // Should support chain 10 via cross-chain routes
	}

	#[test]
	fn test_supports_asset_on_chain() {
		let solver = create_test_solver().with_routes(vec![
			// USDC Ethereum <-> USDC Polygon
			AssetRoute::with_symbols(
				InteropAddress::from_text("eip155:1:0x1234567890123456789012345678901234567890")
					.unwrap(), // USDC on Ethereum
				"USDC".to_string(),
				InteropAddress::from_text("eip155:137:0x1234567890123456789012345678901234567890")
					.unwrap(), // USDC on Polygon
				"USDC".to_string(),
			),
			AssetRoute::with_symbols(
				InteropAddress::from_text("eip155:137:0x1234567890123456789012345678901234567890")
					.unwrap(), // USDC on Polygon
				"USDC".to_string(),
				InteropAddress::from_text("eip155:1:0x1234567890123456789012345678901234567890")
					.unwrap(), // USDC on Ethereum
				"USDC".to_string(),
			),
			// WETH Ethereum <-> USDC Polygon
			AssetRoute::with_symbols(
				InteropAddress::from_text("eip155:1:0xAbCdEf1234567890123456789012345678901234")
					.unwrap(), // WETH on Ethereum
				"WETH".to_string(),
				InteropAddress::from_text("eip155:137:0x1234567890123456789012345678901234567890")
					.unwrap(), // USDC on Polygon
				"USDC".to_string(),
			),
		]);

		// Test: Same address on different chains
		assert!(solver.supports_asset_on_chain(1, "0x1234567890123456789012345678901234567890")); // USDC on Ethereum
		assert!(solver.supports_asset_on_chain(137, "0x1234567890123456789012345678901234567890")); // USDC on Polygon
		assert!(
			!solver.supports_asset_on_chain(42161, "0x1234567890123456789012345678901234567890")
		); // USDC on Arbitrum (not supported)

		// Test: Different address on supported chain
		assert!(solver.supports_asset_on_chain(1, "0xAbCdEf1234567890123456789012345678901234")); // WETH on Ethereum
		assert!(!solver.supports_asset_on_chain(137, "0xAbCdEf1234567890123456789012345678901234")); // WETH on Polygon (not supported)

		// Test: Case insensitive address matching
		assert!(solver.supports_asset_on_chain(1, "0x1234567890123456789012345678901234567890")); // lowercase
		assert!(solver.supports_asset_on_chain(1, "0X1234567890123456789012345678901234567890")); // uppercase
		assert!(solver.supports_asset_on_chain(1, "0x1234567890123456789012345678901234567890"));
		// mixed case
	}

	// ===== SolverMetrics Tests =====

	#[test]
	fn test_solver_metrics_new() {
		let metrics = SolverMetrics::new();
		let now = Utc::now();

		// All counters should start at zero
		assert_eq!(metrics.total_requests, 0);
		assert_eq!(metrics.successful_requests, 0);
		assert_eq!(metrics.service_errors, 0);
		assert_eq!(metrics.recent_total_requests, 0);
		assert_eq!(metrics.recent_successful_requests, 0);
		assert_eq!(metrics.recent_service_errors, 0);
		assert_eq!(metrics.consecutive_failures, 0);

		// Timestamps should be recent
		let time_diff = (now - metrics.last_updated).num_milliseconds().abs();
		assert!(time_diff < 1000, "last_updated should be recent");

		let time_diff = (now - metrics.window_start).num_milliseconds().abs();
		assert!(time_diff < 1000, "window_start should be recent");
	}

	#[test]
	fn test_solver_metrics_default() {
		let metrics = SolverMetrics::default();
		let new_metrics = SolverMetrics::new();

		// Default should be equivalent to new()
		assert_eq!(metrics.total_requests, new_metrics.total_requests);
		assert_eq!(metrics.successful_requests, new_metrics.successful_requests);
		assert_eq!(metrics.service_errors, new_metrics.service_errors);
		assert_eq!(
			metrics.consecutive_failures,
			new_metrics.consecutive_failures
		);
	}

	#[test]
	fn test_solver_metrics_record_success() {
		let mut metrics = SolverMetrics::new();

		metrics.record_success(15, 60, 5);

		// Check lifetime counters
		assert_eq!(metrics.total_requests, 1);
		assert_eq!(metrics.successful_requests, 1);
		assert_eq!(metrics.service_errors, 0);

		// Check windowed counters
		assert_eq!(metrics.recent_total_requests, 1);
		assert_eq!(metrics.recent_successful_requests, 1);
		assert_eq!(metrics.recent_service_errors, 0);

		// Success should reset consecutive failures
		assert_eq!(metrics.consecutive_failures, 0);

		// Test multiple successes
		metrics.record_success(15, 60, 5);
		assert_eq!(metrics.total_requests, 2);
		assert_eq!(metrics.successful_requests, 2);
		assert_eq!(metrics.recent_total_requests, 2);
		assert_eq!(metrics.recent_successful_requests, 2);
	}

	#[test]
	fn test_solver_metrics_record_failure_service_error() {
		let mut metrics = SolverMetrics::new();

		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);

		// Check lifetime counters
		assert_eq!(metrics.total_requests, 1);
		assert_eq!(metrics.successful_requests, 0);
		assert_eq!(metrics.service_errors, 1);

		// Check windowed counters
		assert_eq!(metrics.recent_total_requests, 1);
		assert_eq!(metrics.recent_successful_requests, 0);
		assert_eq!(metrics.recent_service_errors, 1);

		// Check consecutive failures
		assert_eq!(metrics.consecutive_failures, 1);
	}

	#[test]
	fn test_solver_metrics_record_failure_client_error() {
		let mut metrics = SolverMetrics::new();

		metrics.record_failure(Some(ErrorType::ClientError), 15, 60, 5);

		// Check lifetime counters
		assert_eq!(metrics.total_requests, 1);
		assert_eq!(metrics.successful_requests, 0);
		assert_eq!(metrics.service_errors, 0); // Client errors don't count as service errors

		// Check windowed counters
		assert_eq!(metrics.recent_total_requests, 1);
		assert_eq!(metrics.recent_successful_requests, 0);
		assert_eq!(metrics.recent_service_errors, 0); // Client errors don't count

		// Still increments consecutive failures
		assert_eq!(metrics.consecutive_failures, 1);
	}

	#[test]
	fn test_solver_metrics_record_failure_application_error() {
		let mut metrics = SolverMetrics::new();

		metrics.record_failure(Some(ErrorType::ApplicationError), 15, 60, 5);

		// Application errors are treated as service errors
		assert_eq!(metrics.total_requests, 1);
		assert_eq!(metrics.service_errors, 1);
		assert_eq!(metrics.recent_service_errors, 1);
		assert_eq!(metrics.consecutive_failures, 1);
	}

	#[test]
	fn test_solver_metrics_record_failure_unknown_error() {
		let mut metrics = SolverMetrics::new();

		metrics.record_failure(Some(ErrorType::Unknown), 15, 60, 5);

		// Unknown errors are treated as service errors (fail-safe)
		assert_eq!(metrics.total_requests, 1);
		assert_eq!(metrics.service_errors, 1);
		assert_eq!(metrics.recent_service_errors, 1);
		assert_eq!(metrics.consecutive_failures, 1);
	}

	#[test]
	fn test_solver_metrics_record_failure_no_error_type() {
		let mut metrics = SolverMetrics::new();

		metrics.record_failure(None, 15, 60, 5);

		// No error type defaults to service error
		assert_eq!(metrics.total_requests, 1);
		assert_eq!(metrics.service_errors, 1);
		assert_eq!(metrics.recent_service_errors, 1);
		assert_eq!(metrics.consecutive_failures, 1);
	}

	#[test]
	fn test_solver_metrics_consecutive_failures_reset_on_success() {
		let mut metrics = SolverMetrics::new();

		// Record multiple failures
		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);
		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);
		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);

		assert_eq!(metrics.consecutive_failures, 3);

		// Success should reset consecutive failures
		metrics.record_success(15, 60, 5);
		assert_eq!(metrics.consecutive_failures, 0);
	}

	#[test]
	fn test_solver_metrics_window_reset_normal_expiration_sufficient_data() {
		let mut metrics = SolverMetrics::new();

		// Manually set window_start to simulate expired window
		metrics.window_start = Utc::now() - chrono::Duration::minutes(20); // Expired (>15 min)

		// Add sufficient recent data
		metrics.recent_total_requests = 10; // >= min_requests_for_rate_check (5)
		metrics.recent_successful_requests = 8;
		metrics.recent_service_errors = 2;

		metrics.maybe_reset_window(15, 60, 5); // 15 min window, 60 min max, 5 min requests

		// Window should reset because it expired AND we have sufficient data
		assert_eq!(metrics.recent_total_requests, 0);
		assert_eq!(metrics.recent_successful_requests, 0);
		assert_eq!(metrics.recent_service_errors, 0);

		// window_start should be recent
		let now = Utc::now();
		let time_diff = (now - metrics.window_start).num_milliseconds().abs();
		assert!(
			time_diff < 1000,
			"window_start should be recent after reset"
		);
	}

	#[test]
	fn test_solver_metrics_window_reset_normal_expiration_insufficient_data() {
		let mut metrics = SolverMetrics::new();

		// Set up scenario: window expired but insufficient recent data
		let old_window_start = Utc::now() - chrono::Duration::minutes(20);
		metrics.window_start = old_window_start;

		// Insufficient recent data but sufficient lifetime data
		metrics.recent_total_requests = 2; // < min_requests_for_rate_check (5)
		metrics.total_requests = 10; // >= min_requests_for_rate_check (5)

		metrics.maybe_reset_window(15, 60, 5);

		// Window should NOT reset - preserve to accumulate more data
		assert_eq!(metrics.recent_total_requests, 2); // Unchanged
		assert_eq!(metrics.window_start, old_window_start); // Unchanged
	}

	#[test]
	fn test_solver_metrics_window_reset_max_age_exceeded() {
		let mut metrics = SolverMetrics::new();

		// Set window_start to exceed max age
		metrics.window_start = Utc::now() - chrono::Duration::minutes(70); // > 60 min max

		// Even with insufficient data
		metrics.recent_total_requests = 1; // < min_requests_for_rate_check (5)
		metrics.total_requests = 2; // < min_requests_for_rate_check (5)

		metrics.maybe_reset_window(15, 60, 5);

		// Should force reset due to max age
		assert_eq!(metrics.recent_total_requests, 0);
		assert_eq!(metrics.recent_successful_requests, 0);
		assert_eq!(metrics.recent_service_errors, 0);

		let now = Utc::now();
		let time_diff = (now - metrics.window_start).num_milliseconds().abs();
		assert!(
			time_diff < 1000,
			"window_start should be recent after forced reset"
		);
	}

	#[test]
	fn test_solver_metrics_window_reset_brand_new_solver() {
		let mut metrics = SolverMetrics::new();

		// Simulate brand new solver: window expired, no data anywhere
		metrics.window_start = Utc::now() - chrono::Duration::minutes(20);
		metrics.recent_total_requests = 1; // < min_requests_for_rate_check (5)
		metrics.total_requests = 2; // < min_requests_for_rate_check (5)

		let original_recent_requests = metrics.recent_total_requests;

		metrics.maybe_reset_window(15, 60, 5);

		// Should extend window (set to half the normal duration ago)
		assert_eq!(metrics.recent_total_requests, original_recent_requests); // Preserved

		// window_start should be set to ~7.5 minutes ago (half of 15 min)
		let expected_window_start = Utc::now() - chrono::Duration::minutes(7);
		let time_diff = (metrics.window_start - expected_window_start)
			.num_minutes()
			.abs();
		assert!(
			time_diff <= 1,
			"window_start should be about half duration ago"
		);
	}

	#[test]
	fn test_solver_metrics_window_no_reset_when_not_expired() {
		let mut metrics = SolverMetrics::new();

		// Window not expired yet
		let recent_window_start = Utc::now() - chrono::Duration::minutes(5); // < 15 min
		metrics.window_start = recent_window_start;
		metrics.recent_total_requests = 3;

		metrics.maybe_reset_window(15, 60, 5);

		// Nothing should change
		assert_eq!(metrics.recent_total_requests, 3);
		assert_eq!(metrics.window_start, recent_window_start);
	}

	#[test]
	fn test_solver_metrics_mixed_success_and_failure() {
		let mut metrics = SolverMetrics::new();

		// Mix of successes and different error types
		metrics.record_success(15, 60, 5);
		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);
		metrics.record_success(15, 60, 5);
		metrics.record_failure(Some(ErrorType::ClientError), 15, 60, 5);
		metrics.record_failure(Some(ErrorType::ApplicationError), 15, 60, 5);

		// Check final counts
		assert_eq!(metrics.total_requests, 5);
		assert_eq!(metrics.successful_requests, 2);
		assert_eq!(metrics.service_errors, 2); // ServiceError + ApplicationError
		assert_eq!(metrics.recent_total_requests, 5);
		assert_eq!(metrics.recent_successful_requests, 2);
		assert_eq!(metrics.recent_service_errors, 2);

		// Consecutive failures should be 2 (last two were failures)
		assert_eq!(metrics.consecutive_failures, 2);
	}

	#[test]
	fn test_solver_metrics_reset() {
		let mut metrics = SolverMetrics::new();

		// Add some data
		metrics.record_success(15, 60, 5);
		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);

		assert_eq!(metrics.total_requests, 2);
		assert_eq!(metrics.consecutive_failures, 1);

		// Reset should restore to initial state
		metrics.reset();

		assert_eq!(metrics.total_requests, 0);
		assert_eq!(metrics.successful_requests, 0);
		assert_eq!(metrics.service_errors, 0);
		assert_eq!(metrics.recent_total_requests, 0);
		assert_eq!(metrics.recent_successful_requests, 0);
		assert_eq!(metrics.recent_service_errors, 0);
		assert_eq!(metrics.consecutive_failures, 0);
	}

	#[test]
	fn test_solver_metrics_timestamp_updates() {
		let mut metrics = SolverMetrics::new();
		let initial_update = metrics.last_updated;

		// Small delay to ensure timestamp difference
		std::thread::sleep(std::time::Duration::from_millis(10));

		metrics.record_success(15, 60, 5);
		assert!(
			metrics.last_updated > initial_update,
			"Success should update timestamp"
		);

		let success_update = metrics.last_updated;
		std::thread::sleep(std::time::Duration::from_millis(10));

		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);
		assert!(
			metrics.last_updated > success_update,
			"Failure should update timestamp"
		);
	}

	#[test]
	fn test_solver_metrics_window_reset_preserves_important_state() {
		let mut metrics = SolverMetrics::new();

		// Set up state that should be preserved across window resets
		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);
		metrics.record_failure(Some(ErrorType::ServiceError), 15, 60, 5);

		let original_consecutive_failures = metrics.consecutive_failures;
		let original_lifetime_requests = metrics.total_requests;
		let original_lifetime_successes = metrics.successful_requests;
		let original_lifetime_service_errors = metrics.service_errors;

		// Force window reset by setting old window_start
		metrics.window_start = Utc::now() - chrono::Duration::minutes(70); // > 60 min max
		metrics.maybe_reset_window(15, 60, 5);

		// Windowed counters should reset
		assert_eq!(metrics.recent_total_requests, 0);
		assert_eq!(metrics.recent_successful_requests, 0);
		assert_eq!(metrics.recent_service_errors, 0);

		// Important state should be preserved
		assert_eq!(metrics.consecutive_failures, original_consecutive_failures);
		assert_eq!(metrics.total_requests, original_lifetime_requests);
		assert_eq!(metrics.successful_requests, original_lifetime_successes);
		assert_eq!(metrics.service_errors, original_lifetime_service_errors);
	}
}
