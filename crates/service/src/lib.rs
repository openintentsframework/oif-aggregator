//! OIF Service
//!
//! Core logic for quote aggregation and scoring.

pub mod aggregator;
pub mod integrity;
pub mod order;
pub mod solver;
pub mod solver_adapter_service;

pub use aggregator::{AggregatorResult, AggregatorService, AggregatorServiceError};
pub use integrity::IntegrityService;
pub use oif_types::IntegrityPayload;
pub use order::{OrderService, OrderServiceError};
pub use solver::{SolverService, SolverServiceError, SolverStats};
pub use solver_adapter_service::{SolverAdapterError, SolverAdapterService, SolverAdapterTrait};
