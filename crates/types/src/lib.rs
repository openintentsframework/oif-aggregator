//! OIF Types
//!
//! Shared models and traits for the Open Intent Framework Aggregator.
//! This crate contains all domain models organized by business entity.

pub mod adapters;
pub mod auth;
pub mod models;
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
	Adapter, AdapterConfig, AdapterError, AdapterFactoryError, AdapterFactoryResult,
	AdapterResponse, AdapterDetailResponse, AdapterConfigResponse, AdapterNetworksResponse,
	AdapterAssetsResponse, AdapterResult, AdapterStorage, AdapterMetadataStorage, AdapterType, 
	AdapterValidationError, AdapterValidationResult, AssetResponse, AssetStorage, 
	NetworkResponse, NetworkStorage, SolverAdapter,
};

// Re-export shared domain models
pub use models::{Asset, Network};

// Re-export adapter-specific types
pub use adapters::OrderDetails;

pub use orders::{
	Order, OrderError, OrderResponse, OrderStatus, OrderStorage, OrderValidationError,
	OrderValidationResult, OrdersRequest, OrdersResponse,
};

pub use auth::{
	ApiKeyAuthenticator, AuthContext, AuthError, AuthRequest, AuthenticationResult, Authenticator,
	MemoryRateLimiter, NoAuthenticator, Permission, RateLimitCheck, RateLimitError, RateLimiter,
	RateLimits, SimpleRoleAuthenticator,
};

pub use storage::{
	OrderStorageTrait, QuoteStorageTrait, SolverStorageTrait, StorageError, StorageResult,
	StorageTrait,
};
