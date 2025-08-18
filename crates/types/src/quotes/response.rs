//! Quote response model for API layer

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::quotes::request::SolverSelection;
use crate::{QuoteDetails, QuoteOrder};

use super::{Quote, QuoteError, QuoteResult};

/// Metadata collected during quote aggregation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationMetadata {
	/// Total time spent on aggregation in milliseconds
	pub total_duration_ms: u64,
	/// Per-solver timeout used in milliseconds
	pub solver_timeout_ms: u64,
	/// Global timeout used in milliseconds
	pub global_timeout_ms: u64,
	/// Whether early termination occurred (min_quotes satisfied)
	pub early_termination: bool,
	/// Total solvers registered in system
	pub total_solvers_available: usize,
	/// Number of solvers actually queried
	pub solvers_queried: usize,
	/// Number of solvers that responded successfully
	pub solvers_responded_success: usize,
	/// Number of solvers that returned errors
	pub solvers_responded_error: usize,
	/// Number of solvers that timed out
	pub solvers_timed_out: usize,
	/// Minimum quotes required from solver options
	pub min_quotes_required: usize,
	/// Solver selection strategy used
	pub solver_selection_mode: SolverSelection,
}

/// Response format for individual quotes in the API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuoteResponse {
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

/// Collection of quotes response for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotesResponse {
	pub quotes: Vec<QuoteResponse>,
	pub total_quotes: usize,
	/// Optional metadata about the aggregation process
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<AggregationMetadata>,
}

impl QuoteResponse {
	/// Create a new quote response from a domain quote
	pub fn from_domain(quote: &Quote) -> QuoteResult<Self> {
		Self::try_from(quote.clone())
	}
}

impl QuotesResponse {
	/// Create a quotes response from domain quotes
	pub fn from_domain_quotes(quotes: Vec<Quote>) -> QuoteResult<Self> {
		let total_count = quotes.len();
		let quote_responses: Result<Vec<_>, _> =
			quotes.into_iter().map(QuoteResponse::try_from).collect();

		Ok(Self {
			quotes: quote_responses?,
			total_quotes: total_count,
			metadata: None,
		})
	}

	/// Create a quotes response from domain quotes with metadata
	pub fn from_domain_quotes_with_metadata(
		quotes: Vec<Quote>,
		metadata: AggregationMetadata,
	) -> QuoteResult<Self> {
		let total_count = quotes.len();
		let quote_responses: Result<Vec<_>, _> =
			quotes.into_iter().map(QuoteResponse::try_from).collect();

		Ok(Self {
			quotes: quote_responses?,
			total_quotes: total_count,
			metadata: Some(metadata),
		})
	}

	/// Create empty quotes response
	pub fn empty() -> Self {
		Self {
			quotes: Vec::new(),
			total_quotes: 0,
			metadata: None,
		}
	}
}

/// Convert from domain Quote to API QuoteResponse
impl TryFrom<Quote> for QuoteResponse {
	type Error = QuoteError;

	fn try_from(quote: Quote) -> Result<Self, Self::Error> {
		Ok(Self {
			quote_id: quote.quote_id,
			solver_id: quote.solver_id,
			orders: quote.orders,
			details: quote.details,
			valid_until: quote.valid_until,
			eta: quote.eta,
			provider: quote.provider,
			integrity_checksum: quote.integrity_checksum,
		})
	}
}

/// Convert from API QuoteResponse to domain Quote
impl TryFrom<QuoteResponse> for Quote {
	type Error = QuoteError;

	fn try_from(response: QuoteResponse) -> Result<Self, Self::Error> {
		Ok(Quote {
			quote_id: response.quote_id,
			solver_id: response.solver_id,
			orders: response.orders,
			details: response.details,
			valid_until: response.valid_until,
			eta: response.eta,
			provider: response.provider,
			integrity_checksum: response.integrity_checksum,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::quotes::Quote;
	use chrono::{Duration, Utc};

	fn create_test_quote() -> Quote {
		use crate::{
			AvailableInput, InteropAddress, QuoteDetails, QuoteOrder, RequestedOutput,
			SignatureType, U256,
		};

		let input = AvailableInput {
			asset: InteropAddress::from_hex("0x01C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap(),
			amount: U256::new("1000000000000000000".to_string()),
			user: InteropAddress::from_hex("0x01A0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0").unwrap(),
			lock: None,
		};

		let output = RequestedOutput {
			asset: InteropAddress::from_hex("0x01A0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap(),
			amount: U256::new("2500000000".to_string()),
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
		.with_valid_until(1234567890)
		.with_eta(300)
	}

	fn create_test_quote_response() -> QuoteResponse {
		use crate::{
			AvailableInput, InteropAddress, QuoteDetails, QuoteOrder, RequestedOutput,
			SignatureType, U256,
		};

		let input = AvailableInput {
			asset: InteropAddress::from_hex("0x01C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap(),
			amount: U256::new("1000000000000000000".to_string()),
			user: InteropAddress::from_hex("0x01A0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0").unwrap(),
			lock: None,
		};

		let output = RequestedOutput {
			asset: InteropAddress::from_hex("0x01A0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap(),
			amount: U256::new("2500000000".to_string()),
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

		QuoteResponse {
			quote_id: "test-quote-123".to_string(),
			solver_id: "test-solver".to_string(),
			orders: vec![order],
			details,
			valid_until: Some((Utc::now() + Duration::minutes(5)).timestamp() as u64),
			eta: Some(300),
			provider: "test-provider".to_string(),
			integrity_checksum: "test-checksum".to_string(),
		}
	}

	#[test]
	fn test_quote_response_from_domain() {
		let quote = create_test_quote();
		let response = QuoteResponse::from_domain(&quote).unwrap();

		assert_eq!(response.quote_id, quote.quote_id);
		assert_eq!(response.solver_id, quote.solver_id);
		assert_eq!(response.orders, quote.orders);
		assert_eq!(
			response.details.available_inputs,
			quote.details.available_inputs
		);
		assert_eq!(
			response.details.requested_outputs,
			quote.details.requested_outputs
		);
		assert_eq!(response.valid_until, quote.valid_until);
		assert_eq!(response.eta, quote.eta);
		assert_eq!(response.provider, quote.provider);
		assert_eq!(response.integrity_checksum, quote.integrity_checksum);
	}

	// #[test]
	// fn test_quote_response_integrity_payload() {
	// 	let response = create_test_quote_response();
	// 	let payload = response.to_integrity_payload();

	// 	assert!(payload.contains(&format!("quote_id:{}", response.quote_id)));
	// 	assert!(payload.contains(&format!("solver_id:{}", response.solver_id)));
	// 	assert!(payload.contains(&format!("provider:{}", response.provider)));
	// 	assert!(payload.contains("orders_count:1"));
	// 	assert!(payload.contains("details:"));
	// }

	#[test]
	fn test_quotes_response_creation() {
		let quote1 = create_test_quote();
		let mut quote2 = create_test_quote();
		// Better quote - update the first output amount
		quote2.details.requested_outputs[0].amount = crate::U256::new("3000000000".to_string());

		let quotes = vec![quote1, quote2];
		let response = QuotesResponse::from_domain_quotes(quotes).unwrap();

		assert_eq!(response.total_quotes, 2);
	}

	#[test]
	fn test_empty_response() {
		let response = QuotesResponse::empty();
		assert_eq!(response.total_quotes, 0);
		assert!(response.quotes.is_empty());
	}
}
