//! OIF Service
//!
//! Core logic for quote aggregation and scoring.

pub mod aggregator;
pub mod quote;
pub mod order;

pub use aggregator::AggregatorService;
pub use order::{OrderService, OrderServiceError};
