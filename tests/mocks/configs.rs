//! Configuration mocks and builders for tests

use oif_config::settings::*;
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
			security: SecuritySettings {
				integrity_secret: oif_config::ConfigurableValue::from_plain("test-secret"),
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

	/// Create a test solver config
	pub fn test_solver_config() -> SolverConfig {
		SolverConfig {
			solver_id: "test-solver".to_string(),
			adapter_id: "test-adapter".to_string(),
			name: Some("Test Solver".to_string()),
			description: Some("Test solver for unit testing".to_string()),
			endpoint: "http://localhost:8080".to_string(),
			timeout_ms: 1000,
			max_retries: 3,
			headers: Some(HashMap::new()),
			supported_networks: Some(vec![NetworkConfig {
				chain_id: 1,
				name: "Ethereum".to_string(),
				is_testnet: false,
			}]),
			supported_assets: Some(vec![AssetConfig {
				address: "0x0000000000000000000000000000000000000000".to_string(),
				symbol: "ETH".to_string(),
				name: "Ethereum".to_string(),
				decimals: 18,
				chain_id: 1,
			}]),
			enabled: true,
		}
	}

	/// Create custom solver config with builder pattern
	pub fn solver_config() -> SolverConfigBuilder {
		SolverConfigBuilder::new()
	}
}

/// Builder for creating custom solver configurations
pub struct SolverConfigBuilder {
	config: SolverConfig,
}

impl SolverConfigBuilder {
	pub fn new() -> Self {
		Self {
			config: SolverConfig {
				solver_id: "test-solver".to_string(),
				adapter_id: "test-adapter".to_string(),
				name: Some("Test Solver".to_string()),
				description: Some("Test solver".to_string()),
				endpoint: "http://localhost:8080".to_string(),
				timeout_ms: 1000,
				max_retries: 3,
				headers: Some(HashMap::new()),
				supported_networks: Some(vec![NetworkConfig {
					chain_id: 1,
					name: "Ethereum".to_string(),
					is_testnet: false,
				}]),
				supported_assets: Some(vec![AssetConfig {
					address: "0x0000000000000000000000000000000000000000".to_string(),
					symbol: "ETH".to_string(),
					name: "Ethereum".to_string(),
					decimals: 18,
					chain_id: 1,
				}]),
				enabled: true,
			},
		}
	}

	pub fn id(mut self, id: &str) -> Self {
		self.config.solver_id = id.to_string();
		self
	}

	pub fn adapter_id(mut self, adapter_id: &str) -> Self {
		self.config.adapter_id = adapter_id.to_string();
		self
	}

	pub fn name(mut self, name: &str) -> Self {
		self.config.name = Some(name.to_string());
		self
	}

	pub fn description(mut self, description: Option<&str>) -> Self {
		self.config.description = description.map(|s| s.to_string());
		self
	}

	pub fn endpoint(mut self, endpoint: &str) -> Self {
		self.config.endpoint = endpoint.to_string();
		self
	}

	pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
		self.config.timeout_ms = timeout_ms;
		self
	}

	pub fn enabled(mut self, enabled: bool) -> Self {
		self.config.enabled = enabled;
		self
	}

	pub fn build(self) -> SolverConfig {
		self.config
	}
}
