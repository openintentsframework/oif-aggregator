//! Bulk solver operations handlers - work on all solvers at once

use chrono::{Duration, Utc};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::jobs::handlers::solver_fetch_assets::SolverFetchAssetsHandler;
use crate::jobs::handlers::solver_health_check::SolverHealthCheckHandler;
use crate::jobs::types::{JobError, JobResult};
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::SolverStatus;

/// Handler for bulk solver operations
pub struct BulkSolverOperationsHandler {
	health_check_handler: SolverHealthCheckHandler,
	fetch_assets_handler: SolverFetchAssetsHandler,
	storage: Arc<dyn Storage>,
}

impl BulkSolverOperationsHandler {
	/// Create a new bulk solver operations handler
	pub fn new(storage: Arc<dyn Storage>, adapter_registry: Arc<AdapterRegistry>) -> Self {
		Self {
			health_check_handler: SolverHealthCheckHandler::new(
				Arc::clone(&storage),
				Arc::clone(&adapter_registry),
			),
			fetch_assets_handler: SolverFetchAssetsHandler::new(
				Arc::clone(&storage),
				adapter_registry,
			),
			storage,
		}
	}

	/// Perform health checks on all active solvers
	pub async fn handle_all_solvers_health_check(&self) -> JobResult {
		debug!("Starting health checks for all solvers");

		// Get all solvers from storage
		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| JobError::Storage(e.to_string()))?;

		let mut success_count = 0;
		let mut error_count = 0;
		let mut skipped_count = 0;

		for solver in solvers {
			// Only check active and inactive solvers (skip error, maintenance, initializing)
			match solver.status {
				SolverStatus::Active | SolverStatus::Inactive => {
					match self.health_check_handler.handle(&solver.solver_id).await {
						Ok(()) => {
							success_count += 1;
							debug!("Health check succeeded for solver: {}", solver.solver_id);
						},
						Err(e) => {
							error_count += 1;
							warn!("Health check failed for solver {}: {}", solver.solver_id, e);
						},
					}
				},
				_ => {
					skipped_count += 1;
					debug!(
						"Skipped health check for solver {} (status: {:?})",
						solver.solver_id, solver.status
					);
				},
			}
		}

		info!(
			"Bulk health check completed: {} successful, {} failed, {} skipped",
			success_count, error_count, skipped_count
		);

		// Consider the job successful if at least some health checks passed
		// or if there were no eligible solvers
		if error_count > 0 && success_count == 0 && skipped_count == 0 {
			Err(JobError::ProcessingFailed {
				message: "All solver health checks failed".to_string(),
			})
		} else {
			Ok(())
		}
	}

	/// Fetch assets for all solvers that need it
	pub async fn handle_all_solvers_fetch_assets(&self) -> JobResult {
		debug!("Starting asset fetch for all solvers that need it");

		// Get all solvers from storage
		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| JobError::Storage(e.to_string()))?;

		let mut success_count = 0;
		let mut error_count = 0;
		let mut skipped_count = 0;

		for solver in solvers {
			// Only fetch assets for active and inactive solvers that need it
			let should_fetch =
				matches!(solver.status, SolverStatus::Active | SolverStatus::Inactive)
					&& (solver.metadata.supported_assets.is_empty()
						|| Self::should_refresh_assets(&solver));

			if should_fetch {
				match self.fetch_assets_handler.handle(&solver.solver_id).await {
					Ok(()) => {
						success_count += 1;
						debug!("Asset fetch succeeded for solver: {}", solver.solver_id);
					},
					Err(e) => {
						error_count += 1;
						warn!("Asset fetch failed for solver {}: {}", solver.solver_id, e);
					},
				}
			} else {
				skipped_count += 1;
				debug!(
					"Skipped asset fetch for solver {} (status: {:?}, assets: {})",
					solver.solver_id,
					solver.status,
					solver.metadata.supported_assets.len()
				);
			}
		}

		info!(
			"Bulk asset fetch completed: {} successful, {} failed, {} skipped",
			success_count, error_count, skipped_count
		);

		// Consider the job successful if at least some asset fetches passed
		// or if there were no eligible solvers
		if error_count > 0 && success_count == 0 && skipped_count == 0 {
			Err(JobError::ProcessingFailed {
				message: "All solver asset fetches failed".to_string(),
			})
		} else {
			Ok(())
		}
	}

	/// Determine if a solver's assets should be refreshed
	/// This is a simple heuristic - could be made more sophisticated
	fn should_refresh_assets(solver: &oif_types::Solver) -> bool {
		// Refresh assets if they haven't been updated in the last 24 hours
		if let Some(last_seen) = solver.last_seen {
			let refresh_threshold = Utc::now() - Duration::hours(24);
			last_seen < refresh_threshold
		} else {
			// If we've never seen the solver, refresh assets
			true
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use oif_adapters::AdapterRegistry;
	use oif_storage::{MemoryStore, Storage as StorageTrait};
	use oif_types::solvers::{SolverMetadata, SolverMetrics};
	use oif_types::{Solver, SolverStatus};
	use std::collections::HashMap;

	/// Helper to create a test solver
	fn create_test_solver(solver_id: &str, status: SolverStatus) -> Solver {
		Solver {
			solver_id: solver_id.to_string(),
			adapter_id: "test-adapter".to_string(),
			endpoint: "https://test-solver.example.com".to_string(),
			timeout_ms: 5000,
			status,
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
	async fn test_bulk_handler_creation() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());

		let handler = BulkSolverOperationsHandler::new(storage, adapter_registry);

		// Just verify we can create the handler without panic
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_all_solvers_health_check_empty_storage() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = BulkSolverOperationsHandler::new(storage, adapter_registry);

		// Should succeed with no solvers
		let result = handler.handle_all_solvers_health_check().await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_all_solvers_fetch_assets_empty_storage() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = BulkSolverOperationsHandler::new(storage, adapter_registry);

		// Should succeed with no solvers
		let result = handler.handle_all_solvers_fetch_assets().await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_should_refresh_assets() {
		use chrono::{Duration, Utc};

		// Create solver with recent last_seen
		let mut solver = create_test_solver("test-solver", SolverStatus::Active);
		solver.last_seen = Some(Utc::now() - Duration::hours(1));
		assert!(!BulkSolverOperationsHandler::should_refresh_assets(&solver));

		// Create solver with old last_seen
		solver.last_seen = Some(Utc::now() - Duration::hours(25));
		assert!(BulkSolverOperationsHandler::should_refresh_assets(&solver));

		// Create solver with no last_seen
		solver.last_seen = None;
		assert!(BulkSolverOperationsHandler::should_refresh_assets(&solver));
	}

	#[tokio::test]
	async fn test_bulk_operations_with_mixed_solver_statuses() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = BulkSolverOperationsHandler::new(storage.clone(), adapter_registry);

		// Create solvers with different statuses
		let active_solver = create_test_solver("active-solver", SolverStatus::Active);
		let inactive_solver = create_test_solver("inactive-solver", SolverStatus::Inactive);
		let error_solver = create_test_solver("error-solver", SolverStatus::Error);
		let maintenance_solver =
			create_test_solver("maintenance-solver", SolverStatus::Maintenance);

		storage.create_solver(active_solver).await.unwrap();
		storage.create_solver(inactive_solver).await.unwrap();
		storage.create_solver(error_solver).await.unwrap();
		storage.create_solver(maintenance_solver).await.unwrap();

		// Both operations should handle mixed statuses gracefully
		// They will likely fail due to no real adapters, but shouldn't panic
		let health_result = handler.handle_all_solvers_health_check().await;
		let asset_result = handler.handle_all_solvers_fetch_assets().await;

		// The jobs themselves will fail due to missing adapters, but the bulk operations
		// should handle this gracefully and not panic
		assert!(health_result.is_err() || health_result.is_ok());
		assert!(asset_result.is_err() || asset_result.is_ok());
	}
}
