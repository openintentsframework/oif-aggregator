//! Adapter configuration models and validation

use serde::{Deserialize, Serialize};

use super::{Adapter, AdapterValidationError, AdapterValidationResult};

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
		}
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

		adapter.description = self.description.clone();

		Ok(adapter)
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
		}
	}
}

/// Validate semantic version format
fn is_valid_semver(version: &str) -> bool {
	// Simple semver validation - just check basic format
	let parts: Vec<&str> = version.split('.').collect();
	if parts.len() != 3 {
		return false;
	}

	parts
		.iter()
		.all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
	use super::*;

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

	#[test]
	fn test_invalid_adapter_id() {
		let config = AdapterConfig::new(
			"".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		);

		assert!(config.validate().is_err());
	}

	#[test]
	fn test_invalid_version() {
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"invalid".to_string(),
		);

		assert!(config.validate().is_err());
	}

	#[test]
	fn test_to_domain() {
		let config = AdapterConfig::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		);

		let adapter = config.to_domain().unwrap();
		assert_eq!(adapter.adapter_id, "test-adapter");
		assert_eq!(adapter.name, "Test Adapter");
		assert_eq!(adapter.version, "1.0.0");
	}
}
