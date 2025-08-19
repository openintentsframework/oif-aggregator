//! Centralized mocks and fixtures for testing
//!
//! This module provides reusable mock entities, configurations, and test data
//! to reduce duplication across test files.

pub mod adapters;
pub mod api_fixtures;
pub mod configs;
pub mod entities;
pub mod test_server;

// Re-export commonly used items for convenience
#[allow(unused_imports)]
pub use api_fixtures::ApiFixtures;
#[allow(unused_imports)]
pub use test_server::TestServer;
