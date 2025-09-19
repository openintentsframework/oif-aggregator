//! Solver service
//!
//! Service for retrieving solvers.

use std::collections::HashMap;
use std::sync::Arc;

use crate::jobs::scheduler::JobScheduler;
use crate::solver_adapter::SolverAdapterService;
use crate::solver_adapter::SolverAdapterTrait;
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::models::health::SolverStats;
use oif_types::solvers::Solver;
use oif_types::solvers::{AssetSource, SupportedAssets};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use oif_types::{SolverRuntimeConfig, SolverStatus};

use thiserror::Error;
use tracing::{debug, info, warn};

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

	/// Fetch and update supported assets/routes for a specific solver
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

#[derive(Clone)]
pub struct SolverService {
	storage: Arc<dyn Storage>,
	adapter_registry: Arc<AdapterRegistry>,
	job_scheduler: Option<Arc<dyn JobScheduler>>,
}

impl SolverService {
	pub fn new(
		storage: Arc<dyn Storage>,
		adapter_registry: Arc<AdapterRegistry>,
		job_scheduler: Option<Arc<dyn JobScheduler>>,
	) -> Self {
		Self {
			storage,
			adapter_registry,
			job_scheduler,
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

	/// Update solver status and metrics in storage based on health check result
	async fn update_solver_status(
		storage: &Arc<dyn Storage>,
		solver: &Solver,
		is_healthy: bool,
	) -> Result<(), SolverServiceError> {
		use chrono::Utc;

		// Clone solver and update status and metrics
		let mut updated_solver = solver.clone();

		let new_status = if is_healthy {
			SolverStatus::Active
		} else {
			SolverStatus::Error
		};

		updated_solver.status = new_status;
		updated_solver.last_seen = Some(Utc::now());

		// Update metrics
		if is_healthy {
			updated_solver.metrics.successful_requests += 1;
		} else {
			updated_solver.metrics.failed_requests += 1;
			updated_solver.metrics.consecutive_failures += 1;
		}

		// Save to storage
		storage
			.update_solver(updated_solver)
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		Ok(())
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

		// Check if routes are from config (manual) or should be auto-discovered
		let (is_config_source, item_count) = match &solver.metadata.supported_assets {
			SupportedAssets::Assets { assets, source } => {
				(source == &AssetSource::Config, assets.len())
			},
			SupportedAssets::Routes { routes, source } => {
				(source == &AssetSource::Config, routes.len())
			},
		};

		if is_config_source {
			info!(
				"Solver '{}' has {} manually configured items, skipping auto-discovery",
				solver_id, item_count
			);
			return Ok(());
		}

		// Assets/routes should be auto-discovered, proceed with fetching
		info!(
			"Solver '{}' is configured for auto-discovery, fetching from API",
			solver_id
		);

		// Clone solver before moving it to adapter service
		let mut updated_solver = solver.clone();

		// Create adapter service for the solver
		let adapter_service = SolverAdapterService::from_solver(
			solver,
			Arc::clone(&self.adapter_registry),
			self.job_scheduler.clone(),
		)
		.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Call adapter method to get support data (adapter chooses mode)
		let support_result = adapter_service.get_supported_assets().await;

		let mut has_updates = false;

		// Business logic: Handle support result and update solver
		match support_result {
			Ok(support_data) => {
				// Convert adapter data to domain model with AutoDiscovered source
				let supported_assets = match support_data {
					oif_types::SupportedAssetsData::Assets(assets) => SupportedAssets::Assets {
						assets,
						source: AssetSource::AutoDiscovered,
					},
					oif_types::SupportedAssetsData::Routes(routes) => SupportedAssets::Routes {
						routes,
						source: AssetSource::AutoDiscovered,
					},
				};

				let item_count = match &supported_assets {
					SupportedAssets::Assets { assets, .. } => assets.len(),
					SupportedAssets::Routes { routes, .. } => routes.len(),
				};
				info!(
					"Auto-discovered {} items for solver: {}",
					item_count, solver_id
				);
				updated_solver.metadata.supported_assets = supported_assets;
				has_updates = true;
			},
			Err(e) => {
				warn!(
					"Failed to auto-discover assets/routes for solver {}: {}",
					solver_id, e
				);
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
		let adapter_service = SolverAdapterService::from_solver(
			solver,
			Arc::clone(&self.adapter_registry),
			self.job_scheduler.clone(),
		)
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

		// Filter solvers that should have health checks (active/inactive)
		let checkable_solvers: Vec<_> = solvers
			.into_iter()
			.filter(|solver| matches!(solver.status, SolverStatus::Active | SolverStatus::Inactive))
			.collect();

		info!(
			"Found {} solvers eligible for health checks",
			checkable_solvers.len()
		);

		if checkable_solvers.is_empty() {
			info!("No solvers found for health checks - all done");
			return Ok(());
		}

		// Perform health checks in parallel
		let success_count = Arc::new(AtomicUsize::new(0));
		let error_count = Arc::new(AtomicUsize::new(0));

		let results: Vec<(String, bool)> = stream::iter(checkable_solvers)
			.map(|solver| {
				let storage = Arc::clone(&self.storage);
				let adapter_registry = Arc::clone(&self.adapter_registry);
				let job_scheduler = self.job_scheduler.clone();
				let success_count = Arc::clone(&success_count);
				let error_count = Arc::clone(&error_count);

				async move {
					let solver_id = solver.solver_id.clone();

					// Create adapter service and perform health check
					// SolverAdapterService automatically handles metrics collection
					let adapter_result = SolverAdapterService::from_solver(solver.clone(), adapter_registry, job_scheduler.clone());

					let is_healthy = match adapter_result {
						Ok(adapter_service) => {
							// Perform health check with timeout
							let health_result = tokio::time::timeout(
								Duration::from_secs(10), // 10 second timeout
								adapter_service.health_check()
							).await;

							match health_result {
								Ok(Ok(is_healthy)) => {
									if is_healthy {
										success_count.fetch_add(1, Ordering::Relaxed);
									} else {
										error_count.fetch_add(1, Ordering::Relaxed);
									}
									is_healthy
								},
								Ok(Err(_e)) => {
									error_count.fetch_add(1, Ordering::Relaxed);
									false
								},
								Err(_timeout) => {
									error_count.fetch_add(1, Ordering::Relaxed);
									false
								},
							}
						},
						Err(_e) => {
							error_count.fetch_add(1, Ordering::Relaxed);
							false
						},
					};

					// Update solver status in storage
					if let Err(e) = Self::update_solver_status(&storage, &solver, is_healthy).await {
						warn!("Failed to update solver status for {}: {}", solver_id, e);
					}

					(solver_id, is_healthy)
				}
			})
			.buffer_unordered(8) // Process 8 health checks concurrently
			.collect()
			.await;

		// Log results
		for (solver_id, is_healthy) in &results {
			if *is_healthy {
				debug!("Health check passed for solver: {}", solver_id);
			} else {
				warn!("Health check failed for solver: {}", solver_id);
			}
		}

		let final_success_count = success_count.load(Ordering::Relaxed);
		let final_error_count = error_count.load(Ordering::Relaxed);

		info!(
			"Parallel health checks completed - {} successful, {} failed",
			final_success_count, final_error_count
		);

		Ok(())
	}

	async fn fetch_assets_all_solvers(&self) -> Result<(), SolverServiceError> {
		info!("Starting parallel asset auto-discovery for all solvers");

		// Get all solvers from storage
		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| SolverServiceError::Storage(e.to_string()))?;

		// Filter solvers that need auto-discovery (check if source is AutoDiscovered)
		let auto_discovery_solvers: Vec<_> = solvers
			.into_iter()
			.filter(|solver| match &solver.metadata.supported_assets {
				SupportedAssets::Assets { source, .. } => {
					matches!(source, AssetSource::AutoDiscovered)
				},
				SupportedAssets::Routes { source, .. } => {
					matches!(source, AssetSource::AutoDiscovered)
				},
			})
			.collect();

		info!(
			"Found {} solvers requiring asset auto-discovery",
			auto_discovery_solvers.len()
		);

		if auto_discovery_solvers.is_empty() {
			info!("No solvers found requiring asset auto-discovery - all done");
			return Ok(());
		}

		// Perform asset discovery in parallel
		let success_count = Arc::new(AtomicUsize::new(0));
		let error_count = Arc::new(AtomicUsize::new(0));

		stream::iter(auto_discovery_solvers)
			.for_each_concurrent(8, |solver| {
				let success_count = Arc::clone(&success_count);
				let error_count = Arc::clone(&error_count);

				async move {
					let solver_id = solver.solver_id.clone();

					// fetch_and_update_assets calls SolverAdapterService which automatically handles metrics
					match self.fetch_and_update_assets(&solver_id).await {
						Ok(_) => {
							success_count.fetch_add(1, Ordering::Relaxed);
							debug!("Asset auto-discovery completed for solver: {}", solver_id);
						},
						Err(e) => {
							error_count.fetch_add(1, Ordering::Relaxed);
							warn!(
								"Asset auto-discovery failed for solver {}: {}",
								solver_id, e
							);
						},
					}
				}
			})
			.await;

		let final_success_count = success_count.load(Ordering::Relaxed);
		let final_error_count = error_count.load(Ordering::Relaxed);

		info!(
			"Asset auto-discovery completed - {} successful, {} failed",
			final_success_count, final_error_count
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
