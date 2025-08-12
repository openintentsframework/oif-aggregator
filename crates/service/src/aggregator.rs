//! Core aggregation service logic

use crate::integrity::IntegrityService;
use futures::future::join_all;
use oif_adapters::AdapterRegistry;
use oif_types::{Quote, QuoteRequest, Solver, SolverRuntimeConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

/// Service for aggregating quotes from multiple solvers
pub struct AggregatorService {
	solvers: HashMap<String, Solver>,
	adapter_registry: Arc<AdapterRegistry>,
	global_timeout_ms: u64,
	integrity_service: Arc<IntegrityService>,
}

impl AggregatorService {
	/// Create a new aggregator service with pre-configured adapters
	pub fn new(
		solvers: Vec<Solver>,
		adapter_registry: Arc<AdapterRegistry>,
		global_timeout_ms: u64,
		integrity_service: Arc<IntegrityService>,
	) -> Self {
		let mut solver_map = HashMap::new();
		for solver in solvers {
			solver_map.insert(solver.solver_id.clone(), solver);
		}

		Self {
			solvers: solver_map,
			adapter_registry,
			global_timeout_ms,
			integrity_service,
		}
	}

	/// Validate that all solvers have matching adapters
	pub fn validate_solvers(&self) -> Result<(), String> {
		for solver in self.solvers.values() {
			if self.adapter_registry.get(&solver.adapter_id).is_none() {
				return Err(format!(
					"Solver '{}' references unknown adapter '{}'",
					solver.solver_id, solver.adapter_id
				));
			}
		}
		Ok(())
	}

	/// Fetch quotes concurrently from all registered solvers
	pub async fn fetch_quotes(&self, request: QuoteRequest) -> Vec<Quote> {
		info!(
			"Fetching quotes for request {} from {} solvers",
			request.request_id,
			self.solvers.len()
		);

		let tasks =
			self.solvers.iter().map(|(solver_id, solver)| {
				let request = request.clone();
				let solver_id = solver_id.clone();
				let solver = solver.clone();
				let adapter_registry = Arc::clone(&self.adapter_registry);
				let integrity_service = Arc::clone(&self.integrity_service);

				tokio::spawn(async move {
					debug!("Starting quote fetch from solver {}", solver_id);

					let adapter = match adapter_registry.get(&solver.adapter_id) {
						Some(adapter) => adapter,
						None => {
							warn!(
								"No adapter found for solver {} (adapter_id: {})",
								solver_id, solver.adapter_id
							);
							return None;
						},
					};

					let config = SolverRuntimeConfig::from(&solver);
					match adapter.get_quote(request.clone(), &config).await {
						Ok(mut quote) => {
							// Generate integrity checksum for the quote
							let payload = quote.to_integrity_payload();
							match integrity_service.generate_checksum_from_payload(&payload) {
								Ok(checksum) => {
									quote.integrity_checksum = checksum.clone();
									info!(
									"Successfully got quote from solver {} in {}ms with integrity checksum",
									solver_id, quote.response_time_ms
								);
								},
								Err(e) => {
									warn!("Failed to generate integrity checksum for quote from {}: {}", solver_id, e);
									// Still return the quote without checksum rather than failing entirely
								},
							}
							Some(quote)
						},
						Err(e) => {
							warn!("Solver {} returned error: {}", solver_id, e);
							None
						},
					}
				})
			});

		// Apply global timeout to the entire aggregation process
		let aggregation_future = join_all(tasks);
		let global_timeout_duration = Duration::from_millis(self.global_timeout_ms);

		let results = match timeout(global_timeout_duration, aggregation_future).await {
			Ok(results) => results,
			Err(_) => {
				warn!(
					"Global aggregation timeout reached after {}ms",
					self.global_timeout_ms
				);
				Vec::new()
			},
		};

		let quotes: Vec<Quote> = results
			.into_iter()
			.filter_map(|r| r.ok().flatten())
			.collect();

		info!(
			"Quote aggregation completed: {} quotes from {} solvers",
			quotes.len(),
			self.solvers.len()
		);

		quotes
	}

	/// Perform health checks on all adapters
	pub async fn health_check_all(&self) -> HashMap<String, bool> {
		let mut results = HashMap::new();

		for (solver_id, solver) in &self.solvers {
			if let Some(adapter) = self.adapter_registry.get(&solver.adapter_id) {
				let config = SolverRuntimeConfig::from(solver);
				match adapter.health_check(&config).await {
					Ok(is_healthy) => {
						results.insert(solver_id.to_string(), is_healthy);
					},
					Err(_) => {
						results.insert(solver_id.to_string(), false);
					},
				}
			} else {
				results.insert(solver_id.to_string(), false);
			}
		}

		results
	}

	/// Get aggregation statistics
	pub fn get_stats(&self) -> AggregationStats {
		AggregationStats {
			total_solvers: self.solvers.len(),
			initialized_adapters: self.adapter_registry.get_all().len(),
			global_timeout_ms: self.global_timeout_ms,
		}
	}
}

/// Aggregation service statistics
#[derive(Debug, Clone)]
pub struct AggregationStats {
	pub total_solvers: usize,
	pub initialized_adapters: usize,
	pub global_timeout_ms: u64,
}
