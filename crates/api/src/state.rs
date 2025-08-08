use std::sync::Arc;

use oif_service::{AggregatorService, OrderService};
use oif_storage::Storage;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub aggregator_service: Arc<AggregatorService>,
    pub storage: Arc<dyn Storage>,
    pub order_service: Arc<OrderService>,
}


