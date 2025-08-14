//! Centralized mocks and fixtures for testing
//!
//! This module provides reusable mock entities, configurations, and test data
//! to reduce duplication across test files.

pub mod adapters;
pub mod api_fixtures;
pub mod configs;
pub mod entities;

// Re-export commonly used items
pub use adapters::*;
pub use api_fixtures::*;
pub use configs::*;
pub use entities::*;
