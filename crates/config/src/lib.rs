//! OIF Configuration
//!
//! Configuration management and startup utilities for the Open Intent Framework Aggregator.

pub mod configurable_value;
pub mod loader;
pub mod settings;
pub mod startup_logger;

pub use configurable_value::{ConfigurableValue, ConfigurableValueError, ValueType};
pub use loader::{load_config, ConfigLoadError};
pub use settings::{AggregationConfig, AggregationSettings, ConfigValidationError, Settings};
pub use startup_logger::{log_service_info, log_service_shutdown, log_startup_complete};
