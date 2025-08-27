//! Core Solver domain model and business logic

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::models::Asset;
use url::Url;

pub mod config;
pub mod errors;
pub mod response;
pub mod storage;

pub use config::{AdapterConfig, AdapterType, SolverConfig};
pub use errors::{SolverError, SolverValidationError};
pub use response::SolverResponse;
pub use storage::SolverStorage;

/// Result types for solver operations
pub type SolverResult<T> = Result<T, SolverError>;
pub type SolverValidationResult<T> = Result<T, SolverValidationError>;

/// Health check result with detailed information
#[derive(Debug, Clone, PartialEq)]
pub struct HealthCheckResult {
	pub is_healthy: bool,
	pub response_time_ms: u64,
	pub error_message: Option<String>,
	pub last_check: chrono::DateTime<chrono::Utc>,
	pub consecutive_failures: u32,
}

impl HealthCheckResult {
	pub fn healthy(response_time_ms: u64) -> Self {
		Self {
			is_healthy: true,
			response_time_ms,
			error_message: None,
			last_check: chrono::Utc::now(),
			consecutive_failures: 0,
		}
	}

	pub fn unhealthy(error_message: String, consecutive_failures: u32) -> Self {
		Self {
			is_healthy: false,
			response_time_ms: 0,
			error_message: Some(error_message),
			last_check: chrono::Utc::now(),
			consecutive_failures,
		}
	}
}

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
}

/// Solver operational status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum SolverStatus {
	/// Solver is active and available
	Active,
	/// Solver is temporarily inactive
	Inactive,
	/// Solver has encountered errors
	Error,
	/// Solver is in maintenance mode
	Maintenance,
	/// Solver is being initialized
	Initializing,
}

/// Source of supported assets configuration
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AssetSource {
	/// Assets are manually defined in configuration
	Config,
	/// Assets are auto-discovered from solver API
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

	/// Supported assets/tokens
	pub supported_assets: Vec<Asset>,

	/// Source of the supported assets (config vs auto-discovered)
	pub assets_source: AssetSource,

	/// Custom HTTP headers for requests
	pub headers: Option<HashMap<String, String>>,

	/// Solver-specific configuration
	pub config: HashMap<String, serde_json::Value>,
}

/// Performance and health metrics
#[derive(Debug, Clone, PartialEq)]
pub struct SolverMetrics {
	/// Average response time in milliseconds
	pub avg_response_time_ms: f64,

	/// Success rate (0.0 to 1.0)
	pub success_rate: f64,

	/// Total number of requests made
	pub total_requests: u64,

	/// Number of successful requests
	pub successful_requests: u64,

	/// Number of failed requests
	pub failed_requests: u64,

	/// Number of timeout requests
	pub timeout_requests: u64,

	/// Last health check result
	pub last_health_check: Option<HealthCheckResult>,

	/// Consecutive health check failures
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
			status: SolverStatus::Initializing,
			metadata: SolverMetadata::default(),
			created_at: now,
			last_seen: None,
			metrics: SolverMetrics::new(),
			headers: None,
		}
	}

	/// Check if the solver is available for requests
	pub fn is_available(&self) -> bool {
		matches!(self.status, SolverStatus::Active)
	}

	/// Check if the solver is healthy based on recent metrics
	pub fn is_healthy(&self) -> bool {
		if let Some(ref health_check) = self.metrics.last_health_check {
			health_check.is_healthy && health_check.consecutive_failures < 3
		} else {
			false // No health check data means unhealthy
		}
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
	pub fn record_success(&mut self, response_time_ms: u64) {
		self.metrics.record_success(response_time_ms);
		self.mark_seen();

		// Auto-activate if it was in error state
		if matches!(
			self.status,
			SolverStatus::Error | SolverStatus::Initializing
		) {
			self.status = SolverStatus::Active;
		}
	}

	/// Record a failed request
	pub fn record_failure(&mut self, is_timeout: bool) {
		self.metrics.record_failure(is_timeout);
		self.mark_seen();

		// Auto-deactivate if too many failures
		if self.metrics.consecutive_failures >= 5 {
			self.status = SolverStatus::Error;
		}
	}

	/// Record health check result
	pub fn record_health_check(&mut self, result: HealthCheckResult) {
		self.metrics.consecutive_failures = result.consecutive_failures;
		self.metrics.last_health_check = Some(result.clone());
		self.metrics.last_updated = Utc::now();
		self.mark_seen();

		// Update status based on health check
		if result.is_healthy {
			if matches!(
				self.status,
				SolverStatus::Error | SolverStatus::Initializing
			) {
				self.status = SolverStatus::Active;
			}
		} else if result.consecutive_failures >= 3 {
			self.status = SolverStatus::Error;
		}
	}

	/// Check if solver supports a specific chain
	pub fn supports_chain(&self, chain_id: u64) -> bool {
		self.metadata
			.supported_assets
			.iter()
			.any(|a| a.chain_id == chain_id)
	}

	/// Check if solver supports a specific asset by symbol
	pub fn supports_asset_symbol(&self, symbol: &str) -> bool {
		self.metadata
			.supported_assets
			.iter()
			.any(|a| a.symbol.eq_ignore_ascii_case(symbol))
	}

	/// Check if solver supports a specific asset on a specific chain
	pub fn supports_asset_on_chain(&self, chain_id: u64, address: &str) -> bool {
		self.metadata
			.supported_assets
			.iter()
			.any(|asset| asset.chain_id == chain_id && asset.address.eq_ignore_ascii_case(address))
	}

	/// Check if solver supports a specific asset
	pub fn supports_asset(&self, asset: &Asset) -> bool {
		self.metadata.supported_assets.contains(asset)
	}

	/// Get solver priority score (higher is better)
	/// TODO improve
	pub fn priority_score(&self) -> f64 {
		if !self.is_available() {
			return 0.0;
		}

		let base_score = 100.0;
		let success_rate_bonus = self.metrics.success_rate * 50.0;
		let response_time_penalty = self.metrics.avg_response_time_ms / 100.0;
		let failure_penalty = self.metrics.consecutive_failures as f64 * 10.0;

		(base_score + success_rate_bonus - response_time_penalty - failure_penalty).max(0.0)
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

	pub fn with_assets(mut self, assets: Vec<Asset>) -> Self {
		self.metadata.supported_assets = assets;
		self
	}

	pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
		self.metadata.headers = Some(headers);
		self
	}

	pub fn with_config(mut self, key: String, value: serde_json::Value) -> Self {
		self.metadata.config.insert(key, value);
		self
	}
}

impl Default for SolverMetadata {
	fn default() -> Self {
		Self {
			name: None,
			description: None,
			version: None,
			supported_assets: Vec::new(),
			assets_source: AssetSource::Config, // Default to config-based until determined
			headers: None,
			config: HashMap::new(),
		}
	}
}

impl SolverMetrics {
	pub fn new() -> Self {
		Self {
			avg_response_time_ms: 0.0,
			success_rate: 0.0,
			total_requests: 0,
			successful_requests: 0,
			failed_requests: 0,
			timeout_requests: 0,
			last_health_check: None,
			consecutive_failures: 0,
			last_updated: Utc::now(),
		}
	}

	/// Record a successful request
	pub fn record_success(&mut self, response_time_ms: u64) {
		self.total_requests += 1;
		self.successful_requests += 1;
		self.consecutive_failures = 0;

		// Update average response time
		let total_time = self.avg_response_time_ms * (self.total_requests - 1) as f64;
		self.avg_response_time_ms =
			(total_time + response_time_ms as f64) / self.total_requests as f64;

		// Update success rate
		self.success_rate = self.successful_requests as f64 / self.total_requests as f64;

		self.last_updated = Utc::now();
	}

	/// Record a failed request
	pub fn record_failure(&mut self, is_timeout: bool) {
		self.total_requests += 1;
		self.failed_requests += 1;
		self.consecutive_failures += 1;

		if is_timeout {
			self.timeout_requests += 1;
		}

		// Update success rate
		self.success_rate = self.successful_requests as f64 / self.total_requests as f64;

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
		assert_eq!(solver.status, SolverStatus::Initializing);
		assert!(!solver.is_available());
	}

	#[test]
	fn test_solver_availability() {
		let mut solver = create_test_solver();

		assert!(!solver.is_available());

		solver.update_status(SolverStatus::Active);
		assert!(solver.is_available());

		solver.update_status(SolverStatus::Maintenance);
		assert!(!solver.is_available());
	}

	#[test]
	fn test_metrics_recording() {
		let mut solver = create_test_solver();

		// Record some successes
		solver.record_success(100);
		solver.record_success(200);

		assert_eq!(solver.metrics.total_requests, 2);
		assert_eq!(solver.metrics.successful_requests, 2);
		assert_eq!(solver.metrics.success_rate, 1.0);
		assert_eq!(solver.metrics.avg_response_time_ms, 150.0);

		// Record a failure
		solver.record_failure(false);

		assert_eq!(solver.metrics.total_requests, 3);
		assert_eq!(solver.metrics.failed_requests, 1);
		assert!((solver.metrics.success_rate - 0.6667).abs() < 0.001);
	}

	#[test]
	fn test_priority_score() {
		let mut solver = create_test_solver();
		solver.update_status(SolverStatus::Active);

		// Initial score should be positive
		let initial_score = solver.priority_score();
		assert!(initial_score > 0.0);

		// Record success should improve score
		solver.record_success(100);
		let improved_score = solver.priority_score();
		assert!(improved_score > initial_score);

		// Record failures should decrease score
		solver.record_failure(false);
		solver.record_failure(false);
		let decreased_score = solver.priority_score();
		assert!(decreased_score < improved_score);
	}

	#[test]
	fn test_auto_status_updates() {
		let mut solver = create_test_solver();

		// Success should activate initializing solver
		solver.record_success(100);
		assert_eq!(solver.status, SolverStatus::Active);

		// Multiple failures should deactivate
		for _ in 0..5 {
			solver.record_failure(false);
		}
		assert_eq!(solver.status, SolverStatus::Error);

		// Health check success should reactivate
		let health_result = HealthCheckResult::healthy(150);
		solver.record_health_check(health_result);
		assert_eq!(solver.status, SolverStatus::Active);
	}

	#[test]
	fn test_builder_pattern() {
		let solver = create_test_solver()
			.with_name("Test Solver".to_string())
			.with_version("1.0.0".to_string())
			.with_assets(vec![Asset::new(
				"0x0000000000000000000000000000000000000000".to_string(),
				"ETH".to_string(),
				"Ethereum".to_string(),
				18,
				1,
			)]);

		assert_eq!(solver.metadata.name, Some("Test Solver".to_string()));
		assert_eq!(solver.metadata.version, Some("1.0.0".to_string()));
		assert!(solver.supports_chain(1));
	}

	#[test]
	fn test_supports_asset_on_chain() {
		let solver = create_test_solver().with_assets(vec![
			Asset::new(
				"0x1234567890123456789012345678901234567890".to_string(),
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
				1, // Ethereum
			),
			Asset::new(
				"0x1234567890123456789012345678901234567890".to_string(), // Same address
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
				137, // Polygon
			),
			Asset::new(
				"0xAbCdEf1234567890123456789012345678901234".to_string(),
				"WETH".to_string(),
				"Wrapped Ether".to_string(),
				18,
				1, // Ethereum
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
}
