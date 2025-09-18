//! Solver response models for API layer

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
#[allow(unused_imports)]
use serde_json::json;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use super::{Solver, SolverStatus};
use crate::models::{Asset, AssetRouteResponse};

/// Asset response format for API with separate address and chain ID fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "address": "0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86",
    "chainId": 1,
    "symbol": "USDC", 
    "name": "USD Coin",
    "decimals": 6
})))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetResponse {
	/// Token contract address
	pub address: String,
	/// Chain ID where the token exists
	pub chain_id: u64,
	/// Token symbol (e.g., "USDC", "ETH", "WBTC")
	pub symbol: String,
	/// Human-readable token name (e.g., "USD Coin", "Ethereum")
	pub name: String,
	/// Number of decimal places
	pub decimals: u8,
}

impl TryFrom<Asset> for AssetResponse {
	type Error = crate::quotes::errors::QuoteValidationError;

	fn try_from(asset: Asset) -> Result<Self, Self::Error> {
		Ok(Self {
			address: asset.plain_address(),
			chain_id: asset.chain_id()?,
			symbol: asset.symbol,
			name: asset.name,
			decimals: asset.decimals,
		})
	}
}

impl TryFrom<AssetResponse> for Asset {
	type Error = crate::quotes::errors::QuoteValidationError;

	fn try_from(response: AssetResponse) -> Result<Self, Self::Error> {
		Asset::from_chain_and_address(
			response.chain_id,
			response.address,
			response.symbol,
			response.name,
			response.decimals,
		)
	}
}

/// Supported assets response format for API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SupportedAssetsResponse {
	/// Asset-based: supports any-to-any within asset list (including same-chain)
	#[serde(rename = "assets")]
	Assets {
		assets: Vec<AssetResponse>,
		source: String,
	},
	/// Route-based: supports specific origin->destination pairs
	#[serde(rename = "routes")]
	Routes {
		routes: Vec<AssetRouteResponse>,
		source: String,
	},
}

/// Response format for individual solvers in API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "solverId": "example-solver",
    "adapterId": "oif-adapter-v1",
    "name": "Example DeFi Solver",
    "description": "An example solver for cross-chain swaps",
    "endpoint": "https://api.example-solver.com",
    "status": "active",
    "supportedAssets": {
        "type": "assets",
        "assets": [
            {
                "address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                "chainId": 1,
                "symbol": "WETH",
                "name": "Wrapped Ether",
                "decimals": 18
            }
        ],
        "source": "autoDiscovered"
    },
    "createdAt": 1756400000,
    "lastSeen": 1756457492
})))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverResponse {
	pub solver_id: String,
	pub adapter_id: String,
	pub name: Option<String>,
	pub description: Option<String>,
	pub endpoint: String,
	pub status: SolverStatus,
	pub supported_assets: SupportedAssetsResponse,
	pub created_at: i64,
	pub last_seen: Option<i64>,
}

/// Collection of solvers response for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "solvers": [
        {
            "solverId": "example-solver",
            "adapterId": "oif-adapter-v1",
            "name": "Example DeFi Solver",
            "description": "An example solver for cross-chain swaps",
            "endpoint": "https://api.example-solver.com",
            "status": "active",
            "supportedAssets": {
                "type": "assets",
                "assets": [
                    {
                        "address": "0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86",
                        "chainId": 1,
                        "symbol": "USDC",
                        "name": "USD Coin",
                        "decimals": 6
                    }
                ],
                "source": "autoDiscovered"
            },
            "createdAt": 1756400000,
            "lastSeen": 1756457492
        },
        {
            "solverId": "uniswap-solver",
            "adapterId": "uniswap-adapter-v1",
            "name": "Uniswap V3 Solver",
            "description": "Uniswap V3 liquidity pools solver",
            "endpoint": "https://api.uniswap.solver.com",
            "status": "active",
            "supportedAssets": {
                "type": "routes",
                "routes": [
                    {
                        "originChainId": 1,
                        "originTokenAddress": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                        "originTokenSymbol": "WETH",
                        "destinationChainId": 137,
                        "destinationTokenAddress": "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619",
                        "destinationTokenSymbol": "WETH"
                    }
                ],
                "source": "config"
            },
            "createdAt": 1756400000,
            "lastSeen": 1756457490
        }
    ],
    "totalSolvers": 2
})))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolversResponse {
	pub solvers: Vec<SolverResponse>,
	pub total_solvers: usize,
}

/// Convert from domain Solver to API SolverResponse
impl TryFrom<Solver> for SolverResponse {
	type Error = crate::solvers::SolverError;

	fn try_from(solver: Solver) -> Result<Self, Self::Error> {
		SolverResponse::try_from(&solver)
	}
}

/// Convert from a reference to domain Solver to API SolverResponse
impl TryFrom<&Solver> for SolverResponse {
	type Error = crate::solvers::SolverError;

	fn try_from(solver: &Solver) -> Result<Self, Self::Error> {
		let supported_assets = match &solver.metadata.supported_assets {
			crate::solvers::SupportedAssets::Assets { assets, source } => {
				let source_str = match source {
					crate::solvers::AssetSource::Config => "config".to_string(),
					crate::solvers::AssetSource::AutoDiscovered => "autoDiscovered".to_string(),
				};
				let asset_responses: Result<Vec<AssetResponse>, _> = assets
					.iter()
					.map(|asset| AssetResponse::try_from(asset.clone()))
					.collect();

				let asset_responses = asset_responses.map_err(|e| {
					tracing::error!("Failed to convert Asset to AssetResponse: {}", e);
					crate::solvers::SolverError::Configuration(format!(
						"Invalid asset chain ID in supported_assets: {}",
						e
					))
				})?;
				SupportedAssetsResponse::Assets {
					assets: asset_responses,
					source: source_str,
				}
			},
			crate::solvers::SupportedAssets::Routes {
				routes: supported_routes,
				source,
			} => {
				let source_str = match source {
					crate::solvers::AssetSource::Config => "config".to_string(),
					crate::solvers::AssetSource::AutoDiscovered => "autoDiscovered".to_string(),
				};
				let routes: Result<Vec<AssetRouteResponse>, String> = supported_routes
					.iter()
					.map(AssetRouteResponse::try_from)
					.collect();

				let routes = routes.map_err(|e| {
					crate::solvers::SolverError::Configuration(format!(
						"Failed to convert routes to response format: {}",
						e
					))
				})?;

				SupportedAssetsResponse::Routes {
					routes,
					source: source_str,
				}
			},
		};

		Ok(Self {
			solver_id: solver.solver_id.clone(),
			adapter_id: solver.adapter_id.clone(),
			name: solver.metadata.name.clone(),
			description: solver.metadata.description.clone(),
			endpoint: solver.endpoint.clone(),
			status: solver.status.clone(),
			supported_assets,
			created_at: solver.created_at.timestamp(),
			last_seen: solver.last_seen.map(|dt| dt.timestamp()),
		})
	}
}
/// Convert from domain collection to API response
impl TryFrom<Vec<Solver>> for SolversResponse {
	type Error = crate::solvers::SolverError;

	fn try_from(solvers: Vec<Solver>) -> Result<Self, Self::Error> {
		let total = solvers.len();
		let responses: Result<Vec<_>, _> = solvers.iter().map(SolverResponse::try_from).collect();

		Ok(Self {
			solvers: responses?,
			total_solvers: total,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::models::{AssetRoute, InteropAddress};
	use crate::solvers::{Solver, SolverStatus};

	fn create_test_solver() -> Solver {
		Solver::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_name("Test Solver".to_string())
		.with_routes(vec![
			AssetRoute::with_symbols(
				InteropAddress::from_text("eip155:1:0x0000000000000000000000000000000000000000")
					.unwrap(), // ETH on Ethereum
				"ETH".to_string(),
				InteropAddress::from_text("eip155:137:0x0000000000000000000000000000000000000000")
					.unwrap(), // MATIC on Polygon
				"MATIC".to_string(),
			),
			AssetRoute::with_symbols(
				InteropAddress::from_text("eip155:137:0x0000000000000000000000000000000000000000")
					.unwrap(), // MATIC on Polygon
				"MATIC".to_string(),
				InteropAddress::from_text("eip155:1:0x0000000000000000000000000000000000000000")
					.unwrap(), // ETH on Ethereum
				"ETH".to_string(),
			),
		])
	}

	#[test]
	fn test_solver_response_from_domain() {
		let mut solver = create_test_solver();
		solver.update_status(SolverStatus::Active);

		let response = SolverResponse::try_from(&solver).unwrap();

		assert_eq!(response.solver_id, "test-solver");
		assert_eq!(response.name, Some("Test Solver".to_string()));
		assert!(matches!(response.status, SolverStatus::Active));

		// Test the new supported_assets structure
		match &response.supported_assets {
			SupportedAssetsResponse::Routes { routes, source } => {
				assert_eq!(routes.len(), 2);
				assert_eq!(source, "config"); // from with_routes (config source)
				assert!(routes.iter().any(|r| r.origin_chain_id == 1));
				assert!(routes.iter().any(|r| r.destination_chain_id == 137));
			},
			SupportedAssetsResponse::Assets { .. } => {
				panic!("Expected routes mode for test solver created with with_routes()");
			},
		}
	}

	#[test]
	fn test_solvers_response_creation() {
		let mut solver1 = create_test_solver();
		solver1.update_status(SolverStatus::Active);

		let mut solver2 = create_test_solver();
		solver2.solver_id = "solver-2".to_string();
		solver2.update_status(SolverStatus::Error);

		let solvers = vec![solver1, solver2];
		let response = SolversResponse::try_from(solvers).unwrap();

		assert_eq!(response.total_solvers, 2);
	}

	#[test]
	fn test_asset_response_conversion() {
		// Test conversion from Asset to AssetResponse
		let asset = Asset::from_chain_and_address(
			1,
			"0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
		)
		.unwrap();

		let asset_response = AssetResponse::try_from(asset).unwrap();

		assert_eq!(
			asset_response.address,
			"0xa0b86a33e6441e7c81f7c93451777f5f4de78e86"
		);
		assert_eq!(asset_response.chain_id, 1);
		assert_eq!(asset_response.symbol, "USDC");
		assert_eq!(asset_response.name, "USD Coin");
		assert_eq!(asset_response.decimals, 6);

		// Test reverse conversion
		let converted_asset = Asset::try_from(asset_response).unwrap();
		assert_eq!(converted_asset.chain_id().unwrap(), 1);
		assert_eq!(
			converted_asset.plain_address(),
			"0xa0b86a33e6441e7c81f7c93451777f5f4de78e86"
		);
		assert_eq!(converted_asset.symbol, "USDC");
		assert_eq!(converted_asset.name, "USD Coin");
		assert_eq!(converted_asset.decimals, 6);
	}

	#[test]
	fn test_asset_based_solver_response() {
		// Create a solver with assets instead of routes
		let assets = vec![
			Asset::from_chain_and_address(
				1,
				"0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86".to_string(),
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
			)
			.unwrap(),
			Asset::from_chain_and_address(
				10,
				"0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".to_string(),
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
			)
			.unwrap(),
		];

		let mut solver = Solver::new(
			"test-asset-solver".to_string(),
			"across-v1".to_string(),
			"https://api.across.to".to_string(),
		)
		.with_assets(assets);

		solver.update_status(SolverStatus::Active);

		let response = SolverResponse::try_from(&solver).unwrap();

		// Verify that we get AssetResponse format
		match &response.supported_assets {
			SupportedAssetsResponse::Assets { assets, source } => {
				assert_eq!(assets.len(), 2);
				assert_eq!(source, "config");

				// Check first asset
				let asset1 = &assets[0];
				assert_eq!(asset1.address, "0xa0b86a33e6441e7c81f7c93451777f5f4de78e86");
				assert_eq!(asset1.chain_id, 1);
				assert_eq!(asset1.symbol, "USDC");

				// Check second asset
				let asset2 = &assets[1];
				assert_eq!(asset2.address, "0x0b2c639c533813f4aa9d7837caf62653d097ff85");
				assert_eq!(asset2.chain_id, 10);
				assert_eq!(asset2.symbol, "USDC");
			},
			SupportedAssetsResponse::Routes { .. } => {
				panic!("Expected assets mode for solver created with with_assets()");
			},
		}
	}
}
