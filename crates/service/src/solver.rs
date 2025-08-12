//! Solver service
//!
//! Service for retrieving solvers.

use std::sync::Arc;

use oif_storage::Storage;
use oif_types::solvers::Solver;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolverServiceError {
	#[error("storage error: {0}")]
	Storage(String),
	#[error("not found: {0}")]
	NotFound(String),
}

#[derive(Clone)]
pub struct SolverService {
	storage: Arc<dyn Storage>,
}

impl SolverService {
	pub fn new(storage: Arc<dyn Storage>) -> Self {
		Self { storage }
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
}
