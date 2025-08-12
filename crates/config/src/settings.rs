//! Configuration settings structures

use crate::{configurable_value::ConfigurableValue, ConfigurableValueError};
use oif_types::SecretString;
use oif_types::SolverConfig as DomainSolverConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main application settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
	pub server: ServerSettings,
	pub solvers: HashMap<String, SolverConfig>,
	pub timeouts: TimeoutSettings,
	pub environment: EnvironmentSettings,
	pub logging: LoggingSettings,
	pub security: SecuritySettings,
}

/// Server configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerSettings {
	pub host: String,
	pub port: u16,
}

/// Individual solver configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SolverConfig {
	pub solver_id: String,
	pub adapter_id: String,
	pub endpoint: String,
	pub timeout_ms: u64,
	pub enabled: bool,
	pub max_retries: u32,
	pub headers: Option<HashMap<String, String>>,
	// Optional descriptive metadata
	pub name: Option<String>,
	pub description: Option<String>,
	// Optional domain metadata for discoverability
	pub supported_networks: Option<Vec<NetworkConfig>>,
	pub supported_assets: Option<Vec<AssetConfig>>,
}

/// Convert from settings SolverConfig to domain SolverConfig
impl From<SolverConfig> for DomainSolverConfig {
	fn from(settings_config: SolverConfig) -> Self {
		Self {
			solver_id: settings_config.solver_id,
			adapter_id: settings_config.adapter_id,
			endpoint: settings_config.endpoint,
			timeout_ms: settings_config.timeout_ms,
			enabled: settings_config.enabled,
			max_retries: settings_config.max_retries,
			headers: settings_config.headers,
			name: settings_config.name,
			description: settings_config.description,
			version: None,
			supported_networks: settings_config.supported_networks.map(|networks| {
				networks
					.into_iter()
					.map(|n| oif_types::Network::new(n.chain_id, n.name, n.is_testnet))
					.collect()
			}),
			supported_assets: settings_config.supported_assets.map(|assets| {
				assets
					.into_iter()
					.map(|a| {
						oif_types::Asset::new(a.address, a.symbol, a.name, a.decimals, a.chain_id)
					})
					.collect()
			}),
			config: None,
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

/// Minimal asset shape for config to avoid cross-crate cycle
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetConfig {
	pub address: String,
	pub symbol: String,
	pub name: String,
	pub decimals: u8,
	pub chain_id: u64,
}

/// Timeout configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeoutSettings {
	/// Per-solver timeout in milliseconds (1000-3000ms recommended)
	pub per_solver_ms: u64,
	/// Global aggregation timeout in milliseconds (3000-5000ms recommended)
	pub global_ms: u64,
	/// Request timeout for HTTP clients
	pub request_ms: u64,
}

/// Environment-specific settings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvironmentSettings {
	pub profile: EnvironmentProfile,
	pub debug: bool,
	pub metrics_enabled: bool,
	pub rate_limiting: RateLimitSettings,
}

/// Environment profiles
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentProfile {
	Development,
	Staging,
	Production,
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

impl Default for Settings {
	fn default() -> Self {
		Self {
			server: ServerSettings {
				host: "0.0.0.0".to_string(),
				port: 3000,
			},
			solvers: HashMap::new(),
			timeouts: TimeoutSettings {
				per_solver_ms: 2000,
				global_ms: 4000,
				request_ms: 5000,
			},
			environment: EnvironmentSettings {
				profile: EnvironmentProfile::Development,
				debug: true,
				metrics_enabled: false,
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
		}
	}
}

impl Settings {
	/// Get server bind address
	pub fn bind_address(&self) -> String {
		format!("{}:{}", self.server.host, self.server.port)
	}

	/// Get enabled solvers only
	pub fn enabled_solvers(&self) -> HashMap<String, SolverConfig> {
		self.solvers
			.iter()
			.filter(|(_, config)| config.enabled)
			.map(|(k, v)| (k.clone(), v.clone()))
			.collect()
	}

	/// Check if running in production
	pub fn is_production(&self) -> bool {
		self.environment.profile == EnvironmentProfile::Production
	}

	/// Check if debug mode is enabled
	pub fn is_debug(&self) -> bool {
		self.environment.debug && !self.is_production()
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
}
