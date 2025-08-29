//! OIF Service
//!
//! Core logic for quote aggregation and scoring.

pub mod aggregator;
pub mod integrity;
pub mod jobs;
pub mod order;
pub mod solver_adapter;
pub mod solver_filter;
pub mod solver_repository;

pub use aggregator::{
	AggregatorResult, AggregatorService, AggregatorServiceError, AggregatorTrait, SolverTaskResult,
	TaskExecutor, TaskExecutorTrait,
};

#[cfg(test)]
pub use aggregator::{MockAggregatorTrait, MockTaskExecutorTrait};
#[cfg(test)]
pub use integrity::MockIntegrityTrait;
pub use integrity::{IntegrityError, IntegrityService, IntegrityTrait};
pub use oif_types::models::health::SolverStats;
pub use oif_types::IntegrityPayload;
#[cfg(test)]
pub use order::MockOrderServiceTrait;
pub use order::{OrderService, OrderServiceError, OrderServiceTrait};
pub use solver_adapter::{SolverAdapterError, SolverAdapterService, SolverAdapterTrait};
pub use solver_filter::{
	Compatibility, CompatibilityAnalyzer, CompatibilityAnalyzerTrait, SolverFilterService,
	SolverFilterTrait, SolverSelector, SolverSelectorTrait,
};
#[cfg(test)]
pub use solver_filter::{
	MockCompatibilityAnalyzerTrait, MockSolverFilterTrait, MockSolverSelectorTrait,
};
#[cfg(test)]
pub use solver_repository::MockSolverServiceTrait;
pub use solver_repository::{SolverService, SolverServiceError, SolverServiceTrait};

// Background job processing
#[cfg(test)]
pub use jobs::processor::JobHandler;
pub use jobs::{
	BackgroundJob, BackgroundJobHandler, JobError, JobInfo, JobInfoStats, JobProcessor,
	JobProcessorConfig, JobResult, JobStatus, RetryPolicy, ScheduledJob,
};
