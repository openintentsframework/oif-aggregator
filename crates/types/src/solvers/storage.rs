//! Solver storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Asset;

use super::{
	AssetSource, HealthCheckResult, Solver, SolverError, SolverMetadata, SolverMetrics,
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
	pub status: SolverStatus,
	pub metadata: SolverMetadataStorage,
	pub metrics: SolverMetricsStorage,
	pub headers: Option<HashMap<String, String>>,

	// Storage-specific metadata
	pub version: u32,
	pub last_updated: DateTime<Utc>,
	pub created_at: DateTime<Utc>,
	pub last_seen: Option<DateTime<Utc>>,
}

/// Storage-compatible solver metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverMetadataStorage {
	pub name: Option<String>,
	pub description: Option<String>,
	pub version: Option<String>,
	pub supported_assets: Vec<Asset>,
	pub assets_source: AssetSource,
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

impl From<Solver> for SolverStorage {
	fn from(solver: Solver) -> Self {
		Self {
			solver_id: solver.solver_id,
			adapter_id: solver.adapter_id,
			endpoint: solver.endpoint,
			status: solver.status,
			metadata: SolverMetadataStorage::from(solver.metadata),
			created_at: solver.created_at,
			last_seen: solver.last_seen,
			metrics: SolverMetricsStorage::from(solver.metrics),
			headers: solver.headers,
			version: 1,
			last_updated: Utc::now(),
		}
	}
}

impl TryFrom<SolverStorage> for Solver {
	type Error = SolverError;

	fn try_from(storage: SolverStorage) -> Result<Self, Self::Error> {
		Ok(Solver {
			solver_id: storage.solver_id,
			adapter_id: storage.adapter_id,
			endpoint: storage.endpoint,
			status: storage.status,
			metadata: storage.metadata.into(),
			created_at: storage.created_at,
			last_seen: storage.last_seen,
			metrics: storage.metrics.try_into()?,
			headers: storage.headers,
		})
	}
}

impl SolverStorage {
	/// Mark as accessed for analytics
	pub fn mark_accessed(&mut self) {
		self.last_updated = Utc::now();
	}

	/// Update status and timestamp
	pub fn update_status(&mut self, status: SolverStatus) {
		self.status = status;
		self.last_seen = Some(Utc::now());
		self.last_updated = Utc::now();
		self.mark_accessed();
	}

	/// Update metrics from domain metrics
	pub fn update_metrics(&mut self, metrics: SolverMetrics) {
		self.metrics = SolverMetricsStorage::from(metrics);
		self.last_updated = Utc::now();
		self.mark_accessed();
	}

	/// Check if solver is stale (hasn't been updated recently)
	pub fn is_stale(&self, max_age_hours: i64) -> bool {
		let stale_threshold = Utc::now() - chrono::Duration::hours(max_age_hours);
		self.last_updated < stale_threshold
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
		self.last_updated = Utc::now();
	}
}

impl From<SolverMetadata> for SolverMetadataStorage {
	fn from(metadata: SolverMetadata) -> Self {
		Self {
			name: metadata.name,
			description: metadata.description,
			version: metadata.version,
			supported_assets: metadata.supported_assets,
			assets_source: metadata.assets_source,
			headers: metadata.headers,
			config: metadata.config,
		}
	}
}

impl From<SolverMetadataStorage> for SolverMetadata {
	fn from(storage: SolverMetadataStorage) -> Self {
		Self {
			name: storage.name,
			description: storage.description,
			version: storage.version,
			supported_assets: storage.supported_assets,
			assets_source: storage.assets_source,
			headers: storage.headers,
			config: storage.config,
		}
	}
}

impl From<SolverMetrics> for SolverMetricsStorage {
	fn from(metrics: SolverMetrics) -> Self {
		Self {
			avg_response_time_ms: metrics.avg_response_time_ms,
			success_rate: metrics.success_rate,
			total_requests: metrics.total_requests,
			successful_requests: metrics.successful_requests,
			failed_requests: metrics.failed_requests,
			timeout_requests: metrics.timeout_requests,
			last_health_check: metrics.last_health_check.map(HealthCheckStorage::from),
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
}

impl TryFrom<SolverMetricsStorage> for SolverMetrics {
	type Error = SolverError;

	fn try_from(storage: SolverMetricsStorage) -> Result<Self, Self::Error> {
		Ok(SolverMetrics {
			avg_response_time_ms: storage.avg_response_time_ms,
			success_rate: storage.success_rate,
			total_requests: storage.total_requests,
			successful_requests: storage.successful_requests,
			failed_requests: storage.failed_requests,
			timeout_requests: storage.timeout_requests,
			last_health_check: storage
				.last_health_check
				.map(|hc| hc.try_into())
				.transpose()?,
			consecutive_failures: storage.consecutive_failures,
			last_updated: storage.last_updated,
		})
	}
}

impl From<HealthCheckResult> for HealthCheckStorage {
	fn from(health_check: HealthCheckResult) -> Self {
		Self {
			is_healthy: health_check.is_healthy,
			response_time_ms: health_check.response_time_ms,
			error_message: health_check.error_message,
			last_check: health_check.last_check,
			consecutive_failures: health_check.consecutive_failures,
			check_id: uuid::Uuid::new_v4().to_string(),
		}
	}
}

impl TryFrom<HealthCheckStorage> for HealthCheckResult {
	type Error = SolverError;

	fn try_from(storage: HealthCheckStorage) -> Result<Self, Self::Error> {
		Ok(HealthCheckResult {
			is_healthy: storage.is_healthy,
			response_time_ms: storage.response_time_ms,
			error_message: storage.error_message,
			last_check: storage.last_check,
			consecutive_failures: storage.consecutive_failures,
		})
	}
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
		)
		.with_name("Test Solver".to_string())
	}

	#[test]
	fn test_storage_conversion() {
		let solver = create_test_solver();
		let solver_id = solver.solver_id.clone();

		// Convert to storage
		let storage = SolverStorage::from(solver);
		assert_eq!(storage.solver_id, solver_id);
		assert_eq!(storage.version, 1);

		// Convert back to domain
		let domain_solver = Solver::try_from(storage).unwrap();
		assert_eq!(domain_solver.solver_id, solver_id);
	}

	#[test]
	fn test_storage_compaction() {
		let solver = create_test_solver();
		let mut storage = SolverStorage::from(solver);

		// Add some data that should be compacted
		storage.metrics.last_error_message = Some("Old error".to_string());
		storage.metrics.last_error_timestamp = Some(Utc::now() - chrono::Duration::days(10));

		storage.compact();

		// Error message should be cleared for old errors
		assert!(storage.metrics.last_error_message.is_none());
		assert!(storage.metrics.last_error_timestamp.is_none());
	}

	#[test]
	fn test_staleness_check() {
		let solver = create_test_solver();
		let mut storage = SolverStorage::from(solver);

		// Fresh storage should not be stale
		assert!(!storage.is_stale(24));

		// Manually set old timestamp
		storage.last_updated = Utc::now() - chrono::Duration::days(2);
		assert!(storage.is_stale(24)); // 24 hours threshold
	}
}
