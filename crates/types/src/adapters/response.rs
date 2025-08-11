//! Adapter response models for API layer

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{Adapter, AdapterResult, AdapterStorage, AdapterType};
use crate::models::{Asset, Network};

/// Response format for individual adapters in API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterResponse {
	pub adapter_id: String,
	pub adapter_type: String,
	pub name: String,
	pub description: Option<String>,
	pub version: String,
	pub created_at: i64,
}

/// Enhanced adapter response with detailed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterDetailResponse {
	pub adapter_id: String,
	pub adapter_type: String,
	pub name: String,
	pub description: Option<String>,
	pub version: String,
	pub enabled: bool,
	pub supported_networks: Vec<NetworkResponse>,
	pub supported_assets: Vec<AssetResponse>,
	pub endpoint: Option<String>,
	pub timeout_ms: Option<u64>,
	pub created_at: i64,
	pub updated_at: i64,
}

/// Collection of adapters response for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptersResponse {
	pub adapters: Vec<AdapterResponse>,
	pub total_adapters: usize,
	pub timestamp: i64,
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
	pub created_at: i64,
}

/// Network information in API responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkResponse {
	pub chain_id: u64,
	pub name: String,
	pub is_testnet: bool,
}

/// Asset information in API responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetResponse {
	pub address: String,
	pub symbol: String,
	pub name: String,
	pub decimals: u8,
	pub chain_id: u64,
}

/// Response for adapter networks endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterNetworksResponse {
	pub adapter_id: String,
	pub networks: Vec<NetworkResponse>,
	pub total_networks: usize,
	pub timestamp: i64,
}

/// Response for adapter assets endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterAssetsResponse {
	pub adapter_id: String,
	pub chain_id: Option<u64>, // If filtered by network
	pub assets: Vec<AssetResponse>,
	pub total_assets: usize,
	pub timestamp: i64,
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
			created_at: adapter.created_at.timestamp(),
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
			created_at: adapter.created_at.timestamp(),
		})
	}
}

impl AdaptersResponse {
	/// Create adapters response from domain adapters
	pub fn from_domain_adapters(adapters: Vec<Adapter>) -> AdapterResult<Self> {
		let adapter_responses: Result<Vec<_>, _> =
			adapters.iter().map(AdapterResponse::from_domain).collect();

		let responses = adapter_responses?;

		Ok(Self {
			adapters: responses,
			total_adapters: adapters.len(),
			timestamp: Utc::now().timestamp(),
		})
	}

	/// Create minimal adapters response (for public API)
	pub fn minimal_from_domain_adapters(adapters: Vec<Adapter>) -> AdapterResult<Self> {
		let adapter_responses: Result<Vec<_>, _> = adapters
			.iter()
			.map(AdapterResponse::minimal_from_domain)
			.collect();

		let responses = adapter_responses?;

		Ok(Self {
			adapters: responses,
			total_adapters: adapters.len(),
			timestamp: Utc::now().timestamp(),
		})
	}
}

impl AdapterDetailResponse {
	/// Create detailed response from domain adapter (requires storage for operational params)
	pub fn from_domain(adapter: &Adapter) -> AdapterResult<Self> {
		Ok(Self {
			adapter_id: adapter.adapter_id.clone(),
			adapter_type: adapter.adapter_type.to_string(),
			name: adapter.name.clone(),
			description: adapter.description.clone(),
			version: adapter.version.clone(),
			enabled: adapter.enabled,
			supported_networks: vec![], // TODO: Extract from adapter when implemented
			supported_assets: vec![],   // TODO: Extract from adapter when implemented
			endpoint: None,             // Requires storage model for operational params
			timeout_ms: None,           // Requires storage model for operational params
			created_at: adapter.created_at.timestamp(),
			updated_at: adapter.updated_at.timestamp(),
		})
	}

	/// Create detailed response from storage (includes operational parameters)
	pub fn from_storage(storage: &AdapterStorage) -> AdapterResult<Self> {
		Ok(Self {
			adapter_id: storage.adapter_id.clone(),
			adapter_type: storage.adapter_type.to_domain().to_string(),
			name: storage.name.clone(),
			description: storage.metadata.description.clone(),
			version: storage.version.clone(),
			enabled: storage.enabled,
			supported_networks: storage
				.metadata
				.get_networks()
				.iter()
				.map(NetworkResponse::from_domain)
				.collect(),
			supported_assets: storage
				.metadata
				.get_assets()
				.iter()
				.map(AssetResponse::from_domain)
				.collect(),
			endpoint: Some(storage.endpoint.clone()),
			timeout_ms: Some(storage.timeout_ms),
			created_at: storage.created_at.timestamp(),
			updated_at: storage.updated_at.timestamp(),
		})
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
			created_at: adapter.created_at.timestamp(),
		}
	}
}

impl NetworkResponse {
	/// Create from domain network
	pub fn from_domain(network: &Network) -> Self {
		Self {
			chain_id: network.chain_id,
			name: network.name.clone(),
			is_testnet: network.is_testnet,
		}
	}

	/// Convert to domain network
	pub fn to_domain(&self) -> Network {
		Network {
			chain_id: self.chain_id,
			name: self.name.clone(),
			is_testnet: self.is_testnet,
		}
	}
}

impl AssetResponse {
	/// Create from domain asset
	pub fn from_domain(asset: &Asset) -> Self {
		Self {
			address: asset.address.clone(),
			symbol: asset.symbol.clone(),
			name: asset.name.clone(),
			decimals: asset.decimals,
			chain_id: asset.chain_id,
		}
	}

	/// Convert to domain asset
	pub fn to_domain(&self) -> Asset {
		Asset {
			address: self.address.clone(),
			symbol: self.symbol.clone(),
			name: self.name.clone(),
			decimals: self.decimals,
			chain_id: self.chain_id,
		}
	}
}

impl AdapterNetworksResponse {
	/// Create networks response for adapter
	pub fn from_networks(adapter_id: String, networks: Vec<Network>) -> Self {
		let network_responses: Vec<NetworkResponse> =
			networks.iter().map(NetworkResponse::from_domain).collect();

		Self {
			adapter_id,
			networks: network_responses,
			total_networks: networks.len(),
			timestamp: Utc::now().timestamp(),
		}
	}
}

impl AdapterAssetsResponse {
	/// Create assets response for adapter
	pub fn from_assets(adapter_id: String, assets: Vec<Asset>, chain_id: Option<u64>) -> Self {
		let asset_responses: Vec<AssetResponse> =
			assets.iter().map(AssetResponse::from_domain).collect();

		Self {
			adapter_id,
			chain_id,
			assets: asset_responses,
			total_assets: assets.len(),
			timestamp: Utc::now().timestamp(),
		}
	}

	/// Create assets response for specific network
	pub fn from_assets_for_network(
		adapter_id: String,
		network: &Network,
		assets: Vec<Asset>,
	) -> Self {
		Self::from_assets(adapter_id, assets, Some(network.chain_id))
	}
}

/// Convert AdapterType to string for API responses
impl std::fmt::Display for AdapterType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let s = match self {
			AdapterType::OifV1 => "oif-v1",
			AdapterType::LifiV1 => "lifi-v1",
		};
		write!(f, "{}", s)
	}
}

/// Convert domain Adapter to API AdapterResponse
impl TryFrom<Adapter> for AdapterResponse {
	type Error = crate::adapters::AdapterError;

	fn try_from(adapter: Adapter) -> Result<Self, Self::Error> {
		AdapterResponse::from_domain(&adapter)
	}
}

/// Convert domain collection to API response
impl TryFrom<Vec<Adapter>> for AdaptersResponse {
	type Error = crate::adapters::AdapterError;

	fn try_from(adapters: Vec<Adapter>) -> Result<Self, Self::Error> {
		AdaptersResponse::from_domain_adapters(adapters)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapters::{Adapter, AdapterType};

	fn create_test_adapter() -> Adapter {
		Adapter::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_description("Test Description".to_string())
	}

	#[test]
	fn test_adapter_response_from_domain() {
		let adapter = create_test_adapter();

		let response = AdapterResponse::from_domain(&adapter).unwrap();

		assert_eq!(response.adapter_id, "test-adapter");
		assert_eq!(response.name, "Test Adapter");
	}

	#[test]
	fn test_minimal_adapter_response() {
		let adapter = create_test_adapter();
		let response = AdapterResponse::minimal_from_domain(&adapter).unwrap();

		assert_eq!(response.adapter_id, "test-adapter");
	}

	#[test]
	fn test_adapters_response_creation() {
		let adapter1 = create_test_adapter();

		let mut adapter2 = create_test_adapter();
		adapter2.adapter_id = "adapter-2".to_string();

		let adapters = vec![adapter1, adapter2];
		let response = AdaptersResponse::from_domain_adapters(adapters).unwrap();

		assert_eq!(response.total_adapters, 2);
		assert!(response.timestamp > 0);
	}

	#[test]
	fn test_adapter_type_to_string() {
		assert_eq!(AdapterType::OifV1.to_string(), "oif-v1");
		assert_eq!(AdapterType::LifiV1.to_string(), "lifi-v1");
	}

	#[test]
	fn test_adapter_detail_response() {
		let adapter = create_test_adapter();
		let response = AdapterDetailResponse::from_domain(&adapter).unwrap();

		assert_eq!(response.adapter_id, "test-adapter");
		assert_eq!(response.name, "Test Adapter");
		assert_eq!(response.enabled, true);
		assert!(response.created_at > 0);
		assert!(response.updated_at > 0);
		// Domain model doesn't have operational params
		assert_eq!(response.endpoint, None);
		assert_eq!(response.timeout_ms, None);
	}

	#[test]
	fn test_adapter_detail_response_from_storage() {
		let adapter = create_test_adapter();
		let storage = AdapterStorage::from_domain_with_params(
			adapter,
			"https://test.endpoint.com".to_string(),
			15000,
		);

		let response = AdapterDetailResponse::from_storage(&storage).unwrap();

		assert_eq!(response.adapter_id, "test-adapter");
		assert_eq!(response.name, "Test Adapter");
		assert_eq!(response.enabled, true);
		assert_eq!(
			response.endpoint,
			Some("https://test.endpoint.com".to_string())
		);
		assert_eq!(response.timeout_ms, Some(15000));
		assert!(response.created_at > 0);
		assert!(response.updated_at > 0);
	}

	#[test]
	fn test_network_response_conversion() {
		let domain_network = Network::new(1, "Ethereum".to_string(), false);
		let response = NetworkResponse::from_domain(&domain_network);
		let converted_back = response.to_domain();

		assert_eq!(domain_network.chain_id, converted_back.chain_id);
		assert_eq!(domain_network.name, converted_back.name);
		assert_eq!(domain_network.is_testnet, converted_back.is_testnet);
	}

	#[test]
	fn test_asset_response_conversion() {
		let domain_asset = Asset::new(
			"0x0".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
			1,
		);
		let response = AssetResponse::from_domain(&domain_asset);
		let converted_back = response.to_domain();

		assert_eq!(domain_asset.address, converted_back.address);
		assert_eq!(domain_asset.symbol, converted_back.symbol);
		assert_eq!(domain_asset.name, converted_back.name);
		assert_eq!(domain_asset.decimals, converted_back.decimals);
		assert_eq!(domain_asset.chain_id, converted_back.chain_id);
	}

	#[test]
	fn test_adapter_networks_response() {
		let networks = vec![
			Network::new(1, "Ethereum".to_string(), false),
			Network::new(137, "Polygon".to_string(), false),
		];

		let response = AdapterNetworksResponse::from_networks("test-adapter".to_string(), networks);

		assert_eq!(response.adapter_id, "test-adapter");
		assert_eq!(response.total_networks, 2);
		assert_eq!(response.networks.len(), 2);
		assert!(response.timestamp > 0);
	}

	#[test]
	fn test_adapter_assets_response() {
		let assets = vec![
			Asset::new(
				"0x0".to_string(),
				"ETH".to_string(),
				"Ethereum".to_string(),
				18,
				1,
			),
			Asset::new(
				"0x123".to_string(),
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
				1,
			),
		];

		let response =
			AdapterAssetsResponse::from_assets("test-adapter".to_string(), assets, Some(1));

		assert_eq!(response.adapter_id, "test-adapter");
		assert_eq!(response.chain_id, Some(1));
		assert_eq!(response.total_assets, 2);
		assert_eq!(response.assets.len(), 2);
		assert!(response.timestamp > 0);
	}
}
