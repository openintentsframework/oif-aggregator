//! Configuration loading utilities

use crate::Settings;
use config::{Config, ConfigError, File};

/// Load configuration from config file
pub fn load_config() -> Result<Settings, ConfigError> {
	// Load only the default configuration file
	let s = Config::builder()
		.add_source(File::with_name("config/config").required(false))
		.build()?;

	s.try_deserialize()
}
