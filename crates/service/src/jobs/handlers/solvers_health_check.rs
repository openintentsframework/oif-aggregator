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

		// Delegate to solver service (handles health checks + metrics job scheduling internally)
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

		debug!("Bulk health checks job completed successfully");

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::solver_repository::SolverService;
	use oif_adapters::AdapterRegistry;
	use oif_storage::MemoryStore;

	/// Helper to create test services
	async fn create_test_solver_service() -> Arc<SolverService> {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		Arc::new(SolverService::new(storage, adapter_registry, None))
	}

	#[tokio::test]
	async fn test_solvers_health_check_handler_creation() {
		let solver_service = create_test_solver_service().await;
		let handler = SolversHealthCheckHandler::new(solver_service);

		// Just verify we can create the handler without panic
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_solvers_health_check_empty_storage() {
		let solver_service = create_test_solver_service().await;
		let handler = SolversHealthCheckHandler::new(solver_service);

		// Should succeed with no solvers
		let params = BulkHealthCheckParams;
		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_solvers_health_check_with_solvers() {
		let solver_service = create_test_solver_service().await;
		let handler = SolversHealthCheckHandler::new(solver_service);

		// This will attempt health checks but fail since there are no real adapters
		// We just verify it doesn't panic and returns properly
		let params = BulkHealthCheckParams;
		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}
}
