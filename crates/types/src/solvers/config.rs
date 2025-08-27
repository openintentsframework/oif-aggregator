//! Solver configuration models and validation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Solver, SolverValidationError, SolverValidationResult};
use crate::models::Asset;

/// Solver configuration from external sources (config files, API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
	/// Unique identifier for the solver
	pub solver_id: String,

	/// ID of the adapter to use for this solver
	pub adapter_id: String,

	/// HTTP endpoint for the solver API
	pub endpoint: String,

	/// Whether this solver is enabled
	pub enabled: bool,

	/// Custom HTTP headers for requests
	pub headers: Option<HashMap<String, String>>,

	/// Optional human-readable name
	pub name: Option<String>,

	/// Optional description
	pub description: Option<String>,

	/// API version
	pub version: Option<String>,

	/// Supported assets/tokens
	pub supported_assets: Option<Vec<Asset>>,

	/// Solver-specific configuration
	pub config: Option<HashMap<String, serde_json::Value>>,
}

/// Adapter configuration
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

	/// Adapter version
	pub version: String,

	/// Adapter-specific configuration
	pub configuration: serde_json::Value,

	/// Whether this adapter is enabled
	pub enabled: bool,

	/// When the adapter was created/registered
	pub created_at: DateTime<Utc>,
}

/// Types of adapters supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterType {
	/// Open Intent Framework v1 protocol
	OifV1,
	/// LiFi v1 protocol
	LifiV1,
	/// Custom adapter type (for runtime registration)
	Custom(String),
}

impl SolverConfig {
	/// Create a new solver configuration
	pub fn new(solver_id: String, adapter_id: String, endpoint: String) -> Self {
		Self {
			solver_id,
			adapter_id,
			endpoint,
			enabled: true,
			headers: None,
			name: None,
			description: None,
			version: None,
			supported_assets: None,
			config: None,
		}
	}

	pub fn with_name(mut self, name: String) -> Self {
		self.name = Some(name);
		self
	}

	pub fn with_description(mut self, description: String) -> Self {
		self.description = Some(description);
		self
	}

	pub fn with_assets(mut self, assets: Vec<Asset>) -> Self {
		self.supported_assets = Some(assets);
		self
	}

	pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
		self.headers = Some(headers);
		self
	}

	pub fn enabled(mut self, enabled: bool) -> Self {
		self.enabled = enabled;
		self
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
			configuration: serde_json::Value::Object(serde_json::Map::new()),
			enabled: true,
			created_at: Utc::now(),
		}
	}

	/// Validate adapter configuration
	pub fn validate(&self) -> SolverValidationResult<()> {
		if self.adapter_id.is_empty() {
			return Err(SolverValidationError::MissingRequiredField {
				field: "adapter_id".to_string(),
			});
		}

		if self.name.is_empty() {
			return Err(SolverValidationError::MissingRequiredField {
				field: "name".to_string(),
			});
		}

		if self.version.is_empty() {
			return Err(SolverValidationError::MissingRequiredField {
				field: "version".to_string(),
			});
		}

		Ok(())
	}
}

impl AdapterType {
	/// Get the default configuration for this adapter type
	pub fn default_config(&self) -> serde_json::Value {
		match self {
			AdapterType::OifV1 => serde_json::json!({
				"api_version": "v1",
				"quote_endpoint": "/quote",
				"intent_endpoint": "/intent",
				"health_endpoint": "/health"
			}),
			AdapterType::LifiV1 => serde_json::json!({
				"base_url": "https://li.quest",
				"api_version": "v1"
			}),
			AdapterType::Custom(_) => serde_json::json!({
				"api_version": "custom",
				"custom": true
			}),
		}
	}

	/// Get the human-readable name
	pub fn display_name(&self) -> String {
		match self {
			AdapterType::OifV1 => "OIF v1".to_string(),
			AdapterType::LifiV1 => "LiFi v1".to_string(),
			AdapterType::Custom(name) => format!("Custom: {}", name),
		}
	}
}

/// Convert from config SolverConfig to domain Solver
impl TryFrom<SolverConfig> for Solver {
	type Error = SolverValidationError;

	fn try_from(config: SolverConfig) -> Result<Self, Self::Error> {
		let mut solver = Solver::new(config.solver_id, config.adapter_id, config.endpoint);

		// Apply metadata
		if let Some(name) = config.name {
			solver = solver.with_name(name);
		}

		if let Some(description) = config.description {
			solver = solver.with_description(description);
		}

		if let Some(version) = config.version {
			solver = solver.with_version(version);
		}

		if let Some(assets) = config.supported_assets {
			solver = solver.with_assets(assets);
		}

		if let Some(headers) = config.headers {
			solver = solver.with_headers(headers);
		}

		if let Some(config_map) = config.config {
			for (key, value) in config_map {
				solver = solver.with_config(key, value);
			}
		}

		// Validate the constructed solver
		solver.validate()?;
		Ok(solver)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_solver_config_validation() {
		let valid_config = SolverConfig::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
		);

		assert!(Solver::try_from(valid_config).is_ok());
	}

	#[test]
	fn test_invalid_solver_id() {
		let mut config = SolverConfig::new(
			"invalid solver!".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
		);

		assert!(Solver::try_from(config.clone()).is_err());

		config.solver_id = "".to_string();
		assert!(Solver::try_from(config).is_err());
	}

	#[test]
	fn test_invalid_endpoint() {
		let config = SolverConfig::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"not-a-url".to_string(),
		);

		assert!(Solver::try_from(config).is_err());
	}

	#[test]
	fn test_config_to_domain() {
		let config = SolverConfig::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_name("Test Solver".to_string())
		.with_assets(vec![Asset::new(
			"0x0000000000000000000000000000000000000000".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
			1,
		)])
		.with_assets(vec![Asset::new(
			"0x0000000000000000000000000000000000000000".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
			1,
		)]);

		let solver = Solver::try_from(config).unwrap();

		assert_eq!(solver.solver_id, "test-solver");
		assert_eq!(solver.metadata.name, Some("Test Solver".to_string()));
		assert!(solver.supports_chain(1));
		assert!(solver.supports_chain(137));
	}

	#[test]
	fn test_adapter_type_display_names() {
		assert_eq!(AdapterType::OifV1.display_name(), "OIF v1");
		assert_eq!(AdapterType::LifiV1.display_name(), "LiFi v1");
	}

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
}
