//! Configuration mocks and builders for tests

use oif_config::settings::*;
use oif_types::{
	adapters::{AdapterConfig, AdapterType},
	Asset, Network,
};
use std::collections::HashMap;

/// Configuration builders for tests
pub struct MockConfigs;

impl MockConfigs {
	/// Create minimal test settings with sensible defaults
	pub fn test_settings() -> Settings {
		Settings {
			server: ServerSettings {
				host: "127.0.0.1".to_string(),
				port: 3001, // Different port for testing
				workers: None,
			},
			solvers: HashMap::new(), // Empty for testing
			timeouts: TimeoutSettings {
				per_solver_ms: 2000,
				global_ms: 5000,
				request_ms: 1000,
			},
			environment: EnvironmentSettings {
				profile: EnvironmentProfile::Development,
				debug: true,
				metrics_enabled: false,
				rate_limiting: RateLimitSettings {
					enabled: false,
					requests_per_minute: 60,
					burst_size: 10,
				},
			},
			logging: LoggingSettings {
				level: "debug".to_string(),
				format: LogFormat::Compact,
				structured: false,
			},
		}
	}

	/// Create test settings with rate limiting enabled
	pub fn test_settings_with_rate_limit(requests_per_minute: u32, burst_size: u32) -> Settings {
		let mut settings = Self::test_settings();
		settings.environment.rate_limiting = RateLimitSettings {
			enabled: true,
			requests_per_minute,
			burst_size,
		};
		settings
	}

	/// Create test settings for production-like environment
	pub fn test_settings_production() -> Settings {
		let mut settings = Self::test_settings();
		settings.environment.profile = EnvironmentProfile::Production;
		settings.environment.debug = false;
		settings.environment.metrics_enabled = true;
		settings.logging.level = "info".to_string();
		settings.logging.format = LogFormat::Json;
		settings
	}

	/// Create OIF v1 adapter configuration for testing
	pub fn oif_adapter_config() -> AdapterConfig {
		AdapterConfig {
			adapter_id: "test-oif-adapter".to_string(),
			adapter_type: AdapterType::OifV1,
			name: "Test OIF Adapter".to_string(),
			description: Some("Test adapter for unit testing".to_string()),
			version: "1.0.0".to_string(),
			supported_networks: Some(vec![Network::new(1, "Ethereum".to_string(), false)]),
			supported_assets: Some(vec![Asset::new(
				"0x0000000000000000000000000000000000000000".to_string(),
				"ETH".to_string(),
				"Ethereum".to_string(),
				18,
				1,
			)]),
			endpoint: Some("http://localhost:8080".to_string()),
			timeout_ms: Some(1000),
			enabled: Some(true),
		}
	}

	/// Create LiFi adapter configuration for testing
	pub fn lifi_adapter_config() -> AdapterConfig {
		AdapterConfig {
			adapter_id: "test-lifi-adapter".to_string(),
			adapter_type: AdapterType::LifiV1,
			name: "Test LiFi Adapter".to_string(),
			description: Some("Test LiFi adapter for unit testing".to_string()),
			version: "1.0.0".to_string(),
			supported_networks: Some(vec![Network::new(1, "Ethereum".to_string(), false)]),
			supported_assets: Some(vec![Asset::new(
				"0x0000000000000000000000000000000000000000".to_string(),
				"ETH".to_string(),
				"Ethereum".to_string(),
				18,
				1,
			)]),
			endpoint: Some("http://localhost:8080".to_string()),
			timeout_ms: Some(1000),
			enabled: Some(true),
		}
	}

	/// Create custom adapter config with builder pattern
	pub fn adapter_config() -> AdapterConfigBuilder {
		AdapterConfigBuilder::new()
	}
}

/// Builder for creating custom adapter configurations
pub struct AdapterConfigBuilder {
	config: AdapterConfig,
}

impl AdapterConfigBuilder {
	pub fn new() -> Self {
		Self {
			config: AdapterConfig {
				adapter_id: "test-adapter".to_string(),
				adapter_type: AdapterType::OifV1,
				name: "Test Adapter".to_string(),
				description: Some("Test adapter".to_string()),
				version: "1.0.0".to_string(),
				supported_networks: Some(vec![Network::new(1, "Ethereum".to_string(), false)]),
				supported_assets: Some(vec![Asset::new(
					"0x0000000000000000000000000000000000000000".to_string(),
					"ETH".to_string(),
					"Ethereum".to_string(),
					18,
					1,
				)]),
				endpoint: Some("http://localhost:8080".to_string()),
				timeout_ms: Some(1000),
				enabled: Some(true),
			},
		}
	}

	pub fn id(mut self, id: &str) -> Self {
		self.config.adapter_id = id.to_string();
		self
	}

	pub fn adapter_type(mut self, adapter_type: AdapterType) -> Self {
		self.config.adapter_type = adapter_type;
		self
	}

	pub fn name(mut self, name: &str) -> Self {
		self.config.name = name.to_string();
		self
	}

	pub fn description(mut self, description: Option<&str>) -> Self {
		self.config.description = description.map(|s| s.to_string());
		self
	}

	pub fn version(mut self, version: &str) -> Self {
		self.config.version = version.to_string();
		self
	}

	pub fn build(self) -> AdapterConfig {
		self.config
	}
}
