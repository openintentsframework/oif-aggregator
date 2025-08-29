//! Solver response models for API layer

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use serde_json::json;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use super::{Solver, SolverStatus};
use crate::models::Asset as AssetResponse;

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
    "supportedAssets": [
        {
            "address": "0x01000002147a695FbDB2315678afecb367f032d93F642f64180aa3",
            "symbol": "USDC",
            "name": "USD Coin",
            "decimals": 6,
            "chainId": 1
        },
        {
            "address": "0x01000002147a69C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
            "symbol": "WETH",
            "name": "Wrapped Ether",
            "decimals": 18,
            "chainId": 1
        }
    ],
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
	pub supported_assets: Vec<AssetResponse>,
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
            "supportedAssets": [
                {
                    "address": "0x01000002147a695FbDB2315678afecb367f032d93F642f64180aa3",
                    "symbol": "USDC",
                    "name": "USD Coin",
                    "decimals": 6,
                    "chainId": 1
                }
            ],
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
            "supportedAssets": [
                {
                    "address": "0x01000002147a69C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                    "symbol": "WETH",
                    "name": "Wrapped Ether",
                    "decimals": 18,
                    "chainId": 1
                }
            ],
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
		Ok(Self {
			solver_id: solver.solver_id.clone(),
			adapter_id: solver.adapter_id.clone(),
			name: solver.metadata.name.clone(),
			description: solver.metadata.description.clone(),
			endpoint: solver.endpoint.clone(),
			status: solver.status.clone(),
			supported_assets: solver.metadata.supported_assets.clone(),
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
	use crate::solvers::{Solver, SolverStatus};

	fn create_test_solver() -> Solver {
		use crate::models::Asset;

		Solver::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
		)
		.with_name("Test Solver".to_string())
		.with_assets(vec![
			Asset::new(
				"0x0000000000000000000000000000000000000000".to_string(),
				"ETH".to_string(),
				"Ethereum".to_string(),
				18,
				1,
			),
			Asset::new(
				"0x0000000000000000000000000000000000000000".to_string(),
				"MATIC".to_string(),
				"Polygon".to_string(),
				18,
				137,
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
		assert_eq!(response.supported_assets.len(), 2);
		assert!(response.supported_assets.iter().any(|a| a.chain_id == 1));
		assert!(response.supported_assets.iter().any(|a| a.chain_id == 137));
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
}
