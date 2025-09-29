//! Configuration builders and fixtures for testing
//!
//! This module provides helper functions to create test configurations
//! with various circuit breaker settings and solver configurations.

use oif_config::{settings::SecuritySettings, CircuitBreakerSettings, ConfigurableValue, Settings};
use oif_types::PersistentFailureAction;
use std::time::Duration;

/// Test solver configuration structure for adapter tests
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestSolverConfig {
	pub solver_id: String,
	pub adapter_id: String,
	pub endpoint: String,
	pub enabled: bool,
}

/// Builder for circuit breaker test configurations
#[allow(dead_code)]
pub struct CircuitBreakerConfigs;

#[allow(dead_code)]
impl CircuitBreakerConfigs {
	/// Standard circuit breaker configuration for most tests
	pub fn standard() -> CircuitBreakerSettings {
		CircuitBreakerSettings {
			enabled: true,
			failure_threshold: 3,
			success_rate_threshold: 0.5,
			min_requests_for_rate_check: 5,
			base_timeout_seconds: 30,
			max_timeout_seconds: 300,
			half_open_max_calls: 3,
			max_recovery_attempts: 5,
			persistent_failure_action: PersistentFailureAction::ExtendTimeout,
			metrics_max_age_minutes: 10,
			service_error_threshold: 0.5,
			metrics_window_duration_minutes: 15,
			metrics_max_window_age_minutes: 60,
		}
	}

	/// Strict configuration that opens quickly on failures
	pub fn strict_failure_threshold(threshold: u32) -> CircuitBreakerSettings {
		CircuitBreakerSettings {
			enabled: true,
			failure_threshold: threshold,
			success_rate_threshold: 0.1,     // Very lenient success rate
			min_requests_for_rate_check: 20, // High threshold to avoid rate checks
			base_timeout_seconds: 60,
			max_timeout_seconds: 600,
			half_open_max_calls: 2,
			max_recovery_attempts: 3,
			persistent_failure_action: PersistentFailureAction::ExtendTimeout,
			metrics_max_age_minutes: 5,
			service_error_threshold: 0.5,
			metrics_window_duration_minutes: 15,
			metrics_max_window_age_minutes: 60,
		}
	}

	/// Configuration with strict success rate requirements
	pub fn strict_success_rate(rate: f64, min_requests: u64) -> CircuitBreakerSettings {
		CircuitBreakerSettings {
			enabled: true,
			failure_threshold: 100, // High failure threshold to focus on rate
			success_rate_threshold: rate,
			min_requests_for_rate_check: min_requests,
			base_timeout_seconds: 30,
			max_timeout_seconds: 300,
			half_open_max_calls: 3,
			max_recovery_attempts: 5,
			persistent_failure_action: PersistentFailureAction::ExtendTimeout,
			metrics_max_age_minutes: 5,
			service_error_threshold: 0.5,
			metrics_window_duration_minutes: 15,
			metrics_max_window_age_minutes: 60,
		}
	}

	/// Configuration with fast timeout for testing timeout behavior
	pub fn fast_timeout(timeout: Duration) -> CircuitBreakerSettings {
		let timeout_secs = timeout.as_secs();
		CircuitBreakerSettings {
			enabled: true,
			failure_threshold: 5,
			success_rate_threshold: 0.3,
			min_requests_for_rate_check: 10,
			base_timeout_seconds: timeout_secs,
			max_timeout_seconds: timeout_secs * 2,
			half_open_max_calls: 2,
			max_recovery_attempts: 3,
			persistent_failure_action: PersistentFailureAction::ExtendTimeout,
			metrics_max_age_minutes: 5,
			service_error_threshold: 0.5,
			metrics_window_duration_minutes: 15,
			metrics_max_window_age_minutes: 60,
		}
	}

	/// Configuration for testing recovery scenarios
	pub fn fast_recovery(failure_threshold: u32, base_timeout: Duration) -> CircuitBreakerSettings {
		CircuitBreakerSettings {
			enabled: true,
			failure_threshold,
			success_rate_threshold: 0.3,
			min_requests_for_rate_check: 10,
			base_timeout_seconds: base_timeout.as_secs(),
			max_timeout_seconds: base_timeout.as_secs() * 3,
			half_open_max_calls: 1,
			max_recovery_attempts: 2,
			persistent_failure_action: PersistentFailureAction::ExtendTimeout,
			metrics_max_age_minutes: 2, // Short age for quick recovery tests
			service_error_threshold: 0.5,
			metrics_window_duration_minutes: 15,
			metrics_max_window_age_minutes: 60,
		}
	}

	/// Configuration with circuit breaker disabled
	pub fn disabled() -> CircuitBreakerSettings {
		CircuitBreakerSettings {
			enabled: false,
			failure_threshold: 3,
			success_rate_threshold: 0.5,
			min_requests_for_rate_check: 5,
			base_timeout_seconds: 30,
			max_timeout_seconds: 300,
			half_open_max_calls: 3,
			max_recovery_attempts: 5,
			persistent_failure_action: PersistentFailureAction::ExtendTimeout,
			metrics_max_age_minutes: 10,
			service_error_threshold: 0.5,
			metrics_window_duration_minutes: 15,
			metrics_max_window_age_minutes: 60,
		}
	}

	/// Test solver configuration for adapter tests
	pub fn test_solver_config() -> TestSolverConfig {
		TestSolverConfig {
			solver_id: "test-solver".to_string(),
			adapter_id: "test-adapter".to_string(),
			endpoint: "http://localhost:8080".to_string(),
			enabled: true,
		}
	}

	/// Test settings for metrics tests
	pub fn test_settings() -> Settings {
		let mut settings = Settings::default();
		settings.security.integrity_secret =
			ConfigurableValue::from_plain("test-secret-for-metrics-e2e-tests-12345678901234567890");
		settings
	}
}

#[allow(dead_code)]
/// Creates a complete Settings object with circuit breaker configuration
pub fn create_test_settings_with_circuit_breaker(cb_config: CircuitBreakerSettings) -> Settings {
	Settings {
		circuit_breaker: Some(cb_config),
		security: SecuritySettings {
			integrity_secret: ConfigurableValue::from_plain(
				"test-secret-for-e2e-tests-12345678901234567890",
			),
		},
		..Settings::default()
	}
}
