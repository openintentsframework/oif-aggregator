//! Core Adapter domain model and business logic

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod config;
pub mod errors;
pub mod response;
pub mod storage;
pub mod traits;

pub use config::{AdapterConfig, AdapterType};
pub use errors::{
	AdapterError, AdapterFactoryError, AdapterFactoryResult, AdapterResult, AdapterValidationError,
	AdapterValidationResult,
};
pub use response::AdapterResponse;
pub use storage::AdapterStorage;
pub use traits::SolverAdapter;

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

	/// Adapter-specific configuration
	pub configuration: HashMap<String, serde_json::Value>,

	/// Whether the adapter is enabled
	pub enabled: bool,

	/// When the adapter was created/registered
	pub created_at: DateTime<Utc>,

	/// Last time the adapter was updated
	pub updated_at: DateTime<Utc>,
}

/// Supported blockchain network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Network {
	/// Chain ID (e.g., 1 for Ethereum mainnet, 137 for Polygon)
	pub chain_id: u64,
	/// Human-readable name (e.g., "Ethereum", "Polygon", "Base")
	pub name: String,
	/// Whether the network is a testnet
	pub is_testnet: bool,
}

impl Network {
	pub fn new(chain_id: u64, name: String, is_testnet: bool) -> Self {
		Self {
			chain_id,
			name,
			is_testnet,
		}
	}
}

/// Supported asset on a blockchain network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Asset {
	/// Contract address (0x0 for native token)
	pub address: String,
	/// Token symbol (e.g., "USDC", "ETH", "USDT")
	pub symbol: String,
	/// Token name (e.g., "USD Coin", "Ethereum", "Tether USD")
	pub name: String,
	/// Number of decimal places
	pub decimals: u8,
	/// Chain ID this asset exists on
	pub chain_id: u64,
}

impl Asset {
	pub fn new(address: String, symbol: String, name: String, decimals: u8, chain_id: u64) -> Self {
		Self {
			address,
			symbol,
			name,
			decimals,
			chain_id,
		}
	}
}

/// Detailed order information from an adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDetails {
	/// Order ID from the solver
	pub order_id: String,
	/// Current status of the order
	pub status: String,
	/// Transaction hash if available
	pub transaction_hash: Option<String>,
	/// Gas used for the transaction
	pub gas_used: Option<u64>,
	/// Gas price in wei
	pub gas_price: Option<String>,
	/// Transaction fee in wei
	pub transaction_fee: Option<String>,
	/// Block number where transaction was included
	pub block_number: Option<u64>,
	/// Additional metadata from the solver
	pub metadata: HashMap<String, serde_json::Value>,
	/// When the order was last updated
	pub updated_at: DateTime<Utc>,
}

impl OrderDetails {
	pub fn new(order_id: String, status: String) -> Self {
		Self {
			order_id,
			status,
			transaction_hash: None,
			gas_used: None,
			gas_price: None,
			transaction_fee: None,
			block_number: None,
			metadata: HashMap::new(),
			updated_at: Utc::now(),
		}
	}
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
			configuration: HashMap::new(),
			enabled: true,
			created_at: now,
			updated_at: now,
		}
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

	/// Enable the adapter
	pub fn enable(&mut self) {
		self.enabled = true;
		self.updated_at = Utc::now();
	}

	/// Disable the adapter
	pub fn disable(&mut self) {
		self.enabled = false;
		self.updated_at = Utc::now();
	}

	/// Builder methods for easy configuration
	pub fn with_description(mut self, description: String) -> Self {
		self.description = Some(description);
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
		self
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
		assert!(adapter.enabled);
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
	fn test_builder_pattern() {
		let adapter = create_test_adapter()
			.with_description("Test Description".to_string())
			.enabled(true);

		assert_eq!(adapter.description, Some("Test Description".to_string()));
		assert!(adapter.enabled);
	}
}
