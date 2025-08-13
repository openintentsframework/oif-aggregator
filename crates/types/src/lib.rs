//! OIF Types
//!
//! Shared models and traits for the Open Intent Framework Aggregator.
//! This crate contains all domain models organized by business entity.

pub mod adapters;
pub mod auth;
pub mod constants;
pub mod integrity;
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
	HealthCheckResult, Solver, SolverConfig, SolverError, SolverResponse, SolverResult,
	SolverStatus, SolverStorage, SolverValidationError, SolverValidationResult,
};

pub use adapters::{
	Adapter, AdapterError, AdapterFactoryError, AdapterResult, AdapterValidationResult,
	SolverAdapter, SolverRuntimeConfig,
};

// Re-export shared domain models
pub use models::{
	AdapterQuote, Asset, AvailableInput, GetQuoteRequest, GetQuoteResponse, InteropAddress, Lock,
	Network, QuoteDetails, QuoteOrder, QuotePreference, RequestedOutput, SecretString,
	SettlementType, SignatureType, U256,
};

pub use orders::{
	Order, OrderError, OrderResponse, OrderStatus, OrderStorage, OrderValidationError,
	OrderValidationResult, OrdersRequest, OrdersResponse,
};

pub use auth::{
	AuthContext, AuthError, AuthRequest, AuthenticationResult, Authenticator, Permission,
	RateLimitCheck, RateLimitError, RateLimiter, RateLimits,
};

pub use storage::{
	OrderStorageTrait, QuoteStorageTrait, SolverStorageTrait, StorageError, StorageResult,
	StorageTrait,
};
