//! Solver storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
	HealthCheckResult, Solver, SolverError, SolverMetadata, SolverMetrics, SolverResult,
	SolverStatus,
};

/// Storage representation of a solver
///
/// This model is optimized for storage and persistence.
/// It can be converted to/from the domain Solver model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverStorage {
	pub solver_id: String,
	pub adapter_id: String,
	pub endpoint: String,
	pub timeout_ms: u64,
	pub status: SolverStatusStorage,
	pub metadata: SolverMetadataStorage,
	pub created_at: DateTime<Utc>,
	pub last_seen: Option<DateTime<Utc>>,
	pub metrics: SolverMetricsStorage,

	// Storage-specific metadata
	pub version: u32,
	pub last_updated: DateTime<Utc>,
	pub access_count: u64,
	pub storage_size_bytes: u64,
}

/// Storage-compatible solver status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SolverStatusStorage {
	Active,
	Inactive,
	Error,
	Maintenance,
	Initializing,
}

/// Storage-compatible solver metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverMetadataStorage {
	pub name: Option<String>,
	pub description: Option<String>,
	pub version: Option<String>,
	pub supported_chains: Vec<u64>,
	pub supported_protocols: Vec<String>,
	pub max_retries: u32,
	pub headers: Option<HashMap<String, String>>,
	pub config: HashMap<String, serde_json::Value>,
}

/// Storage-compatible solver metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverMetricsStorage {
	pub avg_response_time_ms: f64,
	pub success_rate: f64,
	pub total_requests: u64,
	pub successful_requests: u64,
	pub failed_requests: u64,
	pub timeout_requests: u64,
	pub last_health_check: Option<HealthCheckStorage>,
	pub consecutive_failures: u32,
	pub last_updated: DateTime<Utc>,

	// Extended storage metrics
	pub min_response_time_ms: u64,
	pub max_response_time_ms: u64,
	pub p95_response_time_ms: u64,
	pub requests_per_hour: f64,
	pub uptime_percentage: f64,
	pub last_error_message: Option<String>,
	pub last_error_timestamp: Option<DateTime<Utc>>,
}

/// Storage-compatible health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckStorage {
	pub is_healthy: bool,
	pub response_time_ms: u64,
	pub error_message: Option<String>,
	pub last_check: DateTime<Utc>,
	pub consecutive_failures: u32,
	pub check_id: String,
}

impl SolverStorage {
	/// Create storage solver from domain solver
	pub fn from_domain(solver: Solver) -> Self {
		let storage_size = estimate_storage_size(&solver);

		Self {
			solver_id: solver.solver_id,
			adapter_id: solver.adapter_id,
			endpoint: solver.endpoint,
			timeout_ms: solver.timeout_ms,
			status: SolverStatusStorage::from_domain(&solver.status),
			metadata: SolverMetadataStorage::from_domain(solver.metadata),
			created_at: solver.created_at,
			last_seen: solver.last_seen,
			metrics: SolverMetricsStorage::from_domain(solver.metrics),
			version: 1,
			last_updated: Utc::now(),
			access_count: 0,
			storage_size_bytes: storage_size,
		}
	}

	/// Convert storage solver to domain solver
	pub fn to_domain(self) -> SolverResult<Solver> {
		Ok(Solver {
			solver_id: self.solver_id,
			adapter_id: self.adapter_id,
			endpoint: self.endpoint,
			timeout_ms: self.timeout_ms,
			status: self.status.to_domain(),
			metadata: self.metadata.to_domain(),
			created_at: self.created_at,
			last_seen: self.last_seen,
			metrics: self.metrics.to_domain()?,
		})
	}

	/// Mark as accessed for analytics
	pub fn mark_accessed(&mut self) {
		self.access_count += 1;
		self.last_updated = Utc::now();
	}

	/// Update status and timestamp
	pub fn update_status(&mut self, status: SolverStatusStorage) {
		self.status = status;
		self.last_seen = Some(Utc::now());
		self.last_updated = Utc::now();
		self.mark_accessed();
	}

	/// Update metrics from domain metrics
	pub fn update_metrics(&mut self, metrics: SolverMetrics) {
		self.metrics = SolverMetricsStorage::from_domain(metrics);
		self.last_updated = Utc::now();
		self.mark_accessed();
	}

	/// Check if solver is stale (hasn't been updated recently)
	pub fn is_stale(&self, max_age_hours: i64) -> bool {
		let stale_threshold = Utc::now() - chrono::Duration::hours(max_age_hours);
		self.last_updated < stale_threshold
	}

	/// Get storage statistics
	pub fn storage_stats(&self) -> SolverStorageStats {
		SolverStorageStats {
			solver_id: self.solver_id.clone(),
			version: self.version,
			storage_size_bytes: self.storage_size_bytes,
			access_count: self.access_count,
			created_at: self.created_at,
			last_updated: self.last_updated,
			is_stale: self.is_stale(24), // 24 hours
		}
	}

	/// Compact the storage (remove old data, optimize size)
	pub fn compact(&mut self) {
		// Reset extended metrics to save space
		self.metrics.min_response_time_ms = 0;
		self.metrics.max_response_time_ms = 0;
		self.metrics.p95_response_time_ms = 0;

		// Clear old error messages
		if let Some(error_time) = self.metrics.last_error_timestamp {
			if Utc::now() - error_time > chrono::Duration::days(7) {
				self.metrics.last_error_message = None;
				self.metrics.last_error_timestamp = None;
			}
		}

		// Update storage size
		self.storage_size_bytes = estimate_storage_size_from_storage(self);
		self.last_updated = Utc::now();
	}
}

impl SolverStatusStorage {
	fn from_domain(status: &SolverStatus) -> Self {
		match status {
			SolverStatus::Active => Self::Active,
			SolverStatus::Inactive => Self::Inactive,
			SolverStatus::Error => Self::Error,
			SolverStatus::Maintenance => Self::Maintenance,
			SolverStatus::Initializing => Self::Initializing,
		}
	}

	fn to_domain(&self) -> SolverStatus {
		match self {
			Self::Active => SolverStatus::Active,
			Self::Inactive => SolverStatus::Inactive,
			Self::Error => SolverStatus::Error,
			Self::Maintenance => SolverStatus::Maintenance,
			Self::Initializing => SolverStatus::Initializing,
		}
	}
}

impl SolverMetadataStorage {
	fn from_domain(metadata: SolverMetadata) -> Self {
		Self {
			name: metadata.name,
			description: metadata.description,
			version: metadata.version,
			supported_chains: metadata.supported_chains,
			supported_protocols: metadata.supported_protocols,
			max_retries: metadata.max_retries,
			headers: metadata.headers,
			config: metadata.config,
		}
	}

	fn to_domain(self) -> SolverMetadata {
		SolverMetadata {
			name: self.name,
			description: self.description,
			version: self.version,
			supported_chains: self.supported_chains,
			supported_protocols: self.supported_protocols,
			max_retries: self.max_retries,
			headers: self.headers,
			config: self.config,
		}
	}
}

impl SolverMetricsStorage {
	fn from_domain(metrics: SolverMetrics) -> Self {
		Self {
			avg_response_time_ms: metrics.avg_response_time_ms,
			success_rate: metrics.success_rate,
			total_requests: metrics.total_requests,
			successful_requests: metrics.successful_requests,
			failed_requests: metrics.failed_requests,
			timeout_requests: metrics.timeout_requests,
			last_health_check: metrics
				.last_health_check
				.map(HealthCheckStorage::from_domain),
			consecutive_failures: metrics.consecutive_failures,
			last_updated: metrics.last_updated,

			// Initialize extended metrics
			min_response_time_ms: if metrics.total_requests > 0 {
				metrics.avg_response_time_ms as u64
			} else {
				0
			},
			max_response_time_ms: if metrics.total_requests > 0 {
				metrics.avg_response_time_ms as u64
			} else {
				0
			},
			p95_response_time_ms: if metrics.total_requests > 0 {
				(metrics.avg_response_time_ms * 1.2) as u64 // Estimate
			} else {
				0
			},
			requests_per_hour: 0.0, // Would be calculated from historical data
			uptime_percentage: if metrics.total_requests > 0 {
				metrics.success_rate * 100.0
			} else {
				0.0
			},
			last_error_message: None,
			last_error_timestamp: None,
		}
	}

	fn to_domain(self) -> SolverResult<SolverMetrics> {
		Ok(SolverMetrics {
			avg_response_time_ms: self.avg_response_time_ms,
			success_rate: self.success_rate,
			total_requests: self.total_requests,
			successful_requests: self.successful_requests,
			failed_requests: self.failed_requests,
			timeout_requests: self.timeout_requests,
			last_health_check: self
				.last_health_check
				.map(|hc| hc.to_domain())
				.transpose()?,
			consecutive_failures: self.consecutive_failures,
			last_updated: self.last_updated,
		})
	}
}

impl HealthCheckStorage {
	fn from_domain(health_check: HealthCheckResult) -> Self {
		Self {
			is_healthy: health_check.is_healthy,
			response_time_ms: health_check.response_time_ms,
			error_message: health_check.error_message,
			last_check: health_check.last_check,
			consecutive_failures: health_check.consecutive_failures,
			check_id: uuid::Uuid::new_v4().to_string(),
		}
	}

	fn to_domain(self) -> SolverResult<HealthCheckResult> {
		Ok(HealthCheckResult {
			is_healthy: self.is_healthy,
			response_time_ms: self.response_time_ms,
			error_message: self.error_message,
			last_check: self.last_check,
			consecutive_failures: self.consecutive_failures,
		})
	}
}

/// Storage statistics for a solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverStorageStats {
	pub solver_id: String,
	pub version: u32,
	pub storage_size_bytes: u64,
	pub access_count: u64,
	pub created_at: DateTime<Utc>,
	pub last_updated: DateTime<Utc>,
	pub is_stale: bool,
}

/// Storage query filters for solvers
#[derive(Debug, Clone)]
pub struct SolverStorageFilter {
	pub solver_id: Option<String>,
	pub adapter_id: Option<String>,
	pub status: Option<SolverStatusStorage>,
	pub supported_chain: Option<u64>,
	pub enabled_only: bool,
	pub healthy_only: bool,
	pub created_after: Option<DateTime<Utc>>,
	pub created_before: Option<DateTime<Utc>>,
	pub last_seen_after: Option<DateTime<Utc>>,
	pub min_success_rate: Option<f64>,
	pub max_response_time_ms: Option<f64>,
}

impl Default for SolverStorageFilter {
	fn default() -> Self {
		Self {
			solver_id: None,
			adapter_id: None,
			status: None,
			supported_chain: None,
			enabled_only: false,
			healthy_only: false,
			created_after: None,
			created_before: None,
			last_seen_after: None,
			min_success_rate: None,
			max_response_time_ms: None,
		}
	}
}

impl SolverStorageFilter {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_solver_id(mut self, solver_id: String) -> Self {
		self.solver_id = Some(solver_id);
		self
	}

	pub fn with_adapter_id(mut self, adapter_id: String) -> Self {
		self.adapter_id = Some(adapter_id);
		self
	}

	pub fn with_status(mut self, status: SolverStatusStorage) -> Self {
		self.status = Some(status);
		self
	}

	pub fn with_chain_support(mut self, chain_id: u64) -> Self {
		self.supported_chain = Some(chain_id);
		self
	}

	pub fn enabled_only(mut self) -> Self {
		self.enabled_only = true;
		self
	}

	pub fn healthy_only(mut self) -> Self {
		self.healthy_only = true;
		self
	}

	pub fn with_min_success_rate(mut self, rate: f64) -> Self {
		self.min_success_rate = Some(rate);
		self
	}

	pub fn with_max_response_time(mut self, ms: f64) -> Self {
		self.max_response_time_ms = Some(ms);
		self
	}

	/// Check if a solver matches this filter
	pub fn matches(&self, solver: &SolverStorage) -> bool {
		if let Some(ref solver_id) = self.solver_id {
			if solver.solver_id != *solver_id {
				return false;
			}
		}

		if let Some(ref adapter_id) = self.adapter_id {
			if solver.adapter_id != *adapter_id {
				return false;
			}
		}

		if let Some(ref status) = self.status {
			if solver.status != *status {
				return false;
			}
		}

		if let Some(chain_id) = self.supported_chain {
			if !solver.metadata.supported_chains.contains(&chain_id) {
				return false;
			}
		}

		if self.enabled_only && !matches!(solver.status, SolverStatusStorage::Active) {
			return false;
		}

		if self.healthy_only {
			if let Some(ref health) = solver.metrics.last_health_check {
				if !health.is_healthy {
					return false;
				}
			} else {
				return false;
			}
		}

		if let Some(created_after) = self.created_after {
			if solver.created_at <= created_after {
				return false;
			}
		}

		if let Some(created_before) = self.created_before {
			if solver.created_at >= created_before {
				return false;
			}
		}

		if let Some(last_seen_after) = self.last_seen_after {
			if let Some(last_seen) = solver.last_seen {
				if last_seen <= last_seen_after {
					return false;
				}
			} else {
				return false;
			}
		}

		if let Some(min_rate) = self.min_success_rate {
			if solver.metrics.success_rate < min_rate {
				return false;
			}
		}

		if let Some(max_time) = self.max_response_time_ms {
			if solver.metrics.avg_response_time_ms > max_time {
				return false;
			}
		}

		true
	}
}

/// Conversion traits
impl From<Solver> for SolverStorage {
	fn from(solver: Solver) -> Self {
		Self::from_domain(solver)
	}
}

impl TryFrom<SolverStorage> for Solver {
	type Error = SolverError;

	fn try_from(storage: SolverStorage) -> Result<Self, Self::Error> {
		storage.to_domain()
	}
}

/// Estimate storage size for a solver (in bytes)
fn estimate_storage_size(solver: &Solver) -> u64 {
	let base_size = 512; // Base struct size
	let strings_size = solver.solver_id.len()
		+ solver.adapter_id.len()
		+ solver.endpoint.len()
		+ solver.metadata.name.as_ref().map_or(0, |s| s.len())
		+ solver.metadata.description.as_ref().map_or(0, |s| s.len())
		+ solver.metadata.version.as_ref().map_or(0, |s| s.len());
	let chains_size = solver.metadata.supported_chains.len() * 8;
	let protocols_size = solver
		.metadata
		.supported_protocols
		.iter()
		.map(|s| s.len())
		.sum::<usize>();
	let headers_size = solver.metadata.headers.as_ref().map_or(0, |h| {
		h.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>()
	});
	let config_size = solver
		.metadata
		.config
		.iter()
		.map(|(k, v)| k.len() + v.to_string().len())
		.sum::<usize>();

	(base_size + strings_size + chains_size + protocols_size + headers_size + config_size) as u64
}

/// Estimate storage size from storage model
fn estimate_storage_size_from_storage(storage: &SolverStorage) -> u64 {
	let base_size = 512; // Base struct size
	let strings_size = storage.solver_id.len()
		+ storage.adapter_id.len()
		+ storage.endpoint.len()
		+ storage.metadata.name.as_ref().map_or(0, |s| s.len())
		+ storage.metadata.description.as_ref().map_or(0, |s| s.len())
		+ storage.metadata.version.as_ref().map_or(0, |s| s.len());
	let metrics_size = 256; // Estimated metrics size
	let health_check_size = storage
		.metrics
		.last_health_check
		.as_ref()
		.map_or(0, |_| 128);

	(base_size + strings_size + metrics_size + health_check_size) as u64
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::solvers::Solver;

	fn create_test_solver() -> Solver {
		Solver::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
			2000,
		)
		.with_name("Test Solver".to_string())
		.with_chains(vec![1, 137])
	}

	#[test]
	fn test_storage_conversion() {
		let solver = create_test_solver();
		let solver_id = solver.solver_id.clone();

		// Convert to storage
		let storage = SolverStorage::from_domain(solver);
		assert_eq!(storage.solver_id, solver_id);
		assert_eq!(storage.version, 1);
		assert_eq!(storage.access_count, 0);
		assert!(storage.storage_size_bytes > 0);

		// Convert back to domain
		let domain_solver = storage.to_domain().unwrap();
		assert_eq!(domain_solver.solver_id, solver_id);
	}

	#[test]
	fn test_access_tracking() {
		let solver = create_test_solver();
		let mut storage = SolverStorage::from_domain(solver);

		assert_eq!(storage.access_count, 0);

		storage.mark_accessed();
		assert_eq!(storage.access_count, 1);

		storage.mark_accessed();
		assert_eq!(storage.access_count, 2);
	}

	#[test]
	fn test_status_conversion() {
		assert_eq!(
			SolverStatusStorage::from_domain(&SolverStatus::Active),
			SolverStatusStorage::Active
		);
		assert_eq!(
			SolverStatusStorage::Active.to_domain(),
			SolverStatus::Active
		);
	}

	#[test]
	fn test_storage_filter() {
		let solver = create_test_solver();
		let storage = SolverStorage::from_domain(solver);

		// Test basic filter
		let filter = SolverStorageFilter::new()
			.with_solver_id("test-solver".to_string())
			.with_adapter_id("oif-v1".to_string());
		assert!(filter.matches(&storage));

		// Test non-matching filter
		let filter = SolverStorageFilter::new().with_solver_id("different-solver".to_string());
		assert!(!filter.matches(&storage));

		// Test chain support filter
		let filter = SolverStorageFilter::new().with_chain_support(1);
		assert!(filter.matches(&storage));

		let filter = SolverStorageFilter::new().with_chain_support(56); // Not supported
		assert!(!filter.matches(&storage));
	}

	#[test]
	fn test_storage_stats() {
		let solver = create_test_solver();
		let mut storage = SolverStorage::from_domain(solver);

		storage.mark_accessed();
		storage.mark_accessed();

		let stats = storage.storage_stats();
		assert_eq!(stats.solver_id, "test-solver");
		assert_eq!(stats.access_count, 2);
		assert_eq!(stats.version, 1);
		assert!(stats.storage_size_bytes > 0);
		assert!(!stats.is_stale); // Recently created
	}

	#[test]
	fn test_storage_compaction() {
		let solver = create_test_solver();
		let mut storage = SolverStorage::from_domain(solver);

		// Add some data that should be compacted
		storage.metrics.last_error_message = Some("Old error".to_string());
		storage.metrics.last_error_timestamp = Some(Utc::now() - chrono::Duration::days(10));

		let _size_before = storage.storage_size_bytes;
		storage.compact();

		// Error message should be cleared for old errors
		assert!(storage.metrics.last_error_message.is_none());
		assert!(storage.metrics.last_error_timestamp.is_none());
	}

	#[test]
	fn test_staleness_check() {
		let solver = create_test_solver();
		let mut storage = SolverStorage::from_domain(solver);

		// Fresh storage should not be stale
		assert!(!storage.is_stale(24));

		// Manually set old timestamp
		storage.last_updated = Utc::now() - chrono::Duration::days(2);
		assert!(storage.is_stale(24)); // 24 hours threshold
	}
}
