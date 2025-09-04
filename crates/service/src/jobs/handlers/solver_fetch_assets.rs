//! Solver asset fetching job handler

use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

use crate::jobs::generic_handler::{GenericJobHandler, SolverFetchAssetsParams};
use crate::jobs::types::{JobError, JobResult};
use crate::solver_repository::SolverServiceTrait;

/// Handler for individual solver asset fetching jobs
pub struct SolverFetchAssetsHandler {
	solver_service: Arc<dyn SolverServiceTrait>,
}

impl SolverFetchAssetsHandler {
	/// Create a new solver fetch assets handler
	pub fn new(solver_service: Arc<dyn SolverServiceTrait>) -> Self {
		Self { solver_service }
	}
}

#[async_trait]
impl GenericJobHandler<SolverFetchAssetsParams> for SolverFetchAssetsHandler {
	async fn handle(&self, params: SolverFetchAssetsParams) -> JobResult<()> {
		debug!("Starting route fetch for solver: {}", params.solver_id);

		// Delegate to the solver service which contains the business logic
		self.solver_service
			.fetch_and_update_assets(&params.solver_id)
			.await
			.map_err(|e| match e {
				crate::solver_repository::SolverServiceError::Storage(msg) => {
					JobError::Storage(msg)
				},
				crate::solver_repository::SolverServiceError::NotFound(msg) => {
					JobError::InvalidConfig(msg)
				},
			})?;

		debug!("Route fetch completed for solver: {}", params.solver_id);
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
	fn create_test_solver() -> Solver {
		Solver {
			solver_id: "test-solver".to_string(),
			adapter_id: "test-adapter".to_string(),
			endpoint: "https://test-solver.example.com".to_string(),
			status: SolverStatus::Inactive,
			metadata: SolverMetadata {
				name: Some("Test Solver".to_string()),
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
	async fn test_fetch_assets_handler_creation() {
		let (_, solver_service) = create_test_solver_service().await;

		let handler = SolverFetchAssetsHandler::new(solver_service);

		// Just verify we can create the handler without panic
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_fetch_assets_solver_not_found() {
		let (_, solver_service) = create_test_solver_service().await;
		let handler = SolverFetchAssetsHandler::new(solver_service);

		let params = SolverFetchAssetsParams {
			solver_id: "nonexistent-solver".to_string(),
		};
		let result = handler.handle(params).await;

		assert!(result.is_err());
		if let Err(JobError::InvalidConfig(msg)) = result {
			assert!(msg.contains("not found"));
		} else {
			panic!("Expected InvalidConfig error");
		}
	}

	#[tokio::test]
	async fn test_fetch_assets_with_existing_solver() {
		let (storage, solver_service) = create_test_solver_service().await;
		let handler = SolverFetchAssetsHandler::new(solver_service);

		// Create and store a test solver
		let test_solver = create_test_solver();
		storage.create_solver(test_solver.clone()).await.unwrap();

		// The asset fetch will likely fail since we don't have a real adapter,
		// but we should get a proper error, not a panic
		let params = SolverFetchAssetsParams {
			solver_id: "test-solver".to_string(),
		};
		let result = handler.handle(params).await;

		// We expect this to fail with a Storage error since there's no real adapter
		assert!(result.is_err());
		if let Err(JobError::Storage(_)) = result {
			// This is expected - no real adapter available
		} else {
			panic!("Expected Storage error, got: {:?}", result);
		}
	}

	#[tokio::test]
	async fn test_fetch_assets_handles_partial_failures() {
		// This test would ideally use a mock adapter that fails asset fetch but succeeds network fetch
		// For now, we just verify the handler doesn't panic with valid input
		let (storage, solver_service) = create_test_solver_service().await;
		let handler = SolverFetchAssetsHandler::new(solver_service);

		let test_solver = create_test_solver();
		storage.create_solver(test_solver.clone()).await.unwrap();

		// This will fail but shouldn't panic
		let params = SolverFetchAssetsParams {
			solver_id: "test-solver".to_string(),
		};
		let result = handler.handle(params).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_fetch_assets_empty_solver_id() {
		let (_, solver_service) = create_test_solver_service().await;
		let handler = SolverFetchAssetsHandler::new(solver_service);

		let params = SolverFetchAssetsParams {
			solver_id: "".to_string(),
		};
		let result = handler.handle(params).await;

		assert!(result.is_err());
		// Should fail because empty solver ID won't be found
		if let Err(JobError::InvalidConfig(_)) = result {
			// This is expected
		} else {
			panic!("Expected InvalidConfig error for empty solver ID");
		}
	}
}
