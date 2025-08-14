//! Solver service
//!
//! Service for retrieving solvers.

use std::collections::HashMap;
use std::sync::Arc;

use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::solvers::Solver;
use oif_types::SolverRuntimeConfig;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolverServiceError {
	#[error("storage error: {0}")]
	Storage(String),
	#[error("not found: {0}")]
	NotFound(String),
}

/// Solver statistics for health checks and monitoring
#[derive(Debug, Serialize, Clone)]
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

	pub async fn get_solver(&self, solver_id: &str) -> Result<Solver, SolverServiceError> {
		match self
			.storage
			.get_solver(solver_id)
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?
		{
			Some(s) => Ok(s),
			None => Err(SolverServiceError::NotFound(solver_id.to_string())),
		}
	}

	/// Perform health checks on all registered solvers using their adapters
	pub async fn health_check_all(&self) -> Result<HashMap<String, bool>, SolverServiceError> {
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

	/// Get comprehensive solver statistics including health status
	pub async fn get_stats(&self) -> Result<SolverStats, SolverServiceError> {
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
