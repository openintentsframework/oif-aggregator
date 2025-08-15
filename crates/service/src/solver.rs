//! Solver service
//!
//! Service for retrieving solvers.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::solvers::Solver;
use oif_types::SolverRuntimeConfig;
use serde::Serialize;
use thiserror::Error;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Trait for solver service operations
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait SolverServiceTrait: Send + Sync {
	/// List all solvers
	async fn list_solvers(&self) -> Result<Vec<Solver>, SolverServiceError>;

	/// List solvers with pagination, returns (page_items, total_count, active_count, healthy_count)
	async fn list_solvers_paginated(
		&self,
		page: Option<u32>,
		page_size: Option<u32>,
	) -> Result<(Vec<Solver>, usize, usize, usize), SolverServiceError>;

	/// Get solver by ID - returns None if not found (not an error)
	async fn get_solver(&self, solver_id: &str) -> Result<Option<Solver>, SolverServiceError>;

	/// Perform health checks on all registered solvers
	async fn health_check_all(&self) -> Result<HashMap<String, bool>, SolverServiceError>;

	/// Get comprehensive solver statistics including health status
	async fn get_stats(&self) -> Result<SolverStats, SolverServiceError>;
}

#[derive(Debug, Error)]
pub enum SolverServiceError {
	#[error("storage error: {0}")]
	Storage(String),
	#[error("not found: {0}")]
	NotFound(String),
}

/// Solver statistics for health checks and monitoring
#[derive(Debug, Serialize, Clone)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SolverStats {
	pub total: usize,
	pub active: usize,
	pub inactive: usize,
	pub healthy: usize,
	pub unhealthy: usize,
	pub health_details: HashMap<String, bool>,
}

#[derive(Clone)]
pub struct SolverService {
	storage: Arc<dyn Storage>,
	adapter_registry: Arc<AdapterRegistry>,
}

impl SolverService {
	pub fn new(storage: Arc<dyn Storage>, adapter_registry: Arc<AdapterRegistry>) -> Self {
		Self {
			storage,
			adapter_registry,
		}
	}

	pub async fn list_solvers(&self) -> Result<Vec<Solver>, SolverServiceError> {
		self.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))
	}

	/// List solvers with pagination, and return (page_items, total_count, active_count, healthy_count)
	pub async fn list_solvers_paginated(
		&self,
		page: Option<u32>,
		page_size: Option<u32>,
	) -> Result<(Vec<Solver>, usize, usize, usize), SolverServiceError> {
		let total = self
			.storage
			.count_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Clamp paging parameters and compute offset
		let effective_page_size = page_size.unwrap_or(25).clamp(1, 100);
		let effective_page = page.unwrap_or(1).max(1);
		let start = (effective_page as usize - 1).saturating_mul(effective_page_size as usize);

		let page_items = self
			.storage
			.list_solvers_paginated(start, effective_page_size as usize)
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Active count via convenient method
		let active_count = self
			.storage
			.get_active_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?
			.len();

		// Healthy count across all
		let all = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;
		let healthy_count = all.iter().filter(|s| s.is_healthy()).count();

		Ok((page_items, total, active_count, healthy_count))
	}
}

#[async_trait]
impl SolverServiceTrait for SolverService {
	async fn list_solvers(&self) -> Result<Vec<Solver>, SolverServiceError> {
		self.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))
	}

	async fn list_solvers_paginated(
		&self,
		page: Option<u32>,
		page_size: Option<u32>,
	) -> Result<(Vec<Solver>, usize, usize, usize), SolverServiceError> {
		let total = self
			.storage
			.count_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Clamp paging parameters and compute offset
		let effective_page_size = page_size.unwrap_or(25).clamp(1, 100);
		let effective_page = page.unwrap_or(1).max(1);
		let start = (effective_page as usize - 1).saturating_mul(effective_page_size as usize);

		let page_items = self
			.storage
			.list_solvers_paginated(start, effective_page_size as usize)
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Active count via convenient method
		let active_count = self
			.storage
			.get_active_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?
			.len();

		// Healthy count across all
		let all = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;
		let healthy_count = all.iter().filter(|s| s.is_healthy()).count();

		Ok((page_items, total, active_count, healthy_count))
	}

	/// Fixed to return Option<Solver> instead of treating "not found" as an error
	async fn get_solver(&self, solver_id: &str) -> Result<Option<Solver>, SolverServiceError> {
		self.storage
			.get_solver(solver_id)
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))
	}

	async fn health_check_all(&self) -> Result<HashMap<String, bool>, SolverServiceError> {
		let mut results = HashMap::new();

		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		for solver in &solvers {
			if let Some(adapter) = self.adapter_registry.get(&solver.adapter_id) {
				let config = SolverRuntimeConfig::from(solver);
				match adapter.health_check(&config).await {
					Ok(is_healthy) => {
						results.insert(solver.solver_id.clone(), is_healthy);
					},
					Err(_) => {
						results.insert(solver.solver_id.clone(), false);
					},
				}
			} else {
				results.insert(solver.solver_id.clone(), false);
			}
		}

		Ok(results)
	}

	async fn get_stats(&self) -> Result<SolverStats, SolverServiceError> {
		// Get all solvers
		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		let total = solvers.len();

		// Get active solvers
		let active_solvers = self
			.storage
			.get_active_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		let active = active_solvers.len();
		let inactive = total.saturating_sub(active);

		// Perform health checks
		let health_details = self.health_check_all().await?;
		let healthy = health_details
			.values()
			.filter(|&&is_healthy| is_healthy)
			.count();
		let unhealthy = total.saturating_sub(healthy);

		Ok(SolverStats {
			total,
			active,
			inactive,
			healthy,
			unhealthy,
			health_details,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_mock_solver_service_trait() {
		let mut mock = MockSolverServiceTrait::new();

		// Setup simple expectations to verify the mock trait works
		mock.expect_list_solvers().returning(|| Ok(vec![]));

		mock.expect_get_solver().returning(|_| Ok(None));

		mock.expect_get_stats().returning(|| {
			Ok(SolverStats {
				total: 0,
				active: 0,
				inactive: 0,
				healthy: 0,
				unhealthy: 0,
				health_details: HashMap::new(),
			})
		});

		// Test the mock methods work as expected
		let solvers = mock.list_solvers().await.unwrap();
		assert_eq!(solvers.len(), 0);

		let solver = mock.get_solver("test-solver").await.unwrap();
		assert!(solver.is_none());

		let stats = mock.get_stats().await.unwrap();
		assert_eq!(stats.total, 0);
	}
}
