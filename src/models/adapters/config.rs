//! Adapter configuration models and validation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Adapter, AdapterCapability, AdapterValidationError, AdapterValidationResult};

/// Adapter configuration from external sources (config files, API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Unique identifier for the adapter
    pub adapter_id: String,

    /// Type of adapter
    pub adapter_type: AdapterType,

    /// Human-readable name
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Version of the adapter implementation
    pub version: String,

    /// Supported blockchain networks
    pub supported_chains: Vec<u64>,

    /// Adapter-specific configuration
    pub configuration: serde_json::Value,

    /// Whether this adapter is enabled
    pub enabled: bool,

    /// When the adapter was created/registered
    pub created_at: DateTime<Utc>,

    /// Connection timeout in milliseconds
    pub timeout_ms: Option<u64>,

    /// Maximum number of retries
    pub max_retries: Option<u32>,

    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,

    /// Custom headers for HTTP requests
    pub headers: Option<HashMap<String, String>>,

    /// Required capabilities for this adapter
    pub required_capabilities: Vec<String>,

    /// Optional capabilities that can be disabled
    pub optional_capabilities: Vec<String>,
}

/// Types of adapters supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterType {
    /// Open Intent Framework v1 protocol
    OifV1,
    /// Uniswap V2 direct integration
    UniswapV2,
    /// Uniswap V3 direct integration
    UniswapV3,
    /// 1inch aggregator
    OneInch,
    /// Paraswap aggregator
    Paraswap,
    /// LiFi cross-chain
    Lifi,
    /// Cowswap solver
    Cowswap,
    /// 0x protocol
    ZeroX,
    /// Kyber Network
    Kyber,
    /// SushiSwap
    SushiSwap,
    /// Balancer
    Balancer,
    /// Curve Finance
    Curve,
    /// Custom/proprietary adapter
    Custom,
}

/// Rate limiting configuration for adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub requests_per_second: u32,
    /// Burst capacity
    pub burst_size: u32,
    /// Window size in seconds
    pub window_size: u32,
}

/// Builder for adapter configuration
pub struct AdapterConfigBuilder {
    config: AdapterConfig,
}

impl AdapterConfig {
    /// Create a new adapter configuration
    pub fn new(
        adapter_id: String,
        adapter_type: AdapterType,
        name: String,
        version: String,
    ) -> Self {
        Self {
            adapter_id,
            adapter_type,
            name,
            description: None,
            version,
            supported_chains: Vec::new(),
            configuration: serde_json::Value::Object(serde_json::Map::new()),
            enabled: true,
            created_at: Utc::now(),
            timeout_ms: Some(5000), // Default 5 second timeout
            max_retries: Some(3),
            rate_limit: None,
            headers: None,
            required_capabilities: Vec::new(),
            optional_capabilities: Vec::new(),
        }
    }

    /// Create a builder for this configuration
    pub fn builder(
        adapter_id: String,
        adapter_type: AdapterType,
        name: String,
    ) -> AdapterConfigBuilder {
        AdapterConfigBuilder::new(adapter_id, adapter_type, name)
    }

    /// Validate the adapter configuration
    pub fn validate(&self) -> AdapterValidationResult<()> {
        // Validate adapter ID
        if self.adapter_id.is_empty() {
            return Err(AdapterValidationError::MissingRequiredField {
                field: "adapter_id".to_string(),
            });
        }

        if !self
            .adapter_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AdapterValidationError::InvalidAdapterId {
                adapter_id: self.adapter_id.clone(),
            });
        }

        // Validate name
        if self.name.is_empty() {
            return Err(AdapterValidationError::MissingRequiredField {
                field: "name".to_string(),
            });
        }

        if self.name.len() > 100 {
            return Err(AdapterValidationError::InvalidAdapterName {
                name: self.name.clone(),
            });
        }

        // Validate version
        if self.version.is_empty() {
            return Err(AdapterValidationError::MissingRequiredField {
                field: "version".to_string(),
            });
        }

        if !is_valid_semver(&self.version) {
            return Err(AdapterValidationError::InvalidVersion {
                version: self.version.clone(),
            });
        }

        // Validate timeout
        if let Some(timeout) = self.timeout_ms {
            const MIN_TIMEOUT: u64 = 100; // 100ms minimum
            const MAX_TIMEOUT: u64 = 60000; // 60 seconds maximum

            if timeout < MIN_TIMEOUT || timeout > MAX_TIMEOUT {
                return Err(AdapterValidationError::InvalidTimeout {
                    timeout_ms: timeout,
                    min: MIN_TIMEOUT,
                    max: MAX_TIMEOUT,
                });
            }
        }

        // Validate supported chains
        for &chain_id in &self.supported_chains {
            if !is_valid_chain_id(chain_id) {
                return Err(AdapterValidationError::InvalidChainId { chain_id });
            }
        }

        // Validate adapter type specific configuration
        self.validate_type_specific_config()?;

        // Validate rate limit configuration
        if let Some(ref rate_limit) = self.rate_limit {
            if rate_limit.requests_per_second == 0 {
                return Err(AdapterValidationError::InvalidConfiguration {
                    reason: "Rate limit requests_per_second must be greater than 0".to_string(),
                });
            }

            if rate_limit.burst_size == 0 {
                return Err(AdapterValidationError::InvalidConfiguration {
                    reason: "Rate limit burst_size must be greater than 0".to_string(),
                });
            }

            if rate_limit.window_size == 0 || rate_limit.window_size > 3600 {
                return Err(AdapterValidationError::InvalidConfiguration {
                    reason: "Rate limit window_size must be between 1 and 3600 seconds".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate adapter type specific configuration
    fn validate_type_specific_config(&self) -> AdapterValidationResult<()> {
        match self.adapter_type {
            AdapterType::OifV1 => {
                self.validate_required_config_fields(&["endpoint", "api_version"])?;
            }
            AdapterType::UniswapV2 => {
                self.validate_required_config_fields(&["router_address", "factory_address"])?;
            }
            AdapterType::UniswapV3 => {
                self.validate_required_config_fields(&[
                    "router_address",
                    "factory_address",
                    "quoter_address",
                ])?;
            }
            AdapterType::OneInch => {
                self.validate_required_config_fields(&["base_url", "api_key"])?;
            }
            AdapterType::Paraswap => {
                self.validate_required_config_fields(&["base_url", "api_version"])?;
            }
            AdapterType::Lifi => {
                self.validate_required_config_fields(&["base_url"])?;
            }
            AdapterType::Cowswap => {
                self.validate_required_config_fields(&["base_url", "settlement_contract"])?;
            }
            AdapterType::ZeroX => {
                self.validate_required_config_fields(&["base_url", "api_key"])?;
            }
            AdapterType::Kyber
            | AdapterType::SushiSwap
            | AdapterType::Balancer
            | AdapterType::Curve => {
                self.validate_required_config_fields(&["base_url"])?;
            }
            AdapterType::Custom => {
                // Custom adapters can have any configuration
            }
        }

        Ok(())
    }

    /// Validate that required configuration fields are present
    fn validate_required_config_fields(
        &self,
        required_fields: &[&str],
    ) -> AdapterValidationResult<()> {
        let config_obj = self.configuration.as_object().ok_or_else(|| {
            AdapterValidationError::InvalidConfiguration {
                reason: "Configuration must be an object".to_string(),
            }
        })?;

        for &field in required_fields {
            if !config_obj.contains_key(field) {
                return Err(AdapterValidationError::MissingRequiredField {
                    field: field.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Convert configuration to domain adapter
    pub fn to_domain(&self) -> AdapterValidationResult<Adapter> {
        // Validate first
        self.validate()?;

        let mut adapter = Adapter::new(
            self.adapter_id.clone(),
            self.adapter_type.clone(),
            self.name.clone(),
            self.version.clone(),
        );

        // Apply configuration
        if let Some(ref description) = self.description {
            adapter = adapter.with_description(description.clone());
        }

        adapter = adapter
            .with_chains(self.supported_chains.clone())
            .enabled(self.enabled);

        // Set configuration
        if let serde_json::Value::Object(config_map) = &self.configuration {
            for (key, value) in config_map {
                adapter
                    .set_config(key.clone(), value.clone())
                    .map_err(|e| AdapterValidationError::InvalidConfiguration {
                        reason: format!("Failed to set config {}: {}", key, e),
                    })?;
            }
        }

        // Add required capabilities
        for capability_name in &self.required_capabilities {
            adapter.add_capability(AdapterCapability::new(capability_name.clone(), true));
        }

        // Add optional capabilities
        for capability_name in &self.optional_capabilities {
            adapter.add_capability(AdapterCapability::new(capability_name.clone(), false));
        }

        Ok(adapter)
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
}

impl AdapterType {
    /// Get the default configuration for this adapter type
    pub fn default_config(&self) -> serde_json::Value {
        match self {
            AdapterType::OifV1 => serde_json::json!({
                "api_version": "v1",
                "quote_endpoint": "/quote",
                "intent_endpoint": "/intent",
                "health_endpoint": "/health"
            }),
            AdapterType::UniswapV2 => serde_json::json!({
                "router_address": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
                "factory_address": "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
                "fee_tier": 3000
            }),
            AdapterType::UniswapV3 => serde_json::json!({
                "router_address": "0xE592427A0AEce92De3Edee1F18E0157C05861564",
                "factory_address": "0x1F98431c8aD98523631AE4a59f267346ea31F984",
                "quoter_address": "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
            }),
            AdapterType::OneInch => serde_json::json!({
                "base_url": "https://api.1inch.io",
                "api_version": "v5.0",
                "supported_chains": [1, 10, 56, 137, 250, 42161, 43114]
            }),
            AdapterType::Paraswap => serde_json::json!({
                "base_url": "https://apiv5.paraswap.io",
                "api_version": "v5",
                "supported_chains": [1, 10, 56, 137, 250, 42161, 43114]
            }),
            AdapterType::Lifi => serde_json::json!({
                "base_url": "https://li.quest",
                "api_version": "v1",
                "supported_chains": [1, 10, 56, 100, 137, 250, 8453, 42161, 43114]
            }),
            AdapterType::Cowswap => serde_json::json!({
                "base_url": "https://api.cow.fi",
                "api_version": "v1",
                "settlement_contract": "0x9008D19f58AAbD9eD0D60971565AA8510560ab41",
                "supported_chains": [1, 100]
            }),
            AdapterType::ZeroX => serde_json::json!({
                "base_url": "https://api.0x.org",
                "api_version": "v1",
                "supported_chains": [1, 10, 56, 137, 250, 8453, 42161, 43114]
            }),
            AdapterType::Kyber => serde_json::json!({
                "base_url": "https://aggregator-api.kyberswap.com",
                "api_version": "v1",
                "supported_chains": [1, 10, 56, 137, 250, 8453, 42161, 43114]
            }),
            AdapterType::SushiSwap => serde_json::json!({
                "base_url": "https://api.sushi.com",
                "router_address": "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F",
                "supported_chains": [1, 10, 56, 137, 250, 8453, 42161, 43114]
            }),
            AdapterType::Balancer => serde_json::json!({
                "base_url": "https://api.balancer.fi",
                "vault_address": "0xBA12222222228d8Ba445958a75a0704d566BF2C8",
                "supported_chains": [1, 10, 137, 8453, 42161]
            }),
            AdapterType::Curve => serde_json::json!({
                "base_url": "https://api.curve.fi",
                "registry_address": "0x90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5",
                "supported_chains": [1, 10, 137, 250, 42161]
            }),
            AdapterType::Custom => serde_json::json!({}),
        }
    }

    /// Get the human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            AdapterType::OifV1 => "OIF v1",
            AdapterType::UniswapV2 => "Uniswap V2",
            AdapterType::UniswapV3 => "Uniswap V3",
            AdapterType::OneInch => "1inch",
            AdapterType::Paraswap => "Paraswap",
            AdapterType::Lifi => "LiFi",
            AdapterType::Cowswap => "CoW Swap",
            AdapterType::ZeroX => "0x Protocol",
            AdapterType::Kyber => "Kyber Network",
            AdapterType::SushiSwap => "SushiSwap",
            AdapterType::Balancer => "Balancer",
            AdapterType::Curve => "Curve Finance",
            AdapterType::Custom => "Custom",
        }
    }

    /// Get default supported chains for this adapter type
    pub fn default_chains(&self) -> Vec<u64> {
        match self {
            AdapterType::OifV1 => vec![1, 10, 56, 137, 250, 8453, 42161, 43114],
            AdapterType::UniswapV2 => vec![1, 56, 137],
            AdapterType::UniswapV3 => vec![1, 10, 56, 137, 8453, 42161],
            AdapterType::OneInch => vec![1, 10, 56, 137, 250, 42161, 43114],
            AdapterType::Paraswap => vec![1, 10, 56, 137, 250, 42161, 43114],
            AdapterType::Lifi => vec![1, 10, 56, 100, 137, 250, 8453, 42161, 43114],
            AdapterType::Cowswap => vec![1, 100],
            AdapterType::ZeroX => vec![1, 10, 56, 137, 250, 8453, 42161, 43114],
            AdapterType::Kyber => vec![1, 10, 56, 137, 250, 8453, 42161, 43114],
            AdapterType::SushiSwap => vec![1, 10, 56, 137, 250, 8453, 42161, 43114],
            AdapterType::Balancer => vec![1, 10, 137, 8453, 42161],
            AdapterType::Curve => vec![1, 10, 137, 250, 42161],
            AdapterType::Custom => vec![],
        }
    }

    /// Check if this adapter type supports a specific operation
    pub fn supports_operation(&self, operation: &str) -> bool {
        match self {
            AdapterType::OifV1 => matches!(operation, "quote" | "intent" | "health"),
            AdapterType::UniswapV2 | AdapterType::UniswapV3 => {
                matches!(operation, "quote" | "swap")
            }
            AdapterType::OneInch | AdapterType::Paraswap | AdapterType::ZeroX => {
                matches!(operation, "quote" | "swap" | "allowance")
            }
            AdapterType::Lifi => matches!(operation, "quote" | "swap" | "bridge"),
            AdapterType::Cowswap => matches!(operation, "quote" | "order" | "settlement"),
            AdapterType::Kyber | AdapterType::SushiSwap => matches!(operation, "quote" | "swap"),
            AdapterType::Balancer => matches!(operation, "quote" | "swap" | "pool"),
            AdapterType::Curve => matches!(operation, "quote" | "swap" | "pool"),
            AdapterType::Custom => true, // Custom adapters can support anything
        }
    }
}

impl AdapterConfigBuilder {
    pub fn new(adapter_id: String, adapter_type: AdapterType, name: String) -> Self {
        Self {
            config: AdapterConfig::new(adapter_id, adapter_type, name, "1.0.0".to_string()),
        }
    }

    pub fn version(mut self, version: String) -> Self {
        self.config.version = version;
        self
    }

    pub fn description(mut self, description: String) -> Self {
        self.config.description = Some(description);
        self
    }

    pub fn chains(mut self, chains: Vec<u64>) -> Self {
        self.config.supported_chains = chains;
        self
    }

    pub fn configuration(mut self, config: serde_json::Value) -> Self {
        self.config.configuration = config;
        self
    }

    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.config.timeout_ms = Some(timeout);
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = Some(retries);
        self
    }

    pub fn rate_limit(mut self, rate_limit: RateLimitConfig) -> Self {
        self.config.rate_limit = Some(rate_limit);
        self
    }

    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.config.headers = Some(headers);
        self
    }

    pub fn required_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.config.required_capabilities = capabilities;
        self
    }

    pub fn optional_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.config.optional_capabilities = capabilities;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    pub fn build(self) -> AdapterConfig {
        self.config
    }

    pub fn build_validated(self) -> AdapterValidationResult<AdapterConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Check if a version string is a valid semver
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }

    parts.iter().all(|part| part.parse::<u32>().is_ok())
}

/// Check if a chain ID is valid/supported
fn is_valid_chain_id(chain_id: u64) -> bool {
    const SUPPORTED_CHAINS: &[u64] = &[
        1,        // Ethereum Mainnet
        5,        // Goerli (testnet)
        10,       // Optimism
        56,       // BSC
        100,      // Gnosis
        137,      // Polygon
        250,      // Fantom
        8453,     // Base
        42161,    // Arbitrum One
        43114,    // Avalanche
        11155111, // Sepolia (testnet)
    ];

    SUPPORTED_CHAINS.contains(&chain_id)
}

/// Convert from domain AdapterConfig to Adapter
impl TryFrom<AdapterConfig> for Adapter {
    type Error = AdapterValidationError;

    fn try_from(config: AdapterConfig) -> Result<Self, Self::Error> {
        config.to_domain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_config_validation() {
        let mut valid_config = AdapterConfig::new(
            "test-adapter".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
            "1.0.0".to_string(),
        );

        // Add required configuration for OifV1
        valid_config.configuration = serde_json::json!({
            "endpoint": "https://api.example.com",
            "api_version": "v1"
        });

        assert!(valid_config.validate().is_ok());
    }

    #[test]
    fn test_invalid_adapter_id() {
        let mut config = AdapterConfig::new(
            "invalid adapter!".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
            "1.0.0".to_string(),
        );

        assert!(config.validate().is_err());

        config.adapter_id = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_version() {
        let config = AdapterConfig::new(
            "test-adapter".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
            "not-semver".to_string(),
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = AdapterConfig::builder(
            "test-adapter".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
        )
        .version("2.0.0".to_string())
        .description("Test Description".to_string())
        .chains(vec![1, 137])
        .timeout_ms(3000)
        .max_retries(5)
        .build();

        assert_eq!(config.adapter_id, "test-adapter");
        assert_eq!(config.version, "2.0.0");
        assert_eq!(config.description, Some("Test Description".to_string()));
        assert_eq!(config.supported_chains, vec![1, 137]);
        assert_eq!(config.timeout_ms, Some(3000));
        assert_eq!(config.max_retries, Some(5));
    }

    #[test]
    fn test_adapter_type_defaults() {
        assert_eq!(AdapterType::OifV1.display_name(), "OIF v1");
        assert_eq!(AdapterType::UniswapV3.display_name(), "Uniswap V3");
        assert_eq!(AdapterType::OneInch.display_name(), "1inch");

        assert!(AdapterType::OifV1.supports_operation("quote"));
        assert!(AdapterType::OifV1.supports_operation("intent"));
        assert!(!AdapterType::UniswapV2.supports_operation("intent"));

        assert!(!AdapterType::Cowswap.default_chains().is_empty());
        assert!(!AdapterType::OifV1.default_config().is_null());
    }

    #[test]
    fn test_config_to_domain() {
        let config = AdapterConfig::builder(
            "test-adapter".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
        )
        .configuration(serde_json::json!({
            "endpoint": "https://api.example.com",
            "api_version": "v1"
        }))
        .description("Test Description".to_string())
        .chains(vec![1, 137])
        .required_capabilities(vec!["quotes".to_string()])
        .build();

        let adapter = config.to_domain().unwrap();

        assert_eq!(adapter.adapter_id, "test-adapter");
        assert_eq!(adapter.name, "Test Adapter");
        assert_eq!(adapter.description, Some("Test Description".to_string()));
        assert_eq!(adapter.supported_chains, vec![1, 137]);
        assert!(adapter.has_capability("quotes"));
    }

    #[test]
    fn test_rate_limit_config() {
        let rate_limit = RateLimitConfig {
            requests_per_second: 10,
            burst_size: 20,
            window_size: 60,
        };

        let config = AdapterConfig::builder(
            "test-adapter".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
        )
        .configuration(serde_json::json!({
            "endpoint": "https://api.example.com",
            "api_version": "v1"
        }))
        .rate_limit(rate_limit)
        .build();

        assert!(config.validate().is_ok());
        assert!(config.rate_limit.is_some());
    }

    #[test]
    fn test_semver_validation() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("2.1.3"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.0.0.1"));
        assert!(!is_valid_semver("not-semver"));
    }
}
