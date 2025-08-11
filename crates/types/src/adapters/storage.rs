//! Adapter storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Adapter, AdapterError, AdapterResult, AdapterType};

/// Storage representation of an adapter
///
/// This model is optimized for storage and persistence.
/// It can be converted to/from the domain Adapter model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStorage {
	pub adapter_id: String,
	pub adapter_type: AdapterTypeStorage,
	pub name: String,
	pub description: Option<String>,
	pub version: String,
	pub configuration: HashMap<String, serde_json::Value>,
	pub enabled: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

// No status stored at protocol-level

/// Storage-compatible adapter type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterTypeStorage {
	OifV1,
	LifiV1,
}

impl AdapterStorage {
	/// Create storage adapter from domain adapter
	pub fn from_domain(adapter: Adapter) -> Self {
		Self {
			adapter_id: adapter.adapter_id,
			adapter_type: AdapterTypeStorage::from_domain(&adapter.adapter_type),
			name: adapter.name,
			description: adapter.description,
			version: adapter.version,
			configuration: adapter.configuration,
			enabled: adapter.enabled,
			created_at: adapter.created_at,
			updated_at: adapter.updated_at,
		}
	}

	/// Convert storage adapter to domain adapter
	pub fn to_domain(self) -> AdapterResult<Adapter> {
		Ok(Adapter {
			adapter_id: self.adapter_id,
			adapter_type: self.adapter_type.to_domain(),
			name: self.name,
			description: self.description,
			version: self.version,
			configuration: self.configuration,
			enabled: self.enabled,
			created_at: self.created_at,
			updated_at: self.updated_at,
		})
	}
}

impl AdapterTypeStorage {
	fn from_domain(adapter_type: &AdapterType) -> Self {
		match adapter_type {
			AdapterType::OifV1 => Self::OifV1,
			AdapterType::LifiV1 => Self::LifiV1,
		}
	}

	fn to_domain(&self) -> AdapterType {
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
		.with_description("Test Description".to_string())
	}

	#[test]
	fn test_storage_conversion() {
		let adapter = create_test_adapter();
		let adapter_id = adapter.adapter_id.clone();

		// Convert to storage
		let storage = AdapterStorage::from_domain(adapter);
		assert_eq!(storage.adapter_id, adapter_id);

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
}
