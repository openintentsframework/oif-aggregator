//! Core aggregation service logic

use crate::integrity::IntegrityTrait;
use crate::solver_adapter::{SolverAdapterService, SolverAdapterTrait};
use crate::solver_filter::SolverFilterTrait;
use async_trait::async_trait;
use oif_adapters::AdapterRegistry;
use oif_types::constants::limits::{
	DEFAULT_MAX_CONCURRENT_SOLVERS, DEFAULT_MAX_RETRIES_PER_SOLVER, DEFAULT_MIN_QUOTES,
	DEFAULT_PER_SOLVER_TIMEOUT_MS, DEFAULT_RETRY_DELAY_MS,
};
use oif_types::quotes::request::{SolverOptions, SolverSelection};
use oif_types::quotes::response::AggregationMetadata;
use oif_types::{GetQuoteRequest, IntegrityPayload, Quote, QuoteRequest, Solver};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time::{timeout, Duration, Instant};
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

	#[error("Resource limit exceeded: {0}")]
	ResourceLimitExceeded(String),

	#[error("Solver adapter error: {0}")]
	SolverAdapterError(String),
}

pub type AggregatorResult<T> = Result<T, AggregatorServiceError>;

/// Result from an individual solver task sent via channel
#[derive(Debug)]
pub struct SolverTaskResult {
	pub solver_id: String,
	pub quotes: Vec<Quote>,
	pub success: bool,
	pub error_message: Option<String>,
	pub duration_ms: u64,
	pub retry_count: u32,
}

/// Configuration for aggregation behavior
#[derive(Debug, Clone)]
pub struct AggregationConfig {
	pub max_concurrent_solvers: usize,
	pub max_retries_per_solver: u32,
	pub retry_delay_ms: u64,
}

impl Default for AggregationConfig {
	fn default() -> Self {
		Self {
			max_concurrent_solvers: DEFAULT_MAX_CONCURRENT_SOLVERS,
			max_retries_per_solver: DEFAULT_MAX_RETRIES_PER_SOLVER,
			retry_delay_ms: DEFAULT_RETRY_DELAY_MS,
		}
	}
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AggregatorTrait: Send + Sync {
	/// Validate that all solvers have matching adapters
	fn validate_solvers(&self) -> Result<(), String>;

	/// Fetch quotes concurrently from all registered solvers
	async fn fetch_quotes(
		&self,
		request: QuoteRequest,
	) -> AggregatorResult<(Vec<Quote>, AggregationMetadata)>;
}

/// Trait for executing individual solver tasks with retries and concurrency control
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TaskExecutorTrait: Send + Sync {
	/// Execute a single solver task with retries and concurrency limiting
	async fn execute_solver_task(
		&self,
		solver: Solver,
		request: QuoteRequest,
		per_solver_timeout_ms: u64,
		result_tx: mpsc::UnboundedSender<SolverTaskResult>,
		cancel_rx: broadcast::Receiver<()>,
	);

	/// Execute a single solver attempt (no retries)
	async fn execute_solver_attempt(
		&self,
		solver: &Solver,
		request: &QuoteRequest,
		timeout_ms: u64,
		cancel_rx: &mut broadcast::Receiver<()>,
	) -> Result<Vec<Quote>, String>;
}

/// Helper struct for executing solver tasks asynchronously
pub struct TaskExecutor {
	adapter_registry: Arc<AdapterRegistry>,
	integrity_service: Arc<dyn IntegrityTrait>,
	config: AggregationConfig,
	concurrency_limiter: Arc<Semaphore>,
}

impl TaskExecutor {
	/// Create a new TaskExecutor instance
	pub fn new(
		adapter_registry: Arc<AdapterRegistry>,
		integrity_service: Arc<dyn IntegrityTrait>,
		config: AggregationConfig,
		concurrency_limiter: Arc<Semaphore>,
	) -> Self {
		Self {
			adapter_registry,
			integrity_service,
			config,
			concurrency_limiter,
		}
	}
}

#[async_trait]
impl TaskExecutorTrait for TaskExecutor {
	/// Execute a single solver task with retries and concurrency limiting
	async fn execute_solver_task(
		&self,
		solver: Solver,
		request: QuoteRequest,
		per_solver_timeout_ms: u64,
		result_tx: mpsc::UnboundedSender<SolverTaskResult>,
		mut cancel_rx: broadcast::Receiver<()>,
	) {
		let solver_id = solver.solver_id.clone();
		let task_start = Instant::now();

		// Acquire semaphore permit for concurrency limiting
		let _permit = match self.concurrency_limiter.try_acquire() {
			Ok(permit) => permit,
			Err(_) => {
				debug!("Solver {} skipped due to concurrency limit", solver_id);
				let _ = result_tx.send(SolverTaskResult {
					solver_id: solver_id.clone(),
					quotes: Vec::new(),
					success: false,
					error_message: Some("Concurrency limit exceeded".to_string()),
					duration_ms: 0,
					retry_count: 0,
				});
				return;
			},
		};

		let mut retry_count = 0;
		let max_retries = self.config.max_retries_per_solver;

		// Retry loop
		while retry_count <= max_retries {
			// Check for cancellation before each attempt
			if cancel_rx.try_recv().is_ok() {
				debug!(
					"Solver {} task cancelled (attempt {})",
					solver_id,
					retry_count + 1
				);
				return;
			}

			let attempt_start = Instant::now();
			debug!(
				"Solver {} attempt {} with {}ms timeout",
				solver_id,
				retry_count + 1,
				per_solver_timeout_ms
			);

			// Execute single attempt
			match self
				.execute_solver_attempt(&solver, &request, per_solver_timeout_ms, &mut cancel_rx)
				.await
			{
				Ok(quotes) => {
					// Success - send result and return
					let task_result = SolverTaskResult {
						solver_id: solver_id.clone(),
						quotes,
						success: true,
						error_message: None,
						duration_ms: task_start.elapsed().as_millis() as u64,
						retry_count,
					};

					let attempt_duration = attempt_start.elapsed().as_millis() as u64;
					info!(
						"Solver {} completed attempt {} with {} quotes in {}ms",
						solver_id,
						retry_count + 1,
						task_result.quotes.len(),
						attempt_duration
					);

					let _ = result_tx.send(task_result);
					return;
				},
				Err(error_msg) => {
					retry_count += 1;
					let attempt_duration = attempt_start.elapsed().as_millis() as u64;

					// Check if we should retry
					if retry_count <= max_retries {
						warn!(
							"Solver {} attempt {} failed ({}ms): {} - retrying ({}/{})",
							solver_id,
							retry_count,
							attempt_duration,
							error_msg,
							retry_count,
							max_retries
						);

						// Wait before retry with exponential backoff
						let delay = Duration::from_millis(
							self.config.retry_delay_ms * (retry_count as u64),
						);

						tokio::select! {
							_ = tokio::time::sleep(delay) => {},
							_ = cancel_rx.recv() => {
								debug!("Solver {} task cancelled during retry delay", solver_id);
								return;
							}
						}
					} else {
						// All retries exhausted
						warn!(
							"Solver {} failed after {} attempts: {}",
							solver_id, retry_count, error_msg
						);
						let _ = result_tx.send(SolverTaskResult {
							solver_id: solver_id.clone(),
							quotes: Vec::new(),
							success: false,
							error_message: Some(error_msg),
							duration_ms: task_start.elapsed().as_millis() as u64,
							retry_count: retry_count - 1,
						});
						return;
					}
				},
			}
		}
	}

	/// Execute a single solver attempt (no retries)
	async fn execute_solver_attempt(
		&self,
		solver: &Solver,
		request: &QuoteRequest,
		timeout_ms: u64,
		cancel_rx: &mut broadcast::Receiver<()>,
	) -> Result<Vec<Quote>, String> {
		// Create solver adapter service
		let solver_adapter =
			SolverAdapterService::from_solver(solver.clone(), self.adapter_registry.clone())
				.map_err(|e| format!("Failed to create solver adapter service: {}", e))?;

		let get_quote_request = GetQuoteRequest::try_from(request.clone())
			.map_err(|e| format!("Failed to convert QuoteRequest: {}", e))?;

		// Execute request with timeout and cancellation
		let solver_future = solver_adapter.get_quotes(&get_quote_request);
		let solver_timeout_duration = Duration::from_millis(timeout_ms);

		let response = tokio::select! {
			result = timeout(solver_timeout_duration, solver_future) => {
				match result {
					Ok(Ok(response)) => Ok(response),
					Ok(Err(e)) => Err(format!("Solver error: {}", e)),
					Err(_) => Err(format!("Timed out after {}ms", timeout_ms)),
				}
			}
			_ = cancel_rx.recv() => {
				debug!("Solver {} task cancelled during execution", solver.solver_id);
				return Err("Task cancelled".to_string());
			}
		}?;

		// Process quotes and generate integrity checksums
		let mut domain_quotes = Vec::new();

		for adapter_quote in response.quotes {
			let mut domain_quote = Quote::new(
				solver.solver_id.clone(),
				adapter_quote.orders,
				adapter_quote.details,
				adapter_quote.provider,
				String::new(),
			)
			.with_valid_until(adapter_quote.valid_until.unwrap_or(0))
			.with_eta(adapter_quote.eta.unwrap_or(0));

			let payload = domain_quote.to_integrity_payload();
			match self
				.integrity_service
				.generate_checksum_from_payload(&payload)
			{
				Ok(checksum) => {
					domain_quote.integrity_checksum = checksum;
					domain_quotes.push(domain_quote);
				},
				Err(e) => {
					warn!(
						"Failed to generate integrity checksum for quote {} from solver {}: {}",
						adapter_quote.quote_id, solver.solver_id, e
					);
				},
			}
		}

		if domain_quotes.is_empty() {
			Err("No valid quotes received".to_string())
		} else {
			Ok(domain_quotes)
		}
	}
}

/// Service for aggregating quotes from multiple solvers
pub struct AggregatorService {
	solvers: HashMap<String, Solver>,
	adapter_registry: Arc<AdapterRegistry>,
	global_timeout_ms: u64,
	#[allow(dead_code)]
	integrity_service: Arc<dyn IntegrityTrait>,
	solver_filter_service: Arc<dyn SolverFilterTrait>,
	task_executor: Arc<dyn TaskExecutorTrait>,
	#[allow(dead_code)]
	config: AggregationConfig,
}

impl AggregatorService {
	/// Create a new aggregator service with pre-configured adapters
	pub fn new(
		solvers: Vec<Solver>,
		adapter_registry: Arc<AdapterRegistry>,
		global_timeout_ms: u64,
		integrity_service: Arc<dyn IntegrityTrait>,
		solver_filter_service: Arc<dyn SolverFilterTrait>,
	) -> Self {
		Self::with_config(
			solvers,
			adapter_registry,
			global_timeout_ms,
			integrity_service,
			solver_filter_service,
			AggregationConfig::default(),
		)
	}

	/// Create a new aggregator service with custom configuration
	pub fn with_config(
		solvers: Vec<Solver>,
		adapter_registry: Arc<AdapterRegistry>,
		global_timeout_ms: u64,
		integrity_service: Arc<dyn IntegrityTrait>,
		solver_filter_service: Arc<dyn SolverFilterTrait>,
		config: AggregationConfig,
	) -> Self {
		let mut solver_map = HashMap::new();
		for solver in solvers {
			solver_map.insert(solver.solver_id.clone(), solver);
		}

		// Create TaskExecutor with the configuration
		let concurrency_limiter = Arc::new(Semaphore::new(config.max_concurrent_solvers));
		let task_executor = Arc::new(TaskExecutor::new(
			adapter_registry.clone(),
			integrity_service.clone(),
			config.clone(),
			concurrency_limiter,
		)) as Arc<dyn TaskExecutorTrait>;

		Self {
			solvers: solver_map,
			adapter_registry,
			global_timeout_ms,
			integrity_service,
			solver_filter_service,
			task_executor,
			config,
		}
	}

	/// Extract and validate aggregation configuration from request
	fn extract_aggregation_config(&self, request: &QuoteRequest) -> (u64, u64, usize) {
		let global_timeout_ms = request
			.solver_options
			.as_ref()
			.and_then(|opts| opts.timeout)
			.unwrap_or(self.global_timeout_ms);

		let per_solver_timeout_ms = request
			.solver_options
			.as_ref()
			.and_then(|opts| opts.solver_timeout)
			.unwrap_or(DEFAULT_PER_SOLVER_TIMEOUT_MS);

		let min_quotes_required = request
			.solver_options
			.as_ref()
			.and_then(|opts| opts.min_quotes)
			.unwrap_or(DEFAULT_MIN_QUOTES) as usize;

		(
			global_timeout_ms,
			per_solver_timeout_ms,
			min_quotes_required,
		)
	}

	/// Initialize aggregation metadata with request configuration
	fn initialize_metadata(
		&self,
		request: &QuoteRequest,
		global_timeout_ms: u64,
		per_solver_timeout_ms: u64,
		min_quotes_required: usize,
		selected_solvers_count: usize,
	) -> AggregationMetadata {
		AggregationMetadata {
			solver_timeout_ms: per_solver_timeout_ms,
			global_timeout_ms,
			total_solvers_available: self.solvers.len(),
			solvers_queried: selected_solvers_count,
			min_quotes_required,
			solver_selection_mode: request
				.solver_options
				.as_ref()
				.and_then(|opts| opts.solver_selection.as_ref())
				.unwrap_or(&SolverSelection::All)
				.clone(),
			..Default::default()
		}
	}

	/// Spawn solver tasks and return their handles
	fn spawn_solver_tasks(
		&self,
		selected_solvers: Vec<Solver>,
		request: QuoteRequest,
		per_solver_timeout_ms: u64,
		result_tx: mpsc::UnboundedSender<SolverTaskResult>,
		cancel_tx: broadcast::Sender<()>,
	) -> Vec<tokio::task::JoinHandle<()>> {
		let mut task_handles = Vec::new();

		for solver in selected_solvers {
			let request = request.clone();
			let result_tx = result_tx.clone();
			let cancel_rx = cancel_tx.subscribe();

			// Use the injected task executor trait object
			let task_handle = tokio::spawn({
				let task_executor = self.task_executor.clone();

				async move {
					task_executor
						.execute_solver_task(
							solver,
							request,
							per_solver_timeout_ms,
							result_tx,
							cancel_rx,
						)
						.await;
				}
			});

			task_handles.push(task_handle);
		}

		task_handles
	}

	/// Update final metadata before returning results
	fn finalize_metadata(
		&self,
		metadata: &mut AggregationMetadata,
		aggregation_start: Instant,
		success_count: usize,
		error_count: usize,
		timeout_count: usize,
	) {
		metadata.total_duration_ms = aggregation_start.elapsed().as_millis() as u64;
		metadata.solvers_responded_success = success_count;
		metadata.solvers_responded_error = error_count;
		metadata.solvers_timed_out = timeout_count;
	}
}

#[async_trait]
impl AggregatorTrait for AggregatorService {
	/// Validate that all solvers have matching adapters
	fn validate_solvers(&self) -> Result<(), String> {
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

	/// Fetch quotes concurrently from filtered solvers using enhanced method extraction approach
	async fn fetch_quotes(
		&self,
		request: QuoteRequest,
	) -> AggregatorResult<(Vec<Quote>, AggregationMetadata)> {
		let aggregation_start = Instant::now();

		// Step 1: Validate request
		request
			.validate()
			.map_err(|e| AggregatorServiceError::ValidationError(e.to_string()))?;

		if self.solvers.is_empty() {
			return Err(AggregatorServiceError::NoSolversAvailable);
		}

		// Step 2: Extract configuration
		let (global_timeout_ms, per_solver_timeout_ms, min_quotes_required) =
			self.extract_aggregation_config(&request);

		// Step 3: Filter solvers using SolverFilterService
		let available_solvers: Vec<Solver> = self.solvers.values().cloned().collect();
		let selected_solvers = self
			.solver_filter_service
			.filter_solvers(
				&available_solvers,
				&request,
				request
					.solver_options
					.as_ref()
					.unwrap_or(&SolverOptions::default()),
			)
			.await;

		// Step 4: Initialize metadata
		let mut metadata = self.initialize_metadata(
			&request,
			global_timeout_ms,
			per_solver_timeout_ms,
			min_quotes_required,
			selected_solvers.len(),
		);

		if selected_solvers.is_empty() {
			metadata.total_duration_ms = aggregation_start.elapsed().as_millis() as u64;
			return Err(AggregatorServiceError::NoSolversAvailable);
		}

		info!(
			"Starting aggregation: {} solvers, {}ms global timeout, {}ms per-solver timeout, {} min quotes required",
			selected_solvers.len(), global_timeout_ms, per_solver_timeout_ms, min_quotes_required
		);

		// Step 5: Create communication channels
		let (result_tx, result_rx) = mpsc::unbounded_channel::<SolverTaskResult>();
		let (cancel_tx, _) = broadcast::channel::<()>(selected_solvers.len());

		// Step 6: Spawn solver tasks using helper method
		let task_handles = self.spawn_solver_tasks(
			selected_solvers,
			request,
			per_solver_timeout_ms,
			result_tx.clone(),
			cancel_tx.clone(),
		);
		drop(result_tx); // Close sender to properly detect completion

		// Step 7: Execute collection loop with enhanced while-let approach
		let (collected_quotes, final_metadata) = self
			.collect_quotes_with_early_termination(
				result_rx,
				task_handles,
				cancel_tx,
				global_timeout_ms,
				min_quotes_required,
				aggregation_start,
				metadata,
			)
			.await?;

		let final_count = collected_quotes.len();
		info!(
			"Aggregation completed: {} quotes from {} solvers in {}ms (min required: {})",
			final_count,
			final_metadata.solvers_responded_success,
			final_metadata.total_duration_ms,
			min_quotes_required
		);

		Ok((collected_quotes, final_metadata))
	}
}

impl AggregatorService {
	/// Collect quotes with early termination and enhanced event handling
	async fn collect_quotes_with_early_termination(
		&self,
		mut result_rx: mpsc::UnboundedReceiver<SolverTaskResult>,
		task_handles: Vec<tokio::task::JoinHandle<()>>,
		cancel_tx: broadcast::Sender<()>,
		global_timeout_ms: u64,
		min_quotes_required: usize,
		aggregation_start: Instant,
		mut metadata: AggregationMetadata,
	) -> AggregatorResult<(Vec<Quote>, AggregationMetadata)> {
		let global_timeout_duration = Duration::from_millis(global_timeout_ms);
		let total_tasks = task_handles.len();
		let start_time = Instant::now();

		let mut collected_quotes = Vec::new();
		let mut completed_solvers = 0;
		let mut success_count = 0;
		let mut error_count = 0;
		let mut timeout_count = 0;

		// Create global timeout future
		let global_timeout_future = tokio::time::sleep(global_timeout_duration);
		tokio::pin!(global_timeout_future);

		// Enhanced while-let approach (no explicit loop!)
		while let Some(result) = tokio::select! {
			// Receive next result
			result = result_rx.recv() => result,
			// Global timeout
			_ = &mut global_timeout_future => {
				warn!("Global aggregation timeout reached after {}ms", start_time.elapsed().as_millis());
				let _ = cancel_tx.send(());
				for handle in &task_handles {
					handle.abort();
				}
				None // Break out of while-let loop
			}
		} {
			completed_solvers += 1;

			if result.success {
				success_count += 1;
				collected_quotes.extend(result.quotes);

				// IMMEDIATE early termination check
				if collected_quotes.len() >= min_quotes_required {
					let elapsed = start_time.elapsed();
					info!(
						"Early termination: collected {} quotes (>= {} required) in {}ms",
						collected_quotes.len(),
						min_quotes_required,
						elapsed.as_millis()
					);

					// Signal cancellation and cleanup
					let _ = cancel_tx.send(());
					for handle in &task_handles {
						handle.abort();
					}

					// Update metadata for early termination
					metadata.early_termination = true;
					self.finalize_metadata(
						&mut metadata,
						aggregation_start,
						success_count,
						error_count,
						timeout_count,
					);

					return Ok((collected_quotes, metadata));
				}
			} else {
				// Categorize error types based on enhanced SolverTaskResult
				if result.retry_count > 0 {
					debug!(
						"Solver {} failed after {} retries",
						result.solver_id, result.retry_count
					);
				}

				if let Some(error_msg) = &result.error_message {
					if error_msg.contains("Timed out") || error_msg.contains("timeout") {
						timeout_count += 1;
					} else {
						error_count += 1;
					}
				} else {
					error_count += 1;
				}

				debug!(
					"Solver {} failed ({}ms): {:?}",
					result.solver_id, result.duration_ms, result.error_message
				);
			}

			// Check for natural completion
			if completed_solvers >= total_tasks {
				let elapsed = start_time.elapsed();
				info!(
					"All {} solver tasks completed naturally in {}ms",
					total_tasks,
					elapsed.as_millis()
				);
				break;
			}
		}

		// Finalize metadata using helper method
		self.finalize_metadata(
			&mut metadata,
			aggregation_start,
			success_count,
			error_count,
			timeout_count,
		);

		if collected_quotes.is_empty() {
			return Err(AggregatorServiceError::AllSolversFailed);
		}

		Ok((collected_quotes, metadata))
	}
}

#[cfg(test)]
mod tests {
	//! Tests for AggregatorService focusing on core aggregation behavior.

	use crate::SolverFilterService;

	use super::*;
	use oif_adapters::AdapterRegistry;
	use oif_types::{
		AvailableInput, InteropAddress, QuoteRequest, RequestedOutput, SecretString, U256,
	};

	// Helper function to create test aggregator with no solvers
	fn create_test_aggregator() -> AggregatorService {
		AggregatorService::new(
			vec![], // No solvers
			Arc::new(AdapterRegistry::new()),
			5000,
			Arc::new(crate::integrity::IntegrityService::new(SecretString::from(
				"test-secret",
			))) as Arc<dyn IntegrityTrait>,
			Arc::new(SolverFilterService::new()) as Arc<dyn SolverFilterTrait>,
		)
	}

	// Helper function to create test aggregator with specified solvers
	fn create_test_aggregator_with_solvers(solver_count: usize) -> AggregatorService {
		let mut solvers = Vec::new();
		for i in 1..=solver_count {
			solvers.push(Solver::new(
				format!("solver{}", i),
				format!("adapter{}", i),
				format!("http://localhost:800{}", i),
				5000,
			));
		}

		AggregatorService::new(
			solvers,
			Arc::new(AdapterRegistry::new()),
			5000,
			Arc::new(crate::integrity::IntegrityService::new(SecretString::from(
				"test-secret",
			))) as Arc<dyn IntegrityTrait>,
			Arc::new(SolverFilterService::new()) as Arc<dyn SolverFilterTrait>,
		)
	}

	// Helper function to create a valid quote request
	fn create_valid_quote_request() -> QuoteRequest {
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		QuoteRequest {
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
		}
	}

	// Helper function to create quote request with solver options
	fn create_quote_request_with_options(options: SolverOptions) -> QuoteRequest {
		let mut request = create_valid_quote_request();
		request.solver_options = Some(options);
		request
	}

	// =============================================================================
	// METADATA VALIDATION TESTS
	// =============================================================================

	#[test]
	fn test_aggregation_metadata_structure() {
		// Test that metadata struct has all expected fields
		let metadata = AggregationMetadata {
			total_duration_ms: 1234,
			solver_timeout_ms: 2500,
			global_timeout_ms: 5000,
			early_termination: true,
			total_solvers_available: 10,
			solvers_queried: 5,
			solvers_responded_success: 3,
			solvers_responded_error: 1,
			solvers_timed_out: 1,
			min_quotes_required: 5,
			solver_selection_mode: SolverSelection::Sampled,
		};

		// Verify all counters add up correctly
		assert_eq!(
			metadata.solvers_responded_success
				+ metadata.solvers_responded_error
				+ metadata.solvers_timed_out,
			metadata.solvers_queried
		);
		assert!(metadata.total_duration_ms > 0);
		assert!(metadata.solver_timeout_ms > 0);
		assert!(metadata.global_timeout_ms > 0);
	}

	#[tokio::test]
	async fn test_aggregation_metadata_on_no_solvers() {
		let aggregator = create_test_aggregator(); // No solvers
		let request = create_valid_quote_request();

		let result = aggregator.fetch_quotes(request).await;
		assert!(result.is_err());

		// Error case - no metadata returned, but we can verify the error type
		match result.unwrap_err() {
			AggregatorServiceError::NoSolversAvailable => {
				// Expected when no solvers available
			},
			other => panic!("Expected NoSolversAvailable, got: {:?}", other),
		}
	}

	// =============================================================================
	// BASIC FUNCTIONALITY TESTS
	// =============================================================================

	#[tokio::test]
	async fn test_mock_aggregator_trait() {
		let mut mock = MockAggregatorTrait::new();

		// Setup expectations
		mock.expect_validate_solvers().returning(|| Ok(()));
		mock.expect_fetch_quotes().returning(|_| {
			Ok((
				vec![],
				AggregationMetadata {
					total_duration_ms: 100,
					solver_timeout_ms: 2500,
					global_timeout_ms: 5000,
					early_termination: false,
					total_solvers_available: 0,
					solvers_queried: 0,
					solvers_responded_success: 0,
					solvers_responded_error: 0,
					solvers_timed_out: 0,
					min_quotes_required: 30,
					solver_selection_mode: SolverSelection::All,
				},
			))
		});

		// Use the mock
		assert!(mock.validate_solvers().is_ok());

		let request = create_valid_quote_request();
		let (quotes, _metadata) = mock.fetch_quotes(request).await.unwrap();
		assert_eq!(quotes.len(), 0);
	}

	#[tokio::test]
	async fn test_aggregator_creation() {
		let aggregator = create_test_aggregator();
		assert_eq!(aggregator.solvers.len(), 0);
		assert_eq!(aggregator.global_timeout_ms, 5000); // DEFAULT_GLOBAL_TIMEOUT_MS
	}

	#[tokio::test]
	async fn test_aggregator_creation_with_solvers() {
		let aggregator = create_test_aggregator_with_solvers(5);
		assert_eq!(aggregator.solvers.len(), 5);

		// Verify all solvers are present
		for i in 1..=5 {
			let solver_id = format!("solver{}", i);
			assert!(aggregator.solvers.contains_key(&solver_id));
		}
	}

	#[test]
	fn test_validate_solvers_success() {
		let aggregator = create_test_aggregator();
		let result = aggregator.validate_solvers();
		assert!(result.is_ok());
	}

	#[test]
	fn test_validate_solvers_missing_adapter() {
		let solver = Solver::new(
			"test-solver".to_string(),
			"non-existent-adapter".to_string(),
			"http://localhost:8001".to_string(),
			5000,
		);
		let aggregator = AggregatorService::new(
			vec![solver],
			Arc::new(AdapterRegistry::new()),
			5000,
			Arc::new(crate::integrity::IntegrityService::new(SecretString::from(
				"test-secret",
			))) as Arc<dyn IntegrityTrait>,
			Arc::new(SolverFilterService::new()) as Arc<dyn SolverFilterTrait>,
		);

		let result = aggregator.validate_solvers();
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("unknown adapter"));
	}

	// =============================================================================
	// QUOTE FETCHING ERROR CASES
	// =============================================================================

	#[tokio::test]
	async fn test_fetch_quotes_no_solvers() {
		let aggregator = create_test_aggregator();
		let request = create_valid_quote_request();

		let result = aggregator.fetch_quotes(request).await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			AggregatorServiceError::NoSolversAvailable
		));
	}

	#[tokio::test]
	async fn test_fetch_quotes_validation_error_empty_inputs() {
		let aggregator = create_test_aggregator();
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

	#[tokio::test]
	async fn test_fetch_quotes_validation_error_empty_outputs() {
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
			requested_outputs: vec![], // Empty - should fail validation
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

	// =============================================================================
	// AGGREGATION SERVICE INTEGRATION TESTS
	// =============================================================================

	#[tokio::test]
	async fn test_fetch_quotes_with_solver_filtering_no_results() {
		let aggregator = create_test_aggregator_with_solvers(3);

		let options = SolverOptions {
			include_solvers: Some(vec!["nonexistent".to_string()]),
			exclude_solvers: None,
			timeout: Some(1000),
			solver_timeout: Some(500),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let result = aggregator.fetch_quotes(request).await;

		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			AggregatorServiceError::NoSolversAvailable
		));
	}

	#[tokio::test]
	async fn test_fetch_quotes_uses_custom_timeout() {
		let aggregator = create_test_aggregator_with_solvers(1);

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(50), // Very short timeout
			solver_timeout: Some(25),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should fail quickly due to short timeout and no real adapters
		assert!(elapsed.as_millis() < 200); // Should complete within 200ms
		assert!(result.is_err());

		// Check what error we actually got
		match result.unwrap_err() {
			AggregatorServiceError::AllSolversFailed => {
				// Expected
			},
			AggregatorServiceError::ValidationError(_) => {
				// Also acceptable - timeout validation might fail
			},
			other => panic!("Unexpected error type: {:?}", other),
		}
	}

	#[tokio::test]
	async fn test_fetch_quotes_with_reduced_solver_set() {
		let aggregator = create_test_aggregator_with_solvers(5);

		let options = SolverOptions {
			include_solvers: Some(vec!["solver1".to_string(), "solver3".to_string()]),
			exclude_solvers: None,
			timeout: Some(100),
			solver_timeout: Some(50),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let result = aggregator.fetch_quotes(request).await;

		// Should fail because we don't have real adapters, but it should attempt only 2 solvers
		assert!(result.is_err());

		// Check what error we actually got
		match result.unwrap_err() {
			AggregatorServiceError::AllSolversFailed => {
				// Expected
			},
			AggregatorServiceError::ValidationError(_) => {
				// Also acceptable
			},
			other => panic!("Unexpected error type: {:?}", other),
		}
	}

	// =============================================================================
	// VALIDATION AND ERROR HANDLING TESTS
	// =============================================================================

	#[tokio::test]
	async fn test_fetch_quotes_validation_zero_amount() {
		let aggregator = create_test_aggregator_with_solvers(1);
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
				amount: U256::from(0u64), // Zero amount - should fail validation
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
			AggregatorServiceError::ValidationError(_)
		));
	}

	#[tokio::test]
	async fn test_fetch_quotes_invalid_user_address() {
		let aggregator = create_test_aggregator_with_solvers(1);

		// This will fail during InteropAddress parsing, but let's test validation
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();
		let invalid_user =
			InteropAddress::from_text("eip155:1:0x0000000000000000000000000000000000000000")
				.unwrap();

		let request = QuoteRequest {
			user: invalid_user.clone(),
			available_inputs: vec![AvailableInput {
				user: user.clone(), // Different user in input vs request user
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

		// This particular validation might not catch the user mismatch,
		// but it demonstrates the validation flow
		let _result = aggregator.fetch_quotes(request).await;
		// The request is technically valid even with different users, so it might not fail here
		// but would fail at the solver adapter level
	}

	#[tokio::test]
	async fn test_early_termination_behavior() {
		// Test that early termination works as expected
		let aggregator = create_test_aggregator_with_solvers(10); // 10 solvers

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(5000),       // 5 second global timeout
			solver_timeout: Some(100), // Very short solver timeout to force failures
			min_quotes: Some(1),       // Only need 1 quote for early termination
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);

		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should fail quickly since we don't have real adapters, but test the timing
		// The key is that it should NOT wait for the full 5 second timeout
		assert!(
			elapsed < std::time::Duration::from_millis(1000),
			"Early termination should fail quickly, took {}ms",
			elapsed.as_millis()
		);

		// Should get AllSolversFailed error since we don't have real adapters
		assert!(result.is_err());
		match result.unwrap_err() {
			AggregatorServiceError::AllSolversFailed => {
				// Expected - no real adapters means no quotes
			},
			AggregatorServiceError::NoSolversAvailable => {
				// Also acceptable
			},
			other => panic!("Unexpected error type: {:?}", other),
		}
	}

	#[tokio::test]
	async fn test_early_termination_timing_behavior() {
		// Test that we can observe the timing difference with different min_quotes
		let aggregator = create_test_aggregator_with_solvers(20); // 20 solvers

		// Test with high min_quotes requirement
		let high_requirement_options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(2000),       // 2 second timeout
			solver_timeout: Some(100), // Short per-solver timeout
			min_quotes: Some(50),      // Impossible to meet with 20 solvers
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request_high = create_quote_request_with_options(high_requirement_options);

		let start_time = std::time::Instant::now();
		let _result_high = aggregator.fetch_quotes(request_high).await;
		let elapsed_high = start_time.elapsed();

		// Test with low min_quotes requirement
		let low_requirement_options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(2000),       // Same 2 second timeout
			solver_timeout: Some(100), // Same short per-solver timeout
			min_quotes: Some(1),       // Easy to meet (would terminate early if we had quotes)
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request_low = create_quote_request_with_options(low_requirement_options);

		let start_time = std::time::Instant::now();
		let _result_low = aggregator.fetch_quotes(request_low).await;
		let elapsed_low = start_time.elapsed();

		// Both should fail quickly since no real adapters, but timing should be similar
		// The point is to test that the early termination logic doesn't add significant overhead
		assert!(elapsed_high < std::time::Duration::from_millis(1500));
		assert!(elapsed_low < std::time::Duration::from_millis(1500));

		// The difference should be minimal since both fail due to no real adapters
		let timing_difference = elapsed_high.as_millis().abs_diff(elapsed_low.as_millis());
		assert!(
			timing_difference < 500,
			"Timing difference should be minimal, was {}ms",
			timing_difference
		);
	}

	#[tokio::test]
	async fn test_aggregator_delegates_to_filter_service() {
		let aggregator = create_test_aggregator_with_solvers(5);

		// Test that aggregator properly delegates filtering to SolverFilterService
		let options = SolverOptions {
			include_solvers: Some(vec!["solver1".to_string(), "solver3".to_string()]),
			exclude_solvers: None,
			timeout: Some(5000),
			solver_timeout: Some(2500),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let result = aggregator.fetch_quotes(request).await;

		// Should fail due to no real adapters, but the filtering should have worked
		// (we can't test the filtering result directly, but we can ensure no filtering errors)
		assert!(result.is_err());
		match result.unwrap_err() {
			AggregatorServiceError::AllSolversFailed => {
				// Expected - no real adapters but filtering worked
			},
			AggregatorServiceError::NoSolversAvailable => {
				// Would indicate filtering excluded all solvers (also valid)
			},
			other => {
				// Should not get validation errors since filtering logic is delegated
				println!("Got error: {:?}", other);
			},
		}
	}

	#[tokio::test]
	async fn test_metadata_collection_and_structure() {
		let aggregator = create_test_aggregator_with_solvers(3);

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(1000),
			solver_timeout: Some(500),
			min_quotes: Some(2),
			solver_selection: Some(SolverSelection::Sampled),
			sample_size: Some(2),
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should fail due to no real adapters, but we should get metadata
		assert!(result.is_err());
		match result.unwrap_err() {
			AggregatorServiceError::AllSolversFailed => {
				// Expected - can't verify metadata from error case
			},
			AggregatorServiceError::NoSolversAvailable => {
				// Also expected - can't verify metadata from error case
			},
			other => {
				// Test focused on ensuring proper delegation, not specific errors
				println!("Got error: {:?}", other);
			},
		}

		// Verify that the aggregation completed in reasonable time
		assert!(elapsed < std::time::Duration::from_millis(2000));
	}
}
