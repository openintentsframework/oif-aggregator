//! OIF Types
//!
//! Shared models and traits for the Open Intent Framework Aggregator.
//! This crate contains all domain models organized by business entity.

pub mod adapters;
pub mod auth;
pub mod circuit_breaker;
pub mod constants;
pub mod integrity;
pub mod metrics;
pub mod models;
pub mod oif;
pub mod orders;
pub mod quotes;
pub mod solvers;
pub mod storage;

pub mod test_utils;

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
	Solver, SolverConfig, SolverError, SolverResponse, SolverResult, SolverStatus, SolverStorage,
	SolverValidationError, SolverValidationResult,
};

pub use adapters::{
	Adapter, AdapterError, AdapterFactoryError, AdapterResult, AdapterValidationResult,
	SolverAdapter, SolverRuntimeConfig,
};

// Re-export shared domain models
pub use models::{
	Asset, AssetRoute, AssetRouteResponse, InteropAddress, Network, SecretString,
	SupportedAssetsData, U256,
};

// Re-export common OIF models
pub use oif::common::{
	AssetAmount, AssetLockReference, AuthScheme, FailureHandlingMode, Input, IntentType, LockKind,
	OifVersion, OrderStatus, OriginMode, OriginSubmission, Output, QuotePreference, Settlement,
	SettlementType, SignatureType, SwapType,
};

// Re-export OIF v0-specific models
pub use oif::v0::{GetQuoteRequest, GetQuoteResponse};

// Re-export OIF version-agnostic wrappers
pub use oif::{
	OifGetOrderResponse, OifGetQuoteRequest, OifGetQuoteResponse, OifPostOrderRequest,
	OifPostOrderResponse, OifQuote,
};

pub use orders::{
	Order, OrderError, OrderRequest, OrderResponse, OrderStorage, OrderValidationError,
	OrderValidationResult,
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
	ErrorType, MetricsAggregate, MetricsBucket, MetricsComputationError, MetricsDataPoint,
	MetricsTimeSeries, Operation, OperationStats, RollingMetrics, TimeBucket, TimeRange,
	TimeWindow,
};

pub use circuit_breaker::{
	CircuitBreakerState, CircuitDecision, CircuitState, PersistentFailureAction,
};
