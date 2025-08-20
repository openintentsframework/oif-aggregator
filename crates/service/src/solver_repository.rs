//! Solver service
//!
//! Service for retrieving solvers.

use std::collections::HashMap;
use std::sync::Arc;

use crate::solver_adapter::SolverAdapterTrait;
use async_trait::async_trait;
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::solvers::Solver;
use oif_types::{SolverRuntimeConfig, SolverStatus};
use serde::Serialize;
use thiserror::Error;
use tracing::{debug, info, warn};
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

	/// Fetch and update supported assets and networks for a specific solver
	async fn fetch_and_update_assets(&self, solver_id: &str) -> Result<(), SolverServiceError>;

	/// Perform health check on a specific solver
	async fn health_check_solver(&self, solver_id: &str) -> Result<bool, SolverServiceError>;

	/// Perform health checks on all solvers (bulk operation)
	async fn health_check_all_solvers(&self) -> Result<(), SolverServiceError>;

	/// Fetch and update assets for all solvers that need refreshing (bulk operation)
	async fn fetch_assets_all_solvers(&self) -> Result<(), SolverServiceError>;
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

	async fn fetch_and_update_assets(&self, solver_id: &str) -> Result<(), SolverServiceError> {
		use crate::solver_adapter::SolverAdapterService;
		use chrono::Utc;

		// Get solver from storage
		let solver = self
			.storage
			.get_solver(solver_id)
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?
			.ok_or_else(|| {
				SolverServiceError::NotFound(format!("Solver '{}' not found", solver_id))
			})?;

		// Clone solver before moving it to adapter service
		let mut updated_solver = solver.clone();

		// Create adapter service for the solver
		let adapter_service =
			SolverAdapterService::from_solver(solver, Arc::clone(&self.adapter_registry))
				.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Call standard adapter methods to get data
		let assets_result = adapter_service.get_supported_assets().await;
		let networks_result = adapter_service.get_supported_networks().await;

		let mut has_updates = false;

		// Business logic: Handle assets result and update solver
		match assets_result {
			Ok(assets) => {
				info!("Fetched {} assets for solver: {}", assets.len(), solver_id);
				updated_solver.metadata.supported_assets = assets;
				has_updates = true;
			},
			Err(e) => {
				warn!("Failed to fetch assets for solver {}: {}", solver_id, e);
				// Continue with networks even if assets failed
			},
		}

		// Business logic: Handle networks result (log for now)
		match networks_result {
			Ok(networks) => {
				info!(
					"Fetched {} networks for solver: {}",
					networks.len(),
					solver_id
				);
				debug!("Networks for solver {}: {:?}", solver_id, networks);
				// Future: Store networks in solver metadata if needed
			},
			Err(e) => {
				warn!("Failed to fetch networks for solver {}: {}", solver_id, e);
			},
		}

		// Business logic: Update solver in storage if we have updates
		if has_updates {
			updated_solver.last_seen = Some(Utc::now());
			self.storage
				.update_solver(updated_solver)
				.await
				.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

			info!("Updated solver metadata for: {}", solver_id);
		}

		Ok(())
	}

	async fn health_check_solver(&self, solver_id: &str) -> Result<bool, SolverServiceError> {
		use crate::solver_adapter::SolverAdapterService;
		use chrono::Utc;

		// Get solver from storage
		let solver = self
			.storage
			.get_solver(solver_id)
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?
			.ok_or_else(|| {
				SolverServiceError::NotFound(format!("Solver '{}' not found", solver_id))
			})?;

		// Clone solver before moving it to adapter service
		let mut updated_solver = solver.clone();

		// Create adapter service for the solver
		let adapter_service =
			SolverAdapterService::from_solver(solver, Arc::clone(&self.adapter_registry))
				.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Call standard adapter method to check health
		let is_healthy = adapter_service
			.health_check()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Business logic: Update solver status and metrics based on health check result
		let new_status = if is_healthy {
			info!("Health check passed for solver: {}", solver_id);
			SolverStatus::Active
		} else {
			warn!("Health check failed for solver: {}", solver_id);
			SolverStatus::Error
		};

		// Business logic: Update solver in storage with new status and metrics
		updated_solver.status = new_status;
		updated_solver.last_seen = Some(Utc::now());

		if is_healthy {
			updated_solver.metrics.successful_requests += 1;
		} else {
			updated_solver.metrics.failed_requests += 1;
			updated_solver.metrics.consecutive_failures += 1;
		}

		self.storage
			.update_solver(updated_solver)
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		debug!("Health check completed for solver: {}", solver_id);

		Ok(is_healthy)
	}

	async fn health_check_all_solvers(&self) -> Result<(), SolverServiceError> {
		info!("Starting health checks for all solvers");

		// Get all solvers from storage
		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		let mut success_count = 0;
		let mut error_count = 0;

		// Filter solvers that should have health checks (active/inactive)
		let checkable_solvers: Vec<_> = solvers
			.into_iter()
			.filter(|solver| matches!(solver.status, SolverStatus::Active | SolverStatus::Inactive))
			.collect();

		info!(
			"Found {} solvers eligible for health checks",
			checkable_solvers.len()
		);

		// Perform health check on each solver
		for solver in checkable_solvers {
			match self.health_check_solver(&solver.solver_id).await {
				Ok(is_healthy) => {
					if is_healthy {
						success_count += 1;
						debug!("Health check passed for solver: {}", solver.solver_id);
					} else {
						error_count += 1;
						warn!("Health check failed for solver: {}", solver.solver_id);
					}
				},
				Err(e) => {
					error_count += 1;
					warn!("Health check error for solver {}: {}", solver.solver_id, e);
				},
			}
		}

		info!(
			"Health check completed - {} successful, {} failed",
			success_count, error_count
		);

		Ok(())
	}

	async fn fetch_assets_all_solvers(&self) -> Result<(), SolverServiceError> {
		info!("Starting asset fetch for all solvers");

		// Get all solvers from storage
		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		let mut success_count = 0;
		let mut error_count = 0;

		// Fetch assets for each solver that needs refresh
		for solver in solvers {
			match self.fetch_and_update_assets(&solver.solver_id).await {
				Ok(()) => {
					success_count += 1;
					debug!("Asset fetch completed for solver: {}", solver.solver_id);
				},
				Err(e) => {
					error_count += 1;
					warn!("Asset fetch failed for solver {}: {}", solver.solver_id, e);
				},
			}
		}

		info!(
			"Asset fetch completed - {} successful, {} failed",
			success_count, error_count
		);

		Ok(())
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
