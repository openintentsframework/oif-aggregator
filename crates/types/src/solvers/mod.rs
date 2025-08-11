//! Core Solver domain model and business logic

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Asset, Network};

pub mod config;
pub mod errors;
pub mod response;
pub mod storage;

pub use config::{AdapterConfig, AdapterType, SolverConfig};
pub use errors::{
	AdapterError, AdapterResult, HealthCheckResult, SolverError, SolverResult,
	SolverValidationError, SolverValidationResult,
};
pub use response::SolverResponse;
pub use storage::SolverStorage;

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

	/// Timeout for requests to this solver in milliseconds
	pub timeout_ms: u64,

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
}

/// Solver operational status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Solver metadata and configuration
#[derive(Debug, Clone, PartialEq)]
pub struct SolverMetadata {
	/// Human-readable name
	pub name: Option<String>,

	/// Description of the solver
	pub description: Option<String>,

	/// Version of the solver API
	pub version: Option<String>,

	/// Supported blockchain networks
	pub supported_networks: Vec<Network>,

	/// Supported assets/tokens
	pub supported_assets: Vec<Asset>,

	/// Maximum retry attempts for failed requests
	pub max_retries: u32,

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
	pub fn new(solver_id: String, adapter_id: String, endpoint: String, timeout_ms: u64) -> Self {
		let now = Utc::now();

		Self {
			solver_id,
			adapter_id,
			endpoint,
			timeout_ms,
			status: SolverStatus::Initializing,
			metadata: SolverMetadata::default(),
			created_at: now,
			last_seen: None,
			metrics: SolverMetrics::new(),
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

	/// Get the effective timeout (considering health and status)
	pub fn effective_timeout_ms(&self) -> u64 {
		match self.status {
			SolverStatus::Active => {
				if self.metrics.consecutive_failures > 0 {
					// Reduce timeout for unhealthy solvers
					(self.timeout_ms as f64 * 0.8) as u64
				} else {
					self.timeout_ms
				}
			},
			_ => 0, // No requests for inactive solvers
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
		self.metadata.supported_networks.iter().any(|n| n.chain_id == chain_id)
	}

	/// Check if solver supports a specific network
	pub fn supports_network(&self, network: &Network) -> bool {
		self.metadata.supported_networks.contains(network)
	}

	/// Check if solver supports a specific asset by symbol
	pub fn supports_asset_symbol(&self, symbol: &str) -> bool {
		self.metadata
			.supported_assets
			.iter()
			.any(|a| a.symbol.eq_ignore_ascii_case(symbol))
	}

	/// Check if solver supports a specific asset by address
	pub fn supports_asset_address(&self, address: &str) -> bool {
		self.metadata
			.supported_assets
			.iter()
			.any(|a| a.address.eq_ignore_ascii_case(address))
	}

	/// Check if solver supports a specific asset
	pub fn supports_asset(&self, asset: &Asset) -> bool {
		self.metadata.supported_assets.contains(asset)
	}

	/// Get solver priority score (higher is better)
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

	pub fn with_networks(mut self, networks: Vec<Network>) -> Self {
		self.metadata.supported_networks = networks;
		self
	}

	pub fn with_assets(mut self, assets: Vec<Asset>) -> Self {
		self.metadata.supported_assets = assets;
		self
	}

	/// Helper method for backward compatibility
	pub fn with_chain_ids(mut self, chain_ids: Vec<u64>) -> Self {
		self.metadata.supported_networks = chain_ids
			.into_iter()
			.map(|id| Network::new(id, format!("Chain {}", id), false))
			.collect();
		self
	}

	/// Helper method for backward compatibility  
	pub fn with_asset_symbols(mut self, symbols: Vec<String>) -> Self {
		self.metadata.supported_assets = symbols
			.into_iter()
			.enumerate()
			.map(|(i, symbol)| Asset::new(
				format!("0x{:040x}", i), // Generate dummy address
				symbol.clone(),
				symbol,
				18, // Default decimals
				1,  // Default to Ethereum
			))
			.collect();
		self
	}

	pub fn with_max_retries(mut self, retries: u32) -> Self {
		self.metadata.max_retries = retries;
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
			supported_networks: Vec::new(),
			supported_assets: Vec::new(),
			max_retries: 3,
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
			2000,
		)
	}

	#[test]
	fn test_solver_creation() {
		let solver = create_test_solver();

		assert_eq!(solver.solver_id, "test-solver");
		assert_eq!(solver.adapter_id, "oif-v1");
		assert_eq!(solver.endpoint, "https://api.example.com");
		assert_eq!(solver.timeout_ms, 2000);
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
	fn test_chain_support() {
		let solver = create_test_solver().with_chain_ids(vec![1, 137, 42161]);

		assert!(solver.supports_chain(1));
		assert!(solver.supports_chain(137));
		assert!(!solver.supports_chain(56));
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
			.with_chain_ids(vec![1, 137])
			.with_asset_symbols(vec!["uniswap-v3".to_string()])
			.with_max_retries(5);

		assert_eq!(solver.metadata.name, Some("Test Solver".to_string()));
		assert_eq!(solver.metadata.version, Some("1.0.0".to_string()));
		assert_eq!(solver.metadata.supported_networks.len(), 2);
		assert!(solver.supports_chain(1));
		assert!(solver.supports_chain(137));
		assert_eq!(solver.metadata.max_retries, 5);
	}
}
