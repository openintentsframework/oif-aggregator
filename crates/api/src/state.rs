use std::sync::Arc;

use oif_service::{
	AggregatorTrait, IntegrityTrait, JobProcessor, OrderServiceTrait, SolverServiceTrait,
};
use oif_storage::Storage;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
	pub aggregator_service: Arc<dyn AggregatorTrait>,
	pub order_service: Arc<dyn OrderServiceTrait>,
	pub solver_service: Arc<dyn SolverServiceTrait>,
	pub integrity_service: Arc<dyn IntegrityTrait>,
	pub storage: Arc<dyn Storage>,
	/// Background job processor for maintenance tasks
	pub job_processor: Arc<JobProcessor>,
}
