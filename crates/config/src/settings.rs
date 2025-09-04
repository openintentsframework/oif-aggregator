//! Configuration settings structures

use crate::{configurable_value::ConfigurableValue, ConfigurableValueError};
use oif_types::constants::limits::{
	DEFAULT_GLOBAL_TIMEOUT_MS, DEFAULT_MAX_CONCURRENT_SOLVERS, DEFAULT_MAX_RETRIES_PER_SOLVER,
	DEFAULT_ORDER_RETENTION_DAYS, DEFAULT_RETRY_DELAY_MS,
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
	pub server: ServerSettings,
	pub solvers: HashMap<String, SolverConfig>,
	pub aggregation: AggregationSettings,
	pub environment: EnvironmentSettings,
	pub logging: LoggingSettings,
	pub security: SecuritySettings,
	pub maintenance: Option<MaintenanceSettings>,
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
	pub headers: Option<HashMap<String, String>>,
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
/// All fields are optional - when not specified, sensible defaults from constants will be used.
/// This allows users to override only the settings they care about.
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
	/// Whether to include solvers with unknown compatibility in results (default: true)
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
			include_unknown_compatibility: true, // Default to including unknown solvers
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
			include_unknown_compatibility: settings.include_unknown_compatibility.unwrap_or(true),
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

impl Default for Settings {
	fn default() -> Self {
		Self {
			server: ServerSettings {
				host: "0.0.0.0".to_string(),
				port: 4000,
			},
			solvers: HashMap::new(),
			aggregation: AggregationSettings {
				global_timeout_ms: None,
				per_solver_timeout_ms: None,
				max_concurrent_solvers: None,
				max_retries_per_solver: None,
				retry_delay_ms: None,
				include_unknown_compatibility: None,
			},
			environment: EnvironmentSettings {
				rate_limiting: RateLimitSettings {
					enabled: false,
					requests_per_minute: 100,
					burst_size: 10,
				},
			},
			logging: LoggingSettings {
				level: "info".to_string(),
				format: LogFormat::Pretty,
				structured: false,
			},
			security: SecuritySettings {
				integrity_secret: ConfigurableValue::from_env("INTEGRITY_SECRET"),
			},
			maintenance: None, // Uses defaults when None
		}
	}
}

impl Settings {
	/// Get server bind address
	/// Priority: 1) Environment variables (HOST, PORT) 2) Config file 3) Defaults
	pub fn bind_address(&self) -> String {
		let host = self.get_host();
		let port = self.get_port();
		format!("{}:{}", host, port)
	}

	/// Get resolved server host with environment variable override
	/// Priority: 1) HOST env var 2) Config file value
	pub fn get_host(&self) -> String {
		std::env::var("HOST").unwrap_or_else(|_| self.server.host.clone())
	}

	/// Get resolved server port with environment variable override
	/// Priority: 1) PORT env var 2) Config file value
	pub fn get_port(&self) -> u16 {
		if let Ok(port_str) = std::env::var("PORT") {
			port_str.parse::<u16>().unwrap_or_else(|_| {
				warn!(
					"Warning: Invalid PORT environment variable '{}', using config file value {}",
					port_str, self.server.port
				);
				self.server.port
			})
		} else {
			self.server.port
		}
	}

	/// Get resolved log level with environment variable override
	/// Priority: 1) RUST_LOG env var 2) Config file value
	pub fn get_log_level(&self) -> String {
		std::env::var("RUST_LOG").unwrap_or_else(|_| self.logging.level.clone())
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
}
