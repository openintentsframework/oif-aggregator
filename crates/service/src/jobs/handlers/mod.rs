//! Job handler implementations organized by functionality

use async_trait::async_trait;
use std::sync::Arc;

use crate::{AggregatorTrait, IntegrityTrait, SolverServiceTrait};
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;

use super::generic_handler::{
	BulkFetchAssetsParams, BulkHealthCheckParams, GenericJobHandler, SolverFetchAssetsParams,
	SolverHealthCheckParams,
};
use super::processor::JobHandler;
use super::types::{BackgroundJob, JobResult};

pub mod solver_fetch_assets;
pub mod solver_health_check;
pub mod solvers_fetch_assets;
pub mod solvers_health_check;

// Re-export handler structs for convenience
pub use solver_fetch_assets::SolverFetchAssetsHandler;
pub use solver_health_check::SolverHealthCheckHandler;
pub use solvers_fetch_assets::SolversFetchAssetsHandler;
pub use solvers_health_check::SolversHealthCheckHandler;

/// Handler for background jobs (solver management, system maintenance, etc.)
#[allow(dead_code)]
pub struct BackgroundJobHandler {
	health_check_handler: SolverHealthCheckHandler,
	fetch_assets_handler: SolverFetchAssetsHandler,
	solvers_health_check_handler: SolversHealthCheckHandler,
	solvers_fetch_assets_handler: SolversFetchAssetsHandler,
	solver_service: Arc<dyn SolverServiceTrait>,
	aggregator_service: Arc<dyn AggregatorTrait>,
	integrity_service: Arc<dyn IntegrityTrait>,
}

impl BackgroundJobHandler {
	/// Create a new background job handler
	pub fn new(
		_storage: Arc<dyn Storage>,
		_adapter_registry: Arc<AdapterRegistry>,
		solver_service: Arc<dyn SolverServiceTrait>,
		aggregator_service: Arc<dyn AggregatorTrait>,
		integrity_service: Arc<dyn IntegrityTrait>,
	) -> Self {
		Self {
			health_check_handler: SolverHealthCheckHandler::new(Arc::clone(&solver_service)),
			fetch_assets_handler: SolverFetchAssetsHandler::new(Arc::clone(&solver_service)),
			solvers_health_check_handler: SolversHealthCheckHandler::new(Arc::clone(
				&solver_service,
			)),
			solvers_fetch_assets_handler: SolversFetchAssetsHandler::new(Arc::clone(
				&solver_service,
			)),
			solver_service,
			aggregator_service,
			integrity_service,
		}
	}
}

#[async_trait]
impl JobHandler for BackgroundJobHandler {
	async fn handle(&self, job: BackgroundJob) -> JobResult {
		match job {
			BackgroundJob::SolverHealthCheck { solver_id } => {
				let params = SolverHealthCheckParams { solver_id };
				self.health_check_handler.handle(params).await
			},
			BackgroundJob::FetchSolverAssets { solver_id } => {
				let params = SolverFetchAssetsParams { solver_id };
				self.fetch_assets_handler.handle(params).await
			},
			BackgroundJob::AllSolversHealthCheck => {
				let params = BulkHealthCheckParams;
				self.solvers_health_check_handler.handle(params).await
			},
			BackgroundJob::AllSolversFetchAssets => {
				let params = BulkFetchAssetsParams;
				self.solvers_fetch_assets_handler.handle(params).await
			},
		}
	}
}
