//! Asset route models for cross-chain transaction routing
//!
//! This module defines models for representing supported asset conversion routes
//! between different chains and tokens. Routes provide more precise compatibility
//! checking than just checking if individual assets are supported.

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use super::InteropAddress;

/// Represents a supported asset conversion route
///
/// An asset route defines a specific conversion path from an origin asset
/// on one chain to a destination asset on another (or the same) chain.
/// This provides more precise compatibility checking than just checking
/// if individual assets are supported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct AssetRoute {
	/// Origin asset (includes chain and token address)
	pub origin_asset: InteropAddress,

	/// Optional origin token symbol for readability and debugging (e.g., "WETH", "USDC")
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin_token_symbol: Option<String>,

	/// Destination asset (includes chain and token address)  
	pub destination_asset: InteropAddress,

	/// Optional destination token symbol for readability and debugging (e.g., "WETH", "USDC")
	#[serde(skip_serializing_if = "Option::is_none")]
	pub destination_token_symbol: Option<String>,

	/// Optional route-specific metadata (fees, limits, execution time, etc.)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

impl AssetRoute {
	/// Create a new asset route
	pub fn new(origin_asset: InteropAddress, destination_asset: InteropAddress) -> Self {
		Self {
			origin_asset,
			origin_token_symbol: None,
			destination_asset,
			destination_token_symbol: None,
			metadata: None,
		}
	}

	/// Create a new asset route with metadata
	pub fn with_metadata(
		origin_asset: InteropAddress,
		destination_asset: InteropAddress,
		metadata: serde_json::Value,
	) -> Self {
		Self {
			origin_asset,
			origin_token_symbol: None,
			destination_asset,
			destination_token_symbol: None,
			metadata: Some(metadata),
		}
	}

	/// Create a new asset route with symbols
	pub fn with_symbols(
		origin_asset: InteropAddress,
		origin_symbol: String,
		destination_asset: InteropAddress,
		destination_symbol: String,
	) -> Self {
		Self {
			origin_asset,
			origin_token_symbol: Some(origin_symbol),
			destination_asset,
			destination_token_symbol: Some(destination_symbol),
			metadata: None,
		}
	}

	/// Builder method to add symbols to an existing route
	pub fn add_symbols(
		mut self,
		origin_symbol: Option<String>,
		destination_symbol: Option<String>,
	) -> Self {
		self.origin_token_symbol = origin_symbol;
		self.destination_token_symbol = destination_symbol;
		self
	}

	/// Builder method to add metadata to an existing route
	pub fn add_metadata(mut self, metadata: Option<serde_json::Value>) -> Self {
		self.metadata = metadata;
		self
	}

	/// Create asset route from plain chain IDs and addresses
	pub fn from_chain_and_addresses(
		origin_chain_id: u64,
		origin_address: String,
		destination_chain_id: u64,
		destination_address: String,
	) -> Result<Self, crate::quotes::errors::QuoteValidationError> {
		let origin_asset =
			InteropAddress::from_chain_and_address(origin_chain_id, &origin_address)?;
		let destination_asset =
			InteropAddress::from_chain_and_address(destination_chain_id, &destination_address)?;

		Ok(Self::new(origin_asset, destination_asset))
	}

	/// Create a new asset route with symbols and metadata
	pub fn with_symbols_and_metadata(
		origin_asset: InteropAddress,
		origin_symbol: String,
		destination_asset: InteropAddress,
		destination_symbol: String,
		metadata: serde_json::Value,
	) -> Self {
		Self {
			origin_asset,
			origin_token_symbol: Some(origin_symbol),
			destination_asset,
			destination_token_symbol: Some(destination_symbol),
			metadata: Some(metadata),
		}
	}

	/// Add origin token symbol
	pub fn with_origin_symbol(mut self, symbol: String) -> Self {
		self.origin_token_symbol = Some(symbol);
		self
	}

	/// Add destination token symbol
	pub fn with_destination_symbol(mut self, symbol: String) -> Self {
		self.destination_token_symbol = Some(symbol);
		self
	}

	/// Check if this route matches the given origin and destination assets
	pub fn matches(&self, origin: &InteropAddress, destination: &InteropAddress) -> bool {
		self.origin_asset == *origin && self.destination_asset == *destination
	}

	/// Check if this route involves the same chain (same-chain swap)
	pub fn is_same_chain(&self) -> bool {
		self.origin_asset.extract_chain_id().ok() == self.destination_asset.extract_chain_id().ok()
	}

	/// Check if this route is cross-chain
	pub fn is_cross_chain(&self) -> bool {
		!self.is_same_chain()
	}

	/// Get the origin chain ID
	pub fn origin_chain_id(&self) -> Result<u64, String> {
		self.origin_asset
			.extract_chain_id()
			.map_err(|e| e.to_string())
	}

	/// Get the destination chain ID  
	pub fn destination_chain_id(&self) -> Result<u64, String> {
		self.destination_asset
			.extract_chain_id()
			.map_err(|e| e.to_string())
	}

	/// Get origin token address
	pub fn origin_address(&self) -> String {
		self.origin_asset.extract_address()
	}

	/// Get destination token address
	pub fn destination_address(&self) -> String {
		self.destination_asset.extract_address()
	}
}

/// API response format for asset routes with separate chain ID and address fields
///
/// This provides a more readable format for API consumers compared to the
/// internal InteropAddress format used in the domain model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "originChainId": 1,
    "originTokenAddress": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    "originTokenSymbol": "WETH",
    "destinationChainId": 10,
    "destinationTokenAddress": "0x4200000000000000000000000000000000000006",
    "destinationTokenSymbol": "WETH",
    "metadata": {
        "isNative": false,
        "source": "across-api",
        "estimatedFee": "0.001",
        "estimatedTime": 120
    }
})))]
#[serde(rename_all = "camelCase")]
pub struct AssetRouteResponse {
	/// Origin chain ID
	pub origin_chain_id: u64,

	/// Origin token contract address
	pub origin_token_address: String,

	/// Optional origin token symbol for readability (e.g., "WETH", "USDC")
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin_token_symbol: Option<String>,

	/// Destination chain ID
	pub destination_chain_id: u64,

	/// Destination token contract address
	pub destination_token_address: String,

	/// Optional destination token symbol for readability (e.g., "WETH", "USDC")
	#[serde(skip_serializing_if = "Option::is_none")]
	pub destination_token_symbol: Option<String>,

	/// Optional route-specific metadata (fees, limits, execution time, etc.)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

/// Convert from domain AssetRoute to API AssetRouteResponse
impl TryFrom<&AssetRoute> for AssetRouteResponse {
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

/// Convert from domain AssetRoute to API AssetRouteResponse (owned)
impl TryFrom<AssetRoute> for AssetRouteResponse {
	type Error = String;

	fn try_from(route: AssetRoute) -> Result<Self, Self::Error> {
		AssetRouteResponse::try_from(&route)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::InteropAddress;

	fn create_test_route() -> AssetRoute {
		let origin =
			InteropAddress::from_chain_and_address(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap(); // WETH on Ethereum
		let destination = InteropAddress::from_chain_and_address(
			10,
			"0x4200000000000000000000000000000000000006",
		)
		.unwrap(); // WETH on Optimism
		AssetRoute::new(origin, destination)
	}

	#[test]
	fn test_asset_route_creation() {
		let route = create_test_route();

		assert_eq!(route.origin_chain_id().unwrap(), 1);
		assert_eq!(route.destination_chain_id().unwrap(), 10);
		assert!(route.is_cross_chain());
		assert!(!route.is_same_chain());
		assert!(route.metadata.is_none());
	}

	#[test]
	fn test_asset_route_with_metadata() {
		let origin =
			InteropAddress::from_chain_and_address(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap();
		let destination = InteropAddress::from_chain_and_address(
			10,
			"0x4200000000000000000000000000000000000006",
		)
		.unwrap();
		let metadata = serde_json::json!({
			"min_amount": "1000000000000000000",
			"max_amount": "100000000000000000000",
			"estimated_fee": "50000000000000000"
		});

		let route = AssetRoute::with_metadata(origin, destination, metadata.clone());

		assert!(route.metadata.is_some());
		assert_eq!(route.metadata.unwrap(), metadata);
		assert!(route.origin_token_symbol.is_none());
		assert!(route.destination_token_symbol.is_none());
	}

	#[test]
	fn test_asset_route_with_symbols() {
		let origin =
			InteropAddress::from_chain_and_address(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap();
		let destination = InteropAddress::from_chain_and_address(
			10,
			"0x4200000000000000000000000000000000000006",
		)
		.unwrap();

		let route =
			AssetRoute::with_symbols(origin, "WETH".to_string(), destination, "WETH".to_string());

		assert_eq!(route.origin_token_symbol, Some("WETH".to_string()));
		assert_eq!(route.destination_token_symbol, Some("WETH".to_string()));
		assert!(route.metadata.is_none());
	}

	#[test]
	fn test_asset_route_builder_pattern() {
		let origin =
			InteropAddress::from_chain_and_address(1, "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();
		let destination = InteropAddress::from_chain_and_address(
			137,
			"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
		)
		.unwrap();

		let route = AssetRoute::new(origin, destination)
			.with_origin_symbol("USDC".to_string())
			.with_destination_symbol("USDC".to_string());

		assert_eq!(route.origin_token_symbol, Some("USDC".to_string()));
		assert_eq!(route.destination_token_symbol, Some("USDC".to_string()));
		assert!(route.is_cross_chain());
	}

	#[test]
	fn test_route_matching() {
		let route = create_test_route();
		let origin =
			InteropAddress::from_chain_and_address(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap();
		let destination = InteropAddress::from_chain_and_address(
			10,
			"0x4200000000000000000000000000000000000006",
		)
		.unwrap();
		let wrong_destination = InteropAddress::from_chain_and_address(
			137,
			"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
		)
		.unwrap();

		assert!(route.matches(&origin, &destination));
		assert!(!route.matches(&origin, &wrong_destination));
	}

	#[test]
	fn test_same_chain_detection() {
		let origin =
			InteropAddress::from_chain_and_address(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap(); // WETH
		let destination =
			InteropAddress::from_chain_and_address(1, "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap(); // USDC
		let same_chain_route = AssetRoute::new(origin, destination);

		assert!(same_chain_route.is_same_chain());
		assert!(!same_chain_route.is_cross_chain());
	}

	#[test]
	fn test_asset_route_response_conversion() {
		let origin =
			InteropAddress::from_chain_and_address(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap();
		let destination = InteropAddress::from_chain_and_address(
			137,
			"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
		)
		.unwrap();

		let metadata = serde_json::json!({
			"isNative": false,
			"source": "config",
			"minAmount": "1000000"
		});

		let route = AssetRoute::with_symbols_and_metadata(
			origin,
			"WETH".to_string(),
			destination,
			"USDC".to_string(),
			metadata.clone(),
		);

		let response = AssetRouteResponse::try_from(&route).unwrap();

		// Verify the readable format
		assert_eq!(response.origin_chain_id, 1);
		assert_eq!(
			response.origin_token_address.to_lowercase(),
			"0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
		);
		assert_eq!(response.origin_token_symbol, Some("WETH".to_string()));
		assert_eq!(response.destination_chain_id, 137);
		assert_eq!(
			response.destination_token_address.to_lowercase(),
			"0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
		);
		assert_eq!(response.destination_token_symbol, Some("USDC".to_string()));
		assert_eq!(response.metadata, Some(metadata));
	}
}
