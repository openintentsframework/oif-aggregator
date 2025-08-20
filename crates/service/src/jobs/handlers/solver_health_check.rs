//! Solver health check job handler

use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

use crate::jobs::generic_handler::{GenericJobHandler, SolverHealthCheckParams};
use crate::jobs::types::{JobError, JobResult};
use crate::solver_repository::SolverServiceTrait;

/// Handler for individual solver health check jobs
pub struct SolverHealthCheckHandler {
	solver_service: Arc<dyn SolverServiceTrait>,
}

impl SolverHealthCheckHandler {
	/// Create a new solver health check handler
	pub fn new(solver_service: Arc<dyn SolverServiceTrait>) -> Self {
		Self { solver_service }
	}
}

#[async_trait]
impl GenericJobHandler<SolverHealthCheckParams> for SolverHealthCheckHandler {
	async fn handle(&self, params: SolverHealthCheckParams) -> JobResult<()> {
		debug!("Starting health check for solver: {}", params.solver_id);

		// Delegate to the solver service which contains the business logic
		self.solver_service
			.health_check_solver(&params.solver_id)
			.await
			.map_err(|e| match e {
				crate::solver_repository::SolverServiceError::Storage(msg) => {
					JobError::Storage(msg)
				},
				crate::solver_repository::SolverServiceError::NotFound(msg) => {
					JobError::InvalidConfig(msg)
				},
			})?;

		debug!("Health check completed for solver: {}", params.solver_id);
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
		Arc::new(SolverService::new(storage, adapter_registry))
	}

	#[tokio::test]
	async fn test_health_check_handler_creation() {
		let solver_service = create_test_solver_service().await;
		let handler = SolverHealthCheckHandler::new(solver_service);

		// Just verify we can create the handler without panic
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_health_check_solver_not_found() {
		let solver_service = create_test_solver_service().await;
		let handler = SolverHealthCheckHandler::new(solver_service);

		let params = SolverHealthCheckParams {
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
	async fn test_health_check_with_generic_interface() {
		let solver_service = create_test_solver_service().await;
		let handler = SolverHealthCheckHandler::new(solver_service);

		// Test the generic interface with type-safe parameters
		let params = SolverHealthCheckParams {
			solver_id: "test-solver".to_string(),
		};

		// This will fail since solver doesn't exist, but demonstrates the generic interface
		let result = handler.handle(params).await;
		assert!(result.is_err());
	}
}
