//! Configuration mocks and builders for tests

use oif_config::settings::*;
use std::collections::HashMap;

/// Configuration builders for tests
#[allow(dead_code)]
pub struct MockConfigs;

#[allow(dead_code)]
impl MockConfigs {
	/// Create minimal test settings with sensible defaults
	pub fn test_settings() -> Settings {
		Settings {
			server: Some(ServerSettings {
				host: "127.0.0.1".to_string(),
				port: 3001, // Different port for testing
			}),
			solvers: HashMap::new(), // Empty for testing
			aggregation: Some(AggregationSettings {
				global_timeout_ms: Some(5000), // Override default for testing
				per_solver_timeout_ms: Some(2000),
				max_concurrent_solvers: Some(5), // Lower concurrency for testing
				max_retries_per_solver: Some(2), // Fewer retries for faster tests
				retry_delay_ms: Some(500),       // Faster retries for testing
				include_unknown_compatibility: Some(true), // Include unknown solvers for testing
			}),
			environment: Some(EnvironmentSettings {
				rate_limiting: RateLimitSettings {
					enabled: false,
					requests_per_minute: 60,
					burst_size: 10,
				},
			}),
			logging: Some(LoggingSettings {
				level: "debug".to_string(),
				format: LogFormat::Compact,
				structured: false,
			}),
			security: SecuritySettings {
				integrity_secret: oif_config::ConfigurableValue::from_plain("test-secret"),
			},
			maintenance: None,
			metrics: None,
		}
	}

	/// Create test settings with rate limiting enabled
	pub fn test_settings_with_rate_limit(requests_per_minute: u32, burst_size: u32) -> Settings {
		let mut settings = Self::test_settings();
		if let Some(ref mut env) = settings.environment {
			env.rate_limiting = RateLimitSettings {
				enabled: true,
				requests_per_minute,
				burst_size,
			};
		}
		settings
	}

	/// Create test settings for production-like environment
	pub fn test_settings_production() -> Settings {
		let mut settings = Self::test_settings();
		if let Some(ref mut logging) = settings.logging {
			logging.level = "info".to_string();
			logging.format = LogFormat::Json;
		}
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
			headers: Some(HashMap::new()),
			adapter_metadata: None,
			supported_assets: None,
			enabled: true,
		}
	}

	/// Create custom solver config with builder pattern
	pub fn solver_config() -> SolverConfigBuilder {
		SolverConfigBuilder::new()
	}
}

/// Builder for creating custom solver configurations
#[allow(dead_code)]
pub struct SolverConfigBuilder {
	config: SolverConfig,
}

#[allow(dead_code)]
impl SolverConfigBuilder {
	pub fn new() -> Self {
		Self {
			config: SolverConfig {
				solver_id: "test-solver".to_string(),
				adapter_id: "test-adapter".to_string(),
				name: Some("Test Solver".to_string()),
				description: Some("Test solver".to_string()),
				endpoint: "http://localhost:8080".to_string(),
				headers: Some(HashMap::new()),
				adapter_metadata: None,
				supported_assets: None,
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

	pub fn enabled(mut self, enabled: bool) -> Self {
		self.config.enabled = enabled;
		self
	}

	pub fn build(self) -> SolverConfig {
		self.config
	}
}
