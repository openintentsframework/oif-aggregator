//! Configuration loading utilities

use crate::{settings::ConfigValidationError, Settings};
use config::{Config, ConfigError, File};

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoadError {
	#[error("Configuration loading error: {0}")]
	LoadError(#[from] ConfigError),
	#[error("Configuration validation error: {0}")]
	ValidationError(#[from] ConfigValidationError),
}

/// Load and validate configuration from config file
///
/// This function will:
/// 1. Load the configuration from the file
/// 2. Deserialize with automatic serde defaults for optional fields
/// 3. Validate critical configuration and fail fast with clear error messages
/// 4. Return validated configuration or crash the app with detailed error
pub fn load_config() -> Result<Settings, ConfigLoadError> {
	// Load only the default configuration file
	let s = Config::builder()
		.add_source(File::with_name("config/config").required(false))
		.build()?;

	let settings: Settings = s.try_deserialize()?;

	// Validate configuration and fail fast if there are issues
	settings.validate()?;

	Ok(settings)
}
