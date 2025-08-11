//! Adapter storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Adapter, AdapterError, AdapterResult, AdapterType};
use crate::models::{Asset, Network};

/// Storage representation of an adapter
///
/// This model is optimized for storage and persistence.
/// It can be converted to/from the domain Adapter model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStorage {
	/// Core adapter identity
	pub adapter_id: String,
	pub adapter_type: AdapterTypeStorage,
	pub name: String,
	pub version: String,
	pub enabled: bool,
	
	/// Core operational parameters
	pub endpoint: String,
	pub timeout_ms: u64,
	
	/// Timestamps
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,

	/// Extended configuration and metadata
	pub metadata: AdapterMetadataStorage,
}

/// Storage-compatible adapter metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMetadataStorage {
	/// Optional description
	pub description: Option<String>,
	
	/// Supported blockchain networks
	pub supported_networks: Option<Vec<NetworkStorage>>,
	
	/// Supported assets per network
	pub supported_assets: Option<Vec<AssetStorage>>,
	
	/// Adapter-specific configuration
	pub configuration: HashMap<String, serde_json::Value>,
}

/// Storage-compatible network representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NetworkStorage {
	pub chain_id: u64,
	pub name: String,
	pub is_testnet: bool,
}

/// Storage-compatible asset representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AssetStorage {
	pub address: String,
	pub symbol: String,
	pub name: String,
	pub decimals: u8,
	pub chain_id: u64,
}

/// Storage-compatible adapter type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterTypeStorage {
	OifV1,
	LifiV1,
}

impl AdapterStorage {
	/// Create storage adapter from domain adapter with operational parameters
	pub fn from_domain_with_params(adapter: Adapter, endpoint: String, timeout_ms: u64) -> Self {
		let metadata = AdapterMetadataStorage {
			description: adapter.description,
			supported_networks: None, // TODO: Extract from domain when implemented
			supported_assets: None,   // TODO: Extract from domain when implemented
			configuration: adapter.configuration,
		};

		Self {
			adapter_id: adapter.adapter_id,
			adapter_type: AdapterTypeStorage::from_domain(&adapter.adapter_type),
			name: adapter.name,
			version: adapter.version,
			enabled: adapter.enabled,
			endpoint,
			timeout_ms,
			created_at: adapter.created_at,
			updated_at: adapter.updated_at,
			metadata,
		}
	}

	/// Create storage adapter from domain adapter (uses default operational parameters)
	pub fn from_domain(adapter: Adapter) -> Self {
		// Use default values for operational parameters
		let (endpoint, timeout_ms) = Self::default_operational_params(&adapter.adapter_type);
		Self::from_domain_with_params(adapter, endpoint, timeout_ms)
	}

	/// Convert storage adapter to domain adapter
	pub fn to_domain(self) -> AdapterResult<Adapter> {
		Ok(Adapter {
			adapter_id: self.adapter_id,
			adapter_type: self.adapter_type.to_domain(),
			name: self.name,
			description: self.metadata.description,
			version: self.version,
			configuration: self.metadata.configuration,
			enabled: self.enabled,
			created_at: self.created_at,
			updated_at: self.updated_at,
		})
	}

	/// Get default operational parameters for adapter type
	fn default_operational_params(adapter_type: &AdapterType) -> (String, u64) {
		match adapter_type {
			AdapterType::OifV1 => ("http://localhost:8080".to_string(), 30000),
			AdapterType::LifiV1 => ("https://li.quest".to_string(), 10000),
		}
	}
}

impl AdapterMetadataStorage {
	/// Create metadata storage from domain values
	pub fn new() -> Self {
		Self {
			description: None,
			supported_networks: None,
			supported_assets: None,
			configuration: HashMap::new(),
		}
	}

	/// Add supported networks
	pub fn with_networks(mut self, networks: Vec<Network>) -> Self {
		self.supported_networks = Some(
			networks.into_iter()
				.map(NetworkStorage::from_domain)
				.collect()
		);
		self
	}

	/// Add supported assets
	pub fn with_assets(mut self, assets: Vec<Asset>) -> Self {
		self.supported_assets = Some(
			assets.into_iter()
				.map(AssetStorage::from_domain)
				.collect()
		);
		self
	}

	/// Convert to domain networks
	pub fn get_networks(&self) -> Vec<Network> {
		match &self.supported_networks {
			Some(networks) => networks.iter().map(|n| n.to_domain()).collect(),
			None => vec![],
		}
	}

	/// Convert to domain assets
	pub fn get_assets(&self) -> Vec<Asset> {
		match &self.supported_assets {
			Some(assets) => assets.iter().map(|a| a.to_domain()).collect(),
			None => vec![],
		}
	}
}

impl NetworkStorage {
	/// Create from domain network
	pub fn from_domain(network: Network) -> Self {
		Self {
			chain_id: network.chain_id,
			name: network.name,
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

impl AssetStorage {
	/// Create from domain asset
	pub fn from_domain(asset: Asset) -> Self {
		Self {
			address: asset.address,
			symbol: asset.symbol,
			name: asset.name,
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

impl Default for AdapterMetadataStorage {
	fn default() -> Self {
		Self::new()
	}
}

impl AdapterTypeStorage {
	pub fn from_domain(adapter_type: &AdapterType) -> Self {
		match adapter_type {
			AdapterType::OifV1 => Self::OifV1,
			AdapterType::LifiV1 => Self::LifiV1,
		}
	}

	pub fn to_domain(&self) -> AdapterType {
		match self {
			Self::OifV1 => AdapterType::OifV1,
			Self::LifiV1 => AdapterType::LifiV1,
		}
	}
}

/// Conversion traits
impl From<Adapter> for AdapterStorage {
	fn from(adapter: Adapter) -> Self {
		Self::from_domain(adapter)
	}
}

impl TryFrom<AdapterStorage> for Adapter {
	type Error = AdapterError;

	fn try_from(storage: AdapterStorage) -> Result<Self, Self::Error> {
		storage.to_domain()
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
	}

	#[test]
	fn test_storage_conversion() {
		let adapter = create_test_adapter();
		let adapter_id = adapter.adapter_id.clone();

		// Convert to storage (with defaults)
		let storage = AdapterStorage::from_domain(adapter);
		assert_eq!(storage.adapter_id, adapter_id);
		assert!(!storage.endpoint.is_empty());
		assert!(storage.timeout_ms > 0);

		// Convert back to domain
		let domain_adapter = storage.to_domain().unwrap();
		assert_eq!(domain_adapter.adapter_id, adapter_id);
	}

	#[test]
	fn test_storage_conversion_with_params() {
		let adapter = create_test_adapter();
		let adapter_id = adapter.adapter_id.clone();
		let endpoint = "https://custom.endpoint.com".to_string();
		let timeout_ms = 15000;

		// Convert to storage with custom params
		let storage = AdapterStorage::from_domain_with_params(adapter, endpoint.clone(), timeout_ms);
		assert_eq!(storage.adapter_id, adapter_id);
		assert_eq!(storage.endpoint, endpoint);
		assert_eq!(storage.timeout_ms, timeout_ms);

		// Convert back to domain
		let domain_adapter = storage.to_domain().unwrap();
		assert_eq!(domain_adapter.adapter_id, adapter_id);
	}

	#[test]
	fn test_adapter_type_conversion() {
		assert_eq!(
			AdapterTypeStorage::from_domain(&AdapterType::OifV1),
			AdapterTypeStorage::OifV1
		);
		assert_eq!(AdapterTypeStorage::OifV1.to_domain(), AdapterType::OifV1);
	}

	#[test]
	fn test_metadata_storage() {
		let metadata = AdapterMetadataStorage::new()
			.with_networks(vec![
				Network::new(1, "Ethereum".to_string(), false)
			])
			.with_assets(vec![
				Asset::new(
					"0x0".to_string(),
					"ETH".to_string(),
					"Ethereum".to_string(),
					18,
					1,
				)
			]);

		assert_eq!(metadata.get_networks().len(), 1);
		assert_eq!(metadata.get_assets().len(), 1);
		assert_eq!(metadata.get_networks()[0].chain_id, 1);
		assert_eq!(metadata.get_assets()[0].symbol, "ETH");
	}

	#[test]
	fn test_network_storage_conversion() {
		let domain_network = Network::new(1, "Ethereum".to_string(), false);
		let storage_network = NetworkStorage::from_domain(domain_network.clone());
		let converted_back = storage_network.to_domain();

		assert_eq!(domain_network.chain_id, converted_back.chain_id);
		assert_eq!(domain_network.name, converted_back.name);
		assert_eq!(domain_network.is_testnet, converted_back.is_testnet);
	}

	#[test]
	fn test_asset_storage_conversion() {
		let domain_asset = Asset::new(
			"0x0".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
			1,
		);
		let storage_asset = AssetStorage::from_domain(domain_asset.clone());
		let converted_back = storage_asset.to_domain();

		assert_eq!(domain_asset.address, converted_back.address);
		assert_eq!(domain_asset.symbol, converted_back.symbol);
		assert_eq!(domain_asset.name, converted_back.name);
		assert_eq!(domain_asset.decimals, converted_back.decimals);
		assert_eq!(domain_asset.chain_id, converted_back.chain_id);
	}
}
