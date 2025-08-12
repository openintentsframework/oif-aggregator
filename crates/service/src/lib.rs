//! OIF Service
//!
//! Core logic for quote aggregation and scoring.

pub mod aggregator;
pub mod integrity;
pub mod order;
pub mod solver;

pub use aggregator::AggregatorService;
pub use integrity::{IntegrityPayload, IntegrityService};
pub use order::{OrderService, OrderServiceError};
pub use solver::{SolverService, SolverServiceError};
