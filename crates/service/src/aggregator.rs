//! Core aggregation service logic

use crate::integrity::IntegrityService;
use futures::future::join_all;
use oif_adapters::AdapterRegistry;
use oif_types::{
	GetQuoteRequest, IntegrityPayload, Quote, QuoteRequest, Solver, SolverRuntimeConfig,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

/// Errors that can occur during quote aggregation
#[derive(Debug, thiserror::Error)]
pub enum AggregatorServiceError {
	#[error("Request validation failed: {0}")]
	ValidationError(String),

	#[error("No solvers available for quote aggregation")]
	NoSolversAvailable,

	#[error("All solvers failed to provide quotes")]
	AllSolversFailed,

	#[error("Timeout occurred while fetching quotes from solvers")]
	Timeout,

	#[error("Integrity service error: {0}")]
	IntegrityError(String),
}

pub type AggregatorResult<T> = Result<T, AggregatorServiceError>;

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

	/// Fetch quotes concurrently from all registered solvers using QuoteRequest
	pub async fn fetch_quotes(&self, request: QuoteRequest) -> AggregatorResult<Vec<Quote>> {
		// Validate the request first
		request
			.validate()
			.map_err(|e| AggregatorServiceError::ValidationError(e.to_string()))?;

		if self.solvers.is_empty() {
			return Err(AggregatorServiceError::NoSolversAvailable);
		}

		info!(
			"Fetching quotes for ERC-7930 request with {} inputs, {} outputs from {} solvers",
			request.available_inputs.len(),
			request.requested_outputs.len(),
			self.solvers.len()
		);

		let tasks = self.solvers.iter().map(|(solver_id, solver)| {
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
				let get_quote_request = GetQuoteRequest::try_from(request.clone())
					.map_err(|e| {
						tracing::warn!(
							solver_id = %solver.solver_id,
							error = %e,
							"Failed to convert QuoteRequest to GetQuoteRequest"
						);
						e
					})
					.ok()?;
				match adapter.get_quotes(&get_quote_request, &config).await {
					Ok(response) => {
						// Convert adapter quotes to domain quotes with integrity checksums
						let mut domain_quotes = Vec::new();

						for adapter_quote in response.quotes {
							// Create domain quote first (without integrity checksum)
							let mut domain_quote = Quote::new(
								solver.solver_id.clone(),
								adapter_quote.orders,
								adapter_quote.details,
								adapter_quote.provider,
								String::new(), // Temporary empty checksum
							)
							.with_valid_until(adapter_quote.valid_until.unwrap_or(0))
							.with_eta(adapter_quote.eta.unwrap_or(0));

							// Generate integrity checksum from domain quote (includes solver_id)
							let payload = domain_quote.to_integrity_payload();
							let integrity_checksum =
								match integrity_service.generate_checksum_from_payload(&payload) {
									Ok(checksum) => checksum,
									Err(e) => {
										warn!(
											"Failed to generate integrity checksum for quote {} from solver {}: {}",
											adapter_quote.quote_id, solver_id, e
										);
										continue; // Skip quotes that fail checksum generation
									},
								};

							// Update the domain quote with the calculated integrity checksum
							domain_quote.integrity_checksum = integrity_checksum;
							domain_quotes.push(domain_quote);
						}

						if domain_quotes.is_empty() {
							warn!("No valid quotes received from solver {}", solver_id);
							None
						} else {
							info!(
									"Successfully converted {} quotes from solver {} with integrity checksums",
									domain_quotes.len(),
									solver_id
								);
							Some(domain_quotes)
						}
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
			.filter_map(|r| r.ok().flatten()) // Filter successful results that have Some(Vec<Quote>)
			.flatten() // Flatten Vec<Vec<Quote>> to Vec<Quote>
			.collect();

		if quotes.is_empty() {
			warn!(
				"All {} solvers failed to provide quotes",
				self.solvers.len()
			);
			return Err(AggregatorServiceError::AllSolversFailed);
		}

		info!(
			"Quote aggregation completed: {} quotes from {} solvers",
			quotes.len(),
			self.solvers.len()
		);

		Ok(quotes)
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

#[cfg(test)]
mod tests {
	use super::*;
	use oif_adapters::AdapterRegistry;
	use oif_types::{AvailableInput, InteropAddress, QuoteRequest, RequestedOutput, U256};

	fn create_test_aggregator() -> AggregatorService {
		use oif_types::SecretString;
		AggregatorService::new(
			vec![], // No solvers
			Arc::new(AdapterRegistry::new()),
			5000,
			Arc::new(IntegrityService::new(SecretString::from("test-secret"))),
		)
	}

	#[tokio::test]
	async fn test_fetch_quotes_no_solvers() {
		let aggregator = create_test_aggregator();

		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		let request = QuoteRequest {
			user: user.clone(),
			available_inputs: vec![AvailableInput {
				user: user.clone(),
				asset: asset.clone(),
				amount: U256::from(1000u64),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				receiver: user,
				asset,
				amount: U256::from(500u64),
				calldata: None,
			}],
			min_valid_until: None,
			preference: None,
			solver_options: None,
		};

		let result = aggregator.fetch_quotes(request).await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			AggregatorServiceError::NoSolversAvailable
		));
	}

	#[tokio::test]
	async fn test_fetch_quotes_validation_error() {
		let aggregator = create_test_aggregator();

		// Create invalid request with empty inputs
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let request = QuoteRequest {
			user,
			available_inputs: vec![], // Empty - should fail validation
			requested_outputs: vec![],
			min_valid_until: None,
			preference: None,
			solver_options: None,
		};

		let result = aggregator.fetch_quotes(request).await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			AggregatorServiceError::ValidationError(_)
		));
	}
}

/// Aggregation service statistics
#[derive(Debug, Clone)]
pub struct AggregationStats {
	pub total_solvers: usize,
	pub initialized_adapters: usize,
	pub global_timeout_ms: u64,
}
