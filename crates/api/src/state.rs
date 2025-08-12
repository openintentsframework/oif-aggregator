use std::sync::Arc;

use oif_service::{AggregatorService, IntegrityService, OrderService, SolverService};
use oif_storage::Storage;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
	pub aggregator_service: Arc<AggregatorService>,
	pub order_service: Arc<OrderService>,
	pub solver_service: Arc<SolverService>,
	pub integrity_service: Arc<IntegrityService>,
	pub storage: Arc<dyn Storage>,
}
