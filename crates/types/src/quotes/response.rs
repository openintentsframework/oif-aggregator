//! Quote response model for API layer

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
#[allow(unused_imports)]
use serde_json::json;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::quotes::request::SolverSelection;

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

/// Response format for individual quotes in the API - closely matches OIF Quote response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "quoteId": "6a22e92f-3e5d-4f05-ab5f-007b01e58b21",
    "solverId": "example-solver",
    "order": {
        "type": "oif-escrow-v0",
        "payload": {
            "signatureType": "eip712",
            "domain": {
                "name": "TestDomain",
                "version": "1",
                "chainId": 1
            },
            "primaryType": "Order",
            "message": {
                "orderType": "swap",
                "inputAsset": "0x00010000027a69145FbDB2315678afecb367f032d93F642f64180aa3",
                "outputAsset": "0x00010000027a6a145FbDB2315678afecb367f032d93F642f64180aa3",
                "amount": "1000000000000000000"
            },
            "types": {}
        }
    },
    "validUntil": 1756457492,
    "eta": 30,
    "provider": "Example Solver v1.0",
    "failureHandling": "refund-automatic",
    "partialFill": false,
    "preview": {
        "inputs": [
            {
                "user": "0x00010000027a691470997970C51812dc3A010C7d01b50e0d17dc79C8",
                "asset": "0x00010000027a69145FbDB2315678afecb367f032d93F642f64180aa3",
                "amount": "1000000000000000000"
            }
        ],
        "outputs": [
            {
                "receiver": "0x00010000027a6a143C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
                "asset": "0x00010000027a6a145FbDB2315678afecb367f032d93F642f64180aa3",
                "amount": "1000000"
            }
        ]
    },
    "integrityChecksum": "hmac-sha256:a1b2c3d4e5f6...",
    "metadata": {"aggregator": "metadata"}
})))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuoteResponse {
	/// Unique identifier for the quote (always present in aggregator responses)
	pub quote_id: String,
	/// ID of the solver that provided this quote (aggregator-specific)
	pub solver_id: String,
	/// Order from OIF specification (uses latest version)
	pub order: crate::oif::OifOrderLatest,
	/// Quote validity timestamp (from OIF)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub valid_until: Option<u64>,
	/// Estimated time to completion in seconds (from OIF)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub eta: Option<u64>,
	/// Provider identifier (from OIF)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<String>,
	/// Failure handling policy for execution (from OIF)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failure_handling: Option<crate::oif::common::FailureHandlingMode>,
	/// Whether the quote supports partial fill (from OIF)
	pub partial_fill: bool,
	/// Quote preview (from OIF)
	pub preview: crate::oif::common::QuotePreview,
	/// HMAC-SHA256 integrity checksum for quote verification (aggregator-specific)
	pub integrity_checksum: String,
	/// Metadata from the OIF quote (provider-specific)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

/// Collection of quotes response for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "quotes": [
        {
            "quoteId": "6a22e92f-3e5d-4f05-ab5f-007b01e58b21",
            "solverId": "example-solver",
            "order": {
                "type": "oif-escrow-v0",
                "payload": {
                    "signatureType": "eip712",
                    "domain": {
                        "name": "TestDomain",
                        "version": "1",
                        "chainId": 1
                    },
                    "primaryType": "Order",
                    "message": {
                        "orderType": "swap",
                        "inputAsset": "0x00010000027a69145FbDB2315678afecb367f032d93F642f64180aa3",
                        "outputAsset": "0x00010000027a6a145FbDB2315678afecb367f032d93F642f64180aa3",
                        "amount": "1000000000000000000"
                    },
                    "types": {}
                }
            },
            "validUntil": 1756457492,
            "eta": 30,
            "provider": "Example Solver v1.0",
            "failureHandling": "refund-automatic",
            "partialFill": false,
            "preview": {
                "inputs": [
                    {
                        "user": "0x00010000027a691470997970C51812dc3A010C7d01b50e0d17dc79C8",
                        "asset": "0x00010000027a69145FbDB2315678afecb367f032d93F642f64180aa3",
                        "amount": "1000000000000000000"
                    }
                ],
                "outputs": [
                    {
                        "receiver": "0x00010000027a6a143C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
                        "asset": "0x00010000027a6a145FbDB2315678afecb367f032d93F642f64180aa3",
                        "amount": "1000000"
                    }
                ]
            },
            "integrityChecksum": "hmac-sha256:a1b2c3d4e5f6...",
            "metadata": {"aggregator": "metadata"}
        }
    ],
    "totalQuotes": 1,
    "metadata": {
        "totalDurationMs": 1500,
        "solverTimeoutMs": 5000,
        "globalTimeoutMs": 10000,
        "earlyTermination": false,
        "totalSolversAvailable": 2,
        "solversQueried": 2,
        "solversRespondedSuccess": 1,
        "solversRespondedError": 0,
        "solversTimedOut": 1,
        "minQuotesRequired": 30,
        "solverSelectionMode": "all"
    }
})))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotesResponse {
	/// List of quotes
	pub quotes: Vec<QuoteResponse>,
	/// Total number of quotes
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
			order: quote.quote.order().clone(),
			preview: quote.quote.preview().clone(),
			valid_until: quote.quote.valid_until(),
			eta: quote.quote.eta(),
			provider: quote.quote.provider().cloned(),
			failure_handling: quote.quote.failure_handling().cloned(),
			partial_fill: quote.quote.partial_fill(),
			integrity_checksum: quote.integrity_checksum,
			metadata: quote.quote.metadata().cloned(),
		})
	}
}

/// Convert from API QuoteResponse to domain Quote
impl TryFrom<QuoteResponse> for Quote {
	type Error = QuoteError;

	fn try_from(response: QuoteResponse) -> Result<Self, Self::Error> {
		// Reconstruct the OIF quote from the flattened fields
		let oif_quote = crate::oif::v0::Quote {
			quote_id: Some(response.quote_id.clone()),
			order: response.order,
			valid_until: response.valid_until,
			eta: response.eta,
			provider: response.provider,
			failure_handling: response.failure_handling,
			partial_fill: response.partial_fill,
			metadata: response.metadata,
			preview: response.preview,
		};

		Ok(Quote {
			quote_id: response.quote_id,
			solver_id: response.solver_id,
			quote: crate::oif::OifQuote::new(oif_quote),
			integrity_checksum: response.integrity_checksum,
		})
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use super::*;
	use crate::{quotes::Quote, OifQuote};

	fn create_test_quote() -> Quote {
		let order = crate::oif::v0::Order::OifEscrowV0 {
			payload: crate::oif::v0::OrderPayload {
				signature_type: crate::oif::common::SignatureType::Eip712,
				domain: serde_json::json!({
					"name": "TestDomain",
					"version": "1",
					"chainId": 1
				}),
				primary_type: "Order".to_string(),
				message: serde_json::json!({}),
				types: HashMap::new(),
			},
		};

		let oif_quote = crate::oif::v0::Quote {
			quote_id: Some("test-oif-quote".to_string()),
			order,
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
			OifQuote::new(oif_quote),
			"test-checksum".to_string(),
		)
	}

	#[test]
	fn test_quote_response_from_domain() {
		let quote = create_test_quote();
		let response = QuoteResponse::from_domain(&quote).unwrap();

		// Test flattened fields
		assert_eq!(response.quote_id, quote.quote_id);
		assert_eq!(response.solver_id, quote.solver_id);
		assert_eq!(response.order, quote.quote.order().clone());
		assert_eq!(response.valid_until, quote.valid_until());
		assert_eq!(response.eta, quote.eta());
		assert_eq!(response.provider.as_deref(), quote.provider());
		assert_eq!(response.integrity_checksum, quote.integrity_checksum);
	}

	#[test]
	fn test_quotes_response_creation() {
		let quote1 = create_test_quote();
		let quote2 = create_test_quote();

		let quotes = vec![quote1, quote2];
		let response = QuotesResponse::from_domain_quotes(quotes).unwrap();

		assert_eq!(response.total_quotes, 2);
		assert_eq!(response.quotes.len(), 2);
		assert!(response.metadata.is_none());
	}

	#[test]
	fn test_empty_response() {
		let response = QuotesResponse::empty();
		assert_eq!(response.total_quotes, 0);
		assert!(response.quotes.is_empty());
	}
}
