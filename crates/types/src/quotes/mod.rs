//! Core Quote domain model and business logic

use chrono::Utc;
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;
use uuid::Uuid;

pub mod errors;
pub mod request;
pub mod response;

pub use errors::{QuoteError, QuoteValidationError};
pub use request::QuoteRequest;
pub use response::QuoteResponse;

use crate::oif::OifQuote;

/// Result type for quote operations
pub type QuoteResult<T> = Result<T, QuoteError>;

/// Result type for quote validation operations
pub type QuoteValidationResult<T> = Result<T, QuoteValidationError>;

/// Core Quote domain model using composition over duplication
///
/// This represents a quote in the domain layer with business logic.
/// It wraps the OIF specification quote
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Quote {
	/// Unique identifier for the quote (aggregator-specific)
	pub quote_id: String,
	/// ID of the solver that provided this quote (aggregator-specific)
	pub solver_id: String,
	/// The actual OIF quote containing order and execution details (version-agnostic, version is encoded in wrapper)
	pub quote: OifQuote,
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
		// Using data from the composed OIF quote
		format!(
			"quote_id:{};solver_id:{};version:{};provider:{};valid_until:{};eta:{};orders_count:{};details:{}",
			self.quote_id,
			self.solver_id,
			self.quote.version(),
			self.quote.provider().unwrap_or(&self.solver_id),
			self.quote.valid_until().unwrap_or(0),
			self.quote.eta().unwrap_or(0),
			1, // Single order in OIF spec
			self.details_to_string()
		)
	}
}

impl Quote {
	/// Convert quote details to a canonical string for integrity verification
	///
	/// Uses only the critical fields that define the quote's economic terms:
	/// - Order type (which settlement mechanism)
	/// - Preview inputs/outputs (exact amounts, assets, parties)
	/// - Failure handling and partial fill settings
	///
	/// This avoids serialization issues with complex nested EIP712 structures
	/// while still protecting the quote's essential terms.
	fn details_to_string(&self) -> String {
		// Extract order type string
		let order_type = self.order_type_string();

		// Serialize preview (inputs/outputs) - these are the critical economic terms
		let preview_str = match serde_json::to_string(self.quote.preview()) {
			Ok(s) => s,
			Err(_) => return "preview_serialization_failed".to_string(),
		};

		// Include failure handling and partial fill as they affect execution
		let failure_handling = self
			.quote
			.failure_handling()
			.map(|fh| format!("{:?}", fh))
			.unwrap_or_else(|| "none".to_string());

		format!(
			"order_type:{};preview:{};failure_handling:{};partial_fill:{}",
			order_type,
			preview_str,
			failure_handling,
			self.quote.partial_fill()
		)
	}

	/// Get a string representation of the order type
	fn order_type_string(&self) -> &str {
		match self.quote.order() {
			crate::oif::v0::Order::OifEscrowV0 { .. } => "oif-escrow-v0",
			crate::oif::v0::Order::OifResourceLockV0 { .. } => "oif-resource-lock-v0",
			crate::oif::v0::Order::Oif3009V0 { .. } => "oif-3009-v0",
			crate::oif::v0::Order::OifGenericV0 { .. } => "oif-generic-v0",
			crate::oif::v0::Order::Across { .. } => "across",
		}
	}
}

impl Quote {
	/// Create a new quote with the given parameters using OIF composition
	pub fn new(solver_id: String, quote: OifQuote, integrity_checksum: String) -> Self {
		let quote_id = Uuid::new_v4().to_string();

		Self {
			quote_id,
			solver_id,
			quote,
			integrity_checksum,
		}
	}

	/// Check if the quote has expired
	pub fn is_expired(&self) -> bool {
		if let Some(valid_until) = self.quote.valid_until() {
			Utc::now().timestamp() as u64 > valid_until
		} else {
			false // No expiration if valid_until is not set
		}
	}

	/// Get the preview from the OIF quote
	pub fn preview(&self) -> &crate::oif::common::QuotePreview {
		self.quote.preview()
	}

	/// Get the metadata from the OIF quote
	pub fn metadata(&self) -> Option<&serde_json::Value> {
		self.quote.metadata()
	}

	/// Get the OIF version from embedded wrapper
	pub fn version(&self) -> &'static str {
		self.quote.version()
	}

	/// Get the order from the OIF quote
	pub fn order(&self) -> &crate::oif::OifOrderLatest {
		self.quote.order()
	}

	/// Get the provider from the OIF quote
	pub fn provider(&self) -> Option<&str> {
		self.quote.provider().map(|s| s.as_str())
	}

	/// Get the ETA from the OIF quote
	pub fn eta(&self) -> Option<u64> {
		self.quote.eta()
	}

	/// Get the valid_until from the OIF quote
	pub fn valid_until(&self) -> Option<u64> {
		self.quote.valid_until()
	}

	/// Get the partial_fill flag from the OIF quote
	pub fn partial_fill(&self) -> bool {
		self.quote.partial_fill()
	}

	/// Get the failure_handling from the OIF quote
	pub fn failure_handling(&self) -> Option<&crate::oif::common::FailureHandlingMode> {
		self.quote.failure_handling()
	}
}

impl TryFrom<(crate::oif::OifQuoteLatest, String)> for Quote {
	type Error = QuoteError;

	/// Convert from OIF Quote to domain Quote using composition
	///
	/// This conversion wraps the OIF quote with aggregator-specific metadata.
	/// The solver_id is required since OIF quotes don't contain solver identification.
	fn try_from(
		(oif_quote, solver_id): (crate::oif::OifQuoteLatest, String),
	) -> Result<Self, Self::Error> {
		// Generate a quote_id if the OIF quote doesn't have one
		let quote_id = oif_quote
			.quote_id
			.clone()
			.unwrap_or_else(|| format!("oif-quote-{}", uuid::Uuid::new_v4()));

		Ok(Quote {
			quote_id,
			solver_id,
			quote: OifQuote::new(oif_quote), // Wrap version-specific quote in version-agnostic wrapper
			integrity_checksum: String::new(), // Will be set by aggregator service
		})
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use super::*;

	fn create_test_quote() -> Quote {
		// Create a test OIF quote
		let oif_order = crate::oif::OifOrderLatest::OifEscrowV0 {
			payload: crate::oif::v0::OrderPayload {
				signature_type: crate::oif::common::SignatureType::Eip712,
				domain: serde_json::json!({
					"name": "TestDomain",
					"version": "1",
					"chainId": 1
				}),
				primary_type: "Order".to_string(),
				message: serde_json::json!({
					"user": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
					"amount": "1000000000000000000"
				}),
				types: HashMap::new(),
			},
		};

		let oif_quote = crate::oif::OifQuoteLatest {
			quote_id: Some("test-quote-id".to_string()),
			order: oif_order,
			valid_until: None,
			eta: None,
			provider: Some("test-provider".to_string()),
			failure_handling: None,
			partial_fill: false,
			metadata: None,
			preview: crate::oif::common::QuotePreview {
				inputs: vec![],
				outputs: vec![],
			},
		};

		Quote::new(
			"test-solver".to_string(),
			OifQuote::new(oif_quote), // Wrap version-specific quote in version-agnostic wrapper
			"test-checksum".to_string(),
		)
	}

	#[test]
	fn test_quote_creation() {
		let quote = create_test_quote();

		assert_eq!(quote.solver_id, "test-solver");
		assert_eq!(quote.provider(), Some("test-provider"));
		assert_eq!(quote.integrity_checksum, "test-checksum");
		assert_eq!(quote.version(), "v0");
		assert!(!quote.is_expired());
		// Test OIF spec structure
		assert!(matches!(
			quote.order(),
			crate::oif::OifOrderLatest::OifEscrowV0 { .. }
		));
		assert!(!quote.partial_fill());
		assert!(quote.failure_handling().is_none());
		// Quote ID is generated by the constructor, so just check it's not empty
		assert!(!quote.quote_id.is_empty());
		// Check the OIF quote_id is preserved in the composed structure
		assert_eq!(
			quote.quote.quote_id(),
			Some("test-quote-id".to_string()).as_ref()
		);
	}

	#[test]
	fn test_quote_expiration() {
		// Test no expiration when valid_until is None
		let quote_no_expiry = create_test_quote();
		assert!(!quote_no_expiry.is_expired());
	}

	#[test]
	fn test_quote_integrity_payload() {
		use crate::IntegrityPayload;

		let quote = create_test_quote();
		let payload = quote.to_integrity_payload();

		// Verify payload contains critical fields
		assert!(payload.contains(&format!("quote_id:{}", quote.quote_id)));
		assert!(payload.contains(&format!("solver_id:{}", quote.solver_id)));
		assert!(payload.contains(&format!(
			"provider:{}",
			quote.provider().unwrap_or("unknown")
		)));
		assert!(payload.contains("version:v0"));
		assert!(payload.contains("orders_count:1"));
		assert!(payload.contains("details:"));

		// Verify deterministic output
		let payload2 = quote.to_integrity_payload();
		assert_eq!(payload, payload2);
	}
}
