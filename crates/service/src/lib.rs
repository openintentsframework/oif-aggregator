//! OIF Service
//!
//! Core logic for quote aggregation and scoring.

pub mod aggregator;
pub mod integrity;
pub mod order;
pub mod solver;
pub mod solver_adapter_service;

pub use aggregator::{
	AggregatorResult, AggregatorService, AggregatorServiceError, AggregatorTrait,
};

#[cfg(test)]
pub use aggregator::MockAggregatorTrait;
#[cfg(test)]
pub use integrity::MockIntegrityTrait;
pub use integrity::{IntegrityService, IntegrityTrait};
pub use oif_types::IntegrityPayload;
#[cfg(test)]
pub use order::MockOrderServiceTrait;
pub use order::{OrderService, OrderServiceError, OrderServiceTrait};
#[cfg(test)]
pub use solver::MockSolverServiceTrait;
pub use solver::{SolverService, SolverServiceError, SolverServiceTrait, SolverStats};
pub use solver_adapter_service::{SolverAdapterError, SolverAdapterService, SolverAdapterTrait};
