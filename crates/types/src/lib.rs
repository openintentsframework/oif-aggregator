//! OIF Types
//!
//! Shared models and traits for the Open Intent Framework Aggregator.
//! This crate contains all domain models organized by business entity.

pub mod adapters;
pub mod auth;
pub mod constants;
pub mod integrity;
pub mod metrics;
pub mod models;
pub mod orders;
pub mod quotes;
pub mod solvers;
pub mod storage;

// Re-export chrono and serde_json for convenience
pub use chrono;
pub use serde_json;

// Re-export commonly used types for convenience
pub use integrity::IntegrityPayload;
pub use quotes::{
	Quote, QuoteError, QuoteRequest, QuoteResponse, QuoteResult, QuoteValidationError,
	QuoteValidationResult,
};

pub use solvers::{
	HealthStatus, Solver, SolverConfig, SolverError, SolverResponse, SolverResult, SolverStatus,
	SolverStorage, SolverValidationError, SolverValidationResult,
};

pub use adapters::{
	Adapter, AdapterError, AdapterFactoryError, AdapterResult, AdapterValidationResult,
	SolverAdapter, SolverRuntimeConfig,
};

// Re-export shared domain models
pub use adapters::{
	AdapterQuote, AvailableInput, GetQuoteRequest, GetQuoteResponse, QuoteDetails, QuoteOrder,
	QuotePreference, RequestedOutput, SettlementType, SignatureType,
};
pub use models::{
	Asset, AssetRoute, AssetRouteResponse, InteropAddress, Lock, Network, SecretString,
	SupportedAssetsData, U256,
};

pub use orders::{
	Order, OrderError, OrderRequest, OrderResponse, OrderStatus, OrderStorage,
	OrderValidationError, OrderValidationResult,
};

pub use auth::{
	AuthContext, AuthError, AuthRequest, AuthenticationResult, Authenticator, Permission,
	RateLimitCheck, RateLimitError, RateLimiter, RateLimits,
};

pub use storage::{
	MetricsStorageTrait, OrderStorageTrait, SolverStorageTrait, StorageError, StorageResult,
	StorageTrait,
};

pub use metrics::{
	ErrorType, MetricsAggregate, MetricsBucket, MetricsDataPoint, MetricsTimeSeries,
	RollingMetrics, TimeBucket, TimeRange, TimeWindow,
};
