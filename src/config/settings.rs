//! Configuration settings structures

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
}

/// Server configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
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

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerSettings {
                host: "0.0.0.0".to_string(),
                port: 3000,
                workers: None,
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
}
