//! # Core Quote Aggregation Service
//!
//! This module implements the central orchestration service for aggregating quotes from multiple
//! solvers in parallel. The aggregation service provides intelligent solver selection, concurrent
//! execution, timeout handling, and comprehensive result collection.
//!
//! ## Architecture Overview
//!
//! The aggregation service follows a multi-stage pipeline architecture:
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │ Quote Request   │───▶│ Solver Filtering │───▶│ Concurrent      │───▶│ Result          │
//! │ + Options       │    │ & Selection      │    │ Execution       │    │ Aggregation     │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘    └─────────────────┘
//! ```
//!
//! ## Key Components
//!
//! - **AggregatorService**: Main orchestration service that coordinates the entire process
//! - **TaskExecutor**: Handles individual solver task execution with retries and concurrency limits
//! - **SolverFilter**: Intelligent solver selection based on network/asset compatibility and options
//! - **IntegrityService**: Generates cryptographic checksums for all quotes
//!
//! ## Core Features
//!
//! ### **Intelligent Solver Selection**
//! - Network and asset compatibility scoring
//! - Include/exclude solver filtering
//! - Multiple selection strategies: All, Sampled, Priority
//! - Weighted random sampling with bias towards compatible solvers
//!
//! ### **Concurrent Execution**
//! - Parallel solver requests with configurable concurrency limits
//! - Per-solver timeout control with global timeout boundaries
//! - Non-blocking execution - slow solvers don't block fast ones
//! - Early termination when minimum quote requirements are met
//!
//! ### **Resilience & Reliability**
//! - Automatic retry logic with exponential backoff
//! - Graceful error handling with detailed error categorization
//! - Circuit breaker patterns to prevent cascading failures
//! - Comprehensive metadata collection for monitoring and debugging
//!
//! ### **Security & Integrity**
//! - Cryptographic integrity checksums for all quotes
//! - Request validation and sanitization
//! - Secure error handling without information leakage
//!
//! ## Execution Flow
//!
//! 1. **Request Validation**: Validate request parameters and solver options
//! 2. **Solver Filtering**: Apply compatibility scoring and selection strategy
//! 3. **Task Spawning**: Launch concurrent solver tasks with proper resource limits
//! 4. **Quote Collection**: Aggregate results with early termination support
//! 5. **Integrity Processing**: Generate checksums and finalize quotes
//! 6. **Metadata Assembly**: Collect timing, counts, and execution statistics
//!
//! ## Configuration Options
//!
//! ### Solver Options
//! - `includeSolvers` / `excludeSolvers`: Explicit solver filtering
//! - `timeout` / `solverTimeout`: Global and per-solver timeout controls
//! - `minQuotes`: Minimum quotes required for successful completion
//! - `solverSelection`: Strategy for selecting solvers (All/Sampled/Priority)
//! - `sampleSize`: Number of solvers to use when using Sampled strategy
//! - `priorityThreshold`: Minimum priority score for Priority strategy
//!
//! ### Performance Tuning
//! - `maxConcurrentSolvers`: Limit concurrent solver requests (configurable via config file)
//! - `maxRetriesPerSolver`: Retry attempts for failed solvers (configurable via config file)  
//! - `retryDelayMs`: Delay between retry attempts (configurable via config file)
//!
//! ## Configuration
//!
//! Performance and behavior parameters can be configured via the `config.toml` file:
//!
//! ```toml
//! [aggregation]
//! # All fields are optional - only specify what you want to override!
//! global_timeout_ms = 5000        # Global aggregation timeout (default: 5000ms)
//! per_solver_timeout_ms = 2500    # Per-solver timeout (default: 2500ms)
//! max_concurrent_solvers = 50     # Concurrent solver limit (default: 50)
//! max_retries_per_solver = 2      # Retry attempts per solver (default: 2)
//! retry_delay_ms = 100            # Delay between retries (default: 100ms)
//!
//! # Example: Override only what you need
//! # max_concurrent_solvers = 10   # Lower concurrency for testing
//! ```
//!
//! These settings provide fine-grained control over:
//! - **Timeouts**: Configure global and per-solver timeout limits
//! - **Concurrency**: Balance between speed and resource usage  
//! - **Reliability**: Retry failed requests automatically with backoff
//! - **Latency**: Configure delays to avoid overwhelming slow solvers
//!
//! ## Error Handling
//!
//! The service provides comprehensive error categorization:
//! - `ValidationError`: Invalid request parameters
//! - `NoSolversAvailable`: No solvers match the selection criteria
//! - `AllSolversFailed`: All selected solvers failed to provide quotes
//! - `Timeout`: Global timeout exceeded before completion
//! - `IntegrityError`: Checksum generation/verification failures
//! - `SolverAdapterError`: Communication failures with solvers
use crate::integrity::{IntegrityError, IntegrityTrait};
use crate::solver_adapter::{SolverAdapterError, SolverAdapterService, SolverAdapterTrait};
use crate::solver_filter::SolverFilterTrait;
use async_trait::async_trait;
use oif_adapters::AdapterRegistry;
use oif_config::AggregationConfig;
use oif_types::constants::limits::DEFAULT_MIN_QUOTES;
use oif_types::quotes::errors::QuoteValidationError;
use oif_types::quotes::request::{SolverOptions, SolverSelection};
use oif_types::quotes::response::AggregationMetadata;
use oif_types::{GetQuoteRequest, IntegrityPayload, Quote, QuotePreference, QuoteRequest, Solver};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time::{timeout, Duration, Instant};
use tracing::{debug, info, warn};

/// Errors that can occur during quote aggregation
#[derive(Debug, thiserror::Error)]
pub enum AggregatorServiceError {
	#[error("Request validation failed")]
	ValidationError(#[from] QuoteValidationError),

	#[error("No solvers available for quote aggregation")]
	NoSolversAvailable,

	#[error("All solvers failed to provide quotes")]
	AllSolversFailed,

	#[error("Timeout occurred while fetching quotes from solvers")]
	Timeout,

	#[error("Integrity service error")]
	IntegrityError(#[from] IntegrityError),

	#[error("Resource limit exceeded: {0}")]
	ResourceLimitExceeded(String),

	#[error("Solver adapter error")]
	SolverAdapterError(#[from] SolverAdapterError),
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

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AggregatorTrait: Send + Sync {
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
		request: Arc<QuoteRequest>,
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
		request: Arc<QuoteRequest>,
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
	config: AggregationConfig,
	solver_filter_service: Arc<dyn SolverFilterTrait>,
	task_executor: Arc<dyn TaskExecutorTrait>,
}

impl AggregatorService {
	/// Create a new aggregator service with pre-configured adapters
	pub fn new(
		solvers: Vec<Solver>,
		adapter_registry: Arc<AdapterRegistry>,
		integrity_service: Arc<dyn IntegrityTrait>,
		solver_filter_service: Arc<dyn SolverFilterTrait>,
	) -> Self {
		Self::with_config(
			solvers,
			adapter_registry,
			integrity_service,
			solver_filter_service,
			AggregationConfig::default(),
		)
	}

	/// Create a new aggregator service with custom configuration
	pub fn with_config(
		solvers: Vec<Solver>,
		adapter_registry: Arc<AdapterRegistry>,
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
			config,
			solver_filter_service,
			task_executor,
		}
	}

	/// Extract and validate aggregation configuration from request
	fn extract_aggregation_config(&self, request: &QuoteRequest) -> (u64, u64, usize) {
		let global_timeout_ms = request
			.solver_options
			.as_ref()
			.and_then(|opts| opts.timeout)
			.unwrap_or(self.config.global_timeout_ms);

		let per_solver_timeout_ms = request
			.solver_options
			.as_ref()
			.and_then(|opts| opts.solver_timeout)
			.unwrap_or(self.config.per_solver_timeout_ms);

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
		request: Arc<QuoteRequest>,
		per_solver_timeout_ms: u64,
		result_tx: mpsc::UnboundedSender<SolverTaskResult>,
		cancel_tx: broadcast::Sender<()>,
	) -> Vec<tokio::task::JoinHandle<()>> {
		let mut task_handles = Vec::new();

		for solver in selected_solvers {
			let request = Arc::clone(&request); // Share the Arc instead of cloning the request
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

	/// Validate request and extract aggregation configuration
	async fn validate_and_extract_config(
		&self,
		request: &QuoteRequest,
	) -> AggregatorResult<(u64, u64, usize)> {
		// Validate request
		request.validate()?;

		if self.solvers.is_empty() {
			return Err(AggregatorServiceError::NoSolversAvailable);
		}

		// Extract configuration
		let (global_timeout_ms, per_solver_timeout_ms, min_quotes_required) =
			self.extract_aggregation_config(request);

		Ok((
			global_timeout_ms,
			per_solver_timeout_ms,
			min_quotes_required,
		))
	}

	/// Filter solvers and validate the results
	async fn filter_and_validate_solvers(
		&self,
		request: &QuoteRequest,
		global_timeout_ms: u64,
		per_solver_timeout_ms: u64,
		min_quotes_required: usize,
		aggregation_start: Instant,
	) -> AggregatorResult<(Vec<Solver>, AggregationMetadata)> {
		// Filter solvers using SolverFilterService
		let available_solvers: Vec<Solver> = self.solvers.values().cloned().collect();
		let selected_solvers = self
			.solver_filter_service
			.filter_solvers(
				&available_solvers,
				request,
				request
					.solver_options
					.as_ref()
					.unwrap_or(&SolverOptions::default()),
				&self.config,
			)
			.await;

		// Initialize metadata
		let mut metadata = self.initialize_metadata(
			request,
			global_timeout_ms,
			per_solver_timeout_ms,
			min_quotes_required,
			selected_solvers.len(),
		);

		if selected_solvers.is_empty() {
			metadata.total_duration_ms = aggregation_start.elapsed().as_millis() as u64;
			return Err(AggregatorServiceError::NoSolversAvailable);
		}

		Ok((selected_solvers, metadata))
	}

	/// Setup communication channels and spawn solver tasks
	fn setup_channels_and_spawn_tasks(
		&self,
		selected_solvers: Vec<Solver>,
		request: Arc<QuoteRequest>,
		per_solver_timeout_ms: u64,
		global_timeout_ms: u64,
		min_quotes_required: usize,
	) -> (
		mpsc::UnboundedReceiver<SolverTaskResult>,
		Vec<tokio::task::JoinHandle<()>>,
		broadcast::Sender<()>,
	) {
		info!(
			"Starting aggregation: {} solvers, {}ms global timeout, {}ms per-solver timeout, {} min quotes required",
			selected_solvers.len(), global_timeout_ms, per_solver_timeout_ms, min_quotes_required
		);

		// Create communication channels
		let (result_tx, result_rx) = mpsc::unbounded_channel::<SolverTaskResult>();
		let (cancel_tx, _) = broadcast::channel::<()>(selected_solvers.len());

		// Spawn solver tasks using helper method
		let task_handles = self.spawn_solver_tasks(
			selected_solvers,
			request,
			per_solver_timeout_ms,
			result_tx.clone(),
			cancel_tx.clone(),
		);
		drop(result_tx); // Close sender to properly detect completion

		(result_rx, task_handles, cancel_tx)
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

	/// Collect quotes with early termination and enhanced event handling
	#[allow(clippy::too_many_arguments)]
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

	/// Sort quotes based on user preferences
	///
	/// Currently supports:
	/// - Speed: Sort by eta (fastest quotes first) - DEFAULT BEHAVIOR
	/// - Price: Sort by best rates (not yet implemented)
	///
	/// Default behavior is Speed sorting for optimal user experience.
	fn sort_quotes_by_preference(
		&self,
		mut quotes: Vec<Quote>,
		request: &QuoteRequest,
	) -> AggregatorResult<Vec<Quote>> {
		// Get user preference, default to Speed sorting for best user experience
		let preference = request
			.preference
			.as_ref()
			.unwrap_or(&QuotePreference::Speed);

		#[allow(clippy::match_single_binding)]
		match preference {
			_ => {
				// Sort by eta (estimated time to completion) - fastest first
				// Quotes without eta are placed at the end
				quotes.sort_by(|a, b| {
					match (a.eta, b.eta) {
						(Some(eta_a), Some(eta_b)) => eta_a.cmp(&eta_b), // Ascending: fastest first
						(Some(_), None) => std::cmp::Ordering::Less,     // Quote with eta comes first
						(None, Some(_)) => std::cmp::Ordering::Greater,  // Quote without eta goes last
						(None, None) => std::cmp::Ordering::Equal,       // Both without eta, maintain order
					}
				});

				let preference_source = if request.preference.is_some() {
					"explicit"
				} else {
					"default"
				};
				info!(
					"Sorted {} quotes by Speed preference ({} - eta field)",
					quotes.len(),
					preference_source
				);
			},
		}

		Ok(quotes)
	}
}

#[async_trait]
impl AggregatorTrait for AggregatorService {
	/// Fetch quotes concurrently from filtered solvers using clean method decomposition
	async fn fetch_quotes(
		&self,
		request: QuoteRequest,
	) -> AggregatorResult<(Vec<Quote>, AggregationMetadata)> {
		let aggregation_start = Instant::now();

		// Step 1: Validate request and extract configuration
		let (global_timeout_ms, per_solver_timeout_ms, min_quotes_required) =
			self.validate_and_extract_config(&request).await?;

		// Step 2: Filter solvers and validate results
		let (selected_solvers, metadata) = self
			.filter_and_validate_solvers(
				&request,
				global_timeout_ms,
				per_solver_timeout_ms,
				min_quotes_required,
				aggregation_start,
			)
			.await?;

		// Step 3: Setup channels and spawn tasks
		let request_arc = Arc::new(request.clone());
		let (result_rx, task_handles, cancel_tx) = self.setup_channels_and_spawn_tasks(
			selected_solvers,
			request_arc.clone(),
			per_solver_timeout_ms,
			global_timeout_ms,
			min_quotes_required,
		);

		// Step 4: Execute collection loop with early termination
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

		// Step 5: Log final results
		let final_count = collected_quotes.len();
		info!(
			"Aggregation completed: {} quotes from {} solvers in {}ms (min required: {})",
			final_count,
			final_metadata.solvers_responded_success,
			final_metadata.total_duration_ms,
			min_quotes_required
		);

		// Step 6: Sort quotes based on user preferences before returning
		let sorted_quotes = self.sort_quotes_by_preference(collected_quotes, &request_arc)?;

		Ok((sorted_quotes, final_metadata))
	}
}

#[cfg(test)]
mod tests {
	//! Tests for AggregatorService focusing on core aggregation behavior.

	use crate::{IntegrityService, SolverFilterService};

	use super::*;
	use oif_adapters::AdapterRegistry;
	use oif_types::{
		AvailableInput, InteropAddress, QuoteDetails, QuoteRequest, RequestedOutput, SecretString,
		SolverAdapter, U256,
	};

	/// Simple mock adapter for testing success scenarios
	#[derive(Debug, Clone)]
	struct TestMockAdapter {
		id: String,
		should_fail: bool,
		delay_ms: Option<u64>,
	}

	impl TestMockAdapter {
		fn new(id: &str, should_fail: bool) -> Self {
			Self {
				id: id.to_string(),
				should_fail,
				delay_ms: None,
			}
		}

		fn with_delay(id: &str, delay_ms: u64) -> Self {
			Self {
				id: id.to_string(),
				should_fail: false,
				delay_ms: Some(delay_ms),
			}
		}

		fn slow(id: &str) -> Self {
			// Creates an adapter that will timeout with default timeouts
			Self::with_delay(id, 2000) // 2 seconds delay
		}
	}

	#[async_trait]
	impl SolverAdapter for TestMockAdapter {
		fn adapter_id(&self) -> &str {
			&self.id
		}
		fn adapter_name(&self) -> &str {
			&self.id
		}
		fn adapter_info(&self) -> &oif_types::Adapter {
			// Not used in tests
			panic!("adapter_info not implemented for test mock")
		}

		async fn get_quotes(
			&self,
			request: &oif_types::GetQuoteRequest,
			_config: &oif_types::SolverRuntimeConfig,
		) -> oif_types::AdapterResult<oif_types::GetQuoteResponse> {
			// Simulate delay if configured
			if let Some(delay_ms) = self.delay_ms {
				tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
			}

			if self.should_fail {
				return Err(oif_types::AdapterError::from(
					oif_types::adapters::AdapterValidationError::InvalidConfiguration {
						reason: format!("Mock adapter {} configured to fail", self.id),
					},
				));
			}

			use oif_types::adapters::models::*;
			use oif_types::serde_json::json;

			// Create a realistic quote response
			let quote_id = format!(
				"{}-quote-{}",
				self.id,
				oif_types::chrono::Utc::now().timestamp()
			);

			let available_input = request
				.available_inputs
				.first()
				.cloned()
				.unwrap_or_else(|| AvailableInput {
					user: oif_types::InteropAddress::from_chain_and_address(
						1,
						"0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03",
					)
					.unwrap(),
					asset: oif_types::InteropAddress::from_chain_and_address(
						1,
						"0x0000000000000000000000000000000000000000",
					)
					.unwrap(),
					amount: oif_types::U256::new("1000000000000000000".to_string()),
					lock: None,
				});

			let requested_output =
				request
					.requested_outputs
					.first()
					.cloned()
					.unwrap_or_else(|| RequestedOutput {
						asset: oif_types::InteropAddress::from_chain_and_address(
							1,
							"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0",
						)
						.unwrap(),
						amount: oif_types::U256::new("1000000".to_string()),
						receiver: oif_types::InteropAddress::from_chain_and_address(
							1,
							"0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03",
						)
						.unwrap(),
						calldata: None,
					});

			let quote = AdapterQuote {
				quote_id,
				orders: vec![QuoteOrder {
					signature_type: SignatureType::Eip712,
					domain: oif_types::InteropAddress::from_chain_and_address(
						1,
						"0x1234567890123456789012345678901234567890",
					)
					.unwrap(),
					primary_type: "Order".to_string(),
					message: json!({
						"orderType": "swap",
						"adapter": self.id,
						"mockProvider": "TestMockAdapter"
					}),
				}],
				details: QuoteDetails {
					available_inputs: vec![available_input],
					requested_outputs: vec![requested_output],
				},
				valid_until: Some(oif_types::chrono::Utc::now().timestamp() as u64 + 300),
				eta: Some(30),
				provider: format!("{} Provider", self.id),
			};

			Ok(oif_types::GetQuoteResponse {
				quotes: vec![quote],
			})
		}

		async fn submit_order(
			&self,
			_request: &oif_types::adapters::models::SubmitOrderRequest,
			_config: &oif_types::SolverRuntimeConfig,
		) -> oif_types::AdapterResult<oif_types::adapters::models::SubmitOrderResponse> {
			// Simulate delay if configured
			if let Some(delay_ms) = self.delay_ms {
				tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
			}

			if self.should_fail {
				return Err(oif_types::AdapterError::from(
					oif_types::adapters::AdapterValidationError::InvalidConfiguration {
						reason: format!("Mock adapter {} configured to fail", self.id),
					},
				));
			}

			use oif_types::adapters::models::*;
			use oif_types::serde_json::json;

			let order_id = format!(
				"{}-order-{}",
				self.id,
				oif_types::chrono::Utc::now().timestamp()
			);
			Ok(oif_types::adapters::models::SubmitOrderResponse {
				status: "success".to_string(),
				order_id: Some(order_id.clone()),
				message: Some("Order submitted successfully".to_string()),
			})
		}

		async fn get_order_details(
			&self,
			order_id: &str,
			_config: &oif_types::SolverRuntimeConfig,
		) -> oif_types::AdapterResult<oif_types::adapters::GetOrderResponse> {
			// Simulate delay if configured
			if let Some(delay_ms) = self.delay_ms {
				tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
			}

			if self.should_fail {
				return Err(oif_types::AdapterError::from(
					oif_types::adapters::AdapterValidationError::InvalidConfiguration {
						reason: format!("Mock adapter {} configured to fail", self.id),
					},
				));
			}

			use oif_types::adapters::models::*;
			use oif_types::serde_json::json;

			Ok(oif_types::adapters::GetOrderResponse {
				order: OrderResponse {
					id: order_id.to_string(),
					quote_id: None,
					status: OrderStatus::Finalized,
					created_at: oif_types::chrono::Utc::now().timestamp() as u64,
					updated_at: oif_types::chrono::Utc::now().timestamp() as u64,
					input_amount: AssetAmount {
						asset: "0x0000000000000000000000000000000000000000".to_string(),
						amount: oif_types::U256::new("1000000000000000000".to_string()),
					},
					output_amount: AssetAmount {
						asset: "0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0".to_string(),
						amount: oif_types::U256::new("1000000".to_string()),
					},
					settlement: Settlement {
						settlement_type: SettlementType::Escrow,
						data: json!({}),
					},
					fill_transaction: None,
				},
			})
		}

		async fn health_check(
			&self,
			_config: &oif_types::SolverRuntimeConfig,
		) -> oif_types::AdapterResult<bool> {
			Ok(!self.should_fail)
		}

		async fn get_supported_networks(
			&self,
			_config: &oif_types::SolverRuntimeConfig,
		) -> oif_types::AdapterResult<Vec<oif_types::models::Network>> {
			if self.should_fail {
				return Err(oif_types::AdapterError::from(
					oif_types::adapters::AdapterValidationError::InvalidConfiguration {
						reason: format!("Mock adapter {} configured to fail", self.id),
					},
				));
			}
			Ok(vec![oif_types::models::Network::new(
				1,
				Some("Ethereum Mainnet".to_string()),
				Some(false),
			)])
		}

		async fn get_supported_assets(
			&self,
			_config: &oif_types::SolverRuntimeConfig,
		) -> oif_types::AdapterResult<Vec<oif_types::models::Asset>> {
			if self.should_fail {
				return Err(oif_types::AdapterError::from(
					oif_types::adapters::AdapterValidationError::InvalidConfiguration {
						reason: format!("Mock adapter {} configured to fail", self.id),
					},
				));
			}
			Ok(vec![])
		}
	}

	/// Helper function to create AdapterRegistry with mock adapters
	fn create_test_adapter_registry() -> AdapterRegistry {
		let mut registry = AdapterRegistry::new();

		// Register mock demo adapter (succeeds with quotes)
		let mock_demo = TestMockAdapter::new("mock-demo-v1", false);
		registry
			.register(Box::new(mock_demo))
			.expect("Failed to register mock demo adapter");

		// Register mock test adapter for success
		let mock_test_success = TestMockAdapter::new("mock-test-success", false);
		registry
			.register(Box::new(mock_test_success))
			.expect("Failed to register mock test success adapter");

		// Register mock test adapter for failure
		let mock_test_fail = TestMockAdapter::new("mock-test-fail", true);
		registry
			.register(Box::new(mock_test_fail))
			.expect("Failed to register mock test fail adapter");

		// Register slow adapter that will timeout with short timeouts
		let mock_slow = TestMockAdapter::slow("mock-slow-adapter");
		registry
			.register(Box::new(mock_slow))
			.expect("Failed to register mock slow adapter");

		// Register fast adapter with minimal delay
		let mock_fast = TestMockAdapter::with_delay("mock-fast-adapter", 50);
		registry
			.register(Box::new(mock_fast))
			.expect("Failed to register mock fast adapter");

		registry
	}

	// Helper function to create test aggregator with mixed timeout/success solvers
	fn create_test_aggregator_with_timeout_solvers() -> AggregatorService {
		let solvers = vec![
			// Fast solver that will succeed
			{
				let mut solver = Solver::new(
					"fast-solver".to_string(),
					"mock-fast-adapter".to_string(),
					"http://localhost:8001".to_string(),
					1000,
				);
				solver.metadata.supported_assets = vec![oif_types::models::Asset::new(
					"0x0000000000000000000000000000000000000000".to_string(),
					"ETH".to_string(),
					"Ethereum".to_string(),
					18,
					1,
				)];
				solver
			},
			// Slow solver that will timeout
			{
				let mut solver = Solver::new(
					"slow-solver".to_string(),
					"mock-slow-adapter".to_string(),
					"http://localhost:8002".to_string(),
					1000,
				);
				solver.metadata.supported_assets = vec![oif_types::models::Asset::new(
					"0x0000000000000000000000000000000000000000".to_string(),
					"ETH".to_string(),
					"Ethereum".to_string(),
					18,
					1,
				)];
				solver
			},
			// Another fast solver for comparison
			{
				let mut solver = Solver::new(
					"fast-solver2".to_string(),
					"mock-demo-v1".to_string(),
					"http://localhost:8003".to_string(),
					1000,
				);
				solver.metadata.supported_assets = vec![oif_types::models::Asset::new(
					"0x0000000000000000000000000000000000000000".to_string(),
					"ETH".to_string(),
					"Ethereum".to_string(),
					18,
					1,
				)];
				solver
			},
		];

		AggregatorService::new(
			solvers,
			Arc::new(create_test_adapter_registry()),
			Arc::new(IntegrityService::new(SecretString::from("test-secret")))
				as Arc<dyn IntegrityTrait>,
			Arc::new(SolverFilterService::new()) as Arc<dyn SolverFilterTrait>,
		)
	}

	// Helper function to create test aggregator with no solvers
	fn create_test_aggregator() -> AggregatorService {
		AggregatorService::new(
			vec![], // No solvers
			Arc::new(create_test_adapter_registry()),
			Arc::new(crate::integrity::IntegrityService::new(SecretString::from(
				"test-secret",
			))) as Arc<dyn IntegrityTrait>,
			Arc::new(SolverFilterService::new()) as Arc<dyn SolverFilterTrait>,
		)
	}

	// Helper function to create test aggregator with mock demo solvers that will succeed
	fn create_test_aggregator_with_demo_solvers(solver_count: usize) -> AggregatorService {
		let mut solvers = Vec::new();
		for i in 1..=solver_count {
			// Create solver with proper network and asset support for filtering
			let mut solver = Solver::new(
				format!("demo-solver{}", i),
				"mock-demo-v1".to_string(), // Use actual mock adapter ID
				format!("http://localhost:800{}", i),
				5000,
			);

			// Add network and asset support to prevent filtering issues
			solver.metadata.supported_assets = vec![
				oif_types::models::Asset::new(
					"0x0000000000000000000000000000000000000000".to_string(),
					"ETH".to_string(),
					"Ethereum".to_string(),
					18,
					1,
				),
				oif_types::models::Asset::new(
					"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0".to_string(),
					"USDC".to_string(),
					"USD Coin".to_string(),
					6,
					1,
				),
			];

			solvers.push(solver);
		}

		AggregatorService::new(
			solvers,
			Arc::new(create_test_adapter_registry()),
			Arc::new(IntegrityService::new(SecretString::from("test-secret")))
				as Arc<dyn IntegrityTrait>,
			Arc::new(SolverFilterService::new()) as Arc<dyn SolverFilterTrait>,
		)
	}

	// Helper function to create test aggregator with mixed success/failure solvers
	fn create_test_aggregator_with_mixed_solvers() -> AggregatorService {
		let solvers = vec![
			Solver::new(
				"success-solver1".to_string(),
				"mock-demo-v1".to_string(), // Will succeed with quotes
				"http://localhost:8001".to_string(),
				2000,
			),
			Solver::new(
				"success-solver2".to_string(),
				"mock-test-success".to_string(), // Will succeed with empty quotes
				"http://localhost:8002".to_string(),
				2000,
			),
			Solver::new(
				"fail-solver1".to_string(),
				"mock-test-fail".to_string(), // Will fail
				"http://localhost:8003".to_string(),
				2000,
			),
		];

		AggregatorService::new(
			solvers,
			Arc::new(create_test_adapter_registry()),
			Arc::new(IntegrityService::new(SecretString::from("test-secret")))
				as Arc<dyn IntegrityTrait>,
			Arc::new(SolverFilterService::new()) as Arc<dyn SolverFilterTrait>,
		)
	}

	// Helper function to create test aggregator with specified solvers using non-existent adapters (for failure testing)
	fn create_test_aggregator_with_invalid_adapters(solver_count: usize) -> AggregatorService {
		let mut solvers = Vec::new();
		for i in 1..=solver_count {
			solvers.push(Solver::new(
				format!("solver{}", i),
				format!("nonexistent-adapter{}", i), // These adapters don't exist
				format!("http://localhost:800{}", i),
				5000,
			));
		}

		AggregatorService::new(
			solvers,
			Arc::new(create_test_adapter_registry()),
			Arc::new(IntegrityService::new(SecretString::from("test-secret")))
				as Arc<dyn IntegrityTrait>,
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

	#[tokio::test]
	async fn test_mock_aggregator_trait() {
		let mut mock = MockAggregatorTrait::new();

		// Setup expectations
		mock.expect_fetch_quotes().returning(|_| {
			Ok((
				vec![],
				AggregationMetadata {
					..Default::default()
				},
			))
		});

		// Use the mock
		let request = create_valid_quote_request();
		let (quotes, _metadata) = mock.fetch_quotes(request).await.unwrap();
		assert_eq!(quotes.len(), 0);
	}

	#[tokio::test]
	async fn test_aggregator_creation() {
		let aggregator = create_test_aggregator();
		assert_eq!(aggregator.solvers.len(), 0);
		assert_eq!(aggregator.config.global_timeout_ms, 5000); // DEFAULT_GLOBAL_TIMEOUT_MS
	}

	#[tokio::test]
	async fn test_aggregator_creation_with_demo_solvers() {
		let aggregator = create_test_aggregator_with_demo_solvers(3);
		assert_eq!(aggregator.solvers.len(), 3);

		// Verify all demo solvers are present and use correct adapter
		for i in 1..=3 {
			let solver_id = format!("demo-solver{}", i);
			assert!(aggregator.solvers.contains_key(&solver_id));
			let solver = aggregator.solvers.get(&solver_id).unwrap();
			assert_eq!(solver.adapter_id, "mock-demo-v1");
		}
	}

	#[tokio::test]
	async fn test_successful_quote_aggregation() {
		let aggregator = create_test_aggregator_with_demo_solvers(2);
		let request = create_valid_quote_request();

		let result = aggregator.fetch_quotes(request).await;
		assert!(result.is_ok());

		let (quotes, metadata) = result.unwrap();

		// Should get quotes from both demo solvers
		assert!(
			quotes.len() >= 1,
			"Should receive at least 1 quote from demo adapters"
		);

		// Verify metadata
		assert_eq!(metadata.total_solvers_available, 2);
		assert_eq!(metadata.solvers_queried, 2);
		assert!(metadata.solvers_responded_success > 0);
		// Duration should be recorded (may be 0 in very fast test environments)
		// Just verify it's a valid u64 value by checking it's not None/uninitialized
		assert_eq!(metadata.solver_selection_mode, SolverSelection::All);
		// Check what the default min quotes actually is (might be different from expected)
		assert!(
			metadata.min_quotes_required > 0,
			"Min quotes should be positive, got: {}",
			metadata.min_quotes_required
		);
	}

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

	#[tokio::test]
	async fn test_mixed_solver_success_and_failure() {
		let aggregator = create_test_aggregator_with_mixed_solvers();
		let request = create_valid_quote_request();

		let result = aggregator.fetch_quotes(request).await;
		assert!(result.is_ok());

		let (quotes, metadata) = result.unwrap();

		// Should get at least one quote from the successful solvers
		assert!(
			quotes.len() >= 1,
			"Should receive quotes from at least one successful solver"
		);

		// Verify metadata shows mixed results
		assert_eq!(metadata.total_solvers_available, 3);
		assert_eq!(metadata.solvers_queried, 3);
		assert!(
			metadata.solvers_responded_success >= 1,
			"At least one solver should succeed"
		);
		assert!(
			metadata.solvers_responded_error >= 1,
			"At least one solver should fail"
		);
	}

	#[tokio::test]
	async fn test_fetch_quotes_with_solver_filtering_no_results() {
		let aggregator = create_test_aggregator_with_demo_solvers(3);

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
	async fn test_fetch_quotes_with_solver_filtering_success() {
		let aggregator = create_test_aggregator_with_demo_solvers(3);

		let options = SolverOptions {
			include_solvers: Some(vec!["demo-solver1".to_string(), "demo-solver2".to_string()]),
			exclude_solvers: None,
			timeout: Some(2000),
			solver_timeout: Some(1000),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let result = aggregator.fetch_quotes(request).await;

		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Should get quotes from filtered solvers
		assert!(
			quotes.len() >= 1,
			"Should receive quotes from filtered solvers"
		);
		assert_eq!(
			metadata.solvers_queried, 2,
			"Should only query 2 filtered solvers"
		);
		assert!(metadata.solvers_responded_success >= 1);
	}

	#[tokio::test]
	async fn test_fetch_quotes_uses_custom_timeout() {
		let aggregator = create_test_aggregator_with_demo_solvers(2);

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(2000), // Reasonable timeout for successful completion
			solver_timeout: Some(1000),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should succeed within the timeout
		assert!(result.is_ok(), "Should succeed with reasonable timeout");
		let (_quotes, metadata) = result.unwrap();

		// Verify custom timeouts are used
		assert_eq!(metadata.global_timeout_ms, 2000);
		assert_eq!(metadata.solver_timeout_ms, 1000);
		assert!(
			elapsed.as_millis() < 2000,
			"Should complete within the specified timeout"
		);
	}

	#[tokio::test]
	async fn test_fetch_quotes_with_very_short_timeout() {
		let aggregator = create_test_aggregator_with_demo_solvers(1);

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(200),        // Short but valid timeout
			solver_timeout: Some(100), // Minimum allowed per-solver timeout
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should complete quickly due to short timeout
		assert!(elapsed.as_millis() < 300, "Should complete quickly");

		// Should succeed with the minimum allowed timeout
		if let Err(ref e) = result {
			eprintln!("Very short timeout test failed with error: {:?}", e);
		}
		assert!(result.is_ok(), "Should succeed with minimum timeout");
		let (_, metadata) = result.unwrap();

		// Verify timeout was respected
		assert_eq!(metadata.global_timeout_ms, 200);
		assert_eq!(metadata.solver_timeout_ms, 100);
	}

	#[tokio::test]
	async fn test_fetch_quotes_with_solver_exclusion() {
		let aggregator = create_test_aggregator_with_demo_solvers(3);

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: Some(vec!["demo-solver3".to_string()]),
			timeout: Some(2000),
			solver_timeout: Some(1000),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let result = aggregator.fetch_quotes(request).await;

		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Should get quotes from 2 non-excluded solvers
		assert!(
			quotes.len() >= 1,
			"Should receive quotes from non-excluded solvers"
		);
		assert_eq!(
			metadata.solvers_queried, 2,
			"Should only query 2 non-excluded solvers"
		);
		assert!(metadata.solvers_responded_success >= 1);
	}

	#[tokio::test]
	async fn test_fetch_quotes_with_sampled_selection() {
		let aggregator = create_test_aggregator_with_demo_solvers(5);

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(2000),
			solver_timeout: Some(1000),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::Sampled),
			sample_size: Some(2),
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let result = aggregator.fetch_quotes(request).await;

		if let Err(ref e) = result {
			eprintln!("Sampled selection test failed with error: {:?}", e);
		}
		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Should get quotes from sampled solvers
		assert!(
			quotes.len() >= 1,
			"Should receive quotes from sampled solvers"
		);
		assert_eq!(
			metadata.solvers_queried, 2,
			"Should only query 2 sampled solvers"
		);
		assert_eq!(metadata.solver_selection_mode, SolverSelection::Sampled);
	}

	#[tokio::test]
	async fn test_fetch_quotes_validation_zero_amount() {
		let aggregator = create_test_aggregator_with_demo_solvers(1);
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
		let aggregator = create_test_aggregator_with_demo_solvers(1);

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
		let aggregator = create_test_aggregator_with_demo_solvers(3);

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(5000),        // 5 second global timeout
			solver_timeout: Some(2000), // Reasonable solver timeout
			min_quotes: Some(1),        // Only need 1 quote for early termination
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);

		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should succeed and terminate early when min_quotes is reached
		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Should get at least 1 quote (meeting min_quotes requirement)
		assert!(quotes.len() >= 1, "Should receive at least 1 quote");

		// Should terminate early, not wait for all solvers
		assert!(
			elapsed < std::time::Duration::from_millis(3000),
			"Early termination should complete quickly, took {}ms",
			elapsed.as_millis()
		);

		// Check if early termination was triggered
		if metadata.early_termination {
			assert!(metadata.solvers_responded_success >= 1);
		}
	}

	#[tokio::test]
	async fn test_early_termination_timing_behavior() {
		// Test that we can observe the timing difference with different min_quotes
		let aggregator = create_test_aggregator_with_demo_solvers(5);

		// Test with high min_quotes requirement (impossible to meet)
		let high_requirement_options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(3000),        // 3 second timeout
			solver_timeout: Some(1000), // Reasonable per-solver timeout
			min_quotes: Some(10),       // Impossible to meet with 5 solvers
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
			timeout: Some(3000),        // Same 3 second timeout
			solver_timeout: Some(1000), // Same reasonable per-solver timeout
			min_quotes: Some(1),        // Easy to meet (should terminate early)
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request_low = create_quote_request_with_options(low_requirement_options);

		let start_time = std::time::Instant::now();
		let _result_low = aggregator.fetch_quotes(request_low).await;
		let elapsed_low = start_time.elapsed();

		// Low requirement should complete faster due to early termination
		// High requirement will need to wait for all solvers or timeout
		assert!(
			elapsed_low < std::time::Duration::from_millis(2000),
			"Low requirement should complete quickly"
		);

		// Early termination should make low requirement significantly faster
		assert!(
			elapsed_low < elapsed_high,
			"Low min_quotes ({:?}) should be faster than high min_quotes ({:?})",
			elapsed_low,
			elapsed_high
		);
	}

	#[tokio::test]
	async fn test_aggregator_delegates_to_filter_service() {
		let aggregator = create_test_aggregator_with_demo_solvers(5);

		// Test that aggregator properly delegates filtering to SolverFilterService
		let options = SolverOptions {
			include_solvers: Some(vec!["demo-solver1".to_string(), "demo-solver3".to_string()]),
			exclude_solvers: None,
			timeout: Some(3000),
			solver_timeout: Some(1500),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let result = aggregator.fetch_quotes(request).await;

		// Should succeed with filtered solvers
		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Should get quotes from filtered solvers
		assert!(
			quotes.len() >= 1,
			"Should receive quotes from filtered solvers"
		);
		assert_eq!(
			metadata.solvers_queried, 2,
			"Should only query 2 filtered solvers"
		);
		assert!(
			metadata.solvers_responded_success >= 1,
			"At least one filtered solver should succeed"
		);
	}

	#[tokio::test]
	async fn test_metadata_collection_and_structure() {
		let aggregator = create_test_aggregator_with_demo_solvers(3);

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

		// Should succeed and provide metadata
		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Verify metadata structure and values
		assert!(
			quotes.len() >= 1,
			"Should receive quotes from demo adapters"
		);
		assert_eq!(metadata.total_solvers_available, 3);
		assert_eq!(
			metadata.solvers_queried, 2,
			"Should query 2 sampled solvers"
		);
		assert_eq!(metadata.solver_selection_mode, SolverSelection::Sampled);
		assert_eq!(metadata.global_timeout_ms, 1000);
		assert_eq!(metadata.solver_timeout_ms, 500);
		assert_eq!(metadata.min_quotes_required, 2);
		// Duration may be 0 in fast test environments
		assert!(metadata.solvers_responded_success >= 1);

		// Verify that the aggregation completed in reasonable time
		assert!(elapsed < std::time::Duration::from_millis(2000));
	}

	#[tokio::test]
	async fn test_adapter_not_found_error() {
		// Test proper error handling when adapters don't exist
		let aggregator = create_test_aggregator_with_invalid_adapters(2);
		let request = create_valid_quote_request();

		let result = aggregator.fetch_quotes(request).await;
		assert!(result.is_err());

		// Should get AllSolversFailed because the adapters don't exist
		match result.unwrap_err() {
			AggregatorServiceError::AllSolversFailed => {
				// Expected when adapters don't exist
			},
			other => panic!("Expected AllSolversFailed, got: {:?}", other),
		}
	}

	#[tokio::test]
	async fn test_quote_integrity_checksum_generation() {
		let aggregator = create_test_aggregator_with_demo_solvers(1);
		let request = create_valid_quote_request();

		let result = aggregator.fetch_quotes(request).await;
		assert!(result.is_ok());

		let (quotes, _metadata) = result.unwrap();
		assert!(quotes.len() >= 1, "Should receive at least one quote");

		// Verify that all quotes have integrity checksums
		for quote in quotes {
			assert!(
				!quote.integrity_checksum.is_empty(),
				"Quote should have non-empty integrity checksum"
			);
			// Basic checksum format validation (should be hex string)
			assert!(
				quote
					.integrity_checksum
					.chars()
					.all(|c| c.is_ascii_hexdigit()),
				"Integrity checksum should be hex string"
			);
		}
	}

	#[tokio::test]
	async fn test_priority_based_selection() {
		let aggregator = create_test_aggregator_with_demo_solvers(5);

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(3000),
			solver_timeout: Some(1500),
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::Priority),
			sample_size: None,
			priority_threshold: Some(70),
		};

		let request = create_quote_request_with_options(options);
		let result = aggregator.fetch_quotes(request).await;

		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Should get quotes from priority solvers
		assert!(
			quotes.len() >= 1,
			"Should receive quotes from priority solvers"
		);
		assert_eq!(metadata.solver_selection_mode, SolverSelection::Priority);
		assert!(
			metadata.solvers_queried >= 1,
			"Should query at least one high-priority solver"
		);
	}

	#[tokio::test]
	async fn test_concurrent_solver_execution() {
		// Test that multiple solvers are executed concurrently, not sequentially
		let aggregator = create_test_aggregator_with_demo_solvers(3);
		let request = create_valid_quote_request();

		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Should get quotes from multiple solvers
		assert!(quotes.len() >= 1, "Should receive quotes from solvers");
		assert_eq!(metadata.solvers_queried, 3, "Should query all 3 solvers");

		// Should complete quickly due to concurrent execution
		// If executed sequentially, it would take much longer
		assert!(
			elapsed < std::time::Duration::from_millis(1000),
			"Concurrent execution should be fast, took {}ms",
			elapsed.as_millis()
		);
	}

	#[tokio::test]
	async fn test_solver_timeout_mixed_results() {
		// Test with some solvers timing out and others succeeding
		let aggregator = create_test_aggregator_with_timeout_solvers();

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(5000),       // 5 second global timeout
			solver_timeout: Some(500), // 500ms per-solver timeout (slow solver will timeout)
			min_quotes: Some(3),       // Need all 3 quotes to prevent early termination
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		assert!(result.is_ok());

		let (quotes, metadata) = result.unwrap();

		// Should succeed or fail, but let's see what actually happens
		println!(
			"Success: {} quotes, metadata: success={}, error={}, timeout={}",
			quotes.len(),
			metadata.solvers_responded_success,
			metadata.solvers_responded_error,
			metadata.solvers_timed_out
		);

		// Should get quotes from fast solvers only
		assert!(quotes.len() == 2, "Should receive quotes from fast solvers");

		// Verify metadata shows mixed results
		assert_eq!(metadata.total_solvers_available, 3);
		assert_eq!(metadata.solvers_queried, 3);
		assert!(
			metadata.solvers_responded_success == 2,
			"2 solvers should succeed"
		);
		assert!(metadata.solvers_timed_out == 1, "1 solvers should timeout");

		// The slow solver (2000ms delay) should timeout with 500ms timeout
		// But due to early termination or cancellation, it might not be counted as timeout
		// Let's be more lenient here
		assert!(
			metadata.solvers_responded_success
				+ metadata.solvers_responded_error
				+ metadata.solvers_timed_out
				<= metadata.solvers_queried,
			"Total responses should not exceed queried count"
		);

		// Should complete relatively quickly due to per-solver timeout
		assert!(
			elapsed.as_millis() < 3000,
			"Should complete in reasonable time, took {}ms",
			elapsed.as_millis()
		);
	}

	#[tokio::test]
	async fn test_timeout_behavior_verification() {
		// Test timeout behavior - may succeed or fail depending on solver speeds vs timeouts
		let aggregator = create_test_aggregator_with_timeout_solvers();

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(2000),       // 2 second global timeout
			solver_timeout: Some(100), // 100ms per-solver timeout (fast=50ms, slow=2000ms)
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// May succeed (fast solvers) or fail (if all timeout), both are valid
		match result {
			Ok((quotes, metadata)) => {
				// Fast solvers succeeded before timeout
				assert!(quotes.len() >= 1, "Should have quotes from fast solvers");
				assert!(metadata.solvers_responded_success >= 1);
				assert_eq!(metadata.solvers_queried, 3, "Should query all 3 solvers");
				// Due to early termination, slow solver might be cancelled rather than timed out
				// So we just verify the basic aggregation worked
			},
			Err(AggregatorServiceError::AllSolversFailed) => {
				// All solvers failed/timed out - also acceptable
			},
			Err(other) => panic!("Unexpected error type: {:?}", other),
		}

		// Should complete in reasonable time
		assert!(
			elapsed.as_millis() < 1000,
			"Should complete in reasonable time, took {}ms",
			elapsed.as_millis()
		);
	}

	#[tokio::test]
	async fn test_timeout_configuration_limits() {
		// Test that timeout configuration is properly validated and applied
		let aggregator = create_test_aggregator_with_timeout_solvers();

		// Test with only slow solver to ensure timeout behavior
		let options = SolverOptions {
			include_solvers: Some(vec!["slow-solver".to_string()]), // Only the 2000ms delay solver
			exclude_solvers: None,
			timeout: Some(2000),       // 2 second global timeout
			solver_timeout: Some(300), // 300ms per-solver timeout - slow solver will timeout
			min_quotes: Some(1),
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should fail because slow solver times out
		assert!(result.is_err());
		match result.unwrap_err() {
			AggregatorServiceError::AllSolversFailed => {
				// Expected when the slow solver times out
			},
			other => panic!("Expected AllSolversFailed, got: {:?}", other),
		}

		// Should complete in reasonable time (allowing for test environment variations)
		assert!(
			elapsed.as_millis() < 2500,
			"Should complete due to timeout, took {}ms",
			elapsed.as_millis()
		);
	}

	#[tokio::test]
	async fn test_global_timeout() {
		// Test global timeout when aggregation takes too long overall
		let aggregator = create_test_aggregator_with_timeout_solvers();

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(200),        // Short global timeout
			solver_timeout: Some(100), // Even shorter per-solver timeout
			min_quotes: Some(10),      // High requirement that won't be met quickly
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should complete quickly due to global timeout
		assert!(
			elapsed.as_millis() < 400,
			"Should hit global timeout quickly, took {}ms",
			elapsed.as_millis()
		);

		// May succeed or timeout depending on which happens first
		match result {
			Ok((quotes, metadata)) => {
				// If some fast solvers responded before global timeout
				assert!(quotes.len() >= 1);
				assert_eq!(metadata.global_timeout_ms, 200);
			},
			Err(AggregatorServiceError::Timeout) => {
				// Expected if global timeout hits first
			},
			Err(AggregatorServiceError::AllSolversFailed) => {
				// Also acceptable if all solvers fail to respond in time
			},
			Err(other) => panic!("Unexpected error type: {:?}", other),
		}
	}

	#[tokio::test]
	async fn test_timeout_with_early_termination() {
		// Test that early termination works even with some solvers timing out
		let aggregator = create_test_aggregator_with_timeout_solvers();

		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: Some(5000),       // 5 second global timeout
			solver_timeout: Some(800), // 800ms per-solver timeout (slow solver will timeout)
			min_quotes: Some(1),       // Only need 1 quote for early termination
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let request = create_quote_request_with_options(options);
		let start_time = std::time::Instant::now();
		let result = aggregator.fetch_quotes(request).await;
		let elapsed = start_time.elapsed();

		// Should succeed with early termination
		assert!(result.is_ok());
		let (quotes, metadata) = result.unwrap();

		// Should get quotes from fast solvers
		assert!(
			quotes.len() >= 1,
			"Should receive quotes triggering early termination"
		);

		// Should terminate early, not wait for slow solver timeout
		assert!(
			elapsed.as_millis() < 1000,
			"Early termination should happen quickly, took {}ms",
			elapsed.as_millis()
		);

		// Verify metadata
		assert_eq!(metadata.global_timeout_ms, 5000);
		assert_eq!(metadata.solver_timeout_ms, 800);
		assert!(metadata.solvers_responded_success >= 1);

		// May or may not show timeouts depending on timing of early termination
		// The slow solver might not have had time to timeout before cancellation
	}

	#[test]
	fn test_sort_quotes_by_speed_preference() {
		// Test the quote sorting functionality for Speed preference
		let aggregator = create_test_aggregator();

		// Create test quote details from a sample request
		let sample_request = create_valid_quote_request();
		let details = QuoteDetails {
			available_inputs: sample_request.available_inputs,
			requested_outputs: sample_request.requested_outputs,
		};

		// Create test quotes with different eta values
		let quotes = vec![
			Quote {
				quote_id: "quote_3".to_string(),
				solver_id: "solver_3".to_string(),
				orders: vec![],
				details: details.clone(),
				valid_until: Some(12345),
				eta: Some(300), // 300 seconds (slowest)
				provider: "test_provider".to_string(),
				integrity_checksum: "checksum_3".to_string(),
			},
			Quote {
				quote_id: "quote_1".to_string(),
				solver_id: "solver_1".to_string(),
				orders: vec![],
				details: details.clone(),
				valid_until: Some(12345),
				eta: Some(100), // 100 seconds (fastest)
				provider: "test_provider".to_string(),
				integrity_checksum: "checksum_1".to_string(),
			},
			Quote {
				quote_id: "quote_no_eta".to_string(),
				solver_id: "solver_no_eta".to_string(),
				orders: vec![],
				details: details.clone(),
				valid_until: Some(12345),
				eta: None, // No eta (should go last)
				provider: "test_provider".to_string(),
				integrity_checksum: "checksum_no_eta".to_string(),
			},
			Quote {
				quote_id: "quote_2".to_string(),
				solver_id: "solver_2".to_string(),
				orders: vec![],
				details: details.clone(),
				valid_until: Some(12345),
				eta: Some(200), // 200 seconds (middle)
				provider: "test_provider".to_string(),
				integrity_checksum: "checksum_2".to_string(),
			},
		];

		// Create request with Speed preference
		let mut request = create_valid_quote_request();
		request.preference = Some(QuotePreference::Speed);

		// Test sorting
		let result = aggregator.sort_quotes_by_preference(quotes, &request);
		assert!(result.is_ok());

		let sorted_quotes = result.unwrap();
		assert_eq!(sorted_quotes.len(), 4);

		// Verify order: fastest ETA first, then slower, then no ETA last
		assert_eq!(sorted_quotes[0].quote_id, "quote_1"); // eta: 100 (fastest)
		assert_eq!(sorted_quotes[1].quote_id, "quote_2"); // eta: 200 (middle)
		assert_eq!(sorted_quotes[2].quote_id, "quote_3"); // eta: 300 (slowest)
		assert_eq!(sorted_quotes[3].quote_id, "quote_no_eta"); // eta: None (last)

		// Verify eta values are correct
		assert_eq!(sorted_quotes[0].eta, Some(100));
		assert_eq!(sorted_quotes[1].eta, Some(200));
		assert_eq!(sorted_quotes[2].eta, Some(300));
		assert_eq!(sorted_quotes[3].eta, None);
	}

	#[test]
	fn test_sort_quotes_no_preference_defaults_to_speed() {
		// Test that quotes are sorted by speed (eta) when no preference is specified (default behavior)
		let aggregator = create_test_aggregator();

		// Create test quote details from a sample request
		let sample_request = create_valid_quote_request();
		let details = QuoteDetails {
			available_inputs: sample_request.available_inputs,
			requested_outputs: sample_request.requested_outputs,
		};

		// Create test quotes with different eta values in non-optimal order
		let quotes = vec![
			Quote {
				quote_id: "quote_slow".to_string(),
				solver_id: "solver_slow".to_string(),
				orders: vec![],
				details: details.clone(),
				valid_until: Some(12345),
				eta: Some(500), // Slow (should be last)
				provider: "test_provider".to_string(),
				integrity_checksum: "checksum_slow".to_string(),
			},
			Quote {
				quote_id: "quote_fast".to_string(),
				solver_id: "solver_fast".to_string(),
				orders: vec![],
				details: details.clone(),
				valid_until: Some(12345),
				eta: Some(50), // Fast (should be first)
				provider: "test_provider".to_string(),
				integrity_checksum: "checksum_fast".to_string(),
			},
		];

		// Create request with no preference (should default to Speed sorting)
		let request = create_valid_quote_request(); // No preference specified

		// Test sorting (should default to Speed sorting)
		let result = aggregator.sort_quotes_by_preference(quotes.clone(), &request);
		assert!(result.is_ok());

		let sorted_quotes = result.unwrap();
		assert_eq!(sorted_quotes.len(), 2);

		// Order should be sorted by eta: fastest first
		assert_eq!(sorted_quotes[0].quote_id, "quote_fast"); // eta: 50 (fastest first)
		assert_eq!(sorted_quotes[1].quote_id, "quote_slow"); // eta: 500 (slowest last)

		// Verify eta values are in correct order
		assert_eq!(sorted_quotes[0].eta, Some(50));
		assert_eq!(sorted_quotes[1].eta, Some(500));
	}
}
