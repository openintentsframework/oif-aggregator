//! Configuration settings structures

use crate::{configurable_value::ConfigurableValue, ConfigurableValueError};
use oif_types::constants::limits::{
	DEFAULT_GLOBAL_TIMEOUT_MS, DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY,
	DEFAULT_MAX_CONCURRENT_SOLVERS, DEFAULT_MAX_RETRIES_PER_SOLVER,
	DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES, DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS,
	DEFAULT_METRICS_COLLECTION_ENABLED, DEFAULT_METRICS_RETENTION_HOURS,
	DEFAULT_ORDER_RETENTION_DAYS, DEFAULT_RATE_LIMIT_BURST_SIZE,
	DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE, DEFAULT_RETRY_DELAY_MS,
};
use oif_types::constants::DEFAULT_SOLVER_TIMEOUT_MS;
use oif_types::solvers::config::SupportedAssetsConfig;
use oif_types::SecretString;
use oif_types::SolverConfig as DomainSolverConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::warn;

/// Main application settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
	#[serde(default = "default_server_settings")]
	pub server: Option<ServerSettings>,
	pub solvers: HashMap<String, SolverConfig>,
	#[serde(default = "default_aggregation_settings")]
	pub aggregation: Option<AggregationSettings>,
	#[serde(default = "default_environment_settings")]
	pub environment: Option<EnvironmentSettings>,
	#[serde(default = "default_logging_settings")]
	pub logging: Option<LoggingSettings>,
	pub security: SecuritySettings,
	pub maintenance: Option<MaintenanceSettings>,
	#[serde(default = "default_metrics_settings")]
	pub metrics: Option<MetricsSettings>,
}

/// Default server settings
fn default_server_settings() -> Option<ServerSettings> {
	Some(ServerSettings {
		host: "0.0.0.0".to_string(),
		port: 4000,
	})
}

/// Default aggregation settings
fn default_aggregation_settings() -> Option<AggregationSettings> {
	Some(AggregationSettings {
		global_timeout_ms: Some(DEFAULT_GLOBAL_TIMEOUT_MS),
		per_solver_timeout_ms: Some(DEFAULT_SOLVER_TIMEOUT_MS),
		max_concurrent_solvers: Some(DEFAULT_MAX_CONCURRENT_SOLVERS),
		max_retries_per_solver: Some(DEFAULT_MAX_RETRIES_PER_SOLVER),
		retry_delay_ms: Some(DEFAULT_RETRY_DELAY_MS),
		include_unknown_compatibility: Some(DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY),
	})
}

/// Default environment settings
fn default_environment_settings() -> Option<EnvironmentSettings> {
	Some(EnvironmentSettings {
		rate_limiting: RateLimitSettings {
			enabled: true,
			requests_per_minute: DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE,
			burst_size: DEFAULT_RATE_LIMIT_BURST_SIZE,
		},
	})
}

/// Default logging settings
fn default_logging_settings() -> Option<LoggingSettings> {
	Some(LoggingSettings {
		level: "info".to_string(),
		format: LogFormat::Compact,
		structured: false,
	})
}

/// Default metrics settings
fn default_metrics_settings() -> Option<MetricsSettings> {
	Some(MetricsSettings {
		collection_enabled: DEFAULT_METRICS_COLLECTION_ENABLED,
		retention_hours: DEFAULT_METRICS_RETENTION_HOURS,
		cleanup_interval_hours: DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS,
		aggregation_interval_minutes: DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES,
	})
}

/// Server configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerSettings {
	/// Server host/interface to bind to
	/// Can be overridden by HOST environment variable
	pub host: String,
	/// Server port to bind to
	/// Can be overridden by PORT environment variable
	pub port: u16,
}

/// Individual solver configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SolverConfig {
	pub solver_id: String,
	pub adapter_id: String,
	pub endpoint: String,
	pub enabled: bool,
	/// Static headers for requests
	pub headers: Option<HashMap<String, String>>,
	/// Adapter-specific metadata (JSON configuration for adapter customization)
	pub adapter_metadata: Option<serde_json::Value>,
	// Optional descriptive metadata
	pub name: Option<String>,
	pub description: Option<String>,
	pub supported_assets: Option<SupportedAssetsConfig>,
}

/// Convert from settings SolverConfig to domain SolverConfig
impl From<SolverConfig> for DomainSolverConfig {
	fn from(settings_config: SolverConfig) -> Self {
		Self {
			solver_id: settings_config.solver_id,
			adapter_id: settings_config.adapter_id,
			endpoint: settings_config.endpoint,
			enabled: settings_config.enabled,
			headers: settings_config.headers,
			adapter_metadata: settings_config.adapter_metadata,
			name: settings_config.name,
			description: settings_config.description,
			version: None,
			supported_assets: settings_config.supported_assets,
		}
	}
}

/// Minimal network shape for config to avoid cross-crate cycle
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
	pub chain_id: u64,
	pub name: String,
	pub is_testnet: bool,
}

/// Aggregation behavior configuration
///
/// All fields are optional - when not specified in config file, sensible defaults from constants are used.
/// Users can override any individual setting by providing explicit values in their config file.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AggregationSettings {
	/// Global aggregation timeout in milliseconds (1000-30000ms recommended)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub global_timeout_ms: Option<u64>,
	/// Per-solver timeout in milliseconds (500-10000ms recommended)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub per_solver_timeout_ms: Option<u64>,
	/// Maximum number of solvers to query concurrently (1-50 recommended)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_concurrent_solvers: Option<usize>,
	/// Maximum retry attempts per solver (0-3 recommended)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_retries_per_solver: Option<u32>,
	/// Delay between retry attempts in milliseconds (100-5000ms recommended)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub retry_delay_ms: Option<u64>,
	/// Whether to include solvers with unknown compatibility in results (default: false)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub include_unknown_compatibility: Option<bool>,
}

/// Configuration for aggregation behavior (service layer compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
	pub global_timeout_ms: u64,
	pub per_solver_timeout_ms: u64,
	pub max_concurrent_solvers: usize,
	pub max_retries_per_solver: u32,
	pub retry_delay_ms: u64,
	pub include_unknown_compatibility: bool,
}

impl Default for AggregationConfig {
	fn default() -> Self {
		Self {
			global_timeout_ms: DEFAULT_GLOBAL_TIMEOUT_MS,
			per_solver_timeout_ms: DEFAULT_SOLVER_TIMEOUT_MS,
			max_concurrent_solvers: DEFAULT_MAX_CONCURRENT_SOLVERS,
			max_retries_per_solver: DEFAULT_MAX_RETRIES_PER_SOLVER,
			retry_delay_ms: DEFAULT_RETRY_DELAY_MS,
			include_unknown_compatibility: DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY,
		}
	}
}

impl From<AggregationSettings> for AggregationConfig {
	fn from(settings: AggregationSettings) -> Self {
		Self {
			global_timeout_ms: settings
				.global_timeout_ms
				.unwrap_or(DEFAULT_GLOBAL_TIMEOUT_MS),
			per_solver_timeout_ms: settings
				.per_solver_timeout_ms
				.unwrap_or(DEFAULT_SOLVER_TIMEOUT_MS),
			max_concurrent_solvers: settings
				.max_concurrent_solvers
				.unwrap_or(DEFAULT_MAX_CONCURRENT_SOLVERS),
			max_retries_per_solver: settings
				.max_retries_per_solver
				.unwrap_or(DEFAULT_MAX_RETRIES_PER_SOLVER),
			retry_delay_ms: settings.retry_delay_ms.unwrap_or(DEFAULT_RETRY_DELAY_MS),
			include_unknown_compatibility: settings
				.include_unknown_compatibility
				.unwrap_or(DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY),
		}
	}
}

/// Environment-specific settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvironmentSettings {
	pub rate_limiting: RateLimitSettings,
}

/// Rate limiting configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RateLimitSettings {
	pub enabled: bool,
	pub requests_per_minute: u32,
	pub burst_size: u32,
}

/// Logging configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingSettings {
	/// Log level (can be overridden by RUST_LOG environment variable)
	pub level: String,
	pub format: LogFormat,
	pub structured: bool,
}

/// Log format options
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
	Json,
	Pretty,
	Compact,
}

/// Security configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecuritySettings {
	/// Secret key for HMAC integrity verification
	///
	/// Used to generate and verify integrity checksums for quotes and orders.
	/// Should be a secure random string (minimum 32 characters).
	///
	/// Example configurations:
	/// - Environment variable: `{"type": "env", "value": "INTEGRITY_SECRET"}`
	/// - Plain value: `{"type": "plain", "value": "your-secret-here"}`
	pub integrity_secret: ConfigurableValue,
}

/// Maintenance and cleanup job configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MaintenanceSettings {
	/// Number of days to retain orders in final status before cleanup
	///
	/// Orders with status 'Finalized' or 'Failed' older than this many days
	/// will be automatically deleted by the daily cleanup job.
	///
	/// Default: 10 days
	pub order_retention_days: u32,
}

/// Metrics collection and management configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetricsSettings {
	/// Whether metrics collection is enabled
	///
	/// When enabled, solver performance data will be collected and stored
	/// for circuit breaker decisions and scoring engine analysis.
	///
	/// Default: true
	pub collection_enabled: bool,

	/// Number of hours to retain metrics time-series data
	///
	/// Metrics older than this will be automatically cleaned up.
	/// Recommended values: 168 hours (7 days) to 720 hours (30 days)
	///
	/// Default: 168 hours (7 days)
	pub retention_hours: u32,

	/// Interval in hours between metrics cleanup jobs
	///
	/// How often the system should run cleanup to remove old metrics data.
	///
	/// Default: 24 hours (daily cleanup)
	pub cleanup_interval_hours: u32,

	/// Interval in minutes for updating rolling metrics aggregates
	///
	/// How often the system should recalculate rolling window metrics
	/// (1 hour, 24 hour, 7 day windows) from the raw time-series data.
	///
	/// Default: 5 minutes
	pub aggregation_interval_minutes: u32,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			server: default_server_settings(),
			solvers: HashMap::new(),
			aggregation: default_aggregation_settings(),
			environment: default_environment_settings(),
			logging: default_logging_settings(),
			security: SecuritySettings {
				integrity_secret: ConfigurableValue::from_env("INTEGRITY_SECRET"),
			},
			maintenance: None, // Uses defaults when None
			metrics: default_metrics_settings(),
		}
	}
}

/// Configuration validation errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
	#[error("Critical configuration missing: {field}")]
	MissingCriticalConfig { field: String },
	#[error("Invalid configuration for {field}: {reason}")]
	InvalidConfig { field: String, reason: String },
	#[error("Security configuration error: {0}")]
	SecurityError(#[from] ConfigurableValueError),
}

impl Settings {
	/// Validate critical configuration and fail fast if anything is wrong
	/// Call this at startup to ensure the app crashes early with clear error messages
	pub fn validate(&self) -> Result<(), ConfigValidationError> {
		// Validate critical required configs
		if self.solvers.is_empty() {
			return Err(ConfigValidationError::MissingCriticalConfig {
				field: "solvers".to_string(),
			});
		}

		// Validate enabled solvers exist
		let enabled_count = self.solvers.values().filter(|s| s.enabled).count();
		if enabled_count == 0 {
			return Err(ConfigValidationError::InvalidConfig {
				field: "solvers".to_string(),
				reason: "No enabled solvers found. At least one solver must be enabled."
					.to_string(),
			});
		}

		// Validate security configuration can be resolved
		self.security
			.integrity_secret
			.resolve()
			.map_err(ConfigValidationError::SecurityError)?;

		// Validate individual solver configurations
		for (solver_id, solver_config) in &self.solvers {
			if solver_config.solver_id != *solver_id {
				return Err(ConfigValidationError::InvalidConfig {
					field: format!("solvers.{}", solver_id),
					reason: format!(
						"Solver ID mismatch: key '{}' != config.solver_id '{}'",
						solver_id, solver_config.solver_id
					),
				});
			}

			if solver_config.endpoint.is_empty() {
				return Err(ConfigValidationError::InvalidConfig {
					field: format!("solvers.{}.endpoint", solver_id),
					reason: "Endpoint cannot be empty".to_string(),
				});
			}

			if solver_config.adapter_id.is_empty() {
				return Err(ConfigValidationError::InvalidConfig {
					field: format!("solvers.{}.adapter_id", solver_id),
					reason: "Adapter ID cannot be empty".to_string(),
				});
			}
		}

		// Validate optional configurations have sensible values if present
		if let Some(server) = &self.server {
			if server.host.is_empty() {
				return Err(ConfigValidationError::InvalidConfig {
					field: "server.host".to_string(),
					reason: "Host cannot be empty".to_string(),
				});
			}
			if server.port == 0 {
				return Err(ConfigValidationError::InvalidConfig {
					field: "server.port".to_string(),
					reason: "Port cannot be 0".to_string(),
				});
			}
		}

		if let Some(logging) = &self.logging {
			if logging.level.is_empty() {
				return Err(ConfigValidationError::InvalidConfig {
					field: "logging.level".to_string(),
					reason: "Log level cannot be empty".to_string(),
				});
			}
		}

		Ok(())
	}

	/// Get server configuration with defaults
	pub fn get_server(&self) -> ServerSettings {
		self.server.clone().unwrap_or_else(|| ServerSettings {
			host: "0.0.0.0".to_string(),
			port: 4000,
		})
	}

	/// Get logging configuration with defaults  
	pub fn get_logging(&self) -> LoggingSettings {
		self.logging.clone().unwrap_or_else(|| LoggingSettings {
			level: "info".to_string(),
			format: LogFormat::Pretty,
			structured: false,
		})
	}

	/// Get environment configuration with defaults
	pub fn get_environment(&self) -> EnvironmentSettings {
		self.environment.clone().unwrap_or(EnvironmentSettings {
			rate_limiting: RateLimitSettings {
				enabled: false,
				requests_per_minute: DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE,
				burst_size: DEFAULT_RATE_LIMIT_BURST_SIZE,
			},
		})
	}

	/// Get aggregation configuration with defaults
	pub fn get_aggregation(&self) -> AggregationSettings {
		self.aggregation.clone().unwrap_or(AggregationSettings {
			global_timeout_ms: Some(DEFAULT_GLOBAL_TIMEOUT_MS),
			per_solver_timeout_ms: Some(DEFAULT_SOLVER_TIMEOUT_MS),
			max_concurrent_solvers: Some(DEFAULT_MAX_CONCURRENT_SOLVERS),
			max_retries_per_solver: Some(DEFAULT_MAX_RETRIES_PER_SOLVER),
			retry_delay_ms: Some(DEFAULT_RETRY_DELAY_MS),
			include_unknown_compatibility: Some(DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY),
		})
	}

	/// Get server bind address
	/// Priority: 1) Environment variables (HOST, PORT) 2) Config file 3) Defaults
	pub fn bind_address(&self) -> String {
		let host = self.get_host();
		let port = self.get_port();
		format!("{}:{}", host, port)
	}

	/// Get resolved server host with environment variable override
	/// Priority: 1) HOST env var 2) Config file value 3) Default
	pub fn get_host(&self) -> String {
		std::env::var("HOST").unwrap_or_else(|_| self.get_server().host)
	}

	/// Get resolved server port with environment variable override
	/// Priority: 1) PORT env var 2) Config file value 3) Default
	pub fn get_port(&self) -> u16 {
		if let Ok(port_str) = std::env::var("PORT") {
			port_str.parse::<u16>().unwrap_or_else(|_| {
				let default_port = self.get_server().port;
				warn!(
					"Warning: Invalid PORT environment variable '{}', using config file value {}",
					port_str, default_port
				);
				default_port
			})
		} else {
			self.get_server().port
		}
	}

	/// Get resolved log level with environment variable override
	/// Priority: 1) RUST_LOG env var 2) Config file value 3) Default
	pub fn get_log_level(&self) -> String {
		std::env::var("RUST_LOG").unwrap_or_else(|_| self.get_logging().level)
	}

	/// Get enabled solvers only
	pub fn enabled_solvers(&self) -> HashMap<String, SolverConfig> {
		self.solvers
			.iter()
			.filter(|(_, config)| config.enabled)
			.map(|(k, v)| (k.clone(), v.clone()))
			.collect()
	}

	/// Get integrity secret from configuration
	///
	/// Resolves the configurable value to get the actual secret string.
	/// Supports both environment variables and plain values based on configuration.
	pub fn get_integrity_secret(&self) -> Result<String, ConfigurableValueError> {
		self.security.integrity_secret.resolve()
	}

	/// Get integrity secret for secure handling (caller should wrap in SecretString)
	pub fn get_integrity_secret_secure(&self) -> Result<SecretString, ConfigurableValueError> {
		self.security.integrity_secret.resolve_for_secret()
	}

	/// Get order retention days with fallback to default
	pub fn get_order_retention_days(&self) -> u32 {
		// Check config file maintenance section, then use default
		self.maintenance
			.as_ref()
			.map(|m| m.order_retention_days)
			.unwrap_or(DEFAULT_ORDER_RETENTION_DAYS)
	}

	/// Get metrics configuration with defaults
	pub fn get_metrics(&self) -> MetricsSettings {
		self.metrics.clone().unwrap_or(MetricsSettings {
			collection_enabled: DEFAULT_METRICS_COLLECTION_ENABLED,
			retention_hours: DEFAULT_METRICS_RETENTION_HOURS,
			cleanup_interval_hours: DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS,
			aggregation_interval_minutes: DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES,
		})
	}

	/// Check if metrics collection is enabled
	pub fn is_metrics_collection_enabled(&self) -> bool {
		self.get_metrics().collection_enabled
	}

	/// Get metrics retention period in hours
	pub fn get_metrics_retention_hours(&self) -> u32 {
		self.get_metrics().retention_hours
	}

	/// Get metrics cleanup interval in hours
	pub fn get_metrics_cleanup_interval_hours(&self) -> u32 {
		self.get_metrics().cleanup_interval_hours
	}

	/// Get metrics aggregation interval in minutes
	pub fn get_metrics_aggregation_interval_minutes(&self) -> u32 {
		self.get_metrics().aggregation_interval_minutes
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_config_to_domain_without_supported_assets() {
		let settings_config = SolverConfig {
			solver_id: "test-solver".to_string(),
			adapter_id: "oif-v1".to_string(),
			endpoint: "https://api.test.com".to_string(),
			enabled: true,
			headers: None,
			adapter_metadata: None,
			name: Some("Test Solver".to_string()),
			description: None,
			supported_assets: None, // No supported_assets defined
		};

		// Convert to domain config
		let domain_config = DomainSolverConfig::from(settings_config);
		assert!(domain_config.supported_assets.is_none());

		// Convert to Solver
		let solver = oif_types::solvers::Solver::try_from(domain_config).unwrap();

		// Should use auto-discovery default
		match &solver.metadata.supported_assets {
			oif_types::solvers::SupportedAssets::Routes { routes, source } => {
				assert_eq!(routes.len(), 0);
				assert_eq!(source, &oif_types::solvers::AssetSource::AutoDiscovered);
			},
			_ => panic!("Expected routes mode with auto-discovery"),
		}
	}

	#[test]
	fn test_validation_fails_without_solvers() {
		let settings = Settings {
			server: None,
			solvers: HashMap::new(), // Empty solvers
			aggregation: None,
			environment: None,
			logging: None,
			security: SecuritySettings {
				integrity_secret: ConfigurableValue::from_plain("test-secret"),
			},
			maintenance: None,
			metrics: None,
		};

		let result = settings.validate();
		assert!(result.is_err());
		if let Err(ConfigValidationError::MissingCriticalConfig { field }) = result {
			assert_eq!(field, "solvers");
		} else {
			panic!("Expected MissingCriticalConfig error for solvers");
		}
	}

	#[test]
	fn test_validation_fails_without_enabled_solvers() {
		let mut solvers = HashMap::new();
		solvers.insert(
			"test-solver".to_string(),
			SolverConfig {
				solver_id: "test-solver".to_string(),
				adapter_id: "oif-v1".to_string(),
				endpoint: "https://api.test.com".to_string(),
				enabled: false, // Disabled
				headers: None,
				adapter_metadata: None,
				name: None,
				description: None,
				supported_assets: None,
			},
		);

		let settings = Settings {
			server: None,
			solvers,
			aggregation: None,
			environment: None,
			logging: None,
			security: SecuritySettings {
				integrity_secret: ConfigurableValue::from_plain("test-secret"),
			},
			maintenance: None,
			metrics: None,
		};

		let result = settings.validate();
		assert!(result.is_err());
		if let Err(ConfigValidationError::InvalidConfig { field, reason }) = result {
			assert_eq!(field, "solvers");
			assert!(reason.contains("No enabled solvers found"));
		} else {
			panic!("Expected InvalidConfig error for disabled solvers");
		}
	}

	#[test]
	fn test_validation_passes_with_valid_config() {
		let mut solvers = HashMap::new();
		solvers.insert(
			"test-solver".to_string(),
			SolverConfig {
				solver_id: "test-solver".to_string(),
				adapter_id: "oif-v1".to_string(),
				endpoint: "https://api.test.com".to_string(),
				enabled: true,
				headers: None,
				adapter_metadata: None,
				name: None,
				description: None,
				supported_assets: None,
			},
		);

		let settings = Settings {
			server: None,
			solvers,
			aggregation: None,
			environment: None,
			logging: None,
			security: SecuritySettings {
				integrity_secret: ConfigurableValue::from_plain("test-secret"),
			},
			maintenance: None,
			metrics: None,
		};

		let result = settings.validate();
		assert!(result.is_ok(), "Validation should pass with valid config");
	}

	#[test]
	fn test_validation_fails_with_solver_id_mismatch() {
		let mut solvers = HashMap::new();
		solvers.insert(
			"different-key".to_string(),
			SolverConfig {
				solver_id: "test-solver".to_string(), // Different from key
				adapter_id: "oif-v1".to_string(),
				endpoint: "https://api.test.com".to_string(),
				enabled: true,
				headers: None,
				adapter_metadata: None,
				name: None,
				description: None,
				supported_assets: None,
			},
		);

		let settings = Settings {
			server: None,
			solvers,
			aggregation: None,
			environment: None,
			logging: None,
			security: SecuritySettings {
				integrity_secret: ConfigurableValue::from_plain("test-secret"),
			},
			maintenance: None,
			metrics: None,
		};

		let result = settings.validate();
		assert!(result.is_err());
		if let Err(ConfigValidationError::InvalidConfig { field, reason }) = result {
			assert!(field.contains("different-key"));
			assert!(reason.contains("Solver ID mismatch"));
		} else {
			panic!("Expected InvalidConfig error for solver ID mismatch");
		}
	}

	#[test]
	fn test_get_methods_return_defaults() {
		let settings = Settings {
			server: None,
			solvers: HashMap::new(),
			aggregation: None,
			environment: None,
			logging: None,
			security: SecuritySettings {
				integrity_secret: ConfigurableValue::from_plain("test-secret"),
			},
			maintenance: None,
			metrics: None,
		};

		// Should return defaults when fields are None
		let server = settings.get_server();
		assert_eq!(server.host, "0.0.0.0");
		assert_eq!(server.port, 4000);

		let logging = settings.get_logging();
		assert_eq!(logging.level, "info");
		assert_eq!(logging.structured, false);

		let environment = settings.get_environment();
		assert_eq!(environment.rate_limiting.enabled, false);
		assert_eq!(
			environment.rate_limiting.requests_per_minute,
			DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE
		);
		assert_eq!(
			environment.rate_limiting.burst_size,
			DEFAULT_RATE_LIMIT_BURST_SIZE
		);

		let aggregation = settings.get_aggregation();
		assert_eq!(
			aggregation.global_timeout_ms,
			Some(DEFAULT_GLOBAL_TIMEOUT_MS)
		);
		assert_eq!(
			aggregation.include_unknown_compatibility,
			Some(DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY)
		);
		// Verify the default is false
		assert_eq!(DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY, false);
	}
}
