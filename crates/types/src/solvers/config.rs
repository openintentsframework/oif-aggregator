//! Solver configuration models and validation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Solver, SolverValidationError, SolverValidationResult};
use crate::models::{AssetRoute, InteropAddress};

/// User-friendly route configuration for config files
///
/// This format is more readable in JSON config files than the full InteropAddress format.
/// It gets converted to AssetRoute during solver creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteConfig {
	/// Origin chain ID (e.g., 1 for Ethereum)
	pub origin_chain_id: u64,

	/// Origin token address (e.g., "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
	pub origin_token_address: String,

	/// Optional origin token symbol for readability (e.g., "WETH", "USDC")
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin_token_symbol: Option<String>,

	/// Destination chain ID (e.g., 10 for Optimism)
	pub destination_chain_id: u64,

	/// Destination token address (e.g., "0x4200000000000000000000000000000000000006")
	pub destination_token_address: String,

	/// Optional destination token symbol for readability (e.g., "WETH", "USDC")
	#[serde(skip_serializing_if = "Option::is_none")]
	pub destination_token_symbol: Option<String>,

	/// Optional route-specific metadata (fees, limits, etc.)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

/// Configuration for what assets/routes a solver supports
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SupportedAssetsConfig {
	/// List of assets or routes depending on the type
	pub assets: Vec<AssetOrRouteConfig>,
	/// How to interpret the assets list
	#[serde(rename = "type")]
	pub asset_type: AssetType,
}

/// How to interpret the assets list
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
	/// Treat as individual assets (any-to-any within list)
	Assets,
	/// Treat as explicit routes (origin->destination pairs)
	Routes,
}

/// Either an asset or a route configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AssetOrRouteConfig {
	/// Single asset (for assets mode)
	Asset(AssetConfig),
	/// Route pair (for routes mode)  
	Route(RouteConfig),
}

/// Asset configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetConfig {
	pub chain_id: u64,
	pub address: String,
	pub symbol: Option<String>,
}

/// Convert from config AssetConfig to domain Asset
impl TryFrom<AssetConfig> for crate::models::Asset {
	type Error = String;

	fn try_from(config: AssetConfig) -> Result<Self, Self::Error> {
		let symbol = config.symbol.unwrap_or_else(|| "UNKNOWN".to_string());
		let name = symbol.clone();

		crate::models::Asset::from_chain_and_address(
			config.chain_id,
			config.address,
			symbol,
			name,
			18, // Default to 18 decimals (most common)
		)
		.map_err(|e| format!("Failed to create asset: {}", e))
	}
}

/// Convert from config SupportedAssetsConfig to domain SupportedAssets
impl TryFrom<SupportedAssetsConfig> for crate::solvers::SupportedAssets {
	type Error = String;

	fn try_from(config: SupportedAssetsConfig) -> Result<Self, Self::Error> {
		match config.asset_type {
			AssetType::Assets => {
				let mut assets = Vec::new();
				for asset_or_route in config.assets {
					match asset_or_route {
						AssetOrRouteConfig::Asset(asset_config) => {
							let asset = crate::models::Asset::try_from(asset_config)?;
							assets.push(asset);
						},
						AssetOrRouteConfig::Route(_) => {
							return Err("Found route config in assets mode".to_string());
						},
					}
				}
				Ok(crate::solvers::SupportedAssets::Assets {
					assets,
					source: crate::solvers::AssetSource::Config,
				})
			},
			AssetType::Routes => {
				let mut routes = Vec::new();
				for asset_or_route in config.assets {
					match asset_or_route {
						AssetOrRouteConfig::Route(route_config) => {
							let route = crate::models::AssetRoute::try_from(route_config)?;
							routes.push(route);
						},
						AssetOrRouteConfig::Asset(_) => {
							return Err("Found asset config in routes mode".to_string());
						},
					}
				}
				Ok(crate::solvers::SupportedAssets::Routes {
					routes,
					source: crate::solvers::AssetSource::Config,
				})
			},
		}
	}
}

/// Convert from config RouteConfig to domain AssetRoute
impl TryFrom<RouteConfig> for AssetRoute {
	type Error = String;

	fn try_from(config: RouteConfig) -> Result<Self, Self::Error> {
		let origin_asset = InteropAddress::from_chain_and_address(
			config.origin_chain_id,
			&config.origin_token_address,
		)
		.map_err(|e| format!("Invalid origin asset: {}", e))?;

		let destination_asset = InteropAddress::from_chain_and_address(
			config.destination_chain_id,
			&config.destination_token_address,
		)
		.map_err(|e| format!("Invalid destination asset: {}", e))?;

		// Build AssetRoute with symbols and metadata
		let mut route = AssetRoute::new(origin_asset, destination_asset);

		if let Some(origin_symbol) = config.origin_token_symbol {
			route = route.with_origin_symbol(origin_symbol);
		}

		if let Some(destination_symbol) = config.destination_token_symbol {
			route = route.with_destination_symbol(destination_symbol);
		}

		if let Some(metadata) = config.metadata {
			route.metadata = Some(metadata);
		}

		Ok(route)
	}
}

/// Convert from domain AssetRoute to config RouteConfig (for serialization back to config)
impl TryFrom<&AssetRoute> for RouteConfig {
	type Error = String;

	fn try_from(route: &AssetRoute) -> Result<Self, Self::Error> {
		Ok(Self {
			origin_chain_id: route.origin_chain_id()?,
			origin_token_address: route.origin_address(),
			origin_token_symbol: route.origin_token_symbol.clone(),
			destination_chain_id: route.destination_chain_id()?,
			destination_token_address: route.destination_address(),
			destination_token_symbol: route.destination_token_symbol.clone(),
			metadata: route.metadata.clone(),
		})
	}
}

impl RouteConfig {
	/// Create a new route config with all fields
	pub fn new(
		origin_chain_id: u64,
		origin_token_address: String,
		destination_chain_id: u64,
		destination_token_address: String,
	) -> Self {
		Self {
			origin_chain_id,
			origin_token_address,
			origin_token_symbol: None,
			destination_chain_id,
			destination_token_address,
			destination_token_symbol: None,
			metadata: None,
		}
	}

	/// Add origin token symbol for readability
	pub fn with_origin_symbol(mut self, symbol: String) -> Self {
		self.origin_token_symbol = Some(symbol);
		self
	}

	/// Add destination token symbol for readability
	pub fn with_destination_symbol(mut self, symbol: String) -> Self {
		self.destination_token_symbol = Some(symbol);
		self
	}

	/// Add metadata
	pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
		self.metadata = Some(metadata);
		self
	}
}

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

	/// Adapter-specific metadata (JSON configuration for adapter customization)
	pub adapter_metadata: Option<serde_json::Value>,

	/// Optional human-readable name
	pub name: Option<String>,

	/// Optional description
	pub description: Option<String>,

	/// API version
	pub version: Option<String>,

	pub supported_assets: Option<SupportedAssetsConfig>,
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
			adapter_metadata: None,
			name: None,
			description: None,
			version: None,
			supported_assets: None,
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

	pub fn with_routes(mut self, routes: Vec<RouteConfig>) -> Self {
		self.supported_assets = Some(SupportedAssetsConfig {
			assets: routes.into_iter().map(AssetOrRouteConfig::Route).collect(),
			asset_type: AssetType::Routes,
		});
		self
	}

	pub fn with_assets(mut self, assets: Vec<AssetConfig>) -> Self {
		self.supported_assets = Some(SupportedAssetsConfig {
			assets: assets.into_iter().map(AssetOrRouteConfig::Asset).collect(),
			asset_type: AssetType::Assets,
		});
		self
	}

	pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
		self.headers = Some(headers);
		self
	}

	pub fn with_adapter_metadata(mut self, adapter_metadata: serde_json::Value) -> Self {
		self.adapter_metadata = Some(adapter_metadata);
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

		if let Some(assets_config) = config.supported_assets {
			// Convert to domain SupportedAssets using TryFrom
			let supported_assets = crate::solvers::SupportedAssets::try_from(assets_config)
				.map_err(|e| SolverValidationError::InvalidConfiguration {
					reason: format!("Invalid supported_assets configuration: {}", e),
				})?;

			// Set the supported_assets directly
			solver.metadata.supported_assets = supported_assets;
		}

		if let Some(headers) = config.headers {
			solver = solver.with_headers(headers);
		}

		if let Some(adapter_metadata) = config.adapter_metadata {
			solver = solver.with_adapter_metadata(adapter_metadata);
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
	fn test_route_config_conversion() {
		// Test RouteConfig with symbols
		let route_config = RouteConfig::new(
			1,                                                        // Ethereum
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
			10,                                                       // Optimism
			"0x4200000000000000000000000000000000000006".to_string(), // WETH
		)
		.with_origin_symbol("WETH".to_string())
		.with_destination_symbol("WETH".to_string())
		.with_metadata(serde_json::json!({
			"min_amount": "1000000000000000000",
			"max_amount": "100000000000000000000"
		}));

		// Convert to AssetRoute using TryFrom
		let asset_route = AssetRoute::try_from(route_config).unwrap();

		assert_eq!(asset_route.origin_chain_id().unwrap(), 1);
		assert_eq!(asset_route.destination_chain_id().unwrap(), 10);
		assert!(asset_route.is_cross_chain());
		assert!(asset_route.metadata.is_some());

		// Convert back to RouteConfig using TryFrom
		let back_to_config = RouteConfig::try_from(&asset_route).unwrap();
		assert_eq!(back_to_config.origin_chain_id, 1);
		assert_eq!(back_to_config.destination_chain_id, 10);
		assert_eq!(
			back_to_config.origin_token_address.to_lowercase(),
			"0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
		);
		// Symbols are now preserved in round-trip conversion!
		assert_eq!(back_to_config.origin_token_symbol, Some("WETH".to_string()));
		assert_eq!(
			back_to_config.destination_token_symbol,
			Some("WETH".to_string())
		);
	}

	#[test]
	fn test_solver_config_with_routes() {
		let route = RouteConfig::new(
			1,                                                        // Ethereum
			"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(), // USDC
			137,                                                      // Polygon
			"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(), // USDC
		)
		.with_origin_symbol("USDC".to_string())
		.with_destination_symbol("USDC".to_string());

		let config = SolverConfig::new(
			"test-solver".to_string(),
			"test-adapter".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_routes(vec![route]);

		// Convert to domain Solver
		let solver = Solver::try_from(config).unwrap();

		// Check that it's in routes mode with 1 route
		match &solver.metadata.supported_assets {
			crate::solvers::SupportedAssets::Routes { routes, .. } => {
				assert_eq!(routes.len(), 1);
				let route = &routes[0];
				assert_eq!(route.origin_chain_id().unwrap(), 1);
				assert_eq!(route.destination_chain_id().unwrap(), 137);
			},
			_ => panic!("Expected routes mode"),
		}
		assert!(solver.has_route_info());
	}

	#[test]
	fn test_solver_config_without_supported_assets() {
		let config = SolverConfig::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.test.com".to_string(),
		);

		let solver = Solver::try_from(config).unwrap();

		// When no supported_assets is provided, should default to auto-discovery
		match &solver.metadata.supported_assets {
			crate::solvers::SupportedAssets::Routes { routes, source } => {
				assert_eq!(routes.len(), 0);
				assert_eq!(source, &crate::solvers::AssetSource::AutoDiscovered);
			},
			_ => panic!("Expected routes mode with auto-discovery"),
		}
	}

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
		.with_routes(vec![
			RouteConfig::new(
				1,                                                        // Ethereum
				"0x0000000000000000000000000000000000000000".to_string(), // ETH
				137,                                                      // Polygon
				"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(), // USDC
			),
			RouteConfig::new(
				137,                                                      // Polygon
				"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(), // USDC
				1,                                                        // Ethereum
				"0x0000000000000000000000000000000000000000".to_string(), // ETH
			),
		]);

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
