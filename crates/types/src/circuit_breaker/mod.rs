//! Circuit breaker types and core data structures
//!
//! This module provides the core data structures for implementing circuit breaker patterns
//! to protect against cascading failures in solver communication.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Circuit breaker state machine states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CircuitState {
	/// Normal operation - allow all requests
	Closed,
	/// Block requests - solver is failing  
	Open,
	/// Testing recovery - limited requests allowed
	HalfOpen,
}

/// Persistent circuit breaker state for a solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerState {
	/// Solver identifier this circuit protects
	pub solver_id: String,
	/// Current state of the circuit
	pub state: CircuitState,
	/// When the circuit was opened (None if not currently open)
	pub opened_at: Option<DateTime<Utc>>,
	/// How many failures caused the circuit to open
	pub failure_count_when_opened: u32,
	/// How long to wait before testing recovery
	pub timeout_duration: Duration,
	/// When the next test request should be allowed (None if not waiting)
	pub next_test_at: Option<DateTime<Utc>>,
	/// Human-readable reason why the circuit opened
	pub reason: Option<String>,
	/// Number of test requests made in half-open state
	pub test_request_count: u32,
	/// Number of successful test requests in half-open state
	pub successful_test_requests: u32,
	/// Number of failed test requests in half-open state
	pub failed_test_requests: u32,
	/// Number of times recovery has been attempted and failed
	pub recovery_attempt_count: u32,
	/// When this circuit breaker state was created
	pub created_at: DateTime<Utc>,
	/// When this state was last updated
	pub last_updated: DateTime<Utc>,
}

impl CircuitBreakerState {
	/// Create a new circuit breaker state in closed position
	pub fn new_closed(solver_id: String) -> Self {
		let now = Utc::now();
		Self {
			solver_id,
			state: CircuitState::Closed,
			opened_at: None,
			failure_count_when_opened: 0,
			timeout_duration: Duration::seconds(0),
			next_test_at: None,
			reason: None,
			test_request_count: 0,
			successful_test_requests: 0,
			failed_test_requests: 0,
			recovery_attempt_count: 0,
			created_at: now,
			last_updated: now,
		}
	}

	/// Create a new circuit breaker state in open position
	pub fn new_open(
		solver_id: String,
		reason: String,
		timeout_duration: Duration,
		failure_count: u32,
	) -> Self {
		let now = Utc::now();
		Self {
			solver_id,
			state: CircuitState::Open,
			opened_at: Some(now),
			failure_count_when_opened: failure_count,
			timeout_duration,
			next_test_at: Some(now + timeout_duration),
			reason: Some(reason),
			test_request_count: 0,
			successful_test_requests: 0,
			failed_test_requests: 0,
			recovery_attempt_count: 0,
			created_at: now,
			last_updated: now,
		}
	}

	/// Create a new circuit breaker state in half-open position
	pub fn new_half_open(solver_id: String) -> Self {
		let now = Utc::now();
		Self {
			solver_id,
			state: CircuitState::HalfOpen,
			opened_at: None,
			failure_count_when_opened: 0,
			timeout_duration: Duration::seconds(0),
			next_test_at: None,
			reason: None,
			test_request_count: 0,
			successful_test_requests: 0,
			failed_test_requests: 0,
			recovery_attempt_count: 0,
			created_at: now,
			last_updated: now,
		}
	}

	/// Check if this circuit should transition to half-open state
	pub fn should_attempt_reset(&self) -> bool {
		match self.state {
			CircuitState::Open => {
				if let Some(next_test_at) = self.next_test_at {
					Utc::now() >= next_test_at
				} else {
					false
				}
			},
			_ => false,
		}
	}

	/// Update the last_updated timestamp
	pub fn touch(&mut self) {
		self.last_updated = Utc::now();
	}
}

/// Decision result from circuit breaker logic evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitDecision {
	/// Allow the request - circuit is closed
	Closed,
	/// Block the request - circuit is open
	Open { reason: String },
	/// Not enough data to make a definitive decision
	Inconclusive,
}

impl CircuitDecision {
	/// Check if the decision allows requests
	pub fn allows_request(&self) -> bool {
		matches!(self, CircuitDecision::Closed)
	}

	/// Get the reason string if the circuit is open
	pub fn reason(&self) -> Option<&str> {
		match self {
			CircuitDecision::Open { reason } => Some(reason),
			_ => None,
		}
	}
}

/// Action to take when a solver persistently fails recovery attempts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersistentFailureAction {
	/// Keep trying indefinitely with capped timeout (current behavior)
	KeepTrying,
	/// Administratively disable the solver (requires manual re-enable)
	DisableSolver,
	/// Use very long timeout (24 hours) between recovery attempts  
	ExtendTimeout,
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::{Duration, Utc};

	#[test]
	fn test_circuit_breaker_state_new_closed() {
		let solver_id = "test-solver-1".to_string();
		let state = CircuitBreakerState::new_closed(solver_id.clone());

		assert_eq!(state.solver_id, solver_id);
		assert_eq!(state.state, CircuitState::Closed);
		assert!(state.opened_at.is_none());
		assert_eq!(state.failure_count_when_opened, 0);
		assert_eq!(state.timeout_duration, Duration::seconds(0));
		assert!(state.next_test_at.is_none());
		assert!(state.reason.is_none());
		assert_eq!(state.test_request_count, 0);
		assert_eq!(state.recovery_attempt_count, 0);

		// Timestamps should be recent (within 1 second)
		let now = Utc::now();
		let time_diff = (now - state.created_at).num_milliseconds().abs();
		assert!(
			time_diff < 1000,
			"created_at should be recent, diff: {}ms",
			time_diff
		);

		let time_diff = (now - state.last_updated).num_milliseconds().abs();
		assert!(
			time_diff < 1000,
			"last_updated should be recent, diff: {}ms",
			time_diff
		);

		assert_eq!(state.created_at, state.last_updated);
	}

	#[test]
	fn test_circuit_breaker_state_new_open() {
		let solver_id = "test-solver-2".to_string();
		let reason = "High failure rate detected".to_string();
		let timeout_duration = Duration::seconds(30);
		let failure_count = 5;

		let state = CircuitBreakerState::new_open(
			solver_id.clone(),
			reason.clone(),
			timeout_duration,
			failure_count,
		);

		assert_eq!(state.solver_id, solver_id);
		assert_eq!(state.state, CircuitState::Open);
		assert!(state.opened_at.is_some());
		assert_eq!(state.failure_count_when_opened, failure_count);
		assert_eq!(state.timeout_duration, timeout_duration);
		assert!(state.next_test_at.is_some());
		assert_eq!(state.reason, Some(reason));
		assert_eq!(state.test_request_count, 0);
		assert_eq!(state.recovery_attempt_count, 0);

		// Verify next_test_at is opened_at + timeout_duration
		let opened_at = state.opened_at.unwrap();
		let next_test_at = state.next_test_at.unwrap();
		assert_eq!(next_test_at, opened_at + timeout_duration);

		// Timestamps should be recent
		let now = Utc::now();
		let time_diff = (now - state.created_at).num_milliseconds().abs();
		assert!(time_diff < 1000, "created_at should be recent");

		assert_eq!(state.created_at, state.last_updated);
	}

	#[test]
	fn test_circuit_breaker_state_new_half_open() {
		let solver_id = "test-solver-3".to_string();
		let state = CircuitBreakerState::new_half_open(solver_id.clone());

		assert_eq!(state.solver_id, solver_id);
		assert_eq!(state.state, CircuitState::HalfOpen);
		assert!(state.opened_at.is_none());
		assert_eq!(state.failure_count_when_opened, 0);
		assert_eq!(state.timeout_duration, Duration::seconds(0));
		assert!(state.next_test_at.is_none());
		assert!(state.reason.is_none());
		assert_eq!(state.test_request_count, 0);
		assert_eq!(state.recovery_attempt_count, 0);

		// Timestamps should be recent
		let now = Utc::now();
		let time_diff = (now - state.created_at).num_milliseconds().abs();
		assert!(time_diff < 1000, "created_at should be recent");

		assert_eq!(state.created_at, state.last_updated);
	}

	#[test]
	fn test_should_attempt_reset_closed_state() {
		let state = CircuitBreakerState::new_closed("test-solver".to_string());
		assert!(
			!state.should_attempt_reset(),
			"Closed circuit should not attempt reset"
		);
	}

	#[test]
	fn test_should_attempt_reset_half_open_state() {
		let state = CircuitBreakerState::new_half_open("test-solver".to_string());
		assert!(
			!state.should_attempt_reset(),
			"Half-open circuit should not attempt reset"
		);
	}

	#[test]
	fn test_should_attempt_reset_open_state_not_ready() {
		let solver_id = "test-solver".to_string();
		let reason = "Test failure".to_string();
		let timeout_duration = Duration::seconds(30);
		let state = CircuitBreakerState::new_open(solver_id, reason, timeout_duration, 3);

		// Should not be ready immediately after creation
		assert!(
			!state.should_attempt_reset(),
			"Should not be ready for reset immediately"
		);
	}

	#[test]
	fn test_should_attempt_reset_open_state_ready() {
		let solver_id = "test-solver".to_string();
		let reason = "Test failure".to_string();
		let timeout_duration = Duration::milliseconds(1); // Very short timeout
		let state = CircuitBreakerState::new_open(solver_id, reason, timeout_duration, 3);

		// Wait a bit to ensure timeout has passed
		std::thread::sleep(std::time::Duration::from_millis(5));

		assert!(
			state.should_attempt_reset(),
			"Should be ready for reset after timeout"
		);
	}

	#[test]
	fn test_should_attempt_reset_open_state_no_next_test_at() {
		let mut state = CircuitBreakerState::new_open(
			"test-solver".to_string(),
			"Test failure".to_string(),
			Duration::seconds(30),
			3,
		);

		// Manually clear next_test_at to test edge case
		state.next_test_at = None;

		assert!(
			!state.should_attempt_reset(),
			"Should not attempt reset without next_test_at"
		);
	}

	#[test]
	fn test_touch_updates_timestamp() {
		let mut state = CircuitBreakerState::new_closed("test-solver".to_string());
		let original_updated = state.last_updated;
		let original_created = state.created_at;

		// Wait a small amount to ensure timestamp difference
		std::thread::sleep(std::time::Duration::from_millis(10));

		state.touch();

		assert_eq!(
			state.created_at, original_created,
			"created_at should not change"
		);
		assert!(
			state.last_updated > original_updated,
			"last_updated should be newer"
		);

		// Verify last_updated is recent
		let now = Utc::now();
		let time_diff = (now - state.last_updated).num_milliseconds().abs();
		assert!(
			time_diff < 1000,
			"last_updated should be very recent after touch()"
		);
	}

	#[test]
	fn test_circuit_decision_closed_allows_request() {
		let decision = CircuitDecision::Closed;
		assert!(
			decision.allows_request(),
			"Closed decision should allow requests"
		);
		assert!(
			decision.reason().is_none(),
			"Closed decision should have no reason"
		);
	}

	#[test]
	fn test_circuit_decision_open_blocks_request() {
		let reason = "High failure rate".to_string();
		let decision = CircuitDecision::Open {
			reason: reason.clone(),
		};

		assert!(
			!decision.allows_request(),
			"Open decision should block requests"
		);
		assert_eq!(
			decision.reason(),
			Some(reason.as_str()),
			"Open decision should return reason"
		);
	}

	#[test]
	fn test_circuit_decision_inconclusive_blocks_request() {
		let decision = CircuitDecision::Inconclusive;
		assert!(
			!decision.allows_request(),
			"Inconclusive decision should block requests (fail-safe)"
		);
		assert!(
			decision.reason().is_none(),
			"Inconclusive decision should have no reason"
		);
	}

	#[test]
	fn test_circuit_decision_equality() {
		assert_eq!(CircuitDecision::Closed, CircuitDecision::Closed);
		assert_eq!(CircuitDecision::Inconclusive, CircuitDecision::Inconclusive);

		let reason1 = "Same reason".to_string();
		let reason2 = "Same reason".to_string();
		assert_eq!(
			CircuitDecision::Open { reason: reason1 },
			CircuitDecision::Open { reason: reason2 }
		);

		let different_reason = "Different reason".to_string();
		assert_ne!(
			CircuitDecision::Open {
				reason: "Same reason".to_string()
			},
			CircuitDecision::Open {
				reason: different_reason
			}
		);
	}

	#[test]
	fn test_circuit_state_equality() {
		assert_eq!(CircuitState::Closed, CircuitState::Closed);
		assert_eq!(CircuitState::Open, CircuitState::Open);
		assert_eq!(CircuitState::HalfOpen, CircuitState::HalfOpen);

		assert_ne!(CircuitState::Closed, CircuitState::Open);
		assert_ne!(CircuitState::Open, CircuitState::HalfOpen);
		assert_ne!(CircuitState::HalfOpen, CircuitState::Closed);
	}

	#[test]
	fn test_persistent_failure_action_equality() {
		assert_eq!(
			PersistentFailureAction::KeepTrying,
			PersistentFailureAction::KeepTrying
		);
		assert_eq!(
			PersistentFailureAction::DisableSolver,
			PersistentFailureAction::DisableSolver
		);
		assert_eq!(
			PersistentFailureAction::ExtendTimeout,
			PersistentFailureAction::ExtendTimeout
		);

		assert_ne!(
			PersistentFailureAction::KeepTrying,
			PersistentFailureAction::DisableSolver
		);
		assert_ne!(
			PersistentFailureAction::DisableSolver,
			PersistentFailureAction::ExtendTimeout
		);
		assert_ne!(
			PersistentFailureAction::ExtendTimeout,
			PersistentFailureAction::KeepTrying
		);
	}

	#[test]
	fn test_circuit_breaker_state_serialization() {
		let state = CircuitBreakerState::new_open(
			"test-solver".to_string(),
			"Test reason".to_string(),
			Duration::seconds(60),
			5,
		);

		// Test serialization to JSON
		let json = serde_json::to_string(&state).expect("Should serialize to JSON");
		assert!(
			json.contains("test-solver"),
			"JSON should contain solver_id"
		);
		assert!(json.contains("Test reason"), "JSON should contain reason");

		// Test deserialization from JSON
		let deserialized: CircuitBreakerState =
			serde_json::from_str(&json).expect("Should deserialize from JSON");

		assert_eq!(deserialized.solver_id, state.solver_id);
		assert_eq!(deserialized.state, state.state);
		assert_eq!(deserialized.reason, state.reason);
		assert_eq!(
			deserialized.failure_count_when_opened,
			state.failure_count_when_opened
		);
	}

	#[test]
	fn test_circuit_state_serialization() {
		let closed = CircuitState::Closed;
		let open = CircuitState::Open;
		let half_open = CircuitState::HalfOpen;

		// Test serialization
		assert_eq!(serde_json::to_string(&closed).unwrap(), "\"Closed\"");
		assert_eq!(serde_json::to_string(&open).unwrap(), "\"Open\"");
		assert_eq!(serde_json::to_string(&half_open).unwrap(), "\"HalfOpen\"");

		// Test deserialization
		assert_eq!(
			serde_json::from_str::<CircuitState>("\"Closed\"").unwrap(),
			CircuitState::Closed
		);
		assert_eq!(
			serde_json::from_str::<CircuitState>("\"Open\"").unwrap(),
			CircuitState::Open
		);
		assert_eq!(
			serde_json::from_str::<CircuitState>("\"HalfOpen\"").unwrap(),
			CircuitState::HalfOpen
		);
	}

	#[test]
	fn test_persistent_failure_action_serialization() {
		let keep_trying = PersistentFailureAction::KeepTrying;
		let disable_solver = PersistentFailureAction::DisableSolver;
		let extend_timeout = PersistentFailureAction::ExtendTimeout;

		// Test serialization
		assert_eq!(
			serde_json::to_string(&keep_trying).unwrap(),
			"\"KeepTrying\""
		);
		assert_eq!(
			serde_json::to_string(&disable_solver).unwrap(),
			"\"DisableSolver\""
		);
		assert_eq!(
			serde_json::to_string(&extend_timeout).unwrap(),
			"\"ExtendTimeout\""
		);

		// Test deserialization
		assert_eq!(
			serde_json::from_str::<PersistentFailureAction>("\"KeepTrying\"").unwrap(),
			PersistentFailureAction::KeepTrying
		);
		assert_eq!(
			serde_json::from_str::<PersistentFailureAction>("\"DisableSolver\"").unwrap(),
			PersistentFailureAction::DisableSolver
		);
		assert_eq!(
			serde_json::from_str::<PersistentFailureAction>("\"ExtendTimeout\"").unwrap(),
			PersistentFailureAction::ExtendTimeout
		);
	}
}
