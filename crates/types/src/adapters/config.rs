//! Adapter configuration models and validation

use serde::{Deserialize, Serialize};

use super::{Adapter, AdapterValidationError};
use crate::models::{Asset, Network};

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
	/// If None, will use defaults based on adapter_type
	pub supported_networks: Option<Vec<Network>>,

	/// Supported assets per network
	/// If None, will use defaults based on adapter_type and networks
	pub supported_assets: Option<Vec<Asset>>,

	/// Endpoint URL for the adapter
	pub endpoint: Option<String>,

	/// Request timeout in milliseconds
	pub timeout_ms: Option<u64>,

	/// Whether the adapter is enabled
	pub enabled: Option<bool>,
}

/// Types of adapters supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterType {
	/// Open Intent Framework v1 protocol
	OifV1,
	/// LiFi V1 protocol
	LifiV1,
}

impl AdapterType {
	/// Get display name for UI
	pub fn display_name(&self) -> &'static str {
		match self {
			AdapterType::OifV1 => "OIF v1",
			AdapterType::LifiV1 => "LiFi v1",
		}
	}

	/// Get default supported chains for this adapter type
	pub fn default_chains(&self) -> Vec<u64> {
		match self {
			AdapterType::OifV1 => vec![1],           // Ethereum mainnet
			AdapterType::LifiV1 => vec![1, 137, 56], // Ethereum, Polygon, BSC
		}
	}

	/// Check if adapter type supports a specific operation
	pub fn supports_operation(&self, operation: &str) -> bool {
		match self {
			AdapterType::OifV1 => matches!(operation, "quotes" | "intents"),
			AdapterType::LifiV1 => matches!(operation, "quotes"), // LiFi only supports quotes
		}
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
			supported_networks: None,
			supported_assets: None,
			endpoint: None,
			timeout_ms: None,
			enabled: None,
		}
	}

	/// Builder pattern: Set description
	pub fn with_description(mut self, description: String) -> Self {
		self.description = Some(description);
		self
	}

	/// Builder pattern: Set supported networks
	pub fn with_networks(mut self, networks: Vec<Network>) -> Self {
		self.supported_networks = Some(networks);
		self
	}

	/// Builder pattern: Add a single network
	pub fn with_network(mut self, network: Network) -> Self {
		match &mut self.supported_networks {
			Some(networks) => networks.push(network),
			None => self.supported_networks = Some(vec![network]),
		}
		self
	}

	/// Builder pattern: Set supported assets
	pub fn with_assets(mut self, assets: Vec<Asset>) -> Self {
		self.supported_assets = Some(assets);
		self
	}

	/// Builder pattern: Add a single asset
	pub fn with_asset(mut self, asset: Asset) -> Self {
		match &mut self.supported_assets {
			Some(assets) => assets.push(asset),
			None => self.supported_assets = Some(vec![asset]),
		}
		self
	}

	/// Builder pattern: Set endpoint
	pub fn with_endpoint(mut self, endpoint: String) -> Self {
		self.endpoint = Some(endpoint);
		self
	}

	/// Builder pattern: Set timeout
	pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
		self.timeout_ms = Some(timeout_ms);
		self
	}

	/// Builder pattern: Set enabled status
	pub fn with_enabled(mut self, enabled: bool) -> Self {
		self.enabled = Some(enabled);
		self
	}

	/// Get supported networks, falling back to defaults if not configured
	pub fn get_supported_networks(&self) -> Vec<Network> {
		match &self.supported_networks {
			Some(networks) => networks.clone(),
			None => vec![],
		}
	}

	/// Get supported assets, falling back to defaults if not configured
	pub fn get_supported_assets(&self) -> Vec<Asset> {
		match &self.supported_assets {
			Some(assets) => assets.clone(),
			None => vec![],
		}
	}

	/// Get assets for a specific network
	pub fn get_assets_for_network(&self, network: &Network) -> Vec<Asset> {
		self.get_supported_assets()
			.into_iter()
			.filter(|asset| asset.chain_id == network.chain_id)
			.collect()
	}

	/// Get endpoint with fallback to default
	pub fn get_endpoint(&self) -> String {
		match &self.endpoint {
			Some(endpoint) => endpoint.clone(),
			None => self.default_endpoint(),
		}
	}

	/// Get timeout with fallback to default
	pub fn get_timeout_ms(&self) -> u64 {
		self.timeout_ms.unwrap_or(30000) // Default 30 seconds
	}

	/// Check if adapter is enabled (defaults to true)
	pub fn is_enabled(&self) -> bool {
		self.enabled.unwrap_or(true)
	}

	/// Get default endpoint for this adapter type
	fn default_endpoint(&self) -> String {
		match self.adapter_type {
			AdapterType::OifV1 => "https://api.oif.example.com/v1".to_string(),
			AdapterType::LifiV1 => "https://li.quest/v1".to_string(),
		}
	}
}

impl Default for AdapterConfig {
	fn default() -> Self {
		Self {
			adapter_id: "default-adapter".to_string(),
			adapter_type: AdapterType::OifV1,
			name: "Default Adapter".to_string(),
			description: None,
			version: "1.0.0".to_string(),
			supported_networks: None,
			supported_assets: None,
			endpoint: None,
			timeout_ms: None,
			enabled: None,
		}
	}
}

impl TryFrom<AdapterConfig> for Adapter {
	type Error = AdapterValidationError;

	fn try_from(config: AdapterConfig) -> Result<Self, Self::Error> {
		let mut adapter = Adapter::new(
			config.adapter_id.clone(),
			config.adapter_type.clone(),
			config.name.clone(),
			config.version.clone(),
		);

		adapter.description = config.description.clone();
		adapter.supported_networks = config.get_supported_networks();
		adapter.supported_assets = config.get_supported_assets();
		adapter.endpoint = config.endpoint.clone();
		adapter.timeout_ms = config.timeout_ms;
		adapter.enabled = config.enabled.unwrap_or(true);

		// Additional config-specific validation
		// If networks were explicitly set (not None), they can't be empty
		if let Some(ref networks) = config.supported_networks {
			if networks.is_empty() {
				return Err(AdapterValidationError::InvalidConfiguration {
					reason: "supported_networks cannot be empty if provided".to_string(),
				});
			}
		}

		// If assets were explicitly set (not None), they can't be empty
		if let Some(ref assets) = config.supported_assets {
			if assets.is_empty() {
				return Err(AdapterValidationError::InvalidConfiguration {
					reason: "supported_assets cannot be empty if provided".to_string(),
				});
			}
		}

		// Validate using domain model validation
		adapter.validate()?;

		Ok(adapter)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_adapter_config_conversion() {
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		);

		assert!(Adapter::try_from(config).is_ok());
	}

	#[test]
	fn test_invalid_adapter_id() {
		let config = AdapterConfig::new(
			"".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		);

		assert!(Adapter::try_from(config).is_err());
	}

	#[test]
	fn test_invalid_version() {
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"invalid".to_string(),
		);

		assert!(Adapter::try_from(config).is_err());
	}

	#[test]
	fn test_try_from_config() {
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		);

		let adapter = Adapter::try_from(config).unwrap();
		assert_eq!(adapter.adapter_id, "test-adapter");
		assert_eq!(adapter.name, "Test Adapter");
		assert_eq!(adapter.version, "1.0.0");
	}

	#[test]
	fn test_builder_pattern() {
		let ethereum = Network::new(1, "Ethereum".to_string(), false);
		let usdc = Asset::new(
			"0xA0b86a33E6441D9c8c6d8C7e6b3b34e3C0E4F1e7".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
			1,
		);

		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_description("Test Description".to_string())
		.with_network(ethereum.clone())
		.with_asset(usdc.clone())
		.with_endpoint("https://test.example.com".to_string())
		.with_timeout_ms(5000)
		.with_enabled(true);

		assert_eq!(config.description, Some("Test Description".to_string()));
		assert_eq!(config.supported_networks, Some(vec![ethereum]));
		assert_eq!(config.supported_assets, Some(vec![usdc]));
		assert_eq!(
			config.endpoint,
			Some("https://test.example.com".to_string())
		);
		assert_eq!(config.timeout_ms, Some(5000));
		assert_eq!(config.enabled, Some(true));
		assert!(Adapter::try_from(config).is_ok());
	}

	#[test]
	fn test_explicit_networks_and_assets() {
		use crate::models::{Asset, Network};

		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::LifiV1,
			"Test LiFi Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_networks(vec![
			Network::ethereum(),
			Network::polygon(),
			Network::bsc(),
		])
		.with_assets(vec![
			Asset::eth(),
			Asset::usdc_ethereum(),
			Asset::matic(),
			Asset::usdc_polygon(),
		]);

		let networks = config.get_supported_networks();
		assert_eq!(networks.len(), 3); // Ethereum, Polygon, BSC
		assert!(networks.iter().any(|n| n.chain_id == 1)); // Ethereum
		assert!(networks.iter().any(|n| n.chain_id == 137)); // Polygon
		assert!(networks.iter().any(|n| n.chain_id == 56)); // BSC

		let assets = config.get_supported_assets();
		assert!(!assets.is_empty());

		// Check that assets exist for supported networks
		let ethereum_assets = config.get_assets_for_network(&Network::ethereum());
		assert_eq!(ethereum_assets.len(), 2); // ETH and USDC

		let polygon_assets = config.get_assets_for_network(&Network::polygon());
		assert_eq!(polygon_assets.len(), 2); // MATIC and USDC
	}

	#[test]
	fn test_empty_networks_and_assets_by_default() {
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::LifiV1,
			"Test LiFi Adapter".to_string(),
			"1.0.0".to_string(),
		);

		// Should be empty by default (no implicit defaults)
		let networks = config.get_supported_networks();
		assert_eq!(networks.len(), 0);

		let assets = config.get_supported_assets();
		assert_eq!(assets.len(), 0);
	}

	#[test]
	fn test_invalid_network_configuration() {
		// Test empty networks
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_networks(vec![]); // Empty networks

		assert!(Adapter::try_from(config).is_err());

		// Test duplicate chain IDs
		let network1 = Network::new(1, "Ethereum".to_string(), false);
		let network2 = Network::new(1, "Ethereum Duplicate".to_string(), false);

		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_networks(vec![network1, network2]);

		assert!(Adapter::try_from(config).is_err());
	}

	#[test]
	fn test_invalid_asset_configuration() {
		// Test asset with unsupported chain ID
		let ethereum = Network::new(1, "Ethereum".to_string(), false);
		let polygon_asset = Asset::new(
			"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
			137, // Polygon chain ID but only Ethereum network is supported
		);

		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_network(ethereum)
		.with_asset(polygon_asset);

		assert!(Adapter::try_from(config).is_err());
	}

	#[test]
	fn test_invalid_endpoint() {
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_endpoint("invalid-url".to_string());

		assert!(Adapter::try_from(config).is_err());
	}

	#[test]
	fn test_invalid_timeout() {
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_timeout_ms(0); // Invalid timeout

		assert!(Adapter::try_from(config).is_err());

		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_timeout_ms(400_000); // Too large timeout

		assert!(Adapter::try_from(config).is_err());
	}
}
