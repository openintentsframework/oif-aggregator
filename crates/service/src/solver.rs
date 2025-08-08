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
        self
            .storage
            .get_all_solvers()
            .await
            .map_err(|e| SolverServiceError::Storage(e.to_string()))
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


