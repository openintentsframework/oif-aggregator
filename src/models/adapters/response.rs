//! Adapter response models for API layer

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    Adapter, AdapterCapability, AdapterHealthResult, AdapterPerformanceMetrics, AdapterResult,
    AdapterStatus, AdapterType,
};

/// Response format for individual adapters in API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterResponse {
    pub adapter_id: String,
    pub adapter_type: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub status: AdapterStatusResponse,
    pub supported_chains: Vec<u64>,
    pub capabilities: Vec<AdapterCapabilityResponse>,
    pub metrics: AdapterMetricsResponse,
    pub created_at: i64,
    pub last_accessed: Option<i64>,
    pub is_healthy: bool,
    pub health_score: f64,
    pub priority_score: f64,
}

/// Adapter status for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdapterStatusResponse {
    Active,
    Inactive,
    Error,
    Initializing,
    Maintenance,
    Deprecated,
}

/// Adapter capability for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCapabilityResponse {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// Adapter metrics for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMetricsResponse {
    pub performance: AdapterPerformanceResponse,
    pub health: Option<AdapterHealthResponse>,
    pub uptime_percentage: f64,
    pub total_accesses: u64,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub last_updated: i64,
}

/// Performance metrics for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterPerformanceResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub timeout_requests: u64,
    pub success_rate: f64,
    pub error_rate: f64,
    pub avg_response_time_ms: f64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
    pub p95_response_time_ms: u64,
    pub is_performing_well: bool,
}

/// Health check result for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterHealthResponse {
    pub is_healthy: bool,
    pub status: String,
    pub response_time_ms: u64,
    pub health_score: f64,
    pub consecutive_failures: u32,
    pub last_check: i64,
    pub error_message: Option<String>,
    pub capabilities: Vec<String>,
}

/// Collection of adapters response for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptersResponse {
    pub adapters: Vec<AdapterResponse>,
    pub total_adapters: usize,
    pub active_adapters: usize,
    pub healthy_adapters: usize,
    pub deprecated_adapters: usize,
    pub timestamp: i64,
    pub system_health_score: f64,
}

/// Adapter registry summary for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterRegistryResponse {
    pub supported_types: Vec<AdapterTypeInfo>,
    pub total_registered: usize,
    pub active_count: usize,
    pub healthy_count: usize,
    pub average_health_score: f64,
    pub supported_chains: Vec<u64>,
    pub total_capabilities: Vec<String>,
    pub last_updated: i64,
}

/// Information about adapter types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterTypeInfo {
    pub type_name: String,
    pub display_name: String,
    pub registered_count: usize,
    pub active_count: usize,
    pub default_chains: Vec<u64>,
    pub supported_operations: Vec<String>,
}

/// Adapter configuration summary for admin endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfigResponse {
    pub adapter_id: String,
    pub adapter_type: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub configuration: serde_json::Value,
    pub supported_chains: Vec<u64>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub required_capabilities: Vec<String>,
    pub optional_capabilities: Vec<String>,
    pub created_at: i64,
}

impl AdapterResponse {
    /// Create adapter response from domain adapter
    pub fn from_domain(adapter: &Adapter) -> AdapterResult<Self> {
        Ok(Self {
            adapter_id: adapter.adapter_id.clone(),
            adapter_type: adapter.adapter_type.to_string(),
            name: adapter.name.clone(),
            description: adapter.description.clone(),
            version: adapter.version.clone(),
            status: AdapterStatusResponse::from_domain(&adapter.status),
            supported_chains: adapter.supported_chains.clone(),
            capabilities: adapter
                .capabilities
                .iter()
                .map(AdapterCapabilityResponse::from_domain)
                .collect(),
            metrics: AdapterMetricsResponse::from_domain(adapter),
            created_at: adapter.created_at.timestamp(),
            last_accessed: adapter.last_accessed.map(|dt| dt.timestamp()),
            is_healthy: adapter.is_healthy(),
            health_score: adapter.health_score(),
            priority_score: adapter.priority_score(),
        })
    }

    /// Create minimal adapter response (for public API)
    pub fn minimal_from_domain(adapter: &Adapter) -> AdapterResult<Self> {
        Ok(Self {
            adapter_id: adapter.adapter_id.clone(),
            adapter_type: adapter.adapter_type.to_string(),
            name: adapter.name.clone(),
            description: adapter.description.clone(),
            version: adapter.version.clone(),
            status: AdapterStatusResponse::from_domain(&adapter.status),
            supported_chains: adapter.supported_chains.clone(),
            capabilities: adapter
                .capabilities
                .iter()
                .filter(|cap| cap.available) // Only show available capabilities
                .map(AdapterCapabilityResponse::from_domain)
                .collect(),
            metrics: AdapterMetricsResponse::minimal_from_domain(adapter),
            created_at: adapter.created_at.timestamp(),
            last_accessed: None, // Hide in public API
            is_healthy: adapter.is_healthy(),
            health_score: adapter.health_score(),
            priority_score: adapter.priority_score(),
        })
    }
}

impl AdapterStatusResponse {
    fn from_domain(status: &AdapterStatus) -> Self {
        match status {
            AdapterStatus::Active => Self::Active,
            AdapterStatus::Inactive => Self::Inactive,
            AdapterStatus::Error => Self::Error,
            AdapterStatus::Initializing => Self::Initializing,
            AdapterStatus::Maintenance => Self::Maintenance,
            AdapterStatus::Deprecated => Self::Deprecated,
        }
    }
}

impl AdapterCapabilityResponse {
    fn from_domain(capability: &AdapterCapability) -> Self {
        Self {
            name: capability.name.clone(),
            available: capability.available,
            version: capability.version.clone(),
            description: capability.description.clone(),
        }
    }
}

impl AdapterMetricsResponse {
    fn from_domain(adapter: &Adapter) -> Self {
        Self {
            performance: AdapterPerformanceResponse::from_domain(&adapter.metrics.performance),
            health: adapter
                .metrics
                .last_health_check
                .as_ref()
                .map(AdapterHealthResponse::from_domain),
            uptime_percentage: adapter.metrics.calculate_uptime(),
            total_accesses: adapter.metrics.total_accesses,
            consecutive_failures: adapter.metrics.consecutive_failures,
            last_error: adapter.metrics.last_error.clone(),
            last_updated: adapter.metrics.last_updated.timestamp(),
        }
    }

    fn minimal_from_domain(adapter: &Adapter) -> Self {
        Self {
            performance: AdapterPerformanceResponse::minimal_from_domain(
                &adapter.metrics.performance,
            ),
            health: None, // Hide detailed health in public API
            uptime_percentage: adapter.metrics.calculate_uptime(),
            total_accesses: 0,       // Hide in public API
            consecutive_failures: 0, // Hide in public API
            last_error: None,        // Hide in public API
            last_updated: adapter.metrics.last_updated.timestamp(),
        }
    }
}

impl AdapterPerformanceResponse {
    fn from_domain(performance: &AdapterPerformanceMetrics) -> Self {
        Self {
            total_requests: performance.total_requests,
            successful_requests: performance.successful_requests,
            failed_requests: performance.failed_requests,
            timeout_requests: performance.timeout_requests,
            success_rate: performance.success_rate,
            error_rate: performance.error_rate,
            avg_response_time_ms: performance.avg_response_time_ms,
            min_response_time_ms: performance.min_response_time_ms,
            max_response_time_ms: performance.max_response_time_ms,
            p95_response_time_ms: performance.p95_response_time_ms,
            is_performing_well: performance.is_performing_well(),
        }
    }

    fn minimal_from_domain(performance: &AdapterPerformanceMetrics) -> Self {
        Self {
            total_requests: 0,      // Hide in public API
            successful_requests: 0, // Hide in public API
            failed_requests: 0,     // Hide in public API
            timeout_requests: 0,    // Hide in public API
            success_rate: performance.success_rate,
            error_rate: performance.error_rate,
            avg_response_time_ms: performance.avg_response_time_ms,
            min_response_time_ms: 0, // Hide in public API
            max_response_time_ms: 0, // Hide in public API
            p95_response_time_ms: performance.p95_response_time_ms,
            is_performing_well: performance.is_performing_well(),
        }
    }
}

impl AdapterHealthResponse {
    fn from_domain(health: &AdapterHealthResult) -> Self {
        Self {
            is_healthy: health.is_healthy,
            status: health.status().to_string(),
            response_time_ms: health.response_time_ms,
            health_score: health.health_score,
            consecutive_failures: health.consecutive_failures,
            last_check: health.last_check.timestamp(),
            error_message: health.error_message.clone(),
            capabilities: health.capabilities.clone(),
        }
    }
}

impl AdaptersResponse {
    /// Create adapters response from domain adapters
    pub fn from_domain_adapters(adapters: Vec<Adapter>) -> AdapterResult<Self> {
        let adapter_responses: Result<Vec<_>, _> =
            adapters.iter().map(AdapterResponse::from_domain).collect();

        let responses = adapter_responses?;
        let active_count = adapters.iter().filter(|a| a.is_available()).count();
        let healthy_count = adapters.iter().filter(|a| a.is_healthy()).count();
        let deprecated_count = adapters
            .iter()
            .filter(|a| matches!(a.status, AdapterStatus::Deprecated))
            .count();

        let system_health_score = if adapters.is_empty() {
            0.0
        } else {
            adapters.iter().map(|a| a.health_score()).sum::<f64>() / adapters.len() as f64
        };

        Ok(Self {
            adapters: responses,
            total_adapters: adapters.len(),
            active_adapters: active_count,
            healthy_adapters: healthy_count,
            deprecated_adapters: deprecated_count,
            timestamp: Utc::now().timestamp(),
            system_health_score,
        })
    }

    /// Create minimal adapters response (for public API)
    pub fn minimal_from_domain_adapters(adapters: Vec<Adapter>) -> AdapterResult<Self> {
        let adapter_responses: Result<Vec<_>, _> = adapters
            .iter()
            .filter(|a| a.enabled && !matches!(a.status, AdapterStatus::Deprecated))
            .map(AdapterResponse::minimal_from_domain)
            .collect();

        let responses = adapter_responses?;
        let active_count = adapters.iter().filter(|a| a.is_available()).count();

        Ok(Self {
            adapters: responses,
            total_adapters: adapters.len(),
            active_adapters: active_count,
            healthy_adapters: 0,    // Hide in public API
            deprecated_adapters: 0, // Hide in public API
            timestamp: Utc::now().timestamp(),
            system_health_score: 0.0, // Hide in public API
        })
    }

    /// Filter adapters by status
    pub fn filter_by_status(&mut self, status: AdapterStatusResponse) {
        self.adapters.retain(|adapter| {
            std::mem::discriminant(&adapter.status) == std::mem::discriminant(&status)
        });
        self.update_counts();
    }

    /// Filter adapters by supported chain
    pub fn filter_by_chain(&mut self, chain_id: u64) {
        self.adapters
            .retain(|adapter| adapter.supported_chains.contains(&chain_id));
        self.update_counts();
    }

    /// Filter adapters by capability
    pub fn filter_by_capability(&mut self, capability_name: &str) {
        self.adapters.retain(|adapter| {
            adapter
                .capabilities
                .iter()
                .any(|cap| cap.name == capability_name && cap.available)
        });
        self.update_counts();
    }

    /// Sort adapters by priority score
    pub fn sort_by_priority(&mut self) {
        self.adapters.sort_by(|a, b| {
            b.priority_score
                .partial_cmp(&a.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Sort adapters by health score
    pub fn sort_by_health(&mut self) {
        self.adapters.sort_by(|a, b| {
            b.health_score
                .partial_cmp(&a.health_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn update_counts(&mut self) {
        self.total_adapters = self.adapters.len();
        self.active_adapters = self
            .adapters
            .iter()
            .filter(|a| matches!(a.status, AdapterStatusResponse::Active))
            .count();
        self.healthy_adapters = self.adapters.iter().filter(|a| a.is_healthy).count();
        self.deprecated_adapters = self
            .adapters
            .iter()
            .filter(|a| matches!(a.status, AdapterStatusResponse::Deprecated))
            .count();
    }
}

impl AdapterRegistryResponse {
    /// Create registry response from adapters
    pub fn from_domain_adapters(adapters: &[Adapter]) -> Self {
        // Group adapters by type
        let mut type_map: HashMap<String, Vec<&Adapter>> = HashMap::new();
        for adapter in adapters {
            type_map
                .entry(adapter.adapter_type.to_string())
                .or_insert_with(Vec::new)
                .push(adapter);
        }

        let supported_types: Vec<AdapterTypeInfo> = type_map
            .into_iter()
            .map(|(type_name, adapters_of_type)| {
                let active_count = adapters_of_type.iter().filter(|a| a.is_available()).count();

                // Get default info for this adapter type
                let adapter_type = adapters_of_type[0].adapter_type.clone();

                AdapterTypeInfo {
                    type_name: type_name.clone(),
                    display_name: adapter_type.display_name().to_string(),
                    registered_count: adapters_of_type.len(),
                    active_count,
                    default_chains: adapter_type.default_chains(),
                    supported_operations: get_supported_operations(&adapter_type),
                }
            })
            .collect();

        let total_registered = adapters.len();
        let active_count = adapters.iter().filter(|a| a.is_available()).count();
        let healthy_count = adapters.iter().filter(|a| a.is_healthy()).count();

        let average_health_score = if adapters.is_empty() {
            0.0
        } else {
            adapters.iter().map(|a| a.health_score()).sum::<f64>() / adapters.len() as f64
        };

        // Collect all supported chains
        let mut supported_chains: Vec<u64> = adapters
            .iter()
            .flat_map(|a| &a.supported_chains)
            .copied()
            .collect();
        supported_chains.sort_unstable();
        supported_chains.dedup();

        // Collect all capabilities
        let mut total_capabilities: Vec<String> = adapters
            .iter()
            .flat_map(|a| &a.capabilities)
            .filter(|cap| cap.available)
            .map(|cap| cap.name.clone())
            .collect();
        total_capabilities.sort();
        total_capabilities.dedup();

        Self {
            supported_types,
            total_registered,
            active_count,
            healthy_count,
            average_health_score,
            supported_chains,
            total_capabilities,
            last_updated: Utc::now().timestamp(),
        }
    }
}

impl AdapterConfigResponse {
    /// Create config response from domain adapter
    pub fn from_domain(adapter: &Adapter) -> Self {
        // Convert configuration back to JSON
        let configuration = serde_json::to_value(&adapter.configuration)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        Self {
            adapter_id: adapter.adapter_id.clone(),
            adapter_type: adapter.adapter_type.to_string(),
            name: adapter.name.clone(),
            version: adapter.version.clone(),
            enabled: adapter.enabled,
            configuration,
            supported_chains: adapter.supported_chains.clone(),
            timeout_ms: adapter.get_config("timeout_ms"),
            max_retries: adapter.get_config("max_retries"),
            required_capabilities: adapter
                .capabilities
                .iter()
                .filter(|cap| cap.available)
                .map(|cap| cap.name.clone())
                .collect(),
            optional_capabilities: adapter
                .capabilities
                .iter()
                .filter(|cap| !cap.available)
                .map(|cap| cap.name.clone())
                .collect(),
            created_at: adapter.created_at.timestamp(),
        }
    }
}

/// Get supported operations for an adapter type
fn get_supported_operations(adapter_type: &AdapterType) -> Vec<String> {
    let operations = vec![
        "quote",
        "swap",
        "intent",
        "health",
        "allowance",
        "bridge",
        "order",
        "settlement",
        "pool",
    ];

    operations
        .into_iter()
        .filter(|op| adapter_type.supports_operation(op))
        .map(|op| op.to_string())
        .collect()
}

/// Convert AdapterType to string for API responses
impl ToString for AdapterType {
    fn to_string(&self) -> String {
        match self {
            AdapterType::OifV1 => "oif-v1".to_string(),
            AdapterType::UniswapV2 => "uniswap-v2".to_string(),
            AdapterType::UniswapV3 => "uniswap-v3".to_string(),
            AdapterType::OneInch => "one-inch".to_string(),
            AdapterType::Paraswap => "paraswap".to_string(),
            AdapterType::Lifi => "lifi".to_string(),
            AdapterType::Cowswap => "cowswap".to_string(),
            AdapterType::ZeroX => "zero-x".to_string(),
            AdapterType::Kyber => "kyber".to_string(),
            AdapterType::SushiSwap => "sushi-swap".to_string(),
            AdapterType::Balancer => "balancer".to_string(),
            AdapterType::Curve => "curve".to_string(),
            AdapterType::Custom => "custom".to_string(),
        }
    }
}

/// Convert domain Adapter to API AdapterResponse
impl TryFrom<Adapter> for AdapterResponse {
    type Error = crate::models::adapters::AdapterError;

    fn try_from(adapter: Adapter) -> Result<Self, Self::Error> {
        AdapterResponse::from_domain(&adapter)
    }
}

/// Convert domain collection to API response
impl TryFrom<Vec<Adapter>> for AdaptersResponse {
    type Error = crate::models::adapters::AdapterError;

    fn try_from(adapters: Vec<Adapter>) -> Result<Self, Self::Error> {
        AdaptersResponse::from_domain_adapters(adapters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::adapters::{Adapter, AdapterCapability, AdapterType};

    fn create_test_adapter() -> Adapter {
        Adapter::new(
            "test-adapter".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
            "1.0.0".to_string(),
        )
        .with_description("Test Description".to_string())
        .with_chains(vec![1, 137])
        .with_capability(AdapterCapability::new("quotes".to_string(), true))
    }

    #[test]
    fn test_adapter_response_from_domain() {
        let mut adapter = create_test_adapter();
        adapter.update_status(AdapterStatus::Active);

        let response = AdapterResponse::from_domain(&adapter).unwrap();

        assert_eq!(response.adapter_id, "test-adapter");
        assert_eq!(response.name, "Test Adapter");
        assert!(matches!(response.status, AdapterStatusResponse::Active));
        assert_eq!(response.supported_chains, vec![1, 137]);
        assert_eq!(response.capabilities.len(), 1);
        assert_eq!(response.capabilities[0].name, "quotes");
    }

    #[test]
    fn test_minimal_adapter_response() {
        let adapter = create_test_adapter();
        let response = AdapterResponse::minimal_from_domain(&adapter).unwrap();

        assert_eq!(response.adapter_id, "test-adapter");
        assert!(response.last_accessed.is_none()); // Hidden in minimal response
        // Only available capabilities should be shown
        assert_eq!(response.capabilities.len(), 1);
        assert!(response.capabilities[0].available);
    }

    #[test]
    fn test_adapters_response_creation() {
        let mut adapter1 = create_test_adapter();
        adapter1.update_status(AdapterStatus::Active);

        let mut adapter2 = create_test_adapter();
        adapter2.adapter_id = "adapter-2".to_string();
        adapter2.update_status(AdapterStatus::Error);

        let adapters = vec![adapter1, adapter2];
        let response = AdaptersResponse::from_domain_adapters(adapters).unwrap();

        assert_eq!(response.total_adapters, 2);
        assert_eq!(response.active_adapters, 1);
        assert!(response.timestamp > 0);
        assert!(response.system_health_score >= 0.0);
    }

    #[test]
    fn test_filter_by_status() {
        let mut adapter1 = create_test_adapter();
        adapter1.update_status(AdapterStatus::Active);

        let mut adapter2 = create_test_adapter();
        adapter2.adapter_id = "adapter-2".to_string();
        adapter2.update_status(AdapterStatus::Error);

        let adapters = vec![adapter1, adapter2];
        let mut response = AdaptersResponse::from_domain_adapters(adapters).unwrap();

        response.filter_by_status(AdapterStatusResponse::Active);
        assert_eq!(response.total_adapters, 1);
        assert_eq!(response.adapters[0].adapter_id, "test-adapter");
    }

    #[test]
    fn test_filter_by_chain() {
        let adapter1 = create_test_adapter(); // Supports chains [1, 137]

        let adapter2 = create_test_adapter().with_chains(vec![56, 250]); // Different chains

        let adapters = vec![adapter1, adapter2];
        let mut response = AdaptersResponse::from_domain_adapters(adapters).unwrap();

        response.filter_by_chain(1);
        assert_eq!(response.total_adapters, 1);
        assert_eq!(response.adapters[0].supported_chains, vec![1, 137]);
    }

    #[test]
    fn test_filter_by_capability() {
        let adapter1 = create_test_adapter(); // Has "quotes" capability

        let adapter2 = Adapter::new(
            "adapter-2".to_string(),
            AdapterType::UniswapV3,
            "Adapter 2".to_string(),
            "1.0.0".to_string(),
        )
        .with_chains(vec![1, 137])
        .with_capability(AdapterCapability::new("intents".to_string(), true));

        let adapters = vec![adapter1, adapter2];
        let mut response = AdaptersResponse::from_domain_adapters(adapters).unwrap();

        response.filter_by_capability("quotes");
        assert_eq!(response.total_adapters, 1);
        assert!(
            response.adapters[0]
                .capabilities
                .iter()
                .any(|cap| cap.name == "quotes")
        );
    }

    #[test]
    fn test_adapter_registry_response() {
        let adapter1 = create_test_adapter();
        let mut adapter2 = create_test_adapter();
        adapter2.adapter_id = "adapter-2".to_string();
        adapter2.adapter_type = AdapterType::UniswapV3;

        let adapters = vec![adapter1, adapter2];
        let registry = AdapterRegistryResponse::from_domain_adapters(&adapters);

        assert_eq!(registry.total_registered, 2);
        assert!(registry.supported_types.len() >= 1);
        assert!(!registry.supported_chains.is_empty());
        assert!(!registry.total_capabilities.is_empty());
    }

    #[test]
    fn test_adapter_type_to_string() {
        assert_eq!(AdapterType::OifV1.to_string(), "oif-v1");
        assert_eq!(AdapterType::UniswapV3.to_string(), "uniswap-v3");
        assert_eq!(AdapterType::OneInch.to_string(), "one-inch");
    }

    #[test]
    fn test_sort_by_priority() {
        let mut high_priority = create_test_adapter();
        high_priority.update_status(AdapterStatus::Active);
        high_priority.record_success(100); // Good performance

        let mut low_priority = create_test_adapter();
        low_priority.adapter_id = "low-priority".to_string();
        low_priority.update_status(AdapterStatus::Active);
        low_priority.record_failure("Error".to_string(), false); // Poor performance

        let adapters = vec![low_priority, high_priority];
        let mut response = AdaptersResponse::from_domain_adapters(adapters).unwrap();

        response.sort_by_priority();
        assert_eq!(response.adapters[0].adapter_id, "test-adapter"); // High priority first
        assert_eq!(response.adapters[1].adapter_id, "low-priority"); // Low priority second
    }
}
