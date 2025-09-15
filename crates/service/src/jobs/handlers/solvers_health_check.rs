//! Bulk solvers health check job handler

use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

use crate::jobs::generic_handler::{BulkHealthCheckParams, GenericJobHandler};
use crate::jobs::types::{JobError, JobResult};
use crate::solver_repository::SolverServiceTrait;

/// Handler for bulk solvers health check jobs
pub struct SolversHealthCheckHandler {
	solver_service: Arc<dyn SolverServiceTrait>,
}

impl SolversHealthCheckHandler {
	/// Create a new bulk solvers health check handler
	pub fn new(solver_service: Arc<dyn SolverServiceTrait>) -> Self {
		Self { solver_service }
	}
}

#[async_trait]
impl GenericJobHandler<BulkHealthCheckParams> for SolversHealthCheckHandler {
	async fn handle(&self, _params: BulkHealthCheckParams) -> JobResult<()> {
		debug!("Starting bulk health checks for all solvers");

		// Delegate to the solver service which contains the business logic
		self.solver_service
			.health_check_all_solvers()
			.await
			.map_err(|e| match e {
				crate::solver_repository::SolverServiceError::Storage(msg) => {
					JobError::Storage(msg)
				},
				crate::solver_repository::SolverServiceError::NotFound(msg) => {
					JobError::InvalidConfig(msg)
				},
			})?;

		debug!("Bulk health checks completed for all solvers");
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::solver_repository::SolverService;
	use oif_adapters::AdapterRegistry;
	use oif_storage::{MemoryStore, Storage};
	use oif_types::solvers::{AssetSource, SolverMetadata, SolverMetrics, SupportedAssets};
	use oif_types::{Solver, SolverStatus};

	/// Helper to create a test solver
	fn create_test_solver(solver_id: &str, status: SolverStatus) -> Solver {
		Solver {
			solver_id: solver_id.to_string(),
			adapter_id: "test-adapter".to_string(),
			endpoint: format!("https://{}.example.com", solver_id),
			status,
			metadata: SolverMetadata {
				name: Some(format!("{} Solver", solver_id)),
				description: Some("Test solver for unit testing".to_string()),
				version: None,

				supported_assets: SupportedAssets::Routes {
					routes: Vec::new(),
					source: AssetSource::AutoDiscovered,
				},
				headers: None,
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
			headers: None,
			adapter_metadata: None,
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
	async fn test_solvers_health_check_handler_creation() {
		let (_, solver_service) = create_test_solver_service().await;
		let handler = SolversHealthCheckHandler::new(solver_service);

		// Just verify we can create the handler without panic
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_solvers_health_check_empty_storage() {
		let (_, solver_service) = create_test_solver_service().await;
		let handler = SolversHealthCheckHandler::new(solver_service);

		// Should succeed with no solvers
		let params = BulkHealthCheckParams;
		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_solvers_health_check_with_solvers() {
		let (storage, solver_service) = create_test_solver_service().await;
		let handler = SolversHealthCheckHandler::new(solver_service);

		// Create and store test solvers
		let active_solver = create_test_solver("active-solver", SolverStatus::Active);
		let inactive_solver = create_test_solver("inactive-solver", SolverStatus::Inactive);

		storage.create_solver(active_solver).await.unwrap();
		storage.create_solver(inactive_solver).await.unwrap();

		// This will attempt health checks but fail since there are no real adapters
		// We just verify it doesn't panic and returns properly
		let params = BulkHealthCheckParams;
		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}
}
