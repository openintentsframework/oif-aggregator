//! Job handler implementations organized by functionality

use async_trait::async_trait;
use std::sync::Arc;

use crate::{AggregatorTrait, IntegrityTrait, SolverServiceTrait};
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;

use super::processor::JobHandler;
use super::types::{BackgroundJob, JobResult};

pub mod bulk_solver_operations;
pub mod solver_fetch_assets;
pub mod solver_health_check;

// Re-export handler structs for convenience
pub use bulk_solver_operations::BulkSolverOperationsHandler;
pub use solver_fetch_assets::SolverFetchAssetsHandler;
pub use solver_health_check::SolverHealthCheckHandler;

/// Handler for background jobs (solver management, system maintenance, etc.)
pub struct BackgroundJobHandler {
	health_check_handler: SolverHealthCheckHandler,
	fetch_assets_handler: SolverFetchAssetsHandler,
	bulk_operations_handler: BulkSolverOperationsHandler,
	solver_service: Arc<dyn SolverServiceTrait>,
	aggregator_service: Arc<dyn AggregatorTrait>,
	integrity_service: Arc<dyn IntegrityTrait>,
}

impl BackgroundJobHandler {
	/// Create a new background job handler
	pub fn new(
		storage: Arc<dyn Storage>,
		adapter_registry: Arc<AdapterRegistry>,
		solver_service: Arc<dyn SolverServiceTrait>,
		aggregator_service: Arc<dyn AggregatorTrait>,
		integrity_service: Arc<dyn IntegrityTrait>,
	) -> Self {
		Self {
			health_check_handler: SolverHealthCheckHandler::new(
				Arc::clone(&storage),
				Arc::clone(&adapter_registry),
			),
			fetch_assets_handler: SolverFetchAssetsHandler::new(
				Arc::clone(&storage),
				Arc::clone(&adapter_registry),
			),
			bulk_operations_handler: BulkSolverOperationsHandler::new(storage, adapter_registry),
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
				self.health_check_handler.handle(&solver_id).await
			},
			BackgroundJob::FetchSolverAssets { solver_id } => {
				self.fetch_assets_handler.handle(&solver_id).await
			},
			BackgroundJob::AllSolversHealthCheck => {
				self.bulk_operations_handler
					.handle_all_solvers_health_check()
					.await
			},
			BackgroundJob::AllSolversFetchAssets => {
				self.bulk_operations_handler
					.handle_all_solvers_fetch_assets()
					.await
			},
		}
	}
}
