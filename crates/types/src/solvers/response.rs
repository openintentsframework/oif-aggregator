//! Solver response models for API layer

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{HealthCheckResult, Solver, SolverResult, SolverStatus};
use crate::adapters::{AssetResponse, NetworkResponse};

/// Response format for individual solvers in API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverResponse {
	pub solver_id: String,
	pub adapter_id: String,
	pub name: Option<String>,
	pub description: Option<String>,
	pub endpoint: String,
	pub status: SolverStatusResponse,
	pub supported_networks: Vec<NetworkResponse>,
	pub supported_assets: Vec<AssetResponse>,
	pub metrics: SolverMetricsResponse,
	pub created_at: i64,
	pub last_seen: Option<i64>,
}

/// Solver status for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SolverStatusResponse {
	Active,
	Inactive,
	Error,
	Maintenance,
	Initializing,
}

/// Solver metrics for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverMetricsResponse {
	pub avg_response_time_ms: f64,
	pub success_rate: f64,
	pub total_requests: u64,
	pub successful_requests: u64,
	pub failed_requests: u64,
	pub timeout_requests: u64,
	pub consecutive_failures: u32,
	pub is_healthy: bool,
	pub priority_score: f64,
	pub last_health_check: Option<HealthCheckResponse>,
	pub last_updated: i64,
}

/// Health check result for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
	pub is_healthy: bool,
	pub response_time_ms: u64,
	pub error_message: Option<String>,
	pub last_check: i64,
	pub consecutive_failures: u32,
}

/// Collection of solvers response for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolversResponse {
	pub solvers: Vec<SolverResponse>,
	pub total_solvers: usize,
	pub timestamp: i64,
}

/// Solver statistics for admin/monitoring endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverStatsResponse {
	pub solver_id: String,
	pub status: SolverStatusResponse,
	pub uptime_percentage: f64,
	pub avg_response_time_ms: f64,
	pub requests_per_minute: f64,
	pub error_rate: f64,
	pub last_24h_requests: u64,
	pub last_24h_successes: u64,
	pub last_24h_failures: u64,
	pub peak_response_time_ms: u64,
	pub min_response_time_ms: u64,
}

impl SolverResponse {
	/// Create a solver response from domain solver
	pub fn from_domain(solver: &Solver) -> SolverResult<Self> {
		Ok(Self {
			solver_id: solver.solver_id.clone(),
			adapter_id: solver.adapter_id.clone(),
			name: solver.metadata.name.clone(),
			description: solver.metadata.description.clone(),
			endpoint: solver.endpoint.clone(),
			status: SolverStatusResponse::from_domain(&solver.status),
			supported_networks: solver
				.metadata
				.supported_networks
				.iter()
				.map(NetworkResponse::from_domain)
				.collect(),
			supported_assets: solver
				.metadata
				.supported_assets
				.iter()
				.map(AssetResponse::from_domain)
				.collect(),
			metrics: SolverMetricsResponse::from_domain(solver),
			created_at: solver.created_at.timestamp(),
			last_seen: solver.last_seen.map(|dt| dt.timestamp()),
		})
	}

	/// Create a minimal solver response (for public API)
	pub fn minimal_from_domain(solver: &Solver) -> SolverResult<Self> {
		Ok(Self {
			solver_id: solver.solver_id.clone(),
			adapter_id: solver.adapter_id.clone(),
			name: solver.metadata.name.clone(),
			description: solver.metadata.description.clone(),
			endpoint: "".to_string(), // Hide endpoint in public API
			status: SolverStatusResponse::from_domain(&solver.status),
			supported_networks: solver
				.metadata
				.supported_networks
				.iter()
				.map(NetworkResponse::from_domain)
				.collect(),
			supported_assets: solver
				.metadata
				.supported_assets
				.iter()
				.map(AssetResponse::from_domain)
				.collect(),
			metrics: SolverMetricsResponse::minimal_from_domain(solver),
			created_at: solver.created_at.timestamp(),
			last_seen: None, // Hide last_seen in public API
		})
	}
}

impl SolverStatusResponse {
	fn from_domain(status: &SolverStatus) -> Self {
		match status {
			SolverStatus::Active => Self::Active,
			SolverStatus::Inactive => Self::Inactive,
			SolverStatus::Error => Self::Error,
			SolverStatus::Maintenance => Self::Maintenance,
			SolverStatus::Initializing => Self::Initializing,
		}
	}
}

impl SolverMetricsResponse {
	fn from_domain(solver: &Solver) -> Self {
		Self {
			avg_response_time_ms: solver.metrics.avg_response_time_ms,
			success_rate: solver.metrics.success_rate,
			total_requests: solver.metrics.total_requests,
			successful_requests: solver.metrics.successful_requests,
			failed_requests: solver.metrics.failed_requests,
			timeout_requests: solver.metrics.timeout_requests,
			consecutive_failures: solver.metrics.consecutive_failures,
			is_healthy: solver.is_healthy(),
			priority_score: solver.priority_score(),
			last_health_check: solver
				.metrics
				.last_health_check
				.as_ref()
				.map(HealthCheckResponse::from_domain),
			last_updated: solver.metrics.last_updated.timestamp(),
		}
	}

	fn minimal_from_domain(solver: &Solver) -> Self {
		Self {
			avg_response_time_ms: solver.metrics.avg_response_time_ms,
			success_rate: solver.metrics.success_rate,
			total_requests: 0,       // Hide in public API
			successful_requests: 0,  // Hide in public API
			failed_requests: 0,      // Hide in public API
			timeout_requests: 0,     // Hide in public API
			consecutive_failures: 0, // Hide in public API
			is_healthy: solver.is_healthy(),
			priority_score: solver.priority_score(),
			last_health_check: None, // Hide in public API
			last_updated: solver.metrics.last_updated.timestamp(),
		}
	}
}

impl HealthCheckResponse {
	fn from_domain(health_check: &HealthCheckResult) -> Self {
		Self {
			is_healthy: health_check.is_healthy,
			response_time_ms: health_check.response_time_ms,
			error_message: health_check.error_message.clone(),
			last_check: health_check.last_check.timestamp(),
			consecutive_failures: health_check.consecutive_failures,
		}
	}
}

impl SolversResponse {
	/// Create solvers response from domain solvers
	pub fn from_domain_solvers(solvers: Vec<Solver>) -> SolverResult<Self> {
		let solver_responses: Result<Vec<_>, _> =
			solvers.iter().map(SolverResponse::from_domain).collect();

		let responses = solver_responses?;
		Ok(Self {
			solvers: responses,
			total_solvers: solvers.len(),
			timestamp: Utc::now().timestamp(),
		})
	}

	/// Create minimal solvers response (for public API)
	pub fn minimal_from_domain_solvers(solvers: Vec<Solver>) -> SolverResult<Self> {
		let solver_responses: Result<Vec<_>, _> = solvers
			.iter()
			.map(SolverResponse::minimal_from_domain)
			.collect();

		let responses = solver_responses?;
		Ok(Self {
			solvers: responses,
			total_solvers: solvers.len(),
			timestamp: Utc::now().timestamp(),
		})
	}

	/// Filter solvers by status
	pub fn filter_by_status(&mut self, status: SolverStatusResponse) {
		self.solvers.retain(|solver| {
			std::mem::discriminant(&solver.status) == std::mem::discriminant(&status)
		});
		self.total_solvers = self.solvers.len();
	}

	/// Sort solvers by priority score
	pub fn sort_by_priority(&mut self) {
		self.solvers.sort_by(|a, b| {
			b.metrics
				.priority_score
				.partial_cmp(&a.metrics.priority_score)
				.unwrap_or(std::cmp::Ordering::Equal)
		});
	}

	/// Filter solvers by supported chain
	pub fn filter_by_chain(&mut self, chain_id: u64) {
		self.solvers.retain(|solver| {
			solver
				.supported_networks
				.iter()
				.any(|n| n.chain_id == chain_id)
		});
		self.total_solvers = self.solvers.len();
	}
}

// (removed legacy impl)

/// Convert from domain Solver to API SolverResponse
impl TryFrom<Solver> for SolverResponse {
	type Error = crate::solvers::SolverError;

	fn try_from(solver: Solver) -> Result<Self, Self::Error> {
		SolverResponse::from_domain(&solver)
	}
}

/// Convert from domain collection to API response
impl TryFrom<Vec<Solver>> for SolversResponse {
	type Error = crate::solvers::SolverError;

	fn try_from(solvers: Vec<Solver>) -> Result<Self, Self::Error> {
		SolversResponse::from_domain_solvers(solvers)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::solvers::{Solver, SolverStatus};

	fn create_test_solver() -> Solver {
		Solver::new(
			"test-solver".to_string(),
			"oif-v1".to_string(),
			"https://api.example.com".to_string(),
			2000,
		)
		.with_name("Test Solver".to_string())
		.with_chain_ids(vec![1, 137])
	}

	#[test]
	fn test_solver_response_from_domain() {
		let mut solver = create_test_solver();
		solver.update_status(SolverStatus::Active);

		let response = SolverResponse::from_domain(&solver).unwrap();

		assert_eq!(response.solver_id, "test-solver");
		assert_eq!(response.name, Some("Test Solver".to_string()));
		assert!(matches!(response.status, SolverStatusResponse::Active));
		assert_eq!(response.supported_networks.len(), 2);
		assert!(response.supported_networks.iter().any(|n| n.chain_id == 1));
		assert!(response
			.supported_networks
			.iter()
			.any(|n| n.chain_id == 137));
	}

	#[test]
	fn test_minimal_solver_response() {
		let solver = create_test_solver();
		let response = SolverResponse::minimal_from_domain(&solver).unwrap();

		assert_eq!(response.solver_id, "test-solver");
		assert_eq!(response.endpoint, ""); // Hidden in minimal response
		assert!(response.last_seen.is_none()); // Hidden in minimal response
		assert_eq!(response.metrics.total_requests, 0); // Hidden in minimal response
	}

	#[test]
	fn test_solvers_response_creation() {
		let mut solver1 = create_test_solver();
		solver1.update_status(SolverStatus::Active);

		let mut solver2 = create_test_solver();
		solver2.solver_id = "solver-2".to_string();
		solver2.update_status(SolverStatus::Error);

		let solvers = vec![solver1, solver2];
		let response = SolversResponse::from_domain_solvers(solvers).unwrap();

		assert_eq!(response.total_solvers, 2);
		assert!(response.timestamp > 0);
	}

	#[test]
	fn test_filter_by_status() {
		let mut solver1 = create_test_solver();
		solver1.update_status(SolverStatus::Active);

		let mut solver2 = create_test_solver();
		solver2.solver_id = "solver-2".to_string();
		solver2.update_status(SolverStatus::Error);

		let solvers = vec![solver1, solver2];
		let mut response = SolversResponse::from_domain_solvers(solvers).unwrap();

		response.filter_by_status(SolverStatusResponse::Active);
		assert_eq!(response.total_solvers, 1);
		assert_eq!(response.solvers[0].solver_id, "test-solver");
	}

	#[test]
	fn test_filter_by_chain() {
		let solver1 = create_test_solver(); // Supports chains [1, 137]

		let solver2 = create_test_solver().with_chain_ids(vec![56, 250]); // Different networks

		let solvers = vec![solver1, solver2];
		let mut response = SolversResponse::from_domain_solvers(solvers).unwrap();

		response.filter_by_chain(1);
		assert_eq!(response.total_solvers, 1);
		assert_eq!(response.solvers[0].supported_networks.len(), 2);
		assert!(response.solvers[0]
			.supported_networks
			.iter()
			.any(|n| n.chain_id == 1));
		assert!(response.solvers[0]
			.supported_networks
			.iter()
			.any(|n| n.chain_id == 137));
	}

	// (removed legacy SystemHealthResponse test)

	#[test]
	fn test_sort_by_priority() {
		let mut high_priority = create_test_solver();
		high_priority.update_status(SolverStatus::Active);
		high_priority.record_success(100); // Good performance

		let mut low_priority = create_test_solver();
		low_priority.solver_id = "low-priority".to_string();
		low_priority.update_status(SolverStatus::Active);
		low_priority.record_failure(false); // Poor performance

		let solvers = vec![low_priority, high_priority];
		let mut response = SolversResponse::from_domain_solvers(solvers).unwrap();

		response.sort_by_priority();
		assert_eq!(response.solvers[0].solver_id, "test-solver"); // High priority first
		assert_eq!(response.solvers[1].solver_id, "low-priority"); // Low priority second
	}
}
