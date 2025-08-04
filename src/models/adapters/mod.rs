//! Core Adapter domain model and business logic

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod config;
pub mod errors;
pub mod response;
pub mod storage;

pub use config::{AdapterConfig, AdapterConfigBuilder, AdapterType};
pub use errors::{
    AdapterCapability, AdapterError, AdapterFactoryError, AdapterFactoryResult,
    AdapterHealthResult, AdapterPerformanceMetrics, AdapterResult, AdapterValidationError,
    AdapterValidationResult,
};
pub use response::AdapterResponse;
pub use storage::AdapterStorage;

/// Core Adapter domain model
///
/// This represents an adapter in the domain layer with business logic.
/// It should be converted from AdapterConfig and to AdapterStorage/AdapterResponse.
#[derive(Debug, Clone, PartialEq)]
pub struct Adapter {
    /// Unique identifier for the adapter
    pub adapter_id: String,

    /// Type of adapter (OIF, Uniswap, etc.)
    pub adapter_type: AdapterType,

    /// Human-readable name
    pub name: String,

    /// Description of the adapter
    pub description: Option<String>,

    /// Version of the adapter implementation
    pub version: String,

    /// Supported blockchain networks
    pub supported_chains: Vec<u64>,

    /// Adapter-specific configuration
    pub configuration: HashMap<String, serde_json::Value>,

    /// Whether the adapter is enabled
    pub enabled: bool,

    /// Current operational status
    pub status: AdapterStatus,

    /// Performance and health metrics
    pub metrics: AdapterMetrics,

    /// Available capabilities
    pub capabilities: Vec<AdapterCapability>,

    /// When the adapter was created/registered
    pub created_at: DateTime<Utc>,

    /// Last time the adapter was updated
    pub updated_at: DateTime<Utc>,

    /// Last time the adapter was accessed
    pub last_accessed: Option<DateTime<Utc>>,
}

/// Adapter operational status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AdapterStatus {
    /// Adapter is active and available
    Active,
    /// Adapter is temporarily inactive
    Inactive,
    /// Adapter has encountered errors
    Error,
    /// Adapter is being initialized
    Initializing,
    /// Adapter is in maintenance mode
    Maintenance,
    /// Adapter is deprecated and should not be used
    Deprecated,
}

/// Adapter performance and health metrics
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterMetrics {
    /// Performance metrics
    pub performance: AdapterPerformanceMetrics,

    /// Last health check result
    pub last_health_check: Option<AdapterHealthResult>,

    /// Consecutive health check failures
    pub consecutive_failures: u32,

    /// Total number of times adapter was accessed
    pub total_accesses: u64,

    /// Last time metrics were updated
    pub last_updated: DateTime<Utc>,

    /// Adapter uptime percentage
    pub uptime_percentage: f64,

    /// Last error message
    pub last_error: Option<String>,

    /// Last error timestamp
    pub last_error_at: Option<DateTime<Utc>>,
}

impl Adapter {
    /// Create a new adapter
    pub fn new(
        adapter_id: String,
        adapter_type: AdapterType,
        name: String,
        version: String,
    ) -> Self {
        let now = Utc::now();

        Self {
            adapter_id,
            adapter_type,
            name,
            description: None,
            version,
            supported_chains: Vec::new(),
            configuration: HashMap::new(),
            enabled: true,
            status: AdapterStatus::Initializing,
            metrics: AdapterMetrics::new(),
            capabilities: Vec::new(),
            created_at: now,
            updated_at: now,
            last_accessed: None,
        }
    }

    /// Check if the adapter is available for use
    pub fn is_available(&self) -> bool {
        self.enabled && matches!(self.status, AdapterStatus::Active)
    }

    /// Check if the adapter is healthy based on recent metrics
    pub fn is_healthy(&self) -> bool {
        if let Some(ref health_check) = self.metrics.last_health_check {
            health_check.is_healthy && health_check.consecutive_failures < 3
        } else {
            // No health check data means we don't know if it's healthy
            self.status == AdapterStatus::Active
        }
    }

    /// Get health score (0.0 to 1.0)
    pub fn health_score(&self) -> f64 {
        if let Some(ref health_check) = self.metrics.last_health_check {
            health_check.health_score
        } else {
            match self.status {
                AdapterStatus::Active => 0.8,
                AdapterStatus::Initializing => 0.5,
                AdapterStatus::Inactive => 0.3,
                AdapterStatus::Maintenance => 0.2,
                AdapterStatus::Error => 0.1,
                AdapterStatus::Deprecated => 0.0,
            }
        }
    }

    /// Check if adapter supports a specific chain
    pub fn supports_chain(&self, chain_id: u64) -> bool {
        self.supported_chains.contains(&chain_id)
    }

    /// Check if adapter has a specific capability
    pub fn has_capability(&self, capability_name: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.name == capability_name && cap.available)
    }

    /// Get configuration value
    pub fn get_config<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.configuration
            .get(key)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// Set configuration value
    pub fn set_config<T>(&mut self, key: String, value: T) -> AdapterResult<()>
    where
        T: serde::Serialize,
    {
        let json_value = serde_json::to_value(value).map_err(AdapterError::Serialization)?;
        self.configuration.insert(key, json_value);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update adapter status
    pub fn update_status(&mut self, status: AdapterStatus) {
        self.status = status;
        self.updated_at = Utc::now();

        // Reset consecutive failures if becoming active
        if matches!(self.status, AdapterStatus::Active) {
            self.metrics.consecutive_failures = 0;
        }
    }

    /// Mark adapter as accessed
    pub fn mark_accessed(&mut self) {
        self.last_accessed = Some(Utc::now());
        self.metrics.total_accesses += 1;
        self.metrics.last_updated = Utc::now();
    }

    /// Record a successful operation
    pub fn record_success(&mut self, response_time_ms: u64) {
        self.metrics.performance.record_success(response_time_ms);
        self.metrics.consecutive_failures = 0;
        self.metrics.last_updated = Utc::now();
        self.mark_accessed();

        // Auto-activate if it was in error state
        if matches!(
            self.status,
            AdapterStatus::Error | AdapterStatus::Initializing
        ) {
            self.status = AdapterStatus::Active;
        }
    }

    /// Record a failed operation
    pub fn record_failure(&mut self, error_message: String, is_timeout: bool) {
        self.metrics.performance.record_failure(is_timeout);
        self.metrics.consecutive_failures += 1;
        self.metrics.last_error = Some(error_message);
        self.metrics.last_error_at = Some(Utc::now());
        self.metrics.last_updated = Utc::now();
        self.mark_accessed();

        // Auto-deactivate if too many failures
        if self.metrics.consecutive_failures >= 5 {
            self.status = AdapterStatus::Error;
        }
    }

    /// Record health check result
    pub fn record_health_check(&mut self, result: AdapterHealthResult) {
        self.metrics.consecutive_failures = result.consecutive_failures;
        self.metrics.last_health_check = Some(result.clone());
        self.metrics.last_updated = Utc::now();
        self.mark_accessed();

        // Update status based on health check
        if result.is_healthy {
            if matches!(
                self.status,
                AdapterStatus::Error | AdapterStatus::Initializing
            ) {
                self.status = AdapterStatus::Active;
            }
        } else if result.consecutive_failures >= 3 {
            self.status = AdapterStatus::Error;
        }

        // Update capabilities from health check
        self.capabilities = result
            .capabilities
            .into_iter()
            .map(|name| AdapterCapability::new(name, true))
            .collect();
    }

    /// Calculate priority score for adapter selection (higher is better)
    pub fn priority_score(&self) -> f64 {
        if !self.is_available() {
            return 0.0;
        }

        let base_score = 100.0;
        let health_bonus = self.health_score() * 50.0;
        let performance_bonus = if self.metrics.performance.is_performing_well() {
            25.0
        } else {
            self.metrics.performance.success_rate * 25.0
        };
        let response_time_penalty =
            (self.metrics.performance.avg_response_time_ms / 100.0).min(25.0);
        let failure_penalty = self.metrics.consecutive_failures as f64 * 10.0;

        (base_score + health_bonus + performance_bonus - response_time_penalty - failure_penalty)
            .max(0.0)
    }

    /// Check if adapter has been inactive for too long
    pub fn is_stale(&self, max_age: Duration) -> bool {
        if let Some(last_accessed) = self.last_accessed {
            Utc::now() - last_accessed > max_age
        } else {
            Utc::now() - self.created_at > max_age
        }
    }

    /// Enable the adapter
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
        if matches!(self.status, AdapterStatus::Inactive) {
            self.status = AdapterStatus::Active;
        }
    }

    /// Disable the adapter
    pub fn disable(&mut self) {
        self.enabled = false;
        self.status = AdapterStatus::Inactive;
        self.updated_at = Utc::now();
    }

    /// Add a capability to the adapter
    pub fn add_capability(&mut self, capability: AdapterCapability) {
        // Remove existing capability with same name
        self.capabilities.retain(|cap| cap.name != capability.name);
        self.capabilities.push(capability);
        self.updated_at = Utc::now();
    }

    /// Remove a capability from the adapter
    pub fn remove_capability(&mut self, capability_name: &str) {
        self.capabilities.retain(|cap| cap.name != capability_name);
        self.updated_at = Utc::now();
    }

    /// Builder methods for easy configuration
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_chains(mut self, chains: Vec<u64>) -> Self {
        self.supported_chains = chains;
        self
    }

    pub fn with_capability(mut self, capability: AdapterCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn with_config<T>(mut self, key: String, value: T) -> AdapterResult<Self>
    where
        T: serde::Serialize,
    {
        self.set_config(key, value)?;
        Ok(self)
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.status = if enabled {
            AdapterStatus::Active
        } else {
            AdapterStatus::Inactive
        };
        self
    }
}

impl AdapterMetrics {
    pub fn new() -> Self {
        Self {
            performance: AdapterPerformanceMetrics::new(),
            last_health_check: None,
            consecutive_failures: 0,
            total_accesses: 0,
            last_updated: Utc::now(),
            uptime_percentage: 100.0,
            last_error: None,
            last_error_at: None,
        }
    }

    /// Calculate uptime percentage based on health checks
    pub fn calculate_uptime(&self) -> f64 {
        // Simplified uptime calculation
        // In a real system, this would be based on historical data
        if let Some(ref health_check) = self.last_health_check {
            if health_check.is_healthy {
                (100.0 - (self.consecutive_failures as f64 * 5.0)).max(0.0)
            } else {
                (90.0 - (self.consecutive_failures as f64 * 10.0)).max(0.0)
            }
        } else {
            self.uptime_percentage
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        self.performance.reset();
        self.last_health_check = None;
        self.consecutive_failures = 0;
        self.last_error = None;
        self.last_error_at = None;
        self.last_updated = Utc::now();
        self.uptime_percentage = 100.0;
    }
}

impl Default for AdapterMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_adapter() -> Adapter {
        Adapter::new(
            "test-adapter".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
            "1.0.0".to_string(),
        )
    }

    #[test]
    fn test_adapter_creation() {
        let adapter = create_test_adapter();

        assert_eq!(adapter.adapter_id, "test-adapter");
        assert_eq!(adapter.adapter_type, AdapterType::OifV1);
        assert_eq!(adapter.name, "Test Adapter");
        assert_eq!(adapter.version, "1.0.0");
        assert_eq!(adapter.status, AdapterStatus::Initializing);
        assert!(adapter.enabled);
        assert!(!adapter.is_available()); // Initializing is not available
    }

    #[test]
    fn test_adapter_availability() {
        let mut adapter = create_test_adapter();

        assert!(!adapter.is_available());

        adapter.update_status(AdapterStatus::Active);
        assert!(adapter.is_available());

        adapter.disable();
        assert!(!adapter.is_available());

        adapter.enable();
        assert!(adapter.is_available());
    }

    #[test]
    fn test_chain_support() {
        let adapter = create_test_adapter().with_chains(vec![1, 137, 42161]);

        assert!(adapter.supports_chain(1));
        assert!(adapter.supports_chain(137));
        assert!(!adapter.supports_chain(56));
    }

    #[test]
    fn test_capability_management() {
        let mut adapter = create_test_adapter();

        assert!(!adapter.has_capability("quotes"));

        adapter.add_capability(AdapterCapability::new("quotes".to_string(), true));
        assert!(adapter.has_capability("quotes"));

        adapter.remove_capability("quotes");
        assert!(!adapter.has_capability("quotes"));
    }

    #[test]
    fn test_configuration() {
        let mut adapter = create_test_adapter();

        adapter.set_config("timeout".to_string(), 5000u64).unwrap();
        let timeout: Option<u64> = adapter.get_config("timeout");
        assert_eq!(timeout, Some(5000));

        let missing: Option<String> = adapter.get_config("missing");
        assert_eq!(missing, None);
    }

    #[test]
    fn test_metrics_recording() {
        let mut adapter = create_test_adapter();

        // Record some successes
        adapter.record_success(100);
        adapter.record_success(200);

        assert_eq!(adapter.metrics.performance.total_requests, 2);
        assert_eq!(adapter.metrics.performance.successful_requests, 2);
        assert_eq!(adapter.metrics.performance.success_rate, 1.0);
        assert_eq!(adapter.metrics.total_accesses, 2);

        // Record a failure
        adapter.record_failure("Test error".to_string(), false);

        assert_eq!(adapter.metrics.performance.total_requests, 3);
        assert_eq!(adapter.metrics.performance.failed_requests, 1);
        assert!((adapter.metrics.performance.success_rate - 0.6667).abs() < 0.001);
        assert_eq!(adapter.metrics.last_error, Some("Test error".to_string()));
    }

    #[test]
    fn test_auto_status_updates() {
        let mut adapter = create_test_adapter();

        // Success should activate initializing adapter
        adapter.record_success(100);
        assert_eq!(adapter.status, AdapterStatus::Active);

        // Multiple failures should deactivate
        for _ in 0..5 {
            adapter.record_failure("Test error".to_string(), false);
        }
        assert_eq!(adapter.status, AdapterStatus::Error);

        // Health check success should reactivate
        let health_result = AdapterHealthResult::healthy(150);
        adapter.record_health_check(health_result);
        assert_eq!(adapter.status, AdapterStatus::Active);
    }

    #[test]
    fn test_priority_score() {
        let mut adapter = create_test_adapter();
        adapter.update_status(AdapterStatus::Active);

        // Initial score should be positive
        let initial_score = adapter.priority_score();
        assert!(initial_score > 0.0);

        // Record success should improve score
        adapter.record_success(100);
        let improved_score = adapter.priority_score();
        assert!(improved_score >= initial_score);

        // Record failures should decrease score
        adapter.record_failure("Error".to_string(), false);
        adapter.record_failure("Error".to_string(), false);
        let decreased_score = adapter.priority_score();
        assert!(decreased_score < improved_score);
    }

    #[test]
    fn test_builder_pattern() {
        let adapter = create_test_adapter()
            .with_description("Test Description".to_string())
            .with_chains(vec![1, 137])
            .with_capability(AdapterCapability::new("quotes".to_string(), true))
            .enabled(true);

        assert_eq!(adapter.description, Some("Test Description".to_string()));
        assert_eq!(adapter.supported_chains, vec![1, 137]);
        assert!(adapter.has_capability("quotes"));
        assert!(adapter.enabled);
    }
}
