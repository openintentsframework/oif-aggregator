//! Circuit breaker service for automatic failure protection
//!
//! This module provides the core circuit breaker implementation that automatically
//! protects against cascading failures by temporarily blocking requests to failing solvers.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use oif_config::CircuitBreakerSettings;
use oif_storage::Storage;
use oif_types::{CircuitBreakerState, CircuitDecision, CircuitState, Solver};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Built-in buffer for half-open state to account for async processing delays
const HALF_OPEN_BUFFER: u32 = 1;

/// Result of rate calculation with context
#[derive(Debug)]
struct RateCheckResult {
	rate: f64,
	total_requests: u64,
	data_source: &'static str,
}

/// Trait for circuit breaker operations (enables easy testing and mocking)
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CircuitBreakerTrait: Send + Sync {
	/// Check if the circuit breaker is enabled in configuration
	fn is_enabled(&self) -> bool;

	/// Check if a request should be allowed for the given solver
	async fn should_allow_request(&self, solver: &Solver) -> bool;

	/// Record the result of a request to update circuit state
	async fn record_request_result(&self, solver_id: &str, success: bool);

	/// Fast evaluation using existing SolverMetrics
	fn should_open_primary(&self, solver: &Solver) -> CircuitDecision;
}

/// Core circuit breaker service implementation
pub struct CircuitBreakerService {
	storage: Arc<dyn Storage>,
	config: CircuitBreakerSettings,
}

impl CircuitBreakerService {
	/// Create a new circuit breaker service
	pub fn new(storage: Arc<dyn Storage>, config: CircuitBreakerSettings) -> Self {
		Self { storage, config }
	}

	/// Check if metrics are fresh enough for decision making
	fn are_metrics_fresh(
		&self,
		metrics: &oif_types::solvers::SolverMetrics,
	) -> Option<chrono::Duration> {
		let metrics_age = Utc::now().signed_duration_since(metrics.last_updated);
		let max_age = chrono::Duration::minutes(self.config.metrics_max_age_minutes as i64);

		if metrics_age >= max_age {
			None // Stale
		} else {
			Some(metrics_age) // Fresh, return age
		}
	}

	/// Check if we should examine rate-based metrics (only when there are consecutive failures)
	/// Rate checks are enabled when consecutive failures reach half the failure threshold (rounded up)
	/// This ensures rate-based logic has a chance to work before consecutive failure circuit opening
	fn should_check_rates(&self, consecutive_failures: u32) -> bool {
		let rate_check_threshold = self.config.failure_threshold.div_ceil(2);
		consecutive_failures >= rate_check_threshold
	}

	/// Calculate rate from metrics with intelligent fallback logic
	/// Returns (rate, requests_used, data_source_name)
	fn calculate_rate(
		&self,
		metrics: &oif_types::solvers::SolverMetrics,
		get_recent: impl Fn(&oif_types::solvers::SolverMetrics) -> (u64, u64), // (total, specific_count)
		get_lifetime: impl Fn(&oif_types::solvers::SolverMetrics) -> (u64, u64), // (total, specific_count)
		default_rate: f64,
	) -> RateCheckResult {
		let (recent_total, recent_specific) = get_recent(metrics);

		if recent_total >= self.config.min_requests_for_rate_check {
			// Sufficient windowed data - use recent metrics
			let rate = if recent_total > 0 {
				recent_specific as f64 / recent_total as f64
			} else {
				default_rate
			};
			RateCheckResult {
				rate,
				total_requests: recent_total,
				data_source: "recent",
			}
		} else {
			let (lifetime_total, lifetime_specific) = get_lifetime(metrics);
			if lifetime_total >= self.config.min_requests_for_rate_check {
				// Insufficient windowed data but sufficient lifetime data
				let rate = if lifetime_total > 0 {
					lifetime_specific as f64 / lifetime_total as f64
				} else {
					default_rate
				};
				RateCheckResult {
					rate,
					total_requests: lifetime_total,
					data_source: "lifetime_fallback",
				}
			} else {
				// Insufficient data overall - skip rate check
				RateCheckResult {
					rate: default_rate,
					total_requests: 0,
					data_source: "",
				}
			}
		}
	}

	/// Calculate success rate using intelligent fallback
	fn calculate_success_rate(
		&self,
		metrics: &oif_types::solvers::SolverMetrics,
	) -> RateCheckResult {
		self.calculate_rate(
			metrics,
			|m| (m.recent_total_requests, m.recent_successful_requests),
			|m| (m.total_requests, m.successful_requests),
			1.0, // Default to good success rate when no data
		)
	}

	/// Calculate service error rate using intelligent fallback  
	fn calculate_service_error_rate(
		&self,
		metrics: &oif_types::solvers::SolverMetrics,
	) -> RateCheckResult {
		self.calculate_rate(
			metrics,
			|m| (m.recent_total_requests, m.recent_service_errors),
			|m| (m.total_requests, m.service_errors),
			0.0, // Default to no service errors when no data
		)
	}

	/// Shared hybrid decision logic for half-open state transitions
	fn evaluate_half_open_decision(&self, circuit_state: &CircuitBreakerState) -> Option<bool> {
		let total_results =
			circuit_state.successful_test_requests + circuit_state.failed_test_requests;

		if total_results == 0 {
			return None; // No results yet
		}

		let success_rate = circuit_state.successful_test_requests as f64 / total_results as f64;

		// Smart hybrid decision logic
		if total_results >= 2 {
			// Early decision with multiple samples
			if circuit_state.successful_test_requests >= 2 && success_rate >= 0.6 {
				// Multiple successes with good rate → close
				Some(true)
			} else if circuit_state.failed_test_requests >= 2 && success_rate <= 0.4 {
				// Multiple failures with bad rate → keep open
				Some(false)
			} else if total_results >= self.config.half_open_max_calls {
				// Reached max calls → decide based on success rate threshold
				Some(success_rate >= self.config.success_rate_threshold)
			} else {
				// Need more samples
				None
			}
		} else if total_results >= self.config.half_open_max_calls {
			// Single result but reached max calls → decide based on success rate threshold
			Some(success_rate >= self.config.success_rate_threshold)
		} else {
			// Need more samples
			None
		}
	}

	/// Evaluate if we can make an immediate decision to avoid async race conditions
	fn evaluate_immediate_half_open_decision(
		&self,
		circuit_state: &CircuitBreakerState,
	) -> Option<bool> {
		self.evaluate_half_open_decision(circuit_state)
	}

	/// Update the half-open test results and save to storage  
	async fn update_half_open_test_results(
		&self,
		solver_id: &str,
		mut circuit_state: CircuitBreakerState,
	) {
		circuit_state.touch(); // Update timestamp

		if let Err(e) = self
			.storage
			.update_solver_circuit_state(circuit_state.clone())
			.await
		{
			warn!(
				"Failed to update half-open test results for solver '{}': {}",
				solver_id, e
			);
		} else {
			debug!(
				"Updated half-open test results for solver '{}': {} successes, {} failures, need {} more results",
				solver_id,
				circuit_state.successful_test_requests,
				circuit_state.failed_test_requests,
				self.config.half_open_max_calls.saturating_sub(circuit_state.successful_test_requests + circuit_state.failed_test_requests)
			);
		}
	}

	/// Calculate exponential backoff timeout duration based on recovery attempts
	fn calculate_timeout_duration(&self, recovery_attempts: u32) -> Duration {
		let base = self.config.base_timeout_seconds;
		let max_timeout = self.config.max_timeout_seconds;

		// Exponential backoff: base * 2^recovery_attempts, capped at max
		let timeout_seconds = std::cmp::min(
			base * 2_u64.pow(recovery_attempts.min(10)), // Cap exponent to prevent overflow
			max_timeout,
		);

		Duration::seconds(timeout_seconds as i64)
	}

	/// Transition circuit to open state
	async fn transition_to_open(&self, solver_id: String, reason: String, failure_count: u32) {
		let timeout_duration = self.calculate_timeout_duration(0); // First recovery attempt
		let state =
			CircuitBreakerState::new_open(solver_id, reason, timeout_duration, failure_count);

		info!(
			"Circuit breaker opened for solver '{}': {} (timeout: {}s)",
			state.solver_id,
			state.reason.as_ref().unwrap_or(&"unknown".to_string()),
			timeout_duration.num_seconds()
		);

		if let Err(e) = self.storage.update_solver_circuit_state(state).await {
			warn!("Failed to update circuit state to open: {}", e);
		}
	}

	/// Transition circuit to half-open state for testing recovery
	/// Preserves recovery_attempt_count from the current open state
	async fn transition_to_half_open(&self, solver_id: String, recovery_attempt_count: u32) {
		let mut state = CircuitBreakerState::new_half_open(solver_id.clone());
		state.recovery_attempt_count = recovery_attempt_count; // Preserve recovery attempts
		state.touch();

		debug!(
			"Circuit breaker transitioning to half-open for solver '{}' (recovery attempt #{})",
			solver_id, recovery_attempt_count
		);

		if let Err(e) = self.storage.update_solver_circuit_state(state).await {
			warn!("Failed to update circuit state to half-open: {}", e);
		}
	}

	/// Transition circuit to closed state (recovery complete)
	async fn transition_to_closed(&self, solver_id: String) {
		let state = CircuitBreakerState::new_closed(solver_id.clone());

		info!(
			"Circuit breaker closed for solver '{}' - recovery complete",
			solver_id
		);

		if let Err(e) = self.storage.update_solver_circuit_state(state).await {
			warn!("Failed to update circuit state to closed: {}", e);
		}
	}

	/// Remove circuit breaker state (cleanup)
	#[allow(unused)]
	async fn remove_circuit_state(&self, solver_id: &str) {
		if let Err(e) = self.storage.delete_circuit_state(solver_id).await {
			warn!(
				"Failed to delete circuit state for solver '{}': {}",
				solver_id, e
			);
		}
	}

	/// Transition to open state with recovery attempt tracking
	async fn transition_to_open_with_attempts(
		&self,
		solver_id: String,
		reason: String,
		failure_count: u32,
		recovery_attempts: u32,
	) {
		let timeout_duration = self.calculate_timeout_duration(recovery_attempts);
		let mut state = CircuitBreakerState::new_open(
			solver_id.clone(),
			reason.clone(),
			timeout_duration,
			failure_count,
		);
		state.recovery_attempt_count = recovery_attempts;

		info!(
			"Circuit breaker opened for solver '{}': {} (timeout: {}s, attempt: {})",
			solver_id,
			reason,
			timeout_duration.num_seconds(),
			recovery_attempts
		);

		if let Err(e) = self.storage.update_solver_circuit_state(state).await {
			warn!("Failed to update circuit state to open: {}", e);
		}
	}

	/// Handle persistent failure after max recovery attempts exceeded
	async fn handle_persistent_failure(&self, solver_id: String, recovery_attempts: u32) {
		use oif_types::PersistentFailureAction;

		match self.config.persistent_failure_action {
			PersistentFailureAction::KeepTrying => {
				// Continue with normal exponential backoff (current behavior)
				warn!(
					"Solver '{}' has failed {} recovery attempts, continuing with backoff",
					solver_id, recovery_attempts
				);
				self.transition_to_open_with_attempts(
					solver_id,
					"persistent_failure_keep_trying".to_string(),
					recovery_attempts, // Use recovery attempts as failure count for timeout calculation
					recovery_attempts,
				)
				.await;
			},

			PersistentFailureAction::DisableSolver => {
				// Disable solver administratively - requires manual intervention
				warn!("Solver '{}' has failed {} recovery attempts, disabling solver (requires manual intervention)", solver_id, recovery_attempts);

				// Try to disable the solver in storage
				if let Ok(Some(mut solver)) = self.storage.get_solver(&solver_id).await {
					solver.status = oif_types::SolverStatus::Disabled;
					if let Err(e) = self.storage.update_solver(solver).await {
						warn!(
							"Failed to disable persistently failing solver '{}': {}",
							solver_id, e
						);
						return; // Do not delete circuit state
					}
				}

				// Remove circuit breaker state since solver is disabled
				let _ = self.storage.delete_circuit_state(&solver_id).await;
			},

			PersistentFailureAction::ExtendTimeout => {
				// Use 24-hour timeout to reduce testing frequency
				warn!(
					"Solver '{}' has failed {} recovery attempts, extending timeout to 24 hours",
					solver_id, recovery_attempts
				);
				let extended_timeout = chrono::Duration::hours(24);
				let mut state = CircuitBreakerState::new_open(
					solver_id.clone(),
					"persistent_failure_extended_timeout".to_string(),
					extended_timeout,
					recovery_attempts,
				);
				state.recovery_attempt_count = recovery_attempts;

				if let Err(e) = self.storage.update_solver_circuit_state(state).await {
					warn!(
						"Failed to update circuit state with extended timeout: {}",
						e
					);
				}
			},
		}
	}
}

#[async_trait]
impl CircuitBreakerTrait for CircuitBreakerService {
	/// Check if the circuit breaker is enabled in configuration
	fn is_enabled(&self) -> bool {
		self.config.enabled
	}

	/// PRIMARY DECISION LOGIC: Fast evaluation using existing SolverMetrics
	///
	/// This method implements the core circuit breaker logic without async operations
	/// for maximum performance during request filtering.
	fn should_open_primary(&self, solver: &Solver) -> CircuitDecision {
		let metrics = &solver.metrics;

		// Check if metrics are fresh enough for decision making
		let metrics_age = match self.are_metrics_fresh(metrics) {
			Some(age) => age,
			None => {
				debug!(
					"Skipping metrics-based checks for solver '{}' - metrics are {} minutes old (max: {})",
                solver.solver_id,
					Utc::now().signed_duration_since(metrics.last_updated).num_minutes(),
                self.config.metrics_max_age_minutes
            );
				return CircuitDecision::Inconclusive; // Can't decide with stale data
			},
		};

		// 1. Consecutive failures threshold (immediate danger check)
		if metrics.consecutive_failures >= self.config.failure_threshold {
			return CircuitDecision::Open {
				reason: format!(
					"consecutive_failures: {} (metrics age: {}min)",
					metrics.consecutive_failures,
					metrics_age.num_minutes()
				),
			};
		}

		// Only examine historical rates when there are signs of trouble (consecutive failures)
		if !self.should_check_rates(metrics.consecutive_failures) {
			debug!(
				"Skipping rate checks for solver '{}' - no consecutive failures ({})",
				solver.solver_id, metrics.consecutive_failures
			);
			return CircuitDecision::Closed;
		}

		// 2. Success rate check (when consecutive failures indicate potential issues)
		let success_result = self.calculate_success_rate(metrics);
		if success_result.total_requests > 0
			&& success_result.rate < self.config.success_rate_threshold
		{
			return CircuitDecision::Open {
				reason: format!(
					"success_rate_{}: {:.2} (over {} requests, window: {}min, metrics age: {}min)",
					success_result.data_source,
					success_result.rate,
					success_result.total_requests,
					self.config.metrics_window_duration_minutes,
					metrics_age.num_minutes()
				),
			};
		}

		// 3. Service error rate check (when consecutive failures indicate potential issues)
		let error_result = self.calculate_service_error_rate(metrics);
		if error_result.total_requests > 0
			&& error_result.rate > self.config.service_error_threshold
		{
			return CircuitDecision::Open {
				reason: format!(
					"service_error_rate_{}: {:.2} (over {} requests, window: {}min, metrics age: {}min)",
					error_result.data_source,
					error_result.rate,
					error_result.total_requests,
					self.config.metrics_window_duration_minutes,
						metrics_age.num_minutes()
					),
			};
		}

		// All checks passed - circuit should remain closed
		CircuitDecision::Closed
	}

	/// Main request filtering logic - determines if requests should be allowed
	async fn should_allow_request(&self, solver: &Solver) -> bool {
		if !self.is_enabled() {
			return true;
		}

		// Fast path: check primary decision logic first
		let primary_decision = self.should_open_primary(solver);

		// Get current circuit state from storage
		let circuit_state = match self
			.storage
			.get_solver_circuit_state(&solver.solver_id)
			.await
		{
			Ok(Some(state)) => state,
			Ok(None) => {
				// No circuit state exists - check if we should create one
				match primary_decision {
					CircuitDecision::Open { reason } => {
						// Primary logic says open - create new open circuit
						self.transition_to_open(
							solver.solver_id.clone(),
							reason,
							solver.metrics.consecutive_failures,
						)
						.await;
						return false;
					},
					CircuitDecision::Closed => {
						// All good - allow request
						return true;
					},
					CircuitDecision::Inconclusive => {
						// Not enough data - allow request (fail open)
						return true;
					},
				}
			},
			Err(e) => {
				warn!(
					"Failed to get circuit state for solver '{}': {} - failing open",
					solver.solver_id, e
				);
				return true; // Fail open on storage errors
			},
		};

		// Handle existing circuit state
		match circuit_state.state {
			CircuitState::Closed => {
				// Circuit is closed - check if it should be opened
				match primary_decision {
					CircuitDecision::Open { reason } => {
						self.transition_to_open(
							solver.solver_id.clone(),
							reason,
							solver.metrics.consecutive_failures,
						)
						.await;
						false
					},
					_ => true, // Allow request
				}
			},

			CircuitState::Open => {
				// Circuit is open - check if we should attempt recovery
				if circuit_state.should_attempt_reset() {
					self.transition_to_half_open(
						solver.solver_id.clone(),
						circuit_state.recovery_attempt_count,
					)
					.await;
					true // Allow the test request
				} else {
					debug!(
						"Circuit breaker blocking request to solver '{}'",
						solver.solver_id
					);
					false // Block request
				}
			},

			CircuitState::HalfOpen => {
				// Check if we can make an immediate decision to avoid race conditions
				if let Some(should_close) =
					self.evaluate_immediate_half_open_decision(&circuit_state)
				{
					if should_close {
						// Circuit should close - allow request and transition immediately
						self.transition_to_closed(solver.solver_id.clone()).await;
						return true;
					} else {
						// Circuit should reopen - block request and transition with recovery attempts
						let new_recovery_attempts = circuit_state.recovery_attempt_count + 1;
						let failure_count = circuit_state.failure_count_when_opened
							+ circuit_state.failed_test_requests;

						if new_recovery_attempts >= self.config.max_recovery_attempts {
							// Handle persistent failure
							self.handle_persistent_failure(
								solver.solver_id.clone(),
								new_recovery_attempts,
							)
							.await;
						} else {
							// Reopen with incremented recovery attempts
							self.transition_to_open_with_attempts(
								solver.solver_id.clone(),
								"immediate_failure_decision".to_string(),
								failure_count,
								new_recovery_attempts,
							)
							.await;
						}
						return false;
					}
				}

				// Half-open state - allow limited test requests with buffer for async processing
				let effective_max_calls = self.config.half_open_max_calls + HALF_OPEN_BUFFER;
				if circuit_state.test_request_count < effective_max_calls {
					// Increment test request count and save updated state
					let mut updated_state = circuit_state.clone();
					updated_state.test_request_count += 1;
					updated_state.touch();

					if let Err(e) = self
						.storage
						.update_solver_circuit_state(updated_state)
						.await
					{
						warn!(
							"Failed to update test request count for solver '{}': {} - allowing request anyway",
							solver.solver_id, e
						);
					} else {
						debug!(
							"Incremented test request count to {} for solver '{}' in half-open state",
							circuit_state.test_request_count + 1,
							solver.solver_id
						);
					}

					true // Allow test request
				} else {
					debug!(
						"Half-open circuit for solver '{}' has reached max test requests ({}/{})",
						solver.solver_id,
						circuit_state.test_request_count,
						self.config.half_open_max_calls
					);
					false // Too many test requests already
				}
			},
		}
	}

	/// Record the result of a request to update circuit breaker state
	async fn record_request_result(&self, solver_id: &str, success: bool) {
		if !self.is_enabled() {
			return; // No-op when disabled
		}

		let mut circuit_state = match self.storage.get_solver_circuit_state(solver_id).await {
			Ok(Some(state)) => state,
			Ok(None) => return, // No circuit state - nothing to update
			Err(e) => {
				warn!("Failed to get circuit state for result recording: {}", e);
				return;
			},
		};

		debug!("record_request_result circuit_state: {:?}", circuit_state);

		match circuit_state.state {
			CircuitState::HalfOpen => {
				// Half-open state - collect test results for smart decision making
				if success {
					circuit_state.successful_test_requests += 1;
				} else {
					circuit_state.failed_test_requests += 1;
				}

				let total_results =
					circuit_state.successful_test_requests + circuit_state.failed_test_requests;
				let success_rate = if total_results > 0 {
					circuit_state.successful_test_requests as f64 / total_results as f64
				} else {
					0.0
				};

				debug!(
					"Half-open test result for solver '{}': {} successes / {} total ({}% success rate)",
					solver_id,
					circuit_state.successful_test_requests,
					total_results,
					(success_rate * 100.0).round()
				);

				// Use shared hybrid decision logic
				let decision = self.evaluate_half_open_decision(&circuit_state);

				match decision {
					Some(should_close) => {
						if should_close {
							debug!(
								"Half-open circuit closing for solver '{}' - {} successes / {} total ({}% success rate)",
								solver_id,
								circuit_state.successful_test_requests,
								total_results,
								(success_rate * 100.0).round()
							);
							self.transition_to_closed(solver_id.to_string()).await;
						} else {
							debug!(
								"Half-open circuit reopening for solver '{}' - {} successes / {} total ({}% success rate)",
								solver_id,
								circuit_state.successful_test_requests,
								total_results,
								(success_rate * 100.0).round()
							);
							// Test failed - increment recovery attempts and handle accordingly
							let new_recovery_attempts = circuit_state.recovery_attempt_count + 1;
							let new_failure_count = circuit_state.failure_count_when_opened + 1;

							if new_recovery_attempts >= self.config.max_recovery_attempts {
								self.handle_persistent_failure(
									solver_id.to_string(),
									new_recovery_attempts,
								)
								.await;
							} else {
								self.transition_to_open_with_attempts(
									solver_id.to_string(),
									format!(
										"failed_during_half_open_{}%_success",
										(success_rate * 100.0).round() as u32
									),
									new_failure_count,
									new_recovery_attempts,
								)
								.await;
							}
						}
					},
					None => {
						// Keep collecting more test results - update state and continue
						self.update_half_open_test_results(solver_id, circuit_state)
							.await;
					},
				}
			},

			CircuitState::Closed => {
				// Circuit is closed - no action needed (primary logic handles opening)
			},

			CircuitState::Open => {
				// Circuit is open - no action needed (will attempt recovery on timeout)
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use oif_storage::MemoryStore;
	// Test imports already included in parent scope

	fn create_test_solver(solver_id: &str) -> Solver {
		Solver::new(
			solver_id.to_string(),
			"test-adapter".to_string(),
			"http://localhost:8080".to_string(),
		)
	}

	fn create_test_config() -> CircuitBreakerSettings {
		CircuitBreakerSettings {
			enabled: true,
			failure_threshold: 3,
			success_rate_threshold: 0.5,
			min_requests_for_rate_check: 5,
			base_timeout_seconds: 10,
			max_timeout_seconds: 300,
			half_open_max_calls: 3,
			max_recovery_attempts: 10,
			persistent_failure_action: oif_types::PersistentFailureAction::ExtendTimeout,
			metrics_max_age_minutes: 30,
			service_error_threshold: 0.5,
			metrics_window_duration_minutes: 15,
			metrics_max_window_age_minutes: 60,
		}
	}

	#[tokio::test]
	async fn test_half_open_test_request_count_limits() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());
		let solver = create_test_solver("test-solver");

		// Create a half-open circuit state
		let mut half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		half_open_state.test_request_count = 0;
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// First request should be allowed and count should increment
		let allowed1 = circuit_breaker.should_allow_request(&solver).await;
		assert!(allowed1, "First request should be allowed");

		// Verify count was incremented
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap()
			.unwrap();
		assert_eq!(
			state.test_request_count, 1,
			"Test request count should be 1"
		);

		// Second request should be allowed and count should increment
		let allowed2 = circuit_breaker.should_allow_request(&solver).await;
		assert!(allowed2, "Second request should be allowed");

		// Verify count was incremented again
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap()
			.unwrap();
		assert_eq!(
			state.test_request_count, 2,
			"Test request count should be 2"
		);

		// Third request should be blocked since half_open_max_calls is 3 (but we hit the limit)
		let allowed3 = circuit_breaker.should_allow_request(&solver).await;
		assert!(
			allowed3,
			"Third request should be allowed (config has half_open_max_calls=3)"
		);

		// Verify count was incremented once more
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap()
			.unwrap();
		assert_eq!(
			state.test_request_count, 3,
			"Test request count should be 3"
		);

		// Fourth request should be allowed due to buffer (3 < 3+1)
		let allowed4 = circuit_breaker.should_allow_request(&solver).await;
		assert!(allowed4, "Fourth request should be allowed due to buffer");

		// Fifth request should be blocked since it exceeds buffer (4 >= 3+1)
		let allowed5 = circuit_breaker.should_allow_request(&solver).await;
		assert!(
			!allowed5,
			"Fifth request should be blocked - buffer limit reached"
		);
	}

	#[tokio::test]
	async fn test_half_open_hybrid_early_success() {
		// Test early closing on multiple successful requests (2 successes with 100% rate)
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());

		// Create a half-open circuit state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record 2 successful requests
		circuit_breaker
			.record_request_result("test-solver", true)
			.await;
		circuit_breaker
			.record_request_result("test-solver", true)
			.await;

		// Circuit should now be closed (early success decision)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Closed));
	}

	#[tokio::test]
	async fn test_half_open_hybrid_early_failure() {
		// Test early reopening on multiple failed requests (2 failures with 0% rate)
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());

		// Create a half-open circuit state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record 2 failed requests
		circuit_breaker
			.record_request_result("test-solver", false)
			.await;
		circuit_breaker
			.record_request_result("test-solver", false)
			.await;

		// Circuit should now be open again (early failure decision)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Open));
	}

	#[tokio::test]
	async fn test_half_open_hybrid_mixed_results() {
		// Test waiting for more results when mixed success/failure (50% rate with 2 results)
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());

		// Create a half-open circuit state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record 1 success, 1 failure (50% rate - not decisive)
		circuit_breaker
			.record_request_result("test-solver", true)
			.await;
		circuit_breaker
			.record_request_result("test-solver", false)
			.await;

		// Circuit should still be half-open (waiting for more results)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		let circuit_state = state.unwrap();
		assert!(matches!(circuit_state.state, CircuitState::HalfOpen));
		assert_eq!(circuit_state.successful_test_requests, 1);
		assert_eq!(circuit_state.failed_test_requests, 1);

		// Add one more success to get above threshold (67% > 50%)
		circuit_breaker
			.record_request_result("test-solver", true)
			.await;

		// Circuit should now be closed (reached max_calls with good success rate)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Closed));
	}

	#[tokio::test]
	async fn test_half_open_hybrid_wait_for_max_calls() {
		// Test decision based on success rate threshold when reaching max calls
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		config.half_open_max_calls = 4; // Set higher limit for this test
		config.success_rate_threshold = 0.75; // 75% threshold
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());

		// Create a half-open circuit state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record mixed results that don't trigger early decisions (3 success, 1 failure = 75%)
		circuit_breaker
			.record_request_result("test-solver", true)
			.await; // 1/1 = 100%
		circuit_breaker
			.record_request_result("test-solver", false)
			.await; // 1/2 = 50%
		circuit_breaker
			.record_request_result("test-solver", true)
			.await; // 2/3 = 67%
		circuit_breaker
			.record_request_result("test-solver", true)
			.await; // 3/4 = 75%

		// Circuit should now be closed (reached max_calls with 75% success rate = threshold)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Closed));
	}

	#[tokio::test]
	async fn test_half_open_hybrid_insufficient_results() {
		// Test keeping circuit open when not enough successful results at max calls
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		config.half_open_max_calls = 4; // Set higher limit for this test
		config.success_rate_threshold = 0.75; // 75% threshold
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());

		// Create a half-open circuit state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record results that avoid early decisions but fail at max_calls
		// (1 success, 3 failures = 25% < 75% threshold)
		circuit_breaker
			.record_request_result("test-solver", true)
			.await; // 1/1 = 100% (but only 1 success, no early decision)
		circuit_breaker
			.record_request_result("test-solver", false)
			.await; // 1/2 = 50% (mixed, no early decision)
		circuit_breaker
			.record_request_result("test-solver", false)
			.await; // 1/3 = 33% (only 1 failure, no early decision)
		circuit_breaker
			.record_request_result("test-solver", false)
			.await; // 1/4 = 25% < 75% (reached max_calls, reopen)

		// Circuit should now be open again (reached max_calls with 25% < 75% threshold)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Open));
	}

	#[tokio::test]
	async fn test_immediate_decision_prevents_race_condition() {
		// Test that immediate decisions prevent unnecessary request drops
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());
		let solver = create_test_solver("test-solver");

		// Create a half-open state with 2 successful results (should trigger immediate close)
		let mut half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		half_open_state.successful_test_requests = 2;
		half_open_state.failed_test_requests = 0;
		half_open_state.test_request_count = 2;
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// This request should be allowed AND immediately close the circuit
		let allowed = circuit_breaker.should_allow_request(&solver).await;
		assert!(
			allowed,
			"Request should be allowed due to immediate decision"
		);

		// Circuit should now be closed
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Closed));
	}

	#[tokio::test]
	async fn test_buffer_prevents_unnecessary_blocks() {
		// Test that buffer allows extra requests during async processing
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());
		let solver = create_test_solver("test-solver");

		// Create a half-open state at the limit (3 requests made)
		let mut half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		half_open_state.test_request_count = 3; // At the limit
		half_open_state.successful_test_requests = 1;
		half_open_state.failed_test_requests = 1; // Mixed results, no immediate decision
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// This request should be allowed due to the buffer (3 < 3+1)
		let allowed = circuit_breaker.should_allow_request(&solver).await;
		assert!(allowed, "Request should be allowed due to buffer");

		// Verify count was incremented to 4
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		let circuit_state = state.unwrap();
		assert_eq!(circuit_state.test_request_count, 4);
		assert!(matches!(circuit_state.state, CircuitState::HalfOpen));

		// Next request (5th) should be blocked as it exceeds buffer
		let blocked = circuit_breaker.should_allow_request(&solver).await;
		assert!(!blocked, "5th request should be blocked even with buffer");
	}

	#[tokio::test]
	async fn test_shared_decision_logic_consistency() {
		// Test that immediate and async decision paths use the same logic
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());

		// Create test states with different success patterns
		let test_cases = vec![
			// (successful_requests, failed_requests, expected_decision)
			(2, 0, Some(true)),  // 100% success rate, early close
			(0, 2, Some(false)), // 0% success rate, early reopen
			(1, 1, None),        // 50% mixed, need more samples
			(3, 1, Some(true)),  // 75% at max calls, should close (assuming 50% threshold)
			(1, 2, Some(false)), // 33% at max calls, should reopen
		];

		for (successful, failed, expected) in test_cases {
			let mut circuit_state = CircuitBreakerState::new_half_open("test-solver".to_string());
			circuit_state.successful_test_requests = successful;
			circuit_state.failed_test_requests = failed;

			// Both methods should return the same result
			let immediate_decision =
				circuit_breaker.evaluate_immediate_half_open_decision(&circuit_state);
			let shared_decision = circuit_breaker.evaluate_half_open_decision(&circuit_state);

			assert_eq!(
				immediate_decision, shared_decision,
				"Immediate and shared decision logic must be consistent for {} successes, {} failures",
				successful, failed
			);
			assert_eq!(
				immediate_decision, expected,
				"Decision should match expected result for {} successes, {} failures",
				successful, failed
			);
		}
	}

	#[tokio::test]
	async fn test_fresh_solver_should_be_allowed() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let circuit_breaker = CircuitBreakerService::new(Arc::clone(&storage), config.clone());
		let solver = create_test_solver("fresh-solver");

		// Fresh solver with no circuit state should be allowed
		let allowed = circuit_breaker.should_allow_request(&solver).await;
		assert!(
			allowed,
			"Fresh solver should be allowed - no circuit state exists"
		);

		// Check what primary decision was made
		let decision = circuit_breaker.should_open_primary(&solver);

		// Fresh solver should get Closed decision - they start healthy with no failures
		assert!(
			matches!(decision, CircuitDecision::Closed),
			"Fresh solver should get Closed decision - starts healthy"
		);
	}

	#[tokio::test]
	async fn test_should_open_on_consecutive_failures() {
		let storage = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");
		solver.metrics.consecutive_failures = 5; // Above threshold of 3

		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Open { .. }));
		if let CircuitDecision::Open { reason } = decision {
			assert!(reason.contains("consecutive_failures"));
		}
	}

	#[tokio::test]
	async fn test_should_stay_closed_on_low_failures() {
		let storage = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");
		solver.metrics.consecutive_failures = 1; // Below threshold

		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Closed));
	}

	#[tokio::test]
	async fn test_should_open_on_low_success_rate() {
		let storage = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		// Set failure threshold to 10, so rate checks start at 6 (ceil(10/2))
		config.failure_threshold = 10;
		let service = CircuitBreakerService::new(storage, config.clone());

		let mut solver = create_test_solver("test-solver");
		solver.metrics.total_requests = 10; // Above min threshold
		solver.metrics.successful_requests = 2; // 20% success rate (below 50% threshold)
		solver.metrics.consecutive_failures = (config.failure_threshold + 1) / 2; // Enable rate checks (6 < 10, so won't trigger consecutive failure check)

		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Open { .. }));
		if let CircuitDecision::Open { reason } = decision {
			assert!(reason.contains("success_rate"));
		}
	}

	#[tokio::test]
	async fn test_should_allow_request_when_circuit_closed() {
		let storage = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let solver = create_test_solver("test-solver");
		// Solver with good metrics should be allowed

		let should_allow = service.should_allow_request(&solver).await;
		assert!(should_allow);
	}

	#[tokio::test]
	async fn test_exponential_backoff_timeout() {
		let storage = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		// Test exponential backoff calculation
		let timeout0 = service.calculate_timeout_duration(0);
		let timeout1 = service.calculate_timeout_duration(1);
		let timeout2 = service.calculate_timeout_duration(2);

		assert_eq!(timeout0.num_seconds(), 10); // base: 10 * 2^0 = 10
		assert_eq!(timeout1.num_seconds(), 20); // base: 10 * 2^1 = 20
		assert_eq!(timeout2.num_seconds(), 40); // base: 10 * 2^2 = 40
	}

	#[tokio::test]
	async fn test_stale_metrics_skip_checks() {
		let storage = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");

		// Set metrics that would normally trigger circuit breaker
		solver.metrics.consecutive_failures = 5; // Above threshold (3)
		solver.metrics.total_requests = 10; // Above min threshold (5)
		solver.metrics.successful_requests = 2; // 20% success rate (below 50% threshold)

		// Make metrics stale (older than 30 minutes)
		solver.metrics.last_updated = Utc::now() - chrono::Duration::minutes(45);

		// Should NOT open circuit due to stale metrics (fail-open approach)
		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Inconclusive));
	}

	#[tokio::test]
	async fn test_fresh_metrics_apply_checks() {
		let storage = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");

		// Set metrics that should trigger circuit breaker
		solver.metrics.consecutive_failures = 5; // Above threshold (3)
		solver.metrics.total_requests = 10; // Above min threshold (5)
		solver.metrics.successful_requests = 2; // 20% success rate (below 50% threshold)

		// Make metrics fresh (within 30 minutes)
		solver.metrics.last_updated = Utc::now() - chrono::Duration::minutes(5);

		// Should open circuit due to consecutive failures (first check to trigger)
		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Open { .. }));

		if let CircuitDecision::Open { reason } = decision {
			assert!(reason.contains("consecutive_failures"));
		}
	}

	#[tokio::test]
	async fn test_service_error_rate_threshold() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		// Set failure threshold to 10, so rate checks start at 6 (ceil(10/2))
		config.failure_threshold = 10;
		let service = CircuitBreakerService::new(storage, config.clone());

		let mut solver = create_test_solver("test-solver");
		solver.metrics.total_requests = 10; // Above min threshold
		solver.metrics.service_errors = 8; // 80% service error rate (above 50% threshold)
		solver.metrics.successful_requests = 8; // 80% success rate (good) - the rest are service errors, not failed requests
		solver.metrics.consecutive_failures = (config.failure_threshold + 1) / 2; // Enable rate checks (6 < 10, so won't trigger consecutive failure check)

		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Open { .. }));
		if let CircuitDecision::Open { reason } = decision {
			assert!(reason.contains("service_error_rate"));
		}
	}

	#[tokio::test]
	async fn test_windowed_metrics_preferred_over_lifetime() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");

		// Set up lifetime metrics that would normally trigger circuit breaker
		solver.metrics.total_requests = 100;
		solver.metrics.successful_requests = 10; // 10% success rate (bad)

		// Set up recent windowed metrics that are good
		solver.metrics.recent_total_requests = 10; // Above min threshold (5)
		solver.metrics.recent_successful_requests = 9; // 90% success rate (good)

		let decision = service.should_open_primary(&solver);
		// Should stay closed because windowed metrics (90% success) are preferred over lifetime (10% success)
		assert!(matches!(decision, CircuitDecision::Closed));
	}

	#[tokio::test]
	async fn test_fallback_to_lifetime_when_insufficient_windowed_data() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		// Set failure threshold to 10, so rate checks start at 6 (ceil(10/2))
		config.failure_threshold = 10;
		let service = CircuitBreakerService::new(storage, config.clone());

		let mut solver = create_test_solver("test-solver");

		// Set up insufficient windowed data
		solver.metrics.recent_total_requests = 2; // Below min threshold (5)
		solver.metrics.recent_successful_requests = 2; // 100% success rate but insufficient data

		// Set up lifetime data with bad success rate
		solver.metrics.total_requests = 10; // Above min threshold
		solver.metrics.successful_requests = 2; // 20% success rate (bad)
		solver.metrics.consecutive_failures = (config.failure_threshold + 1) / 2; // Enable rate checks (6 < 10, so won't trigger consecutive failure check)

		let decision = service.should_open_primary(&solver);
		// Should open circuit because it falls back to lifetime metrics (20% success rate)
		assert!(matches!(decision, CircuitDecision::Open { .. }));
		if let CircuitDecision::Open { reason } = decision {
			assert!(reason.contains("success_rate_lifetime_fallback"));
		}
	}

	#[tokio::test]
	async fn test_rate_checks_skipped_when_no_consecutive_failures() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		// Set failure threshold to 10, so rate checks start at 6 (ceil(10/2))
		config.failure_threshold = 10;
		let service = CircuitBreakerService::new(storage, config.clone());

		let mut solver = create_test_solver("test-solver");

		// Set up bad metrics that would normally trigger circuit breaker
		solver.metrics.recent_total_requests = 10; // Above min threshold
		solver.metrics.recent_successful_requests = 1; // 10% success rate (bad)
		solver.metrics.recent_service_errors = 8; // 80% service error rate (bad)

		solver.metrics.total_requests = 100; // Above min threshold
		solver.metrics.successful_requests = 10; // 10% success rate (bad)
		solver.metrics.service_errors = 80; // 80% service error rate (bad)

		// But consecutive_failures is low (no current problems)
		solver.metrics.consecutive_failures = 3; // Below rate check threshold (6), so skip rate checks

		let decision = service.should_open_primary(&solver);
		// Should stay CLOSED - rate checks are skipped when no consecutive failures
		assert!(matches!(decision, CircuitDecision::Closed));
	}

	#[tokio::test]
	async fn test_rate_checks_enabled_with_consecutive_failures() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		// Set failure threshold to 10, so rate checks start at 6 (ceil(10/2))
		config.failure_threshold = 10;
		let service = CircuitBreakerService::new(storage, config.clone());

		let mut solver = create_test_solver("test-solver");

		// Same bad lifetime data
		solver.metrics.recent_total_requests = 2; // Below min threshold (5)
		solver.metrics.total_requests = 100; // Above min threshold
		solver.metrics.successful_requests = 10; // 10% success rate (bad)

		// Now consecutive_failures is high (rate checks enabled)
		solver.metrics.consecutive_failures = (config.failure_threshold + 1) / 2; // Rate checks enabled (6 < 10, so won't trigger consecutive failure check)

		let decision = service.should_open_primary(&solver);
		// Should open circuit because rate checks are now enabled, lifetime fallback applies
		assert!(matches!(decision, CircuitDecision::Open { .. }));
		if let CircuitDecision::Open { reason } = decision {
			assert!(reason.contains("success_rate_lifetime_fallback"));
		}
	}

	#[tokio::test]
	async fn test_insufficient_data_for_rate_checks() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");

		// Set up insufficient data for both windowed and lifetime
		solver.metrics.recent_total_requests = 2; // Below min threshold (5)
		solver.metrics.total_requests = 3; // Also below min threshold
		solver.metrics.consecutive_failures = 1; // Below failure threshold (3)

		let decision = service.should_open_primary(&solver);
		// Should stay closed due to insufficient data (rate checks are skipped)
		assert!(matches!(decision, CircuitDecision::Closed));
	}

	#[tokio::test]
	async fn test_half_open_successful_recovery() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);

		// Create a half-open circuit state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record 2 successful requests (triggers early success decision with 100% rate >= 60%)
		service.record_request_result("test-solver", true).await;
		service.record_request_result("test-solver", true).await;

		// Circuit should now be closed
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Closed));
	}

	#[tokio::test]
	async fn test_half_open_failed_recovery() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);

		// Create a half-open circuit state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record 2 failed requests (triggers early failure decision with 0% rate <= 40%)
		service.record_request_result("test-solver", false).await;
		service.record_request_result("test-solver", false).await;

		// Circuit should now be open again
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Open));
	}

	#[tokio::test]
	async fn test_persistent_failure_keep_trying() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		config.max_recovery_attempts = 2; // Low number for testing
		config.persistent_failure_action = oif_types::PersistentFailureAction::KeepTrying;
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);

		// Create a half-open state with high recovery attempt count
		let mut half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		half_open_state.recovery_attempt_count = 1; // One less than max
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record 2 failed requests (triggers early failure decision and persistent failure handling)
		service.record_request_result("test-solver", false).await;
		service.record_request_result("test-solver", false).await;

		// Circuit should still be open (KeepTrying action)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		let state = state.unwrap();
		assert!(matches!(state.state, CircuitState::Open));
		assert_eq!(state.recovery_attempt_count, 2); // Should be incremented
	}

	#[tokio::test]
	async fn test_persistent_failure_disable_solver() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		config.max_recovery_attempts = 1; // Low number for testing
		config.persistent_failure_action = oif_types::PersistentFailureAction::DisableSolver;
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);

		// First create and store the solver
		let solver = create_test_solver("test-solver");
		storage.create_solver(solver).await.unwrap();

		// Create a half-open state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record 2 failed requests (triggers early failure decision and persistent failure handling)
		service.record_request_result("test-solver", false).await;
		service.record_request_result("test-solver", false).await;

		// Solver should be disabled
		let solver = storage.get_solver("test-solver").await.unwrap();
		assert!(solver.is_some());
		assert!(matches!(
			solver.unwrap().status,
			oif_types::SolverStatus::Disabled
		));

		// Circuit state should be removed
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_none());
	}

	#[tokio::test]
	async fn test_persistent_failure_extend_timeout() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		config.max_recovery_attempts = 1; // Low number for testing
		config.persistent_failure_action = oif_types::PersistentFailureAction::ExtendTimeout;
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);

		// Create a half-open state
		let half_open_state = CircuitBreakerState::new_half_open("test-solver".to_string());
		storage
			.update_solver_circuit_state(half_open_state)
			.await
			.unwrap();

		// Record 2 failed requests (triggers early failure decision and persistent failure handling)
		service.record_request_result("test-solver", false).await;
		service.record_request_result("test-solver", false).await;

		// Circuit should be open with extended timeout
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		let state = state.unwrap();
		assert!(matches!(state.state, CircuitState::Open));

		// Should have extended timeout (24 hours)
		let expected_timeout_hours = 24;
		let actual_timeout_hours = state.timeout_duration.num_hours();
		// Allow some tolerance for test execution time
		assert!((actual_timeout_hours - expected_timeout_hours).abs() <= 1);
	}

	#[tokio::test]
	async fn test_open_to_half_open_transition() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);
		let solver = create_test_solver("test-solver");

		// Create an open circuit state with expired timeout
		let past_time = Utc::now() - chrono::Duration::minutes(30);
		let timeout_duration = chrono::Duration::seconds(10);
		let mut open_state = CircuitBreakerState::new_open(
			"test-solver".to_string(),
			"test reason".to_string(),
			timeout_duration,
			1,
		);
		open_state.opened_at = Some(past_time); // Set to 30 minutes ago
		open_state.next_test_at = Some(past_time + timeout_duration); // Should be 29 minutes 50 seconds ago (expired)
		storage
			.update_solver_circuit_state(open_state)
			.await
			.unwrap();

		// Verify the circuit thinks it should attempt reset
		let saved_state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap()
			.unwrap();
		assert!(
			saved_state.should_attempt_reset(),
			"Circuit should be ready for reset attempt"
		);

		// Request should be allowed (triggers transition to half-open)
		let allowed = service.should_allow_request(&solver).await;
		assert!(
			allowed,
			"Request should be allowed when circuit timeout has expired"
		);

		// Circuit should now be half-open
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::HalfOpen));
	}

	#[tokio::test]
	async fn test_is_enabled_functionality() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());

		// Test enabled circuit breaker
		let enabled_config = create_test_config(); // enabled: true by default
		let enabled_service = CircuitBreakerService::new(Arc::clone(&storage), enabled_config);
		assert!(enabled_service.is_enabled());

		// Test disabled circuit breaker
		let mut disabled_config = create_test_config();
		disabled_config.enabled = false;
		let disabled_service = CircuitBreakerService::new(storage, disabled_config);
		assert!(!disabled_service.is_enabled());
	}

	#[tokio::test]
	async fn test_storage_error_handling() {
		// This test would ideally use a mock storage that fails, but since we don't have that
		// we can at least test the basic error paths by testing with invalid solver IDs
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);
		let solver = create_test_solver("nonexistent-solver");

		// Should handle missing circuit state gracefully
		let allowed = service.should_allow_request(&solver).await;
		assert!(allowed, "Should fail open when no circuit state exists");

		// Recording result for non-existent circuit should not crash
		service
			.record_request_result("nonexistent-solver", true)
			.await;
		service
			.record_request_result("nonexistent-solver", false)
			.await;
	}

	#[tokio::test]
	async fn test_closed_circuit_no_action_on_record_result() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);

		// Create a closed circuit state
		let closed_state = CircuitBreakerState::new_closed("test-solver".to_string());
		storage
			.update_solver_circuit_state(closed_state)
			.await
			.unwrap();

		// Record both success and failure
		service.record_request_result("test-solver", true).await;
		service.record_request_result("test-solver", false).await;

		// Circuit should still be closed (no action taken)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Closed));
	}

	#[tokio::test]
	async fn test_open_circuit_no_action_on_record_result() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);

		// Create an open circuit state
		let open_state = CircuitBreakerState::new_open(
			"test-solver".to_string(),
			"test".to_string(),
			chrono::Duration::minutes(10),
			1,
		);
		storage
			.update_solver_circuit_state(open_state)
			.await
			.unwrap();

		// Record both success and failure
		service.record_request_result("test-solver", true).await;
		service.record_request_result("test-solver", false).await;

		// Circuit should still be open (no action taken)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap();
		assert!(state.is_some());
		assert!(matches!(state.unwrap().state, CircuitState::Open));
	}

	#[tokio::test]
	async fn test_timeout_calculation_edge_cases() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		config.base_timeout_seconds = 1;
		config.max_timeout_seconds = 16;
		let service = CircuitBreakerService::new(storage, config);

		// Test various failure counts
		assert_eq!(service.calculate_timeout_duration(0).num_seconds(), 1); // 1 * 2^0 = 1
		assert_eq!(service.calculate_timeout_duration(1).num_seconds(), 2); // 1 * 2^1 = 2
		assert_eq!(service.calculate_timeout_duration(2).num_seconds(), 4); // 1 * 2^2 = 4
		assert_eq!(service.calculate_timeout_duration(3).num_seconds(), 8); // 1 * 2^3 = 8
		assert_eq!(service.calculate_timeout_duration(4).num_seconds(), 16); // 1 * 2^4 = 16, at max
		assert_eq!(service.calculate_timeout_duration(10).num_seconds(), 16); // Should cap at max_timeout
		assert_eq!(service.calculate_timeout_duration(20).num_seconds(), 16); // Should cap at max_timeout
	}

	#[tokio::test]
	async fn test_solver_recovery_ux_scenario() {
		// This test demonstrates the UX improvement: a solver that failed historically
		// but is currently working should NOT be penalized by historical bad data
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		// Set failure threshold to 10, so rate checks start at 6 (ceil(10/2))
		config.failure_threshold = 10;
		let service = CircuitBreakerService::new(storage, config.clone());

		let mut solver = create_test_solver("recovering-solver");

		// Historical bad data (what would cause the original UX issue)
		solver.metrics.total_requests = 100;
		solver.metrics.successful_requests = 20; // 20% success rate (bad historically)
		solver.metrics.service_errors = 50; // 50% service error rate (bad historically)

		// Recent window shows it's working well now
		solver.metrics.recent_total_requests = 5; // Recent activity
		solver.metrics.recent_successful_requests = 5; // 100% recent success rate
		solver.metrics.recent_service_errors = 0; // 0% recent service error rate

		// Key: solver is currently working (no consecutive failures)
		solver.metrics.consecutive_failures = 1; // Below threshold for rate checks

		let decision = service.should_open_primary(&solver);
		// Should stay CLOSED - even though historical data is bad,
		// current performance (no consecutive failures) means we skip rate checks
		assert!(matches!(decision, CircuitDecision::Closed));

		// Now simulate the solver having issues again
		solver.metrics.consecutive_failures = (config.failure_threshold + 1) / 2; // Now rate checks are enabled (6 < 10, so won't trigger consecutive failure check)

		let decision_with_rate_checks = service.should_open_primary(&solver);
		// NOW the circuit breaker will look at rates and decide based on recent data
		// Since recent data is good (100% success, 0% service errors), it should stay closed
		assert!(matches!(decision_with_rate_checks, CircuitDecision::Closed));
	}

	#[tokio::test]
	async fn test_dynamic_rate_check_threshold() {
		// Test that rate check threshold scales with failure_threshold configuration using ceiling division
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());

		// Test with failure_threshold = 6, rate checks should start at 3 (6.div_ceil(2))
		let mut config = create_test_config();
		config.failure_threshold = 6;
		let service = CircuitBreakerService::new(storage.clone(), config.clone());

		let mut solver = create_test_solver("test-solver");
		solver.metrics.total_requests = 100;
		solver.metrics.successful_requests = 10; // 10% success rate (bad)
		solver.metrics.consecutive_failures = 2; // Below rate check threshold (3), rate checks should be skipped

		let decision = service.should_open_primary(&solver);
		assert!(
			matches!(decision, CircuitDecision::Closed),
			"Should stay closed when consecutive_failures (2) < failure_threshold.div_ceil(2) (3)"
		);

		// Now test with consecutive_failures = 3 (exactly at threshold)
		solver.metrics.consecutive_failures = 3; // At rate check threshold (3), rate checks should be enabled
		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Open { .. }), "Should open when consecutive_failures (3) >= failure_threshold.div_ceil(2) (3) and metrics are bad");

		// Test with different failure_threshold = 9, rate checks should start at 5 (9.div_ceil(2))
		config.failure_threshold = 9;
		let service = CircuitBreakerService::new(storage.clone(), config.clone());

		solver.metrics.consecutive_failures = 4; // Below rate check threshold (5), rate checks should be skipped
		let decision = service.should_open_primary(&solver);
		assert!(
			matches!(decision, CircuitDecision::Closed),
			"Should stay closed when consecutive_failures (4) < failure_threshold.div_ceil(2) (5)"
		);

		solver.metrics.consecutive_failures = 5; // At rate check threshold (5), rate checks should be enabled
		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Open { .. }), "Should open when consecutive_failures (5) >= failure_threshold.div_ceil(2) (5) and metrics are bad");

		// Test edge case: failure_threshold = 1, rate checks should start at 1 (1.div_ceil(2))
		config.failure_threshold = 1;
		let service = CircuitBreakerService::new(storage.clone(), config.clone());

		solver.metrics.consecutive_failures = 0; // Below rate check threshold (1), rate checks should be skipped
		let decision = service.should_open_primary(&solver);
		assert!(
			matches!(decision, CircuitDecision::Closed),
			"Should stay closed when consecutive_failures (0) < failure_threshold.div_ceil(2) (1)"
		);

		solver.metrics.consecutive_failures = 1; // At rate check threshold (1), rate checks should be enabled
		let decision = service.should_open_primary(&solver);
		assert!(matches!(decision, CircuitDecision::Open { .. }), "Should open when consecutive_failures (1) >= failure_threshold.div_ceil(2) (1) and metrics are bad");
	}

	#[tokio::test]
	async fn test_recovery_attempt_count_preserved_in_transition() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage.clone(), config.clone());

		// Create solver and set it to open state with recovery attempts
		let solver = create_test_solver("test-solver");

		let mut open_state = CircuitBreakerState::new_open(
			"test-solver".to_string(),
			"Test failure".to_string(),
			Duration::seconds(1), // Short timeout for test
			5,
		);
		open_state.recovery_attempt_count = 3; // Set specific recovery attempt count
		storage
			.update_solver_circuit_state(open_state.clone())
			.await
			.unwrap();

		// Wait for timeout to pass
		tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

		// Make a request - should transition to half-open and preserve recovery_attempt_count
		let _should_allow = service.should_allow_request(&solver).await;

		// Verify the state was transitioned to half-open with preserved recovery_attempt_count
		let states = storage.list_circuit_states().await.unwrap();
		let state = states
			.iter()
			.find(|s| s.solver_id == "test-solver")
			.unwrap();

		assert_eq!(state.state, CircuitState::HalfOpen);
		assert_eq!(
			state.recovery_attempt_count, 3,
			"Recovery attempt count should be preserved when transitioning to half-open"
		);

		// Verify debug log shows the recovery attempt number
		// (This would require a log capturing mechanism in a real test environment)
	}

	#[tokio::test]
	async fn test_windowed_service_error_rate() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let mut config = create_test_config();
		// Set failure threshold to 10, so rate checks start at 6 (ceil(10/2))
		config.failure_threshold = 10;
		let service = CircuitBreakerService::new(storage, config.clone());

		let mut solver = create_test_solver("test-solver");

		// Set up good lifetime service error rate
		solver.metrics.total_requests = 100;
		solver.metrics.service_errors = 10; // 10% service error rate (good)
		solver.metrics.successful_requests = 90; // Keep success rate good to isolate service error test

		// Set up bad recent windowed service error rate
		solver.metrics.recent_total_requests = 10; // Above min threshold (5)
		solver.metrics.recent_service_errors = 8; // 80% service error rate (bad)
		solver.metrics.recent_successful_requests = 8; // 80% success rate (good) - the rest are service errors
		solver.metrics.consecutive_failures = (config.failure_threshold + 1) / 2; // Enable rate checks (6 < 10, so won't trigger consecutive failure check)

		let decision = service.should_open_primary(&solver);
		// Should open circuit because windowed metrics (80% service error rate) are used
		assert!(matches!(decision, CircuitDecision::Open { .. }));
		if let CircuitDecision::Open { reason } = decision {
			assert!(reason.contains("service_error_rate_recent"));
		}
	}
}
