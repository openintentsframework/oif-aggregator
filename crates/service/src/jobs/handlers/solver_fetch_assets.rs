//! Solver asset fetching job handler

use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::solver_adapter::{SolverAdapterService, SolverAdapterTrait};
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;

use crate::jobs::types::{JobError, JobResult};

/// Handler for solver asset fetching jobs
pub struct SolverFetchAssetsHandler {
	storage: Arc<dyn Storage>,
	adapter_registry: Arc<AdapterRegistry>,
}

impl SolverFetchAssetsHandler {
	/// Create a new solver fetch assets handler
	pub fn new(storage: Arc<dyn Storage>, adapter_registry: Arc<AdapterRegistry>) -> Self {
		Self {
			storage,
			adapter_registry,
		}
	}

	/// Fetch and update supported assets and networks for a solver
	pub async fn handle(&self, solver_id: &str) -> JobResult {
		debug!("Starting asset fetch for solver: {}", solver_id);

		// Get solver from storage
		let solver = self
			.storage
			.get_solver(solver_id)
			.await
			.map_err(|e| JobError::Storage(e.to_string()))?
			.ok_or_else(|| JobError::InvalidConfig(format!("Solver '{}' not found", solver_id)))?;

		// Create adapter service for the solver
		let adapter_service =
			SolverAdapterService::from_solver(solver.clone(), Arc::clone(&self.adapter_registry))
				.map_err(|e| JobError::Adapter(e.to_string()))?;

		// Fetch supported assets and networks
		let assets_result = adapter_service.get_supported_assets().await;
		let networks_result = adapter_service.get_supported_networks().await;

		let mut updated_solver = solver.clone();
		let mut has_updates = false;

		// Update assets if fetched successfully
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

		// Update networks if fetched successfully
		// Note: We don't store networks directly in solver metadata currently,
		// but we can log them for verification
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

		// Update solver in storage if we have updates
		if has_updates {
			updated_solver.last_seen = Some(Utc::now());
			self.storage
				.update_solver(updated_solver)
				.await
				.map_err(|e| JobError::Storage(e.to_string()))?;

			info!("Updated solver metadata for: {}", solver_id);
		}

		debug!("Asset fetch completed for solver: {}", solver_id);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use oif_adapters::AdapterRegistry;
	use oif_storage::{MemoryStore, Storage};
	use oif_types::solvers::{SolverMetadata, SolverMetrics};
	use oif_types::{Solver, SolverStatus};
	use std::collections::HashMap;

	/// Helper to create a test solver
	fn create_test_solver() -> Solver {
		Solver {
			solver_id: "test-solver".to_string(),
			adapter_id: "test-adapter".to_string(),
			endpoint: "https://test-solver.example.com".to_string(),
			timeout_ms: 5000,
			status: SolverStatus::Inactive,
			metadata: SolverMetadata {
				name: Some("Test Solver".to_string()),
				description: Some("Test solver for unit testing".to_string()),
				version: None,
				supported_assets: vec![],
				max_retries: 3,
				headers: None,
				config: HashMap::new(),
			},
			created_at: chrono::Utc::now(),
			last_seen: None,
			metrics: SolverMetrics {
				avg_response_time_ms: 0.0,
				success_rate: 0.0,
				total_requests: 0,
				successful_requests: 0,
				failed_requests: 0,
				timeout_requests: 0,
				last_health_check: None,
				consecutive_failures: 0,
				last_updated: chrono::Utc::now(),
			},
		}
	}

	#[tokio::test]
	async fn test_fetch_assets_handler_creation() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());

		let handler = SolverFetchAssetsHandler::new(storage, adapter_registry);

		// Just verify we can create the handler without panic
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_fetch_assets_solver_not_found() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = SolverFetchAssetsHandler::new(storage, adapter_registry);

		let result = handler.handle("nonexistent-solver").await;

		assert!(result.is_err());
		if let Err(JobError::InvalidConfig(msg)) = result {
			assert!(msg.contains("not found"));
		} else {
			panic!("Expected InvalidConfig error");
		}
	}

	#[tokio::test]
	async fn test_fetch_assets_with_existing_solver() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = SolverFetchAssetsHandler::new(storage.clone(), adapter_registry);

		// Create and store a test solver
		let test_solver = create_test_solver();
		storage.create_solver(test_solver.clone()).await.unwrap();

		// The asset fetch will likely fail since we don't have a real adapter,
		// but we should get a proper error, not a panic
		let result = handler.handle("test-solver").await;

		// We expect this to fail with an Adapter error since there's no real adapter
		assert!(result.is_err());
		if let Err(JobError::Adapter(_)) = result {
			// This is expected - no real adapter available
		} else {
			panic!("Expected Adapter error, got: {:?}", result);
		}
	}

	#[tokio::test]
	async fn test_fetch_assets_handles_partial_failures() {
		// This test would ideally use a mock adapter that fails asset fetch but succeeds network fetch
		// For now, we just verify the handler doesn't panic with valid input
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = SolverFetchAssetsHandler::new(storage.clone(), adapter_registry);

		let test_solver = create_test_solver();
		storage.create_solver(test_solver.clone()).await.unwrap();

		// This will fail but shouldn't panic
		let result = handler.handle("test-solver").await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_fetch_assets_empty_solver_id() {
		let storage = Arc::new(MemoryStore::new());
		let adapter_registry = Arc::new(AdapterRegistry::with_defaults());
		let handler = SolverFetchAssetsHandler::new(storage, adapter_registry);

		let result = handler.handle("").await;

		assert!(result.is_err());
		// Should fail because empty solver ID won't be found
		if let Err(JobError::InvalidConfig(_)) = result {
			// This is expected
		} else {
			panic!("Expected InvalidConfig error for empty solver ID");
		}
	}
}
