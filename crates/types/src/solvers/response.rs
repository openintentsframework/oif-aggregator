//! Solver response models for API layer

use serde::{Deserialize, Serialize};

use super::{Solver, SolverStatus};
use crate::adapters::{AssetResponse, NetworkResponse};

/// Response format for individual solvers in API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverResponse {
	pub solver_id: String,
	pub adapter_id: String,
	pub name: Option<String>,
	pub description: Option<String>,
	pub endpoint: String,
	pub status: SolverStatus,
	pub supported_networks: Vec<NetworkResponse>,
	pub supported_assets: Vec<AssetResponse>,
	pub created_at: i64,
	pub last_seen: Option<i64>,
}

/// Collection of solvers response for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolversResponse {
	pub solvers: Vec<SolverResponse>,
	pub total_solvers: usize,
	pub timestamp: i64,
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
			supported_networks: solver
				.metadata
				.supported_networks
				.iter()
				.map(NetworkResponse::from)
				.collect(),
			supported_assets: solver
				.metadata
				.supported_assets
				.iter()
				.map(AssetResponse::from)
				.collect(),
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
			timestamp: chrono::Utc::now().timestamp(),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		solvers::{Solver, SolverStatus},
		Network,
	};

	fn create_test_solver() -> Solver {
		Solver::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
			2000,
		)
		.with_name("Test Solver".to_string())
		.with_networks(vec![
			Network::new(1, "Ethereum".to_string(), false),
			Network::new(137, "Polygon".to_string(), false),
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
		assert_eq!(response.supported_networks.len(), 2);
		assert!(response.supported_networks.iter().any(|n| n.chain_id == 1));
		assert!(response
			.supported_networks
			.iter()
			.any(|n| n.chain_id == 137));
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
		assert!(response.timestamp > 0);
	}
}
