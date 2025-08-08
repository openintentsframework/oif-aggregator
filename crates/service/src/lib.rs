//! OIF Service
//!
//! Core logic for quote aggregation and scoring.

pub mod aggregator;
pub mod order;
pub mod quote;
pub mod solver;

pub use aggregator::AggregatorService;
pub use order::{OrderService, OrderServiceError};
pub use solver::{SolverService, SolverServiceError};
