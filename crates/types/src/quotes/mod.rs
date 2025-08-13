//! Core Quote domain model and business logic

use chrono::Utc;
use uuid::Uuid;

pub mod errors;
pub mod request;
pub mod response;

pub use errors::{QuoteError, QuoteValidationError};
pub use request::QuoteRequest;
pub use response::QuoteResponse;

use crate::{QuoteDetails, QuoteOrder};

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
	/// Array of EIP-712 compliant orders
	pub orders: Vec<QuoteOrder>,
	/// Quote details matching request structure
	pub details: QuoteDetails,
	/// Quote validity timestamp
	pub valid_until: Option<u64>,
	/// Estimated time to completion in seconds
	pub eta: Option<u64>,
	/// Provider identifier
	pub provider: String,
	/// HMAC-SHA256 integrity checksum for quote verification
	/// This ensures the quote originated from the aggregator service
	pub integrity_checksum: String,
}

// ================================
// INTEGRITY PAYLOAD IMPLEMENTATION
// ================================

impl crate::IntegrityPayload for Quote {
	fn to_integrity_payload(&self) -> String {
		// Create a canonical string representation for integrity verification
		format!(
			"quote_id:{};solver_id:{};provider:{};valid_until:{};eta:{};orders_count:{};details:{}",
			self.quote_id,
			self.solver_id,
			self.provider,
			self.valid_until.unwrap_or(0),
			self.eta.unwrap_or(0),
			self.orders.len(),
			self.details_to_string()
		)
	}
}

impl Quote {
	/// Convert quote details to a canonical string for integrity verification
	fn details_to_string(&self) -> String {
		let inputs_str = self
			.details
			.available_inputs
			.iter()
			.map(|input| format!("{}:{}", input.asset, input.amount.as_str()))
			.collect::<Vec<_>>()
			.join(",");

		let outputs_str = self
			.details
			.requested_outputs
			.iter()
			.map(|output| format!("{}:{}", output.asset, output.amount.as_str()))
			.collect::<Vec<_>>()
			.join(",");

		format!("inputs:[{}];outputs:[{}]", inputs_str, outputs_str)
	}
}

impl Quote {
	/// Create a new quote with the given parameters
	pub fn new(
		solver_id: String,
		orders: Vec<QuoteOrder>,
		details: QuoteDetails,
		provider: String,
		integrity_checksum: String,
	) -> Self {
		let quote_id = Uuid::new_v4().to_string();

		Self {
			quote_id,
			solver_id,
			orders,
			details,
			valid_until: None,
			eta: None,
			provider,
			integrity_checksum,
		}
	}

	/// Check if the quote has expired
	pub fn is_expired(&self) -> bool {
		if let Some(valid_until) = self.valid_until {
			Utc::now().timestamp() as u64 > valid_until
		} else {
			false // No expiration if valid_until is not set
		}
	}

	/// Calculate the exchange rate (total output / total input)
	pub fn exchange_rate(&self) -> QuoteResult<f64> {
		// Calculate total input amount
		let total_input: f64 = self
			.details
			.available_inputs
			.iter()
			.map(|input| input.amount.as_str().parse::<f64>().unwrap_or(0.0))
			.sum();

		if total_input == 0.0 {
			return Err(QuoteError::ProcessingFailed {
				reason: "Cannot calculate exchange rate: total input is zero".to_string(),
			});
		}

		// Calculate total output amount
		let total_output: f64 = self
			.details
			.requested_outputs
			.iter()
			.map(|output| output.amount.as_str().parse::<f64>().unwrap_or(0.0))
			.sum();

		Ok(total_output / total_input)
	}

	/// Check if this quote is better than another (higher output amount)
	pub fn is_better_than(&self, other: &Quote) -> QuoteResult<bool> {
		// For simplicity, compare total output amounts - higher is better
		let self_total_output: f64 = self
			.details
			.requested_outputs
			.iter()
			.map(|output| output.amount.as_str().parse::<f64>().unwrap_or(0.0))
			.sum();

		let other_total_output: f64 = other
			.details
			.requested_outputs
			.iter()
			.map(|output| output.amount.as_str().parse::<f64>().unwrap_or(0.0))
			.sum();

		Ok(self_total_output > other_total_output)
	}

	/// Update the expiration time
	pub fn with_valid_until(mut self, valid_until: u64) -> Self {
		self.valid_until = Some(valid_until);
		self
	}

	/// Set ETA
	pub fn with_eta(mut self, eta: u64) -> Self {
		self.eta = Some(eta);
		self
	}
}

#[cfg(test)]
mod tests {
	use crate::U256;

	use super::*;

	fn create_test_quote() -> Quote {
		use crate::{
			AvailableInput, InteropAddress, QuoteDetails, QuoteOrder, RequestedOutput,
			SignatureType, U256,
		};

		let input = AvailableInput {
			asset: InteropAddress::from_hex("0x01A0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap(),
			amount: U256::new("1000000000000000000".to_string()),
			user: InteropAddress::from_hex("0x01A0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0").unwrap(),
			lock: None,
		};

		let output = RequestedOutput {
			asset: InteropAddress::from_hex("0x01C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap(),
			amount: U256::new("2500000000000000000000".to_string()),
			receiver: InteropAddress::from_hex("0x01A0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap(),
			calldata: None,
		};

		let details = QuoteDetails {
			available_inputs: vec![input],
			requested_outputs: vec![output],
		};

		let order = QuoteOrder {
			signature_type: SignatureType::Eip712,
			domain: InteropAddress::from_hex("0x01A0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap(),
			primary_type: "Order".to_string(),
			message: serde_json::json!({}),
		};

		Quote::new(
			"test-solver".to_string(),
			vec![order],
			details,
			"test-provider".to_string(),
			"test-checksum".to_string(),
		)
	}

	#[test]
	fn test_quote_creation() {
		let quote = create_test_quote();

		assert_eq!(quote.solver_id, "test-solver");
		assert_eq!(quote.provider, "test-provider");
		assert_eq!(quote.integrity_checksum, "test-checksum");
		assert!(!quote.is_expired());
		assert_eq!(quote.orders.len(), 1);
		assert_eq!(quote.details.available_inputs.len(), 1);
		assert_eq!(quote.details.requested_outputs.len(), 1);
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

		// Update quote2 to have a higher output amount
		quote2.details.requested_outputs[0].amount =
			U256::new("3000000000000000000000".to_string()); // 3000 USDC

		assert!(quote2.is_better_than(&quote1).unwrap());
		assert!(!quote1.is_better_than(&quote2).unwrap());
	}

	#[test]
	fn test_quote_expiration() {
		let mut quote = create_test_quote();
		let past_timestamp = (Utc::now() - chrono::Duration::minutes(1)).timestamp() as u64;
		quote.valid_until = Some(past_timestamp);

		assert!(quote.is_expired());

		// Test no expiration when valid_until is None
		quote.valid_until = None;
		assert!(!quote.is_expired());
	}

	#[test]
	fn test_quote_builder_pattern() {
		let quote = create_test_quote()
			.with_valid_until(1234567890)
			.with_eta(300);

		assert_eq!(quote.valid_until, Some(1234567890));
		assert_eq!(quote.eta, Some(300));
	}

	#[test]
	fn test_quote_integrity_payload() {
		use crate::IntegrityPayload;

		let quote = create_test_quote();
		let payload = quote.to_integrity_payload();

		// Verify payload contains critical fields
		assert!(payload.contains(&format!("quote_id:{}", quote.quote_id)));
		assert!(payload.contains(&format!("solver_id:{}", quote.solver_id)));
		assert!(payload.contains(&format!("provider:{}", quote.provider)));
		assert!(payload.contains("orders_count:1"));
		assert!(payload.contains("details:"));

		// Verify deterministic output
		let payload2 = quote.to_integrity_payload();
		assert_eq!(payload, payload2);
	}
}
