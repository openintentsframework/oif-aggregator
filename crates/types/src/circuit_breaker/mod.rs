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
