//! Solver health check job handler

use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::solver_adapter::{SolverAdapterService, SolverAdapterTrait};
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::SolverStatus;

use crate::jobs::types::{JobError, JobResult};

/// Handler for solver health check jobs
pub struct SolverHealthCheckHandler {
	storage: Arc<dyn Storage>,
	adapter_registry: Arc<AdapterRegistry>,
}

impl SolverHealthCheckHandler {
	/// Create a new solver health check handler
	pub fn new(storage: Arc<dyn Storage>, adapter_registry: Arc<AdapterRegistry>) -> Self {
		Self {
			storage,
			adapter_registry,
		}
	}

	/// Perform health check on a solver
	pub async fn handle(&self, solver_id: &str) -> JobResult {
		debug!("Starting health check for solver: {}", solver_id);

		// Get solver from storage
		let solver = self
			.storage
			.get_solver(solver_id)
			.await
			.map_err(|e| JobError::Storage(e.to_string()))?
			.ok_or_else(|| JobError::InvalidConfig(format!("Solver '{}' not found", solver_id)))?;

		// Create adapter service for the solver
		let adapter_service =
			SolverAdapterService::from_solver(solver.clone(), Arc::clone(&self.adapter_registry))
				.map_err(|e| JobError::Adapter(e.to_string()))?;

		// Perform health check
		let is_healthy = adapter_service
			.health_check()
			.await
			.map_err(|e| JobError::Adapter(e.to_string()))?;

		// Update solver status and metrics
		let new_status = if is_healthy {
			info!("Health check passed for solver: {}", solver_id);
			SolverStatus::Active
		} else {
			warn!("Health check failed for solver: {}", solver_id);
			SolverStatus::Error
		};

		// Update solver in storage
		let mut updated_solver = solver.clone();
		updated_solver.status = new_status;
		updated_solver.last_seen = Some(Utc::now());

		if is_healthy {
			updated_solver.metrics.successful_requests += 1;
		} else {
			updated_solver.metrics.failed_requests += 1;
			updated_solver.metrics.consecutive_failures += 1;
		}

		self.storage
			.update_solver(updated_solver)
			.await
			.map_err(|e| JobError::Storage(e.to_string()))?;

		debug!("Health check completed for solver: {}", solver_id);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use oif_adapters::AdapterRegistry;
	use oif_storage::{MemoryStore, Storage};
	use oif_types::solvers::{SolverMetadata, SolverMetrics};
	use oif_types::{Solver, SolverStatus};
	use std::collections::HashMap;

	/// Helper to create a test solver
	fn create_test_solver() -> Solver {
		Solver {
			solver_id: "test-solver".to_string(),
			adapter_id: "test-adapter".to_string(),
			endpoint: "https://test-solver.example.com".to_string(),
			timeout_ms: 5000,
			status: SolverStatus::Inactive,
			metadata: SolverMetadata {
				name: Some("Test Solver".to_string()),
				description: Some("Test solver for unit testing".to_string()),
				version: None,
				supported_assets: vec![],
				max_retries: 3,
				headers: None,
				config: HashMap::new(),
			},
			created_at: chrono::Utc::now(),
			last_seen: None,
			metrics: SolverMetrics {
				avg_response_time_ms: 0.0,
				success_rate: 0.0,
				total_requests: 0,
				successful_requests: 0,
				failed_requests: 0,
				timeout_requests: 0,
				last_health_check: None,
				consecutive_failures: 0,
				last_updated: chrono::Utc::now(),
			},
		}
	}

	#[tokio::test]
	async fn test_health_check_handler_creation() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());

		let handler = SolverHealthCheckHandler::new(storage, adapter_registry);

		// Just verify we can create the handler without panic
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_health_check_solver_not_found() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = SolverHealthCheckHandler::new(storage, adapter_registry);

		let result = handler.handle("nonexistent-solver").await;

		assert!(result.is_err());
		if let Err(JobError::InvalidConfig(msg)) = result {
			assert!(msg.contains("not found"));
		} else {
			panic!("Expected InvalidConfig error");
		}
	}

	#[tokio::test]
	async fn test_health_check_with_existing_solver() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = SolverHealthCheckHandler::new(storage.clone(), adapter_registry);

		// Create and store a test solver
		let test_solver = create_test_solver();
		storage.create_solver(test_solver.clone()).await.unwrap();

		// The health check will likely fail since we don't have a real adapter,
		// but we should get a proper error, not a panic
		let result = handler.handle("test-solver").await;

		// We expect this to fail with an Adapter error since there's no real adapter
		assert!(result.is_err());
		if let Err(JobError::Adapter(_)) = result {
			// This is expected - no real adapter available
		} else {
			panic!("Expected Adapter error, got: {:?}", result);
		}
	}

	#[tokio::test]
	async fn test_health_check_updates_solver_metrics() {
		// This test would ideally use a mock adapter that we can control
		// For now, we just verify the handler doesn't panic with valid input
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = SolverHealthCheckHandler::new(storage.clone(), adapter_registry);

		let test_solver = create_test_solver();
		storage.create_solver(test_solver.clone()).await.unwrap();

		// This will fail but shouldn't panic
		let result = handler.handle("test-solver").await;
		assert!(result.is_err());
	}
}
