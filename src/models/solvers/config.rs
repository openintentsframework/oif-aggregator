//! Solver configuration models and validation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use super::{Solver, SolverValidationError, SolverValidationResult};

/// Solver configuration from external sources (config files, API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    /// Unique identifier for the solver
    pub solver_id: String,

    /// ID of the adapter to use for this solver
    pub adapter_id: String,

    /// HTTP endpoint for the solver API
    pub endpoint: String,

    /// Timeout for requests in milliseconds
    pub timeout_ms: u64,

    /// Whether this solver is enabled
    pub enabled: bool,

    /// Maximum retry attempts for failed requests
    pub max_retries: u32,

    /// Custom HTTP headers for requests
    pub headers: Option<HashMap<String, String>>,

    /// Optional human-readable name
    pub name: Option<String>,

    /// Optional description
    pub description: Option<String>,

    /// API version
    pub version: Option<String>,

    /// Supported blockchain networks
    pub supported_chains: Option<Vec<u64>>,

    /// Supported protocols/DEXs
    pub supported_protocols: Option<Vec<String>>,

    /// Solver-specific configuration
    pub config: Option<HashMap<String, serde_json::Value>>,
}

/// Adapter configuration
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

    /// Adapter version
    pub version: String,

    /// Supported blockchain networks
    pub supported_chains: Vec<u64>,

    /// Adapter-specific configuration
    pub configuration: serde_json::Value,

    /// Whether this adapter is enabled
    pub enabled: bool,

    /// When the adapter was created/registered
    pub created_at: DateTime<Utc>,
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
    /// Custom/proprietary adapter
    Custom,
}

impl SolverConfig {
    /// Create a new solver configuration
    pub fn new(solver_id: String, adapter_id: String, endpoint: String) -> Self {
        Self {
            solver_id,
            adapter_id,
            endpoint,
            timeout_ms: 2000, // Default 2 second timeout
            enabled: true,
            max_retries: 3,
            headers: None,
            name: None,
            description: None,
            version: None,
            supported_chains: None,
            supported_protocols: None,
            config: None,
        }
    }

    /// Validate the solver configuration
    pub fn validate(&self) -> SolverValidationResult<()> {
        // Validate solver ID
        if self.solver_id.is_empty() {
            return Err(SolverValidationError::MissingRequiredField {
                field: "solver_id".to_string(),
            });
        }

        if !self
            .solver_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(SolverValidationError::InvalidSolverId {
                solver_id: self.solver_id.clone(),
            });
        }

        // Validate adapter ID
        if self.adapter_id.is_empty() {
            return Err(SolverValidationError::MissingRequiredField {
                field: "adapter_id".to_string(),
            });
        }

        // Validate endpoint URL
        if self.endpoint.is_empty() {
            return Err(SolverValidationError::MissingRequiredField {
                field: "endpoint".to_string(),
            });
        }

        match Url::parse(&self.endpoint) {
            Ok(url) => {
                if !matches!(url.scheme(), "http" | "https") {
                    return Err(SolverValidationError::InvalidEndpoint {
                        endpoint: self.endpoint.clone(),
                        reason: "Only HTTP and HTTPS schemes are supported".to_string(),
                    });
                }
                if url.host().is_none() {
                    return Err(SolverValidationError::InvalidEndpoint {
                        endpoint: self.endpoint.clone(),
                        reason: "URL must have a valid host".to_string(),
                    });
                }
            }
            Err(e) => {
                return Err(SolverValidationError::InvalidEndpoint {
                    endpoint: self.endpoint.clone(),
                    reason: e.to_string(),
                });
            }
        }

        // Validate timeout
        const MIN_TIMEOUT: u64 = 100; // 100ms minimum
        const MAX_TIMEOUT: u64 = 30000; // 30 seconds maximum

        if self.timeout_ms < MIN_TIMEOUT || self.timeout_ms > MAX_TIMEOUT {
            return Err(SolverValidationError::InvalidTimeout {
                timeout_ms: self.timeout_ms,
                min: MIN_TIMEOUT,
                max: MAX_TIMEOUT,
            });
        }

        // Validate retry count
        const MAX_RETRIES: u32 = 10;
        if self.max_retries > MAX_RETRIES {
            return Err(SolverValidationError::InvalidRetryCount {
                retries: self.max_retries,
                max: MAX_RETRIES,
            });
        }

        // Validate supported chains (if provided)
        if let Some(ref chains) = self.supported_chains {
            for &chain_id in chains {
                if !is_valid_chain_id(chain_id) {
                    return Err(SolverValidationError::InvalidChainId { chain_id });
                }
            }
        }

        Ok(())
    }

    /// Convert configuration to domain solver
    pub fn to_domain(&self) -> SolverValidationResult<Solver> {
        // Validate first
        self.validate()?;

        let mut solver = Solver::new(
            self.solver_id.clone(),
            self.adapter_id.clone(),
            self.endpoint.clone(),
            self.timeout_ms,
        );

        // Apply metadata
        if let Some(ref name) = self.name {
            solver = solver.with_name(name.clone());
        }

        if let Some(ref description) = self.description {
            solver = solver.with_description(description.clone());
        }

        if let Some(ref version) = self.version {
            solver = solver.with_version(version.clone());
        }

        if let Some(ref chains) = self.supported_chains {
            solver = solver.with_chains(chains.clone());
        }

        if let Some(ref protocols) = self.supported_protocols {
            solver = solver.with_protocols(protocols.clone());
        }

        solver = solver.with_max_retries(self.max_retries);

        if let Some(ref headers) = self.headers {
            solver = solver.with_headers(headers.clone());
        }

        if let Some(ref config) = self.config {
            for (key, value) in config {
                solver = solver.with_config(key.clone(), value.clone());
            }
        }

        Ok(solver)
    }

    /// Builder methods for easy configuration
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_chains(mut self, chains: Vec<u64>) -> Self {
        self.supported_chains = Some(chains);
        self
    }

    pub fn with_protocols(mut self, protocols: Vec<String>) -> Self {
        self.supported_protocols = Some(protocols);
        self
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
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
        }
    }

    /// Validate adapter configuration
    pub fn validate(&self) -> SolverValidationResult<()> {
        if self.adapter_id.is_empty() {
            return Err(SolverValidationError::MissingRequiredField {
                field: "adapter_id".to_string(),
            });
        }

        if self.name.is_empty() {
            return Err(SolverValidationError::MissingRequiredField {
                field: "name".to_string(),
            });
        }

        if self.version.is_empty() {
            return Err(SolverValidationError::MissingRequiredField {
                field: "version".to_string(),
            });
        }

        // Validate supported chains
        for &chain_id in &self.supported_chains {
            if !is_valid_chain_id(chain_id) {
                return Err(SolverValidationError::InvalidChainId { chain_id });
            }
        }

        Ok(())
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
                "factory_address": "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
            }),
            AdapterType::UniswapV3 => serde_json::json!({
                "router_address": "0xE592427A0AEce92De3Edee1F18E0157C05861564",
                "factory_address": "0x1F98431c8aD98523631AE4a59f267346ea31F984",
                "quoter_address": "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
            }),
            AdapterType::OneInch => serde_json::json!({
                "base_url": "https://api.1inch.io",
                "api_version": "v5.0"
            }),
            AdapterType::Paraswap => serde_json::json!({
                "base_url": "https://apiv5.paraswap.io",
                "api_version": "v5"
            }),
            AdapterType::Lifi => serde_json::json!({
                "base_url": "https://li.quest",
                "api_version": "v1"
            }),
            AdapterType::Cowswap => serde_json::json!({
                "base_url": "https://api.cow.fi",
                "api_version": "v1"
            }),
            AdapterType::ZeroX => serde_json::json!({
                "base_url": "https://api.0x.org",
                "api_version": "v1"
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
            AdapterType::Custom => "Custom",
        }
    }
}

/// Check if a chain ID is valid/supported
fn is_valid_chain_id(chain_id: u64) -> bool {
    const SUPPORTED_CHAINS: &[u64] = &[
        1,        // Ethereum Mainnet
        10,       // Optimism
        56,       // BSC
        100,      // Gnosis
        137,      // Polygon
        250,      // Fantom
        8453,     // Base
        42161,    // Arbitrum One
        43114,    // Avalanche
        5,        // Goerli (testnet)
        11155111, // Sepolia (testnet)
    ];

    SUPPORTED_CHAINS.contains(&chain_id)
}

/// Convert from config SolverConfig to domain Solver
impl TryFrom<SolverConfig> for Solver {
    type Error = SolverValidationError;

    fn try_from(config: SolverConfig) -> Result<Self, Self::Error> {
        config.to_domain()
    }
}

/// Convert from crate::config::settings::SolverConfig to domain SolverConfig
impl From<crate::config::settings::SolverConfig> for SolverConfig {
    fn from(settings_config: crate::config::settings::SolverConfig) -> Self {
        Self {
            solver_id: settings_config.solver_id,
            adapter_id: settings_config.adapter_id,
            endpoint: settings_config.endpoint,
            timeout_ms: settings_config.timeout_ms,
            enabled: settings_config.enabled,
            max_retries: settings_config.max_retries,
            headers: settings_config.headers,
            name: None,
            description: None,
            version: None,
            supported_chains: None,
            supported_protocols: None,
            config: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_config_validation() {
        let valid_config = SolverConfig::new(
            "test-solver".to_string(),
            "oif-v1".to_string(),
            "https://api.example.com".to_string(),
        );

        assert!(valid_config.validate().is_ok());
    }

    #[test]
    fn test_invalid_solver_id() {
        let mut config = SolverConfig::new(
            "invalid solver!".to_string(),
            "oif-v1".to_string(),
            "https://api.example.com".to_string(),
        );

        assert!(config.validate().is_err());

        config.solver_id = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_endpoint() {
        let config = SolverConfig::new(
            "test-solver".to_string(),
            "oif-v1".to_string(),
            "not-a-url".to_string(),
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_timeout() {
        let mut config = SolverConfig::new(
            "test-solver".to_string(),
            "oif-v1".to_string(),
            "https://api.example.com".to_string(),
        );

        config.timeout_ms = 50; // Too low
        assert!(config.validate().is_err());

        config.timeout_ms = 50000; // Too high
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_to_domain() {
        let config = SolverConfig::new(
            "test-solver".to_string(),
            "oif-v1".to_string(),
            "https://api.example.com".to_string(),
        )
        .with_name("Test Solver".to_string())
        .with_chains(vec![1, 137]);

        let solver = config.to_domain().unwrap();

        assert_eq!(solver.solver_id, "test-solver");
        assert_eq!(solver.metadata.name, Some("Test Solver".to_string()));
        assert_eq!(solver.metadata.supported_chains, vec![1, 137]);
    }

    #[test]
    fn test_adapter_type_display_names() {
        assert_eq!(AdapterType::OifV1.display_name(), "OIF v1");
        assert_eq!(AdapterType::UniswapV3.display_name(), "Uniswap V3");
        assert_eq!(AdapterType::OneInch.display_name(), "1inch");
    }

    #[test]
    fn test_valid_chain_ids() {
        assert!(is_valid_chain_id(1)); // Ethereum
        assert!(is_valid_chain_id(137)); // Polygon
        assert!(!is_valid_chain_id(999999)); // Invalid
    }

    #[test]
    fn test_adapter_config_validation() {
        let config = AdapterConfig::new(
            "test-adapter".to_string(),
            AdapterType::OifV1,
            "Test Adapter".to_string(),
            "1.0.0".to_string(),
        );

        assert!(config.validate().is_ok());
    }
}
