//! Domain models organized by entity
//!
//! This module contains all domain models organized by business entity.
//! Each entity has its own submodule with:
//! - Core domain model (mod.rs)
//! - Request models for API input validation
//! - Response models for API output formatting  
//! - Storage models for persistence
//! - Error types specific to the entity

pub mod adapters;
pub mod intents;
pub mod quotes;
pub mod solvers;

// Re-export commonly used types for convenience
pub use quotes::{
    Quote, QuoteError, QuoteRequest, QuoteResponse, QuoteResult, QuoteStatus, QuoteStorage,
    QuoteValidationError, QuoteValidationResult,
};

pub use solvers::{
    HealthCheckResult, Solver, SolverConfig, SolverError, SolverResponse, SolverResult,
    SolverStatus, SolverStorage, SolverValidationError, SolverValidationResult,
};

pub use adapters::{
    Adapter, AdapterCapability, AdapterConfig, AdapterError, AdapterFactoryError,
    AdapterFactoryResult, AdapterHealthResult, AdapterPerformanceMetrics, AdapterResponse,
    AdapterResult, AdapterStatus, AdapterStorage, AdapterType, AdapterValidationError,
    AdapterValidationResult,
};

pub use intents::{
    Intent, IntentError, IntentExecutionDetail, IntentExecutionResult, IntentPriority,
    IntentResponse, IntentStatus, IntentStorage, IntentSystemHealth, IntentValidationError,
    IntentValidationResult, IntentsRequest, IntentsResponse,
};

// TODO: Add other entities as they are implemented
// pub mod config;
