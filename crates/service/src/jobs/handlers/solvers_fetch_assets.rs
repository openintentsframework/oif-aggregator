//! Bulk solvers asset fetching job handler

use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

use crate::jobs::generic_handler::{BulkFetchAssetsParams, GenericJobHandler};
use crate::jobs::types::{JobError, JobResult};
use crate::solver_repository::SolverServiceTrait;

/// Handler for bulk solvers asset fetching jobs
pub struct SolversFetchAssetsHandler {
	solver_service: Arc<dyn SolverServiceTrait>,
}

impl SolversFetchAssetsHandler {
	/// Create a new bulk solvers asset fetching handler
	pub fn new(solver_service: Arc<dyn SolverServiceTrait>) -> Self {
		Self { solver_service }
	}
}

#[async_trait]
impl GenericJobHandler<BulkFetchAssetsParams> for SolversFetchAssetsHandler {
	async fn handle(&self, _params: BulkFetchAssetsParams) -> JobResult<()> {
		debug!("Starting bulk asset fetch for all solvers");

		// Delegate to the solver service which contains the business logic
		self.solver_service
			.fetch_assets_all_solvers()
			.await
			.map_err(|e| match e {
				crate::solver_repository::SolverServiceError::Storage(msg) => {
					JobError::Storage(msg)
				},
				crate::solver_repository::SolverServiceError::NotFound(msg) => {
					JobError::InvalidConfig(msg)
				},
			})?;

		debug!("Bulk asset fetch completed for all solvers");
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::solver_repository::SolverService;
	use oif_adapters::AdapterRegistry;
	use oif_storage::{MemoryStore, Storage};
	use oif_types::solvers::{AssetSource, SolverMetadata, SolverMetrics};
	use oif_types::{Solver, SolverStatus};
	use std::collections::HashMap;

	/// Helper to create a test solver
	fn create_test_solver(solver_id: &str, status: SolverStatus) -> Solver {
		Solver {
			solver_id: solver_id.to_string(),
			adapter_id: "test-adapter".to_string(),
			endpoint: format!("https://{}.example.com", solver_id),
			timeout_ms: 5000,
			status,
			metadata: SolverMetadata {
				name: Some(format!("{} Solver", solver_id)),
				description: Some("Test solver for unit testing".to_string()),
				version: None,
				supported_assets: vec![],
				assets_source: AssetSource::AutoDiscovered, // Test solver for auto-discovery
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

	/// Helper to create test services
	async fn create_test_solver_service() -> (Arc<dyn Storage>, Arc<SolverService>) {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let solver_service = Arc::new(SolverService::new(storage.clone(), adapter_registry));
		(storage, solver_service)
	}

	#[tokio::test]
	async fn test_solvers_fetch_assets_handler_creation() {
		let (_, solver_service) = create_test_solver_service().await;
		let handler = SolversFetchAssetsHandler::new(solver_service);

		// Just verify we can create the handler without panic
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_solvers_fetch_assets_empty_storage() {
		let (_, solver_service) = create_test_solver_service().await;
		let handler = SolversFetchAssetsHandler::new(solver_service);

		// Should succeed with no solvers
		let params = BulkFetchAssetsParams;
		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_solvers_fetch_assets_with_solvers() {
		let (storage, solver_service) = create_test_solver_service().await;
		let handler = SolversFetchAssetsHandler::new(solver_service);

		// Create and store test solvers with empty assets (will need refresh)
		let solver1 = create_test_solver("solver1", SolverStatus::Active);
		let solver2 = create_test_solver("solver2", SolverStatus::Inactive);

		storage.create_solver(solver1).await.unwrap();
		storage.create_solver(solver2).await.unwrap();

		// This will attempt asset fetching but fail since there are no real adapters
		// We just verify it doesn't panic and returns properly
		let params = BulkFetchAssetsParams;
		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_solvers_fetch_assets_with_old_last_seen() {
		use chrono::{Duration, Utc};

		let (storage, solver_service) = create_test_solver_service().await;
		let handler = SolversFetchAssetsHandler::new(solver_service);

		// Create solver with old last_seen (should need refresh)
		let mut solver = create_test_solver("old-solver", SolverStatus::Active);
		solver.last_seen = Some(Utc::now() - Duration::hours(25));

		storage.create_solver(solver).await.unwrap();

		// This will attempt asset fetching
		let params = BulkFetchAssetsParams;
		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}
}
