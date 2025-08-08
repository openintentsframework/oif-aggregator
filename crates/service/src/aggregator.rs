//! Core aggregation service logic

use futures::future::join_all;
use oif_adapters::{AdapterError, AdapterFactory};
use oif_types::{AdapterConfig, AdapterType, Quote, QuoteRequest, Solver};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// Service for aggregating quotes from multiple solvers
pub struct AggregatorService {
	solvers: HashMap<String, Solver>,
	adapter_factory: Arc<AdapterFactory>,
	global_timeout_ms: u64,
}

impl AggregatorService {
	/// Create a new aggregator service
	pub fn new(solvers: Vec<Solver>, global_timeout_ms: u64) -> Self {
		let mut solver_map = HashMap::new();
		for solver in solvers {
			solver_map.insert(solver.solver_id.clone(), solver);
		}

		Self {
			solvers: solver_map,
			adapter_factory: Arc::new(AdapterFactory::new()),
			global_timeout_ms,
		}
	}

	/// Create aggregator service with pre-configured adapters
	pub fn with_adapters(
		solvers: Vec<Solver>,
		adapter_factory: AdapterFactory,
		global_timeout_ms: u64,
	) -> Self {
		let mut solver_map = HashMap::new();
		for solver in solvers {
			solver_map.insert(solver.solver_id.clone(), solver);
		}

		Self {
			solvers: solver_map,
			adapter_factory: Arc::new(adapter_factory),
			global_timeout_ms,
		}
	}

	/// Initialize adapters for all solvers
	pub async fn initialize_adapters(&mut self) -> Result<(), AdapterError> {
		let mut factory = AdapterFactory::new();

		for (solver_id, solver) in &self.solvers {
			// Build adapter configuration from solver metadata
			let adapter_config = AdapterConfig {
				adapter_id: solver.adapter_id.clone(),
				adapter_type: AdapterType::OifV1, // TODO: derive from config/solver.adapter_id if encoded
				name: solver
					.metadata
					.name
					.clone()
					.unwrap_or_else(|| solver.solver_id.clone()),
				description: solver.metadata.description.clone(),
				version: solver
					.metadata
					.version
					.clone()
					.unwrap_or_else(|| "1.0.0".to_string()),
			};

			// Use solver-provided endpoint and timeout
			match AdapterFactory::create_for_solver(
				&adapter_config,
				solver.endpoint.clone(),
				solver.timeout_ms,
			) {
				Ok(adapter) => {
					info!(
						"Initialized adapter {} for solver {}",
						adapter_config.adapter_id, solver_id
					);
					factory.register(solver_id.clone(), adapter);
				},
				Err(e) => {
					error!(
						"Failed to initialize adapter for solver {}: {}",
						solver_id, e
					);
					return Err(e);
				},
			}
		}

		self.adapter_factory = Arc::new(factory);
		Ok(())
	}

	/// Fetch quotes concurrently from all registered solvers
	pub async fn fetch_quotes(&self, request: QuoteRequest) -> Vec<Quote> {
		info!(
			"Fetching quotes for request {} from {} solvers",
			request.request_id,
			self.solvers.len()
		);

		let tasks = self.solvers.iter().map(|(solver_id, solver)| {
			let request = request.clone();
			let solver_id = solver_id.clone();
			let solver = solver.clone();
			let adapter_factory = Arc::clone(&self.adapter_factory);

			tokio::spawn(async move {
				debug!("Starting quote fetch from solver {}", solver_id);

				let adapter = match adapter_factory.get(&solver_id) {
					Some(adapter) => adapter,
					None => {
						warn!("No adapter found for solver {}", solver_id);
						return None;
					},
				};

				let quote_future = adapter.get_quote(&request);
				let timeout_duration = Duration::from_millis(solver.timeout_ms);

				match timeout(timeout_duration, quote_future).await {
					Ok(Ok(quote)) => {
						info!(
							"Successfully got quote from solver {} in {}ms",
							solver_id, quote.response_time_ms
						);
						Some(quote)
					},
					Ok(Err(e)) => {
						warn!("Solver {} returned error: {}", solver_id, e);
						None
					},
					Err(_) => {
						warn!(
							"Solver {} timed out after {}ms",
							solver_id, solver.timeout_ms
						);
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

		for (solver_id, _solver) in &self.solvers {
			if let Some(adapter) = self.adapter_factory.get(solver_id) {
				match adapter.health_check().await {
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
			initialized_adapters: self.adapter_factory.get_all().len(),
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
