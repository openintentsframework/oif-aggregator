//! Solver storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
	HealthStatus, Solver, SolverError, SolverMetadata, SolverMetrics, SolverStatus, SupportedAssets,
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
	pub adapter_metadata: Option<serde_json::Value>,

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
	pub supported_assets: SupportedAssets,
	pub headers: Option<HashMap<String, String>>,
}

/// Storage-compatible solver metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverMetricsStorage {
	pub total_requests: u64,
	pub successful_requests: u64,
	pub timeout_requests: u64,
	pub service_errors: u64,
	pub client_errors: u64,
	pub health_status: Option<HealthStatusStorage>,
	pub consecutive_failures: u32,
	pub last_updated: DateTime<Utc>,
}

/// Storage-compatible health status (new improved structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatusStorage {
	pub is_healthy: bool,
	pub last_check_at: DateTime<Utc>,
	pub error_message: Option<String>,
	pub storage_id: String,
}

impl From<HealthStatus> for HealthStatusStorage {
	fn from(health_status: HealthStatus) -> Self {
		Self {
			is_healthy: health_status.is_healthy,
			last_check_at: health_status.last_check_at,
			error_message: health_status.error_message,
			storage_id: format!("health_{}", chrono::Utc::now().timestamp()),
		}
	}
}

impl From<HealthStatusStorage> for HealthStatus {
	fn from(storage: HealthStatusStorage) -> Self {
		Self {
			is_healthy: storage.is_healthy,
			last_check_at: storage.last_check_at,
			error_message: storage.error_message,
		}
	}
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
			adapter_metadata: solver.adapter_metadata,
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
			adapter_metadata: storage.adapter_metadata,
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
}

impl From<SolverMetadata> for SolverMetadataStorage {
	fn from(metadata: SolverMetadata) -> Self {
		Self {
			name: metadata.name,
			description: metadata.description,
			version: metadata.version,
			supported_assets: metadata.supported_assets,
			headers: metadata.headers,
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
			headers: storage.headers,
		}
	}
}

impl From<SolverMetrics> for SolverMetricsStorage {
	fn from(metrics: SolverMetrics) -> Self {
		Self {
			total_requests: metrics.total_requests,
			successful_requests: metrics.successful_requests,
			timeout_requests: metrics.timeout_requests,
			service_errors: metrics.service_errors,
			client_errors: metrics.client_errors,
			health_status: metrics.health_status.map(HealthStatusStorage::from),
			consecutive_failures: metrics.consecutive_failures,
			last_updated: metrics.last_updated,
		}
	}
}

impl TryFrom<SolverMetricsStorage> for SolverMetrics {
	type Error = SolverError;

	fn try_from(storage: SolverMetricsStorage) -> Result<Self, Self::Error> {
		let health_status = storage
			.health_status
			.map(|health_status_storage| health_status_storage.into());

		Ok(SolverMetrics {
			total_requests: storage.total_requests,
			successful_requests: storage.successful_requests,
			timeout_requests: storage.timeout_requests,
			service_errors: storage.service_errors,
			client_errors: storage.client_errors,
			health_status,
			consecutive_failures: storage.consecutive_failures,
			last_updated: storage.last_updated,
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
	fn test_staleness_check() {
		let solver = create_test_solver();
		let mut storage = SolverStorage::from(solver);

		// Fresh storage should not be stale
		assert!(!storage.is_stale(24));

		// Manually set old timestamp
		storage.last_updated = Utc::now() - chrono::Duration::days(2);
		assert!(storage.is_stale(24)); // 24 hours threshold
	}

	#[test]
	fn test_adapter_metadata_storage() {
		// Create test metadata
		let test_metadata = serde_json::json!({
			"auth": {
				"type": "jwt_self_register",
				"client_id": "env:TEST_CLIENT_ID",
				"client_secret": "env:TEST_CLIENT_SECRET"
			},
			"retry_attempts": 3,
			"timeout_ms": 5000
		});

		// Create solver with adapter metadata
		let solver = create_test_solver().with_adapter_metadata(test_metadata.clone());

		// Convert to storage
		let storage = SolverStorage::from(solver);
		assert_eq!(storage.adapter_metadata, Some(test_metadata.clone()));

		// Convert back to domain
		let domain_solver = Solver::try_from(storage).unwrap();
		assert_eq!(domain_solver.adapter_metadata, Some(test_metadata));
	}

	#[test]
	fn test_no_adapter_metadata_storage() {
		// Create solver without adapter metadata
		let solver = create_test_solver();

		// Convert to storage
		let storage = SolverStorage::from(solver);
		assert_eq!(storage.adapter_metadata, None);

		// Convert back to domain
		let domain_solver = Solver::try_from(storage).unwrap();
		assert_eq!(domain_solver.adapter_metadata, None);
	}
}
