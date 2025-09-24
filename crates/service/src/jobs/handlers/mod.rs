//! Job handler implementations organized by functionality

use async_trait::async_trait;
use std::sync::Arc;

use crate::jobs::JobScheduler;
use crate::{order::OrderServiceTrait, AggregatorTrait, IntegrityTrait, SolverServiceTrait};
use oif_adapters::AdapterRegistry;
use oif_config::Settings;
use oif_storage::Storage;

use super::generic_handler::{
	BulkFetchAssetsParams, BulkHealthCheckParams, GenericJobHandler, SolverFetchAssetsParams,
	SolverHealthCheckParams,
};
use super::processor::JobHandler;
use super::types::{BackgroundJob, JobResult};
use orders_cleanup::OrdersCleanupParams;

pub mod metrics_cleanup;
pub mod metrics_update;
pub mod order_status_monitor;
pub mod orders_cleanup;
pub mod solver_fetch_assets;
pub mod solver_health_check;
pub mod solvers_fetch_assets;
pub mod solvers_health_check;

// Re-export handler structs for convenience
pub use metrics_cleanup::MetricsCleanupHandler;
pub use metrics_update::MetricsUpdateHandler;
pub use order_status_monitor::OrderStatusMonitorHandler;
pub use orders_cleanup::OrdersCleanupHandler;
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
	orders_cleanup_handler: OrdersCleanupHandler,
	order_status_monitor_handler: OrderStatusMonitorHandler,
	metrics_update_handler: MetricsUpdateHandler,
	metrics_cleanup_handler: MetricsCleanupHandler,
	// Core services available to all handlers
	solver_service: Arc<dyn SolverServiceTrait>,
	aggregator_service: Arc<dyn AggregatorTrait>,
	integrity_service: Arc<dyn IntegrityTrait>,
	order_service: Arc<dyn OrderServiceTrait>,
	storage: Arc<dyn Storage>,
	settings: Settings,
}

impl BackgroundJobHandler {
	/// Create a new background job handler
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		storage: Arc<dyn Storage>,
		_adapter_registry: Arc<AdapterRegistry>,
		solver_service: Arc<dyn SolverServiceTrait>,
		aggregator_service: Arc<dyn AggregatorTrait>,
		integrity_service: Arc<dyn IntegrityTrait>,
		order_service: Arc<dyn OrderServiceTrait>,
		job_scheduler: Arc<dyn JobScheduler>,
		settings: Settings,
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
			orders_cleanup_handler: OrdersCleanupHandler::new(Arc::clone(&order_service)),
			order_status_monitor_handler: OrderStatusMonitorHandler::new(
				Arc::clone(&order_service),
				Arc::clone(&storage),
				Arc::clone(&job_scheduler),
				None, // Use default monitoring configuration
			),
			metrics_update_handler: MetricsUpdateHandler::new(
				Arc::clone(&storage),
				settings.clone(),
			),
			metrics_cleanup_handler: MetricsCleanupHandler::new(
				Arc::clone(&storage),
				settings.clone(),
			),
			solver_service,
			aggregator_service,
			integrity_service,
			order_service,
			storage,
			settings,
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
			BackgroundJob::OrdersCleanup => {
				let retention_days = self.settings.get_order_retention_days();
				let params = OrdersCleanupParams::new(retention_days);
				self.orders_cleanup_handler.handle(params).await
			},
			BackgroundJob::OrderStatusMonitor { order_id, attempt } => {
				// Delegate to the order status monitor handler
				self.order_status_monitor_handler
					.handle_order_monitoring(&order_id, attempt)
					.await
			},
			BackgroundJob::AggregationMetricsUpdate {
				aggregation_id,
				solver_metrics,
				..
			} => {
				// Delegate to the metrics update handler
				self.metrics_update_handler
					.handle_aggregation_metrics_update(&aggregation_id, solver_metrics)
					.await
			},
			BackgroundJob::MetricsCleanup => {
				self.metrics_cleanup_handler.handle_metrics_cleanup().await
			},
		}
	}
}
