//! Configuration loading utilities

use crate::Settings;
use config::{Config, ConfigError, Environment, File};
use std::env;

/// Load configuration from multiple sources with precedence:
/// 1. Environment variables (highest priority)
/// 2. .env file
/// 3. config/{environment}.json file
/// 4. config/default.json file (lowest priority)
pub fn load_config() -> Result<Settings, ConfigError> {
	let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

	let s = Config::builder()
        // Start with default configuration
        .add_source(File::with_name("config/default").required(false))
        // Add environment-specific configuration
        .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
        // Add local configuration (useful for overrides)
        .add_source(File::with_name("config/local").required(false))
        // Add environment variables with a prefix of APP and '__' as separator
        // This makes it so `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(Environment::with_prefix("APP").separator("__"))
        .build()?;

	// Deserialize and return the configuration
	s.try_deserialize()
}
