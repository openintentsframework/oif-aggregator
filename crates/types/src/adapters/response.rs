//! Adapter response models for API layer

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{Adapter, AdapterResult, AdapterType};

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
		let mut adapter = create_test_adapter();

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
}
