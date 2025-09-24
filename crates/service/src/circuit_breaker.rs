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

	/// PRIMARY DECISION LOGIC: Fast evaluation using existing SolverMetrics
	///
	/// This method implements the core circuit breaker logic without async operations
	/// for maximum performance during request filtering.
	pub fn should_open_primary(&self, solver: &Solver) -> CircuitDecision {
		let metrics = &solver.metrics;

		// Early return for stale metrics - avoid all metrics-based checks if data is too old
		let metrics_age = Utc::now().signed_duration_since(metrics.last_updated);
		let max_age = chrono::Duration::minutes(self.config.metrics_max_age_minutes as i64);

		if metrics_age >= max_age {
			debug!(
				"Skipping metrics-based checks for solver '{}' - metrics are {} minutes old (max: {})",
				solver.solver_id,
				metrics_age.num_minutes(),
				self.config.metrics_max_age_minutes
			);
			return CircuitDecision::Inconclusive; // Can't decide with stale data
		}

		// 1. Consecutive failures threshold
		if metrics.consecutive_failures >= self.config.failure_threshold {
			return CircuitDecision::Open {
				reason: format!(
					"consecutive_failures: {} (metrics age: {}min)",
					metrics.consecutive_failures,
					metrics_age.num_minutes()
				),
			};
		}

		// 2. Success rate check with intelligent fallback
		let (success_rate, total_requests, data_source) =
			if metrics.recent_total_requests >= self.config.min_requests_for_rate_check {
				// Sufficient windowed data - use recent metrics
				let rate = if metrics.recent_total_requests > 0 {
					metrics.recent_successful_requests as f64 / metrics.recent_total_requests as f64
				} else {
					1.0
				};
				(rate, metrics.recent_total_requests, "recent")
			} else if metrics.total_requests >= self.config.min_requests_for_rate_check {
				// Insufficient windowed data but sufficient lifetime data - fallback to lifetime
				let rate = if metrics.total_requests > 0 {
					metrics.successful_requests as f64 / metrics.total_requests as f64
				} else {
					1.0
				};
				(rate, metrics.total_requests, "lifetime_fallback")
			} else {
				// Insufficient data overall - skip rate check
				(1.0, 0, "")
			};

		if total_requests > 0 && success_rate < self.config.success_rate_threshold {
			return CircuitDecision::Open {
				reason: format!(
					"success_rate_{}: {:.2} (over {} requests, window: {}min, metrics age: {}min)",
					data_source,
					success_rate,
					total_requests,
					self.config.metrics_window_duration_minutes,
					metrics_age.num_minutes()
				),
			};
		}

		// 3. Service error rate check with intelligent fallback
		let (service_error_rate, total_requests, data_source) =
			if metrics.recent_total_requests >= self.config.min_requests_for_rate_check {
				// Sufficient windowed data - use recent metrics
				let rate = if metrics.recent_total_requests > 0 {
					metrics.recent_service_errors as f64 / metrics.recent_total_requests as f64
				} else {
					0.0
				};
				(rate, metrics.recent_total_requests, "recent")
			} else if metrics.total_requests >= self.config.min_requests_for_rate_check {
				// Insufficient windowed data but sufficient lifetime data - fallback to lifetime
				let rate = if metrics.total_requests > 0 {
					metrics.service_errors as f64 / metrics.total_requests as f64
				} else {
					0.0
				};
				(rate, metrics.total_requests, "lifetime_fallback")
			} else {
				// Insufficient data overall - skip error rate check
				(0.0, 0, "")
			};

		if total_requests > 0 && service_error_rate > self.config.service_error_threshold {
			return CircuitDecision::Open {
				reason: format!(
					"service_error_rate_{}: {:.2} (over {} requests, window: {}min, metrics age: {}min)",
					data_source,
					service_error_rate,
					total_requests,
					self.config.metrics_window_duration_minutes,
					metrics_age.num_minutes()
				),
			};
		}

		// All checks passed - circuit should remain closed
		CircuitDecision::Closed
	}

	/// Calculate exponential backoff timeout duration
	fn calculate_timeout_duration(&self, failure_count: u32) -> Duration {
		let base = self.config.base_timeout_seconds;
		let max_timeout = self.config.max_timeout_seconds;

		// Exponential backoff: base * 2^failure_count, capped at max
		let timeout_seconds = std::cmp::min(
			base * 2_u64.pow(failure_count.min(10)), // Cap exponent to prevent overflow
			max_timeout,
		);

		Duration::seconds(timeout_seconds as i64)
	}

	/// Transition circuit to open state
	async fn transition_to_open(&self, solver_id: String, reason: String, failure_count: u32) {
		let timeout_duration = self.calculate_timeout_duration(failure_count);
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
	async fn transition_to_half_open(&self, solver_id: String) {
		let mut state = CircuitBreakerState::new_half_open(solver_id.clone());
		state.touch();

		debug!(
			"Circuit breaker transitioning to half-open for solver '{}'",
			solver_id
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
		let timeout_duration = self.calculate_timeout_duration(failure_count);
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

	/// Main request filtering logic - determines if requests should be allowed
	async fn should_allow_request(&self, solver: &Solver) -> bool {
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
					self.transition_to_half_open(solver.solver_id.clone()).await;
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
				// Half-open state - allow limited test requests
				if circuit_state.test_request_count < self.config.half_open_max_calls {
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
		let circuit_state = match self.storage.get_solver_circuit_state(solver_id).await {
			Ok(Some(state)) => state,
			Ok(None) => return, // No circuit state - nothing to update
			Err(e) => {
				warn!("Failed to get circuit state for result recording: {}", e);
				return;
			},
		};

		match circuit_state.state {
			CircuitState::HalfOpen => {
				// Half-open state - test request result determines next state
				if success {
					// Test succeeded - close the circuit
					self.transition_to_closed(solver_id.to_string()).await;
				} else {
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
							"failed_during_half_open".to_string(),
							new_failure_count,
							new_recovery_attempts,
						)
						.await;
					}
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
			half_open_max_calls: 2,
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

		// Third request should be blocked since half_open_max_calls is 2
		let allowed3 = circuit_breaker.should_allow_request(&solver).await;
		assert!(
			!allowed3,
			"Third request should be blocked - max calls reached"
		);

		// Verify count stayed at 2 (not incremented for blocked request)
		let state = storage
			.get_solver_circuit_state("test-solver")
			.await
			.unwrap()
			.unwrap();
		assert_eq!(
			state.test_request_count, 2,
			"Test request count should still be 2"
		);
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
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");
		solver.metrics.total_requests = 10; // Above min threshold
		solver.metrics.successful_requests = 2; // 20% success rate (below 50% threshold)

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
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");
		solver.metrics.total_requests = 10; // Above min threshold
		solver.metrics.service_errors = 8; // 80% service error rate (above 50% threshold)
		solver.metrics.successful_requests = 8; // 80% success rate (good) - the rest are service errors, not failed requests
		solver.metrics.consecutive_failures = 0; // Keep below failure threshold (3) to isolate service error test

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
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");
		
		// Set up insufficient windowed data
		solver.metrics.recent_total_requests = 2; // Below min threshold (5)
		solver.metrics.recent_successful_requests = 2; // 100% success rate but insufficient data
		
		// Set up lifetime data with bad success rate
		solver.metrics.total_requests = 10; // Above min threshold
		solver.metrics.successful_requests = 2; // 20% success rate (bad)

		let decision = service.should_open_primary(&solver);
		// Should open circuit because it falls back to lifetime metrics (20% success rate)
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
		storage.update_solver_circuit_state(half_open_state).await.unwrap();

		// Record a successful request
		service.record_request_result("test-solver", true).await;

		// Circuit should now be closed
		let state = storage.get_solver_circuit_state("test-solver").await.unwrap();
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
		storage.update_solver_circuit_state(half_open_state).await.unwrap();

		// Record a failed request
		service.record_request_result("test-solver", false).await;

		// Circuit should now be open again
		let state = storage.get_solver_circuit_state("test-solver").await.unwrap();
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
		storage.update_solver_circuit_state(half_open_state).await.unwrap();

		// Record a failed request (should trigger persistent failure handling)
		service.record_request_result("test-solver", false).await;

		// Circuit should still be open (KeepTrying action)
		let state = storage.get_solver_circuit_state("test-solver").await.unwrap();
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
		storage.update_solver_circuit_state(half_open_state).await.unwrap();

		// Record a failed request (should trigger persistent failure handling)
		service.record_request_result("test-solver", false).await;

		// Solver should be disabled
		let solver = storage.get_solver("test-solver").await.unwrap();
		assert!(solver.is_some());
		assert!(matches!(solver.unwrap().status, oif_types::SolverStatus::Disabled));

		// Circuit state should be removed
		let state = storage.get_solver_circuit_state("test-solver").await.unwrap();
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
		storage.update_solver_circuit_state(half_open_state).await.unwrap();

		// Record a failed request (should trigger persistent failure handling)
		service.record_request_result("test-solver", false).await;

		// Circuit should be open with extended timeout
		let state = storage.get_solver_circuit_state("test-solver").await.unwrap();
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
			1
		);
		open_state.opened_at = Some(past_time); // Set to 30 minutes ago
		open_state.next_test_at = Some(past_time + timeout_duration); // Should be 29 minutes 50 seconds ago (expired)
		storage.update_solver_circuit_state(open_state).await.unwrap();

		// Verify the circuit thinks it should attempt reset
		let saved_state = storage.get_solver_circuit_state("test-solver").await.unwrap().unwrap();
		assert!(saved_state.should_attempt_reset(), "Circuit should be ready for reset attempt");

		// Request should be allowed (triggers transition to half-open)
		let allowed = service.should_allow_request(&solver).await;
		assert!(allowed, "Request should be allowed when circuit timeout has expired");

		// Circuit should now be half-open
		let state = storage.get_solver_circuit_state("test-solver").await.unwrap();
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
		service.record_request_result("nonexistent-solver", true).await;
		service.record_request_result("nonexistent-solver", false).await;
	}

	#[tokio::test]
	async fn test_closed_circuit_no_action_on_record_result() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(Arc::clone(&storage), config);

		// Create a closed circuit state
		let closed_state = CircuitBreakerState::new_closed("test-solver".to_string());
		storage.update_solver_circuit_state(closed_state).await.unwrap();

		// Record both success and failure
		service.record_request_result("test-solver", true).await;
		service.record_request_result("test-solver", false).await;

		// Circuit should still be closed (no action taken)
		let state = storage.get_solver_circuit_state("test-solver").await.unwrap();
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
			1
		);
		storage.update_solver_circuit_state(open_state).await.unwrap();

		// Record both success and failure
		service.record_request_result("test-solver", true).await;
		service.record_request_result("test-solver", false).await;

		// Circuit should still be open (no action taken)
		let state = storage.get_solver_circuit_state("test-solver").await.unwrap();
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
	async fn test_windowed_service_error_rate() {
		let storage: Arc<dyn Storage> = Arc::new(MemoryStore::new());
		let config = create_test_config();
		let service = CircuitBreakerService::new(storage, config);

		let mut solver = create_test_solver("test-solver");
		
		// Set up good lifetime service error rate
		solver.metrics.total_requests = 100;
		solver.metrics.service_errors = 10; // 10% service error rate (good)
		solver.metrics.successful_requests = 90; // Keep success rate good to isolate service error test
		
		// Set up bad recent windowed service error rate
		solver.metrics.recent_total_requests = 10; // Above min threshold (5)
		solver.metrics.recent_service_errors = 8; // 80% service error rate (bad)
		solver.metrics.recent_successful_requests = 8; // 80% success rate (good) - the rest are service errors
		solver.metrics.consecutive_failures = 0; // Keep below failure threshold (3) to isolate service error test

		let decision = service.should_open_primary(&solver);
		// Should open circuit because windowed metrics (80% service error rate) are used
		assert!(matches!(decision, CircuitDecision::Open { .. }));
		if let CircuitDecision::Open { reason } = decision {
			assert!(reason.contains("service_error_rate_recent"));
		}
	}
}
