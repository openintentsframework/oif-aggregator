use std::sync::Arc;

use oif_service::{AggregatorService, OrderService, SolverService};
use oif_storage::Storage;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
	pub aggregator_service: Arc<AggregatorService>,
	pub order_service: Arc<OrderService>,
	pub solver_service: Arc<SolverService>,
	pub storage: Arc<dyn Storage>,
}
