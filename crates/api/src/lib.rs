//! OIF Aggregator API
//!
//! Axum-based API with routes and middleware for the Aggregator.

pub mod auth;
pub mod handlers;
pub mod pagination;
pub mod router;
pub mod security;
pub mod state;

pub use router::create_router;
pub use state::AppState;

#[cfg(feature = "openapi")]
pub mod openapi;
