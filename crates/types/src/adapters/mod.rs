//! Core Adapter domain model and business logic

use chrono::{DateTime, Utc};
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
