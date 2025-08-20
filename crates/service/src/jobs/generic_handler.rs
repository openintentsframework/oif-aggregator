//! Generic job handler with type-safe parameters

use async_trait::async_trait;
use std::sync::Arc;

use crate::jobs::types::JobResult;
use crate::solver_repository::SolverServiceTrait;

/// Generic trait for job handlers with typed parameters
#[async_trait]
pub trait GenericJobHandler<T>: Send + Sync {
	async fn handle(&self, params: T) -> JobResult<()>;
}

/// Job parameter types
#[derive(Debug, Clone)]
pub struct SolverHealthCheckParams {
	pub solver_id: String,
}

#[derive(Debug, Clone)]
pub struct SolverFetchAssetsParams {
	pub solver_id: String,
}

#[derive(Debug, Clone)]
pub struct BulkHealthCheckParams;

#[derive(Debug, Clone)]
pub struct BulkFetchAssetsParams;

/// Universal job handler that can handle any job type via SolverService
pub struct UniversalJobHandler {
	solver_service: Arc<dyn SolverServiceTrait>,
}

impl UniversalJobHandler {
	pub fn new(solver_service: Arc<dyn SolverServiceTrait>) -> Self {
		Self { solver_service }
	}
}
