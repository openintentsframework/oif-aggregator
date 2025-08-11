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
pub use response::{
	AdapterAssetsResponse, AdapterConfigResponse, AdapterDetailResponse, AdapterNetworksResponse,
	AdapterResponse, AssetResponse, NetworkResponse,
};
pub use storage::{AdapterMetadataStorage, AdapterStorage, AssetStorage, NetworkStorage};
pub use traits::SolverAdapter;

/// Core Adapter domain model
///
/// This represents an adapter in the domain layer with business logic.
/// It should be converted from AdapterConfig and to AdapterStorage/AdapterResponse.
#[derive(Debug, Clone, PartialEq)]
pub struct Adapter {
	/// Unique identifier for the adapter
	pub adapter_id: String,

	/// Type of adapter (OIF, LiFi, etc.)
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

	/// Supported networks for this adapter
	pub supported_networks: Vec<crate::models::Network>,

	/// Supported assets for this adapter
	pub supported_assets: Vec<crate::models::Asset>,

	/// Endpoint URL for the adapter service
	pub endpoint: Option<String>,

	/// Request timeout in milliseconds
	pub timeout_ms: Option<u64>,

	/// When the adapter was created/registered
	pub created_at: DateTime<Utc>,

	/// Last time the adapter was updated
	pub updated_at: DateTime<Utc>,
}

/// Detailed order information from an adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDetails {
	/// Order ID from the solver
	pub order_id: String,
	/// Current status of the order
	pub status: String,
	/// Transaction hash when available
	pub transaction_hash: Option<String>,
	/// Gas used in the transaction
	pub gas_used: Option<u64>,
	/// Gas price used
	pub gas_price: Option<String>,
	/// Transaction fee
	pub transaction_fee: Option<String>,
	/// Block number when mined
	pub block_number: Option<u64>,
	/// Additional metadata
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
			supported_networks: Vec::new(),
			supported_assets: Vec::new(),
			endpoint: None,
			timeout_ms: None,
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

		// Validate networks if provided
		if !self.supported_networks.is_empty() {
			// Check for duplicate chain IDs
			let mut chain_ids = std::collections::HashSet::new();
			for network in &self.supported_networks {
				if !chain_ids.insert(network.chain_id) {
					return Err(AdapterValidationError::InvalidConfiguration {
						reason: format!(
							"Duplicate chain ID {} in supported networks",
							network.chain_id
						),
					});
				}
			}
		}

		// Validate assets if provided
		if !self.supported_assets.is_empty() {
			// Check that all assets have valid chain IDs from supported networks
			let supported_chain_ids: std::collections::HashSet<u64> =
				self.supported_networks.iter().map(|n| n.chain_id).collect();

			for asset in &self.supported_assets {
				if !supported_chain_ids.contains(&asset.chain_id) {
					return Err(AdapterValidationError::InvalidConfiguration {
						reason: format!(
							"Asset {} has chain ID {} which is not in supported networks",
							asset.symbol, asset.chain_id
						),
					});
				}
			}
		}

		// Validate endpoint if provided
		if let Some(endpoint) = &self.endpoint {
			if endpoint.is_empty() {
				return Err(AdapterValidationError::InvalidConfiguration {
					reason: "endpoint cannot be empty if provided".to_string(),
				});
			}

			// Basic URL validation
			if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
				return Err(AdapterValidationError::InvalidConfiguration {
					reason: "endpoint must be a valid URL starting with http:// or https://"
						.to_string(),
				});
			}
		}

		// Validate timeout if provided
		if let Some(timeout_ms) = self.timeout_ms {
			if timeout_ms == 0 {
				return Err(AdapterValidationError::InvalidConfiguration {
					reason: "timeout_ms must be greater than 0".to_string(),
				});
			}

			if timeout_ms > 300_000 {
				return Err(AdapterValidationError::InvalidConfiguration {
					reason: "timeout_ms cannot exceed 5 minutes (300,000ms)".to_string(),
				});
			}
		}

		Ok(())
	}
}

/// Helper function to validate semantic version format
fn is_valid_semver(version: &str) -> bool {
	// Basic semver validation: X.Y.Z where X, Y, Z are numbers
	let parts: Vec<&str> = version.split('.').collect();
	if parts.len() != 3 {
		return false;
	}

	parts.iter().all(|part| part.parse::<u32>().is_ok())
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
