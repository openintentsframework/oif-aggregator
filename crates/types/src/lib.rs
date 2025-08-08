//! OIF Types
//!
//! Shared models and traits for the Open Intent Framework Aggregator.
//! This crate contains all domain models organized by business entity.

pub mod adapters;
pub mod auth;
pub mod orders;
pub mod quotes;
pub mod solvers;
pub mod storage;

// Re-export chrono and serde_json for convenience
pub use chrono;
pub use serde_json;

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
	AdapterValidationResult, SolverAdapter,
};

pub use orders::{
	Order, OrderError, OrderExecutionDetail, OrderExecutionResult, OrderPriority, OrderResponse,
	OrderStatus, OrderStorage, OrderSystemHealth, OrderValidationError, OrderValidationResult,
	OrdersRequest, OrdersResponse,
};

pub use auth::{
	ApiKeyAuthenticator, AuthContext, AuthError, AuthRequest, AuthenticationResult, Authenticator,
	MemoryRateLimiter, NoAuthenticator, Permission, RateLimitCheck, RateLimitError, RateLimiter,
	RateLimits, SimpleRoleAuthenticator,
};

pub use storage::{
	OrderStorageTrait, QuoteStorageTrait, SolverStorageTrait, StorageError, StorageResult,
	StorageStats, StorageTrait,
};
