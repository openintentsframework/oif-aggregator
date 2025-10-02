//! Configuration settings structures

use crate::{configurable_value::ConfigurableValue, ConfigurableValueError};
use oif_types::constants::limits::{
	DEFAULT_GLOBAL_TIMEOUT_MS, DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY,
	DEFAULT_MAX_CONCURRENT_SOLVERS, DEFAULT_MAX_RETRIES_PER_SOLVER,
	DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES, DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS,
	DEFAULT_METRICS_RETENTION_HOURS, DEFAULT_ORDER_RETENTION_DAYS, DEFAULT_RATE_LIMIT_BURST_SIZE,
	DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE, DEFAULT_RETRY_DELAY_MS,
};
use oif_types::constants::{
	DEFAULT_CIRCUIT_BREAKER_BASE_TIMEOUT_SECONDS, DEFAULT_CIRCUIT_BREAKER_ENABLED,
	DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD, DEFAULT_CIRCUIT_BREAKER_HALF_OPEN_MAX_CALLS,
	DEFAULT_CIRCUIT_BREAKER_MAX_RECOVERY_ATTEMPTS, DEFAULT_CIRCUIT_BREAKER_MAX_TIMEOUT_SECONDS,
	DEFAULT_CIRCUIT_BREAKER_METRICS_MAX_AGE_MINUTES,
	DEFAULT_CIRCUIT_BREAKER_MIN_REQUESTS_FOR_RATE_CHECK,
	DEFAULT_CIRCUIT_BREAKER_SERVICE_ERROR_THRESHOLD,
	DEFAULT_CIRCUIT_BREAKER_SUCCESS_RATE_THRESHOLD, DEFAULT_SOLVER_TIMEOUT_MS,
};
use oif_types::solvers::config::SupportedAssetsConfig;
use oif_types::SolverConfig as DomainSolverConfig;
use oif_types::{PersistentFailureAction, SecretString};
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
	#[serde(default = "default_circuit_breaker_settings")]
	pub circuit_breaker: Option<CircuitBreakerSettings>,
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
		retention_hours: DEFAULT_METRICS_RETENTION_HOURS,
		cleanup_interval_hours: DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS,
		aggregation_interval_minutes: DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES,
		min_timeout_for_metrics_ms: DEFAULT_SOLVER_TIMEOUT_MS,
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
	/// Number of hours to retain metrics time-series data
	///
	/// Metrics older than this will be automatically cleaned up.
	/// Recommended values: 168 hours (7 days) to 720 hours (30 days)
	///
	/// Default: 168 hours (7 days)
	#[serde(default = "default_metrics_retention_hours")]
	pub retention_hours: u32,

	/// Interval in hours between metrics cleanup jobs
	///
	/// How often the system should run cleanup to remove old metrics data.
	///
	/// Default: 24 hours (daily cleanup)
	#[serde(default = "default_metrics_cleanup_interval_hours")]
	pub cleanup_interval_hours: u32,

	/// Interval in minutes for updating rolling metrics aggregates
	///
	/// How often the system should recalculate rolling window metrics
	/// (1 hour, 24 hour, 7 day windows) from the raw time-series data.
	///
	/// Default: 5 minutes
	#[serde(default = "default_metrics_aggregation_interval_minutes")]
	pub aggregation_interval_minutes: u32,

	/// Minimum timeout threshold for collecting timeout metrics in milliseconds
	///
	/// Timeouts below this value will be treated as user-induced cancellations,
	/// not solver performance issues, and won't affect circuit breaker decisions
	/// or solver priority scoring.
	///
	/// Default: 5000ms (5 seconds)
	#[serde(default = "default_metrics_min_timeout_for_metrics_ms")]
	pub min_timeout_for_metrics_ms: u64,
}

// Default functions for serde
fn default_metrics_retention_hours() -> u32 {
	DEFAULT_METRICS_RETENTION_HOURS
}

fn default_metrics_cleanup_interval_hours() -> u32 {
	DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS
}

fn default_metrics_aggregation_interval_minutes() -> u32 {
	DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES
}

fn default_metrics_min_timeout_for_metrics_ms() -> u64 {
	DEFAULT_SOLVER_TIMEOUT_MS
}

impl Default for MetricsSettings {
	fn default() -> Self {
		Self {
			retention_hours: default_metrics_retention_hours(),
			cleanup_interval_hours: default_metrics_cleanup_interval_hours(),
			aggregation_interval_minutes: default_metrics_aggregation_interval_minutes(),
			min_timeout_for_metrics_ms: default_metrics_min_timeout_for_metrics_ms(),
		}
	}
}

impl MetricsSettings {
	/// Validate metrics configuration
	pub fn validate(&self) -> Result<(), String> {
		if self.retention_hours == 0 {
			return Err("metrics.retention_hours must be > 0".to_string());
		}

		if self.cleanup_interval_hours == 0 {
			return Err("metrics.cleanup_interval_hours must be > 0".to_string());
		}

		if self.aggregation_interval_minutes == 0 {
			return Err("metrics.aggregation_interval_minutes must be > 0".to_string());
		}

		if self.min_timeout_for_metrics_ms == 0 {
			return Err("metrics.min_timeout_for_metrics_ms must be > 0".to_string());
		}

		Ok(())
	}
}

/// Circuit breaker configuration for automatic failure protection
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CircuitBreakerSettings {
	/// Whether circuit breaker protection is enabled
	///
	/// When disabled, all solvers will be treated as available regardless of
	/// failure patterns. Enable this to automatically protect against cascading
	/// failures by temporarily blocking requests to failing solvers.
	///
	/// Default: false (disabled)
	#[serde(default = "default_circuit_breaker_enabled")]
	pub enabled: bool,

	/// Number of consecutive failures required to open the circuit
	///
	/// When a solver reaches this many failures in a row, the circuit breaker
	/// will open and block requests to protect the system from cascading failures.
	///
	/// Must be > 0. Default: 5
	#[serde(default = "default_circuit_breaker_failure_threshold")]
	pub failure_threshold: u32,

	/// Success rate threshold below which the circuit opens (0.0 - 1.0)
	///
	/// If a solver's success rate falls below this percentage (with sufficient
	/// requests), the circuit breaker will open. For example, 0.3 means 30%
	/// success rate threshold.
	///
	/// Must be between 0.0 and 1.0. Default: 0.3 (30%)
	#[serde(default = "default_circuit_breaker_success_rate_threshold")]
	pub success_rate_threshold: f64,

	/// Minimum number of requests required before checking success rate
	///
	/// The circuit breaker will only consider success rate for circuits that have
	/// processed at least this many requests. This prevents premature circuit
	/// opening on newly added solvers with limited data.
	///
	/// Must be > 0. Default: 20
	#[serde(default = "default_circuit_breaker_min_requests")]
	pub min_requests_for_rate_check: u64,

	/// Base timeout duration in seconds when circuit opens
	///
	/// When a circuit opens, it will remain open for this duration before
	/// attempting recovery. Actual timeout may be longer due to exponential
	/// backoff on repeated failures.
	///
	/// Must be > 0 and <= max_timeout_seconds. Default: 30
	#[serde(default = "default_circuit_breaker_base_timeout")]
	pub base_timeout_seconds: u64,

	/// Maximum timeout duration in seconds (caps exponential backoff)
	///
	/// Even with exponential backoff, the circuit will never remain open longer
	/// than this duration before attempting recovery.
	///
	/// Must be > 0 and >= base_timeout_seconds. Default: 600 (10 minutes)
	#[serde(default = "default_circuit_breaker_max_timeout")]
	pub max_timeout_seconds: u64,

	/// Maximum number of test requests allowed in half-open state
	///
	/// When a circuit transitions to half-open state for testing recovery,
	/// it will allow up to this many test requests before making a decision
	/// to either close (on success) or reopen (on failure).
	///
	/// Must be > 0. Default: 3
	#[serde(default = "default_circuit_breaker_half_open_calls")]
	pub half_open_max_calls: u32,

	/// Maximum number of recovery attempts before taking persistent failure action
	///
	/// After this many failed recovery attempts, the circuit breaker will take
	/// the action specified by `persistent_failure_action`.
	///
	/// Must be > 0. Default: 10
	#[serde(default = "default_circuit_breaker_max_recovery_attempts")]
	pub max_recovery_attempts: u32,

	/// Action to take when a solver persistently fails recovery
	///
	/// Determines what happens when `max_recovery_attempts` is exceeded:
	/// - `KeepTrying`: Continue indefinitely with capped timeout (current behavior)  
	/// - `DisableSolver`: Set solver status to Disabled (requires manual intervention)
	/// - `ExtendTimeout`: Use 24-hour timeout between recovery attempts
	///
	/// Default: ExtendTimeout (reduces noise while allowing eventual recovery)
	#[serde(default = "default_circuit_breaker_persistent_failure_action")]
	pub persistent_failure_action: PersistentFailureAction,

	/// Maximum age of metrics and health checks in minutes for circuit breaker decisions
	///
	/// Circuit breaker will only evaluate success rates and health-based decisions if:
	/// - Solver metrics were updated within this time window
	/// - Health check results are within this time window
	///
	/// This prevents decisions based on stale data while keeping the implementation simple.
	/// If data is older than this, those specific evaluations are skipped
	/// (fail-open approach to avoid false positives).
	///
	/// Must be > 0. Default: 30 minutes (allows for temporary metric collection gaps)
	#[serde(default = "default_circuit_breaker_metrics_max_age")]
	pub metrics_max_age_minutes: u32,

	/// Service error rate threshold below which the circuit opens (0.0 - 1.0)
	///
	/// If a solver's recent service error rate (5xx, timeouts, network issues)
	/// exceeds this percentage, the circuit breaker will open. This provides
	/// additional protection against infrastructure-level issues.
	///
	/// Service errors are more specific than general failures and indicate
	/// solver-side problems rather than request/configuration issues.
	///
	/// Must be between 0.0 and 1.0. Default: 0.5 (50%)
	#[serde(default = "default_circuit_breaker_service_error_threshold")]
	pub service_error_threshold: f64,

	/// Duration of metrics window in minutes for windowed metrics
	///
	/// Controls how long metrics are accumulated before being reset.
	/// Shorter windows provide more recent data but less statistical significance.
	/// Longer windows provide better statistics but less responsiveness.
	///
	/// Must be > 0. Default: 15 minutes (good balance of recent data vs statistical significance)
	#[serde(default = "default_metrics_window_duration")]
	pub metrics_window_duration_minutes: u32,

	/// Maximum age of metrics window in minutes before forcing reset
	///
	/// Even if a window has insufficient data for statistical significance,
	/// it will be reset after this duration to prevent indefinite staleness.
	/// This is especially important for low-traffic solvers.
	///
	/// Must be > metrics_window_duration_minutes. Default: 60 minutes (4x normal window)
	#[serde(default = "default_metrics_max_window_age")]
	pub metrics_max_window_age_minutes: u32,
}

// Default functions for serde
fn default_circuit_breaker_enabled() -> bool {
	DEFAULT_CIRCUIT_BREAKER_ENABLED
}

fn default_circuit_breaker_failure_threshold() -> u32 {
	DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD
}

fn default_circuit_breaker_success_rate_threshold() -> f64 {
	DEFAULT_CIRCUIT_BREAKER_SUCCESS_RATE_THRESHOLD
}

fn default_circuit_breaker_min_requests() -> u64 {
	DEFAULT_CIRCUIT_BREAKER_MIN_REQUESTS_FOR_RATE_CHECK
}

fn default_circuit_breaker_base_timeout() -> u64 {
	DEFAULT_CIRCUIT_BREAKER_BASE_TIMEOUT_SECONDS
}

fn default_circuit_breaker_max_timeout() -> u64 {
	DEFAULT_CIRCUIT_BREAKER_MAX_TIMEOUT_SECONDS
}

fn default_circuit_breaker_half_open_calls() -> u32 {
	DEFAULT_CIRCUIT_BREAKER_HALF_OPEN_MAX_CALLS
}

fn default_circuit_breaker_max_recovery_attempts() -> u32 {
	DEFAULT_CIRCUIT_BREAKER_MAX_RECOVERY_ATTEMPTS
}

fn default_circuit_breaker_persistent_failure_action() -> PersistentFailureAction {
	PersistentFailureAction::ExtendTimeout
}

fn default_circuit_breaker_metrics_max_age() -> u32 {
	DEFAULT_CIRCUIT_BREAKER_METRICS_MAX_AGE_MINUTES
}

fn default_circuit_breaker_service_error_threshold() -> f64 {
	DEFAULT_CIRCUIT_BREAKER_SERVICE_ERROR_THRESHOLD
}

fn default_metrics_window_duration() -> u32 {
	oif_types::constants::limits::DEFAULT_METRICS_WINDOW_DURATION_MINUTES
}

fn default_metrics_max_window_age() -> u32 {
	oif_types::constants::limits::DEFAULT_METRICS_MAX_WINDOW_AGE_MINUTES
}

impl Default for CircuitBreakerSettings {
	fn default() -> Self {
		Self {
			enabled: default_circuit_breaker_enabled(),
			failure_threshold: default_circuit_breaker_failure_threshold(),
			success_rate_threshold: default_circuit_breaker_success_rate_threshold(),
			min_requests_for_rate_check: default_circuit_breaker_min_requests(),
			base_timeout_seconds: default_circuit_breaker_base_timeout(),
			max_timeout_seconds: default_circuit_breaker_max_timeout(),
			half_open_max_calls: default_circuit_breaker_half_open_calls(),
			max_recovery_attempts: default_circuit_breaker_max_recovery_attempts(),
			persistent_failure_action: default_circuit_breaker_persistent_failure_action(),
			metrics_max_age_minutes: default_circuit_breaker_metrics_max_age(),
			service_error_threshold: default_circuit_breaker_service_error_threshold(),
			metrics_window_duration_minutes: default_metrics_window_duration(),
			metrics_max_window_age_minutes: default_metrics_max_window_age(),
		}
	}
}

impl CircuitBreakerSettings {
	/// Validate circuit breaker configuration
	pub fn validate(&self) -> Result<(), String> {
		// All fields have concrete values (thanks to serde defaults), so validation is straightforward

		if self.failure_threshold == 0 {
			return Err("circuit_breaker.failure_threshold must be > 0".to_string());
		}

		if !(0.0..=1.0).contains(&self.success_rate_threshold) {
			return Err(format!(
				"circuit_breaker.success_rate_threshold must be between 0.0 and 1.0, got {}",
				self.success_rate_threshold
			));
		}

		if !(0.0..=1.0).contains(&self.service_error_threshold) {
			return Err(format!(
				"circuit_breaker.service_error_threshold must be between 0.0 and 1.0, got {}",
				self.service_error_threshold
			));
		}

		if self.min_requests_for_rate_check == 0 {
			return Err("circuit_breaker.min_requests_for_rate_check must be > 0".to_string());
		}

		if self.base_timeout_seconds == 0 {
			return Err("circuit_breaker.base_timeout_seconds must be > 0".to_string());
		}

		if self.max_timeout_seconds == 0 {
			return Err("circuit_breaker.max_timeout_seconds must be > 0".to_string());
		}

		if self.base_timeout_seconds > self.max_timeout_seconds {
			return Err(format!(
				"circuit_breaker.base_timeout_seconds ({}) must be <= max_timeout_seconds ({})",
				self.base_timeout_seconds, self.max_timeout_seconds
			));
		}

		if self.half_open_max_calls == 0 {
			return Err("circuit_breaker.half_open_max_calls must be > 0".to_string());
		}

		if self.max_recovery_attempts == 0 {
			return Err("circuit_breaker.max_recovery_attempts must be > 0".to_string());
		}

		if self.metrics_max_age_minutes == 0 {
			return Err("circuit_breaker.metrics_max_age_minutes must be > 0".to_string());
		}

		if self.metrics_window_duration_minutes == 0 {
			return Err("circuit_breaker.metrics_window_duration_minutes must be > 0".to_string());
		}

		if self.metrics_max_window_age_minutes == 0 {
			return Err("circuit_breaker.metrics_max_window_age_minutes must be > 0".to_string());
		}

		if self.metrics_max_window_age_minutes <= self.metrics_window_duration_minutes {
			return Err(format!(
				"circuit_breaker.metrics_max_window_age_minutes ({}) must be > metrics_window_duration_minutes ({})",
				self.metrics_max_window_age_minutes,
				self.metrics_window_duration_minutes
			));
		}

		Ok(())
	}
}

/// Default circuit breaker settings
fn default_circuit_breaker_settings() -> Option<CircuitBreakerSettings> {
	Some(CircuitBreakerSettings::default())
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
			circuit_breaker: default_circuit_breaker_settings(),
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

		// Validate circuit breaker configuration
		if let Some(cb_settings) = &self.circuit_breaker {
			cb_settings
				.validate()
				.map_err(|err| ConfigValidationError::InvalidConfig {
					field: "circuit_breaker".to_string(),
					reason: err,
				})?;
		}

		// Validate metrics configuration
		if let Some(metrics_settings) = &self.metrics {
			metrics_settings
				.validate()
				.map_err(|err| ConfigValidationError::InvalidConfig {
					field: "metrics".to_string(),
					reason: err,
				})?;
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
		self.metrics.clone().unwrap_or_default()
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

	/// Get circuit breaker configuration with defaults applied
	pub fn get_circuit_breaker(&self) -> CircuitBreakerSettings {
		self.circuit_breaker.clone().unwrap_or_default()
	}

	/// Get raw circuit breaker configuration (for validation)
	pub fn get_circuit_breaker_raw(&self) -> &Option<CircuitBreakerSettings> {
		&self.circuit_breaker
	}

	/// Validate circuit breaker configuration specifically
	pub fn validate_circuit_breaker(&self) -> Result<(), String> {
		// Validate circuit breaker settings if provided
		if let Some(ref cb_settings) = self.circuit_breaker {
			cb_settings
				.validate()
				.map_err(|err| format!("Configuration validation failed: {}", err))?;
		}

		Ok(())
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
			circuit_breaker: None,
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
			circuit_breaker: None,
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
			circuit_breaker: None,
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
			circuit_breaker: None,
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
			circuit_breaker: None,
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

	#[test]
	fn test_circuit_breaker_optional_fields() {
		use oif_types::constants::limits::*;

		// Test that users can specify minimal configuration
		let minimal_config = CircuitBreakerSettings {
			enabled: true,
			failure_threshold: 3,
			..Default::default()
		};

		assert_eq!(minimal_config.enabled, true);
		assert_eq!(minimal_config.failure_threshold, 3);
		// Other fields should use defaults
		assert_eq!(
			minimal_config.success_rate_threshold,
			DEFAULT_CIRCUIT_BREAKER_SUCCESS_RATE_THRESHOLD
		);
		assert_eq!(
			minimal_config.base_timeout_seconds,
			DEFAULT_CIRCUIT_BREAKER_BASE_TIMEOUT_SECONDS
		);
	}

	#[test]
	fn test_circuit_breaker_validation_valid() {
		let valid_config = CircuitBreakerSettings {
			enabled: true,
			failure_threshold: 5,
			success_rate_threshold: 0.3,
			base_timeout_seconds: 30,
			max_timeout_seconds: 600,
			..Default::default()
		};

		assert!(valid_config.validate().is_ok());
	}

	#[test]
	fn test_circuit_breaker_validation_invalid_failure_threshold() {
		let invalid_config = CircuitBreakerSettings {
			failure_threshold: 0, // Invalid: must be > 0
			..Default::default()
		};

		let result = invalid_config.validate();
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.contains("failure_threshold must be > 0"));
	}

	#[test]
	fn test_circuit_breaker_validation_invalid_success_rate() {
		let invalid_config = CircuitBreakerSettings {
			success_rate_threshold: 1.5, // Invalid: must be 0.0-1.0
			..Default::default()
		};

		let result = invalid_config.validate();
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.contains("success_rate_threshold must be between 0.0 and 1.0"));
	}

	#[test]
	fn test_circuit_breaker_validation_timeout_relationship() {
		let invalid_config = CircuitBreakerSettings {
			base_timeout_seconds: 600,
			max_timeout_seconds: 300, // Invalid: base > max
			..Default::default()
		};

		let result = invalid_config.validate();
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.contains("base_timeout_seconds (600) must be <= max_timeout_seconds (300)"));
	}

	#[test]
	fn test_circuit_breaker_defaults_vs_validation() {
		use oif_types::constants::limits::*;

		// Default should have concrete values (applied by serde defaults)
		let default_config = CircuitBreakerSettings::default();
		assert_eq!(default_config.enabled, DEFAULT_CIRCUIT_BREAKER_ENABLED);
		assert_eq!(
			default_config.failure_threshold,
			DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD
		);

		// Validation should work with defaults applied
		assert!(default_config.validate().is_ok());

		// Config should have actual values directly accessible
		assert_eq!(default_config.enabled, DEFAULT_CIRCUIT_BREAKER_ENABLED);
		assert_eq!(
			default_config.failure_threshold,
			DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD
		);
	}

	#[test]
	fn test_circuit_breaker_serde_defaults() {
		// Test that serde defaults work correctly during deserialization
		let json_config = r#"{"failure_threshold": 3}"#;
		let config: CircuitBreakerSettings = serde_json::from_str(json_config).unwrap();

		// Validation should pass (defaults applied automatically by serde)
		assert!(config.validate().is_ok());

		// Config should have our override and defaults
		assert_eq!(config.failure_threshold, 3); // Our override
		assert_eq!(
			config.base_timeout_seconds,
			DEFAULT_CIRCUIT_BREAKER_BASE_TIMEOUT_SECONDS
		); // Default
		assert_eq!(config.enabled, DEFAULT_CIRCUIT_BREAKER_ENABLED); // Default
	}

	#[test]
	fn test_circuit_breaker_validation_catches_bad_relationships() {
		// This should fail validation because the relationship is invalid
		let invalid_config = CircuitBreakerSettings {
			base_timeout_seconds: 600,
			max_timeout_seconds: 300, // Invalid relationship
			..Default::default()
		};

		// Validation should catch this invalid relationship
		let result = invalid_config.validate();
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.contains("base_timeout_seconds (600) must be <= max_timeout_seconds (300)"));
	}

	#[test]
	fn test_circuit_breaker_validation_with_custom_base_timeout() {
		// Test that custom base timeout works with default max timeout
		let custom_config = CircuitBreakerSettings {
			base_timeout_seconds: 600, // Custom base timeout
			..Default::default()
		};

		// Validation should pass since default max_timeout (600) >= our base_timeout (600)
		assert!(custom_config.validate().is_ok());

		// Config should have our custom value and defaults
		assert_eq!(custom_config.base_timeout_seconds, 600);
		assert_eq!(
			custom_config.max_timeout_seconds,
			DEFAULT_CIRCUIT_BREAKER_MAX_TIMEOUT_SECONDS
		); // 600
		assert!(custom_config.base_timeout_seconds <= custom_config.max_timeout_seconds);
	}

	#[test]
	fn test_metrics_settings_defaults() {
		use oif_types::constants::limits::*;

		// Test that default configuration has correct values
		let default_config = MetricsSettings::default();

		// All fields should have default values
		assert_eq!(
			default_config.retention_hours,
			DEFAULT_METRICS_RETENTION_HOURS
		);
		assert_eq!(
			default_config.cleanup_interval_hours,
			DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS
		);
		assert_eq!(
			default_config.aggregation_interval_minutes,
			DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES
		);
		assert_eq!(
			default_config.min_timeout_for_metrics_ms,
			DEFAULT_SOLVER_TIMEOUT_MS
		);
	}

	#[test]
	fn test_metrics_settings_serde_defaults() {
		use oif_types::constants::limits::*;

		// Test that serde defaults work correctly during deserialization
		let json_config = r#"{"retention_hours": 48}"#;
		let config: MetricsSettings = serde_json::from_str(json_config).unwrap();

		// Validation should pass (defaults applied automatically by serde)
		assert!(config.validate().is_ok());

		// Config should have our override and defaults
		assert_eq!(config.retention_hours, 48); // Our override
		assert_eq!(
			config.cleanup_interval_hours,
			DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS
		); // Default
		assert_eq!(
			config.aggregation_interval_minutes,
			DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES
		); // Default
		assert_eq!(config.min_timeout_for_metrics_ms, DEFAULT_SOLVER_TIMEOUT_MS); // Default
	}

	#[test]
	fn test_metrics_settings_validation_valid() {
		let valid_config = MetricsSettings {
			retention_hours: 168,
			cleanup_interval_hours: 24,
			aggregation_interval_minutes: 5,
			min_timeout_for_metrics_ms: 5000,
		};

		assert!(valid_config.validate().is_ok());
	}

	#[test]
	fn test_metrics_settings_validation_invalid() {
		// Test zero values (should fail)
		let invalid_configs = vec![
			MetricsSettings {
				retention_hours: 0, // Invalid
				..Default::default()
			},
			MetricsSettings {
				cleanup_interval_hours: 0, // Invalid
				..Default::default()
			},
			MetricsSettings {
				aggregation_interval_minutes: 0, // Invalid
				..Default::default()
			},
			MetricsSettings {
				min_timeout_for_metrics_ms: 0, // Invalid
				..Default::default()
			},
		];

		for config in invalid_configs {
			let result = config.validate();
			assert!(
				result.is_err(),
				"Expected validation to fail for config: {:?}",
				config
			);
		}
	}

	#[test]
	fn test_metrics_settings_validation_integration() {
		use crate::configurable_value::ConfigurableValue;

		// Test that metrics validation is integrated into main Settings validation
		let mut settings = Settings {
			solvers: std::collections::HashMap::new(),
			metrics: Some(MetricsSettings {
				retention_hours: 0, // Invalid
				..Default::default()
			}),
			security: SecuritySettings {
				integrity_secret: ConfigurableValue::from_plain(
					"test-secret-long-enough-for-validation",
				),
			},
			..Settings::default()
		};

		// Insert a valid solver to avoid other validation errors
		settings.solvers.insert(
			"test-solver".to_string(),
			SolverConfig {
				solver_id: "test-solver".to_string(),
				adapter_id: "oif".to_string(),
				endpoint: "http://test.example.com".to_string(),
				enabled: true,
				headers: None,
				adapter_metadata: None,
				name: Some("Test Solver".to_string()),
				description: Some("Test description".to_string()),
				supported_assets: None,
			},
		);

		let result = settings.validate();
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("metrics"));
	}

	#[test]
	fn test_circuit_breaker_validation_with_invalid_config() {
		use serde_json::json;

		// Create an invalid configuration
		let invalid_config_json = json!({
			"solvers": {
				"test-solver": {
					"solver_id": "test-solver",
					"adapter_id": "oif",
					"endpoint": "http://test.example.com",
					"enabled": true
				}
			},
			"circuit_breaker": {
				"enabled": true,
				"failure_threshold": 0,  // Invalid: must be > 0
				"success_rate_threshold": 1.5,  // Invalid: must be <= 1.0
				"base_timeout_seconds": 600,  // Will be invalid when combined with max_timeout
				"max_timeout_seconds": 300   // Invalid: base > max
			},
			"security": {
				"integrity_secret": {
					"type": "plain",
					"value": "test-secret-that-is-long-enough-for-validation"
				}
			}
		});

		// Parse should succeed
		let settings: Settings = serde_json::from_value(invalid_config_json)
			.expect("Configuration should parse successfully");

		// But validation should fail
		let result = settings.validate();
		assert!(result.is_err(), "Validation should fail for invalid config");

		let error_msg = result.unwrap_err().to_string();
		assert!(
			error_msg.contains("circuit_breaker"),
			"Error should mention circuit_breaker"
		);
		assert!(
			error_msg.contains("failure_threshold must be > 0"),
			"Error should mention failure_threshold"
		);
	}

	#[test]
	fn test_circuit_breaker_validation_with_valid_config() {
		use serde_json::json;

		// Create a valid configuration
		let valid_config_json = json!({
			"solvers": {
				"test-solver": {
					"solver_id": "test-solver",
					"adapter_id": "oif",
					"endpoint": "http://test.example.com",
					"enabled": true
				}
			},
			"circuit_breaker": {
				"enabled": true,
				"failure_threshold": 5,
				"success_rate_threshold": 0.3,
				"base_timeout_seconds": 30,
				"max_timeout_seconds": 600
			},
			"security": {
				"integrity_secret": {
					"type": "plain",
					"value": "test-secret-that-is-long-enough-for-validation"
				}
			}
		});

		// Parse should succeed
		let settings: Settings = serde_json::from_value(valid_config_json)
			.expect("Configuration should parse successfully");

		// Validation should also succeed
		let result = settings.validate();
		assert!(
			result.is_ok(),
			"Validation should pass for valid config: {:?}",
			result
		);
	}

	#[test]
	fn test_partial_circuit_breaker_config_validation() {
		use serde_json::json;

		// Test that partial config (only some fields provided) validates correctly
		let partial_config_json = json!({
			"solvers": {
				"test-solver": {
					"solver_id": "test-solver",
					"adapter_id": "oif",
					"endpoint": "http://test.example.com",
					"enabled": true
				}
			},
			"circuit_breaker": {
				"enabled": true,
				"failure_threshold": 3,  // Only override this field
				// All other fields should use defaults
			},
			"security": {
				"integrity_secret": {
					"type": "plain",
					"value": "test-secret-that-is-long-enough-for-validation"
				}
			}
		});

		let settings: Settings = serde_json::from_value(partial_config_json)
			.expect("Configuration should parse successfully");

		// Validation should pass (defaults applied to missing fields)
		let result = settings.validate();
		assert!(
			result.is_ok(),
			"Validation should pass for partial config with defaults: {:?}",
			result
		);

		// Verify that resolved config has our override and defaults
		let circuit_config = settings.get_circuit_breaker();
		assert_eq!(circuit_config.failure_threshold, 3); // Our override
		assert_eq!(circuit_config.success_rate_threshold, 0.2); // Default
	}
}
