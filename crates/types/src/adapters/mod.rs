//! Core Adapter domain model and business logic

use std::collections::HashMap;

pub mod errors;
pub mod models;
pub mod traits;

pub use errors::{AdapterError, AdapterFactoryError, AdapterValidationError};
pub use models::{
	AdapterQuote, AssetAmount, AvailableInput, GetOrderResponse, GetQuoteRequest, GetQuoteResponse,
	OrderResponse, OrderStatus, QuoteDetails, QuoteOrder, QuotePreference, RequestedOutput,
	Settlement, SettlementType, SignatureType,
};
pub use traits::SolverAdapter;

/// Result types for adapter operations
pub type AdapterResult<T> = Result<T, AdapterError>;
pub type AdapterValidationResult<T> = Result<T, AdapterValidationError>;

/// Minimal runtime configuration needed by adapters
///
/// This contains only the essential fields that adapter implementations actually need,
/// providing a clean separation from the full Solver configuration which includes
/// aggregator-specific metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SolverRuntimeConfig {
	/// Unique solver instance identifier
	pub solver_id: String,

	/// HTTP endpoint for the solver API
	pub endpoint: String,

	/// Timeout for requests in milliseconds
	pub timeout_ms: u64,

	/// Optional custom HTTP headers for requests
	pub headers: Option<HashMap<String, String>>,
}

impl SolverRuntimeConfig {
	/// Create a new runtime config
	pub fn new(solver_id: String, endpoint: String, timeout_ms: u64) -> Self {
		Self {
			solver_id,
			endpoint,
			timeout_ms,
			headers: None,
		}
	}

	/// Create runtime config with optional headers
	pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
		self.headers = Some(headers);
		self
	}
}

impl From<&crate::solvers::Solver> for SolverRuntimeConfig {
	fn from(solver: &crate::solvers::Solver) -> Self {
		Self {
			solver_id: solver.solver_id.clone(),
			endpoint: solver.endpoint.clone(),
			timeout_ms: solver.timeout_ms,
			headers: solver.headers.clone(),
		}
	}
}

/// Core Adapter domain model
///
/// This represents an adapter in the domain layer with business logic.
/// It should be converted from AdapterConfig and to AdapterStorage/AdapterResponse.
#[derive(Debug, Clone, PartialEq)]
pub struct Adapter {
	/// Unique identifier for the adapter
	pub adapter_id: String,

	/// Human-readable name
	pub name: String,

	/// Description of the adapter
	pub description: Option<String>,

	/// Version of the adapter implementation
	pub version: String,

	/// Adapter-specific configuration
	pub configuration: HashMap<String, serde_json::Value>,
}

impl Adapter {
	/// Create a new adapter
	pub fn new(adapter_id: String, description: String, name: String, version: String) -> Self {
		Self {
			adapter_id,
			name,
			description: Some(description),
			version,
			configuration: HashMap::new(),
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
		Ok(())
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
			"OIF v1".to_string(),
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
	}

	#[test]
	fn test_adapter_creation() {
		let adapter = create_test_adapter();

		assert_eq!(adapter.adapter_id, "test-adapter");
		assert_eq!(adapter.name, "Test Adapter");
		assert_eq!(adapter.version, "1.0.0");
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
		let adapter = create_test_adapter().with_description("Test Description".to_string());

		assert_eq!(adapter.description, Some("Test Description".to_string()));
	}
}
