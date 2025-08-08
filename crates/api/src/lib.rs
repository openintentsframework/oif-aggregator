//! OIF API
//!
//! Axum-based API with routes and middleware for the Open Intent Framework Aggregator.

pub mod auth;
pub mod security;
pub mod handlers;
pub mod router;
pub mod state;

pub use router::create_router;
pub use state::AppState;

#[cfg(feature = "openapi")]
pub mod openapi;

// Provide a basic router with common middleware (CORS, timeouts, limits) when needed later
