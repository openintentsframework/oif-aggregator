//! Centralized mocks and fixtures for testing
//!
//! This module provides reusable mock entities, configurations, and test data
//! to reduce duplication across test files.

pub mod adapters;
pub mod api_fixtures;
pub mod configs;
pub mod entities;

// Re-export commonly used items for test convenience
pub use adapters::{MockDemoAdapter, MockTestAdapter};
pub use api_fixtures::{ApiFixtures, AppStateBuilder};
pub use configs::MockConfigs;
pub use entities::{MockEntities, TestConstants};
