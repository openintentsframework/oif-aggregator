//! Core Quote domain model and business logic

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod errors;
pub mod request;
pub mod response;
pub mod storage;

pub use errors::{QuoteError, QuoteValidationError};
pub use request::QuoteRequest;
pub use response::{FeeInfo, QuoteResponse, RouteStep, TokenInfo};
pub use storage::QuoteStorage;

/// Result type for quote operations
pub type QuoteResult<T> = Result<T, QuoteError>;

/// Result type for quote validation operations
pub type QuoteValidationResult<T> = Result<T, QuoteValidationError>;

/// Core Quote domain model
///
/// This represents a quote in the domain layer with business logic.
/// It should be converted from QuoteRequest and to QuoteStorage/QuoteResponse.
#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
	/// Unique identifier for the quote
	pub quote_id: String,

	/// ID of the solver that provided this quote
	pub solver_id: String,

	/// ID of the original request that generated this quote
	pub request_id: String,

	/// Input token address
	pub token_in: String,

	/// Output token address  
	pub token_out: String,

	/// Input amount (as string to preserve precision)
	pub amount_in: String,

	/// Output amount (as string to preserve precision)
	pub amount_out: String,

	/// Chain ID where the swap occurs
	pub chain_id: u64,

	/// Estimated gas cost for the transaction
	pub estimated_gas: Option<u64>,

	/// Price impact as a percentage (0.01 = 1%)
	pub price_impact: Option<f64>,

	/// Response time from solver in milliseconds
	pub response_time_ms: u64,

	/// Confidence score (0.0 to 1.0)
	pub confidence_score: Option<f64>,

	/// Quote status
	pub status: QuoteStatus,

	/// When the quote was created
	pub created_at: DateTime<Utc>,

	/// When the quote expires
	pub expires_at: DateTime<Utc>,

	/// Raw response from the solver (for debugging/future use)
	pub raw_response: serde_json::Value,

	/// HMAC-SHA256 integrity checksum for quote verification
	/// This ensures the quote originated from the aggregator service
	pub integrity_checksum: Option<String>,
}

/// Quote status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QuoteStatus {
	/// Quote is valid and can be used
	Valid,
	/// Quote has expired
	Expired,
	/// Quote processing failed
	Failed,
	/// Quote is being processed
	Processing,
}

impl Quote {
	/// Create a new quote with the given parameters
	pub fn new(
		solver_id: String,
		request_id: String,
		token_in: String,
		token_out: String,
		amount_in: String,
		amount_out: String,
		chain_id: u64,
	) -> Self {
		let now = Utc::now();
		let quote_id = Uuid::new_v4().to_string();

		Self {
			quote_id,
			solver_id,
			request_id,
			token_in,
			token_out,
			amount_in,
			amount_out,
			chain_id,
			estimated_gas: None,
			price_impact: None,
			response_time_ms: 0,
			confidence_score: None,
			status: QuoteStatus::Valid,
			created_at: now,
			expires_at: now + Duration::minutes(5), // Default 5-minute TTL
			raw_response: serde_json::Value::Null,
			integrity_checksum: None,
		}
	}

	/// Check if the quote has expired
	pub fn is_expired(&self) -> bool {
		Utc::now() > self.expires_at
	}

	/// Calculate the exchange rate (amount_out / amount_in)
	pub fn exchange_rate(&self) -> QuoteResult<f64> {
		let amount_in: f64 = self
			.amount_in
			.parse()
			.map_err(|_| QuoteError::ProcessingFailed {
				reason: "Invalid amount_in format".to_string(),
			})?;
		let amount_out: f64 =
			self.amount_out
				.parse()
				.map_err(|_| QuoteError::ProcessingFailed {
					reason: "Invalid amount_out format".to_string(),
				})?;

		if amount_in == 0.0 {
			return Err(QuoteError::ProcessingFailed {
				reason: "Cannot calculate exchange rate: amount_in is zero".to_string(),
			});
		}

		Ok(amount_out / amount_in)
	}

	/// Check if this quote is better than another (higher output amount)
	pub fn is_better_than(&self, other: &Quote) -> QuoteResult<bool> {
		if self.chain_id != other.chain_id {
			return Err(QuoteError::ProcessingFailed {
				reason: "Cannot compare quotes from different chains".to_string(),
			});
		}

		if self.token_in != other.token_in || self.token_out != other.token_out {
			return Err(QuoteError::ProcessingFailed {
				reason: "Cannot compare quotes for different token pairs".to_string(),
			});
		}

		let self_amount: f64 =
			self.amount_out
				.parse()
				.map_err(|_| QuoteError::ProcessingFailed {
					reason: "Invalid amount_out format in self".to_string(),
				})?;
		let other_amount: f64 =
			other
				.amount_out
				.parse()
				.map_err(|_| QuoteError::ProcessingFailed {
					reason: "Invalid amount_out format in other".to_string(),
				})?;

		Ok(self_amount > other_amount)
	}

	/// Set the quote as expired
	pub fn mark_expired(&mut self) {
		self.status = QuoteStatus::Expired;
	}

	/// Set the quote as failed
	pub fn mark_failed(&mut self) {
		self.status = QuoteStatus::Failed;
	}

	/// Update the expiration time
	pub fn extend_ttl(&mut self, duration: Duration) {
		self.expires_at = self.expires_at + duration;
	}

	/// Set additional metadata
	pub fn with_estimated_gas(mut self, gas: u64) -> Self {
		self.estimated_gas = Some(gas);
		self
	}

	pub fn with_price_impact(mut self, impact: f64) -> Self {
		self.price_impact = Some(impact);
		self
	}

	pub fn with_response_time(mut self, time_ms: u64) -> Self {
		self.response_time_ms = time_ms;
		self
	}

	pub fn with_confidence_score(mut self, score: f64) -> Self {
		self.confidence_score = Some(score);
		self
	}

	pub fn with_raw_response(mut self, raw: serde_json::Value) -> Self {
		self.raw_response = raw;
		self
	}

	pub fn with_custom_ttl(mut self, duration: Duration) -> Self {
		self.expires_at = self.created_at + duration;
		self
	}
}

impl Quote {
	/// Generate a canonical payload string for integrity verification
	///
	/// This method is used by the integrity service to create a consistent
	/// string representation of the quote for HMAC generation.
	pub fn to_integrity_payload(&self) -> String {
		format!(
            "quote_id={}|solver_id={}|request_id={}|token_in={}|token_out={}|amount_in={}|amount_out={}|chain_id={}|created_at={}",
            self.quote_id,
            self.solver_id,
            self.request_id,
            self.token_in,
            self.token_out,
            self.amount_in,
            self.amount_out,
            self.chain_id,
            self.created_at.timestamp_millis()
        )
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn create_test_quote() -> Quote {
		Quote::new(
			"test-solver".to_string(),
			"req-123".to_string(),
			"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
			"1000000000000000000".to_string(),    // 1 ETH
			"2500000000000000000000".to_string(), // 2500 USDC
			1,
		)
	}

	#[test]
	fn test_quote_creation() {
		let quote = create_test_quote();

		assert_eq!(quote.solver_id, "test-solver");
		assert_eq!(quote.request_id, "req-123");
		assert_eq!(quote.chain_id, 1);
		assert_eq!(quote.status, QuoteStatus::Valid);
		assert!(!quote.is_expired());
	}

	#[test]
	fn test_exchange_rate_calculation() {
		let quote = create_test_quote();
		let rate = quote.exchange_rate().unwrap();
		assert_eq!(rate, 2500.0);
	}

	#[test]
	fn test_quote_comparison() {
		let quote1 = create_test_quote();
		let mut quote2 = create_test_quote();
		quote2.amount_out = "3000000000000000000000".to_string(); // 3000 USDC

		assert!(quote2.is_better_than(&quote1).unwrap());
		assert!(!quote1.is_better_than(&quote2).unwrap());
	}

	#[test]
	fn test_quote_expiration() {
		let mut quote = create_test_quote();
		quote.expires_at = Utc::now() - Duration::minutes(1);

		assert!(quote.is_expired());
	}

	#[test]
	fn test_quote_builder_pattern() {
		let quote = create_test_quote()
			.with_estimated_gas(21000)
			.with_price_impact(0.01)
			.with_response_time(150)
			.with_confidence_score(0.95);

		assert_eq!(quote.estimated_gas, Some(21000));
		assert_eq!(quote.price_impact, Some(0.01));
		assert_eq!(quote.response_time_ms, 150);
		assert_eq!(quote.confidence_score, Some(0.95));
	}

	#[test]
	fn test_quote_integrity_payload() {
		let quote = create_test_quote();
		let payload = quote.to_integrity_payload();

		// Verify payload contains all critical fields
		assert!(payload.contains(&format!("quote_id={}", quote.quote_id)));
		assert!(payload.contains(&format!("solver_id={}", quote.solver_id)));
		assert!(payload.contains(&format!("request_id={}", quote.request_id)));
		assert!(payload.contains(&format!("token_in={}", quote.token_in)));
		assert!(payload.contains(&format!("token_out={}", quote.token_out)));
		assert!(payload.contains(&format!("amount_in={}", quote.amount_in)));
		assert!(payload.contains(&format!("amount_out={}", quote.amount_out)));
		assert!(payload.contains(&format!("chain_id={}", quote.chain_id)));
		assert!(payload.contains(&format!(
			"created_at={}",
			quote.created_at.timestamp_millis()
		)));

		// Verify deterministic output
		let payload2 = quote.to_integrity_payload();
		assert_eq!(payload, payload2);
	}
}
