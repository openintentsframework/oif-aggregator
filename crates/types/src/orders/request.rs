//! Order request models and validation

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
#[allow(unused_imports)]
use serde_json::json;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::QuoteResponse;

/// API request body for submitting orders - flexible design for multi-adapter support
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "quoteResponse": {
        "quoteId": "6a22e92f-3e5d-4f05-ab5f-007b01e58b21",
        "solverId": "example-solver",
        "orders": [
            {
                "signatureType": "eip712",
                "domain": "0x01000002147a69000000000022d473030f116ddee9f6b43ac78ba3",
                "primaryType": "PermitBatchWitnessTransferFrom",
                "message": {
                    "digest": "0xdfbfeb9aed6340d513ef52f716cef5b50b677118d364c8448bff1c9ea9fd0b14"
                }
            }
        ],
        "details": {
            "requestedOutputs": [
                {
                    "receiver": "0x01000002147a6a3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
                    "asset": "0x01000002147a6a5FbDB2315678afecb367f032d93F642f64180aa3",
                    "amount": "1000000000000000000"
                }
            ]
        },
        "provider": "Example Solver v1.0",
        "integrityChecksum": "hmac-sha256:a1b2c3d4e5f6..."
    },
    "signature": "0x1234567890abcdef...",
    "metadata": {
        "order": "0xfedcba0987654321...",
        "sponsor": "0x70997970c51812dc3a010c7d01b50e0d17dc79c8"
    }
})))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrderRequest {
	/// Quote data
	pub quote_response: QuoteResponse,

	/// User's signature for authorization
	pub signature: String,

	/// Adapter-specific metadata that can store order data, sponsor info, and other custom data
	/// This allows flexibility for different adapters to include the specific information they need
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

impl TryFrom<&OrderRequest> for crate::oif::OifPostOrderRequest {
	type Error = crate::orders::OrderValidationError;

	/// Convert from OrderRequest to OifPostOrderRequest using proper error handling
	///
	/// This conversion validates the order request and extracts the OIF-compliant request
	/// that adapters expect, providing better error handling than manual construction.
	fn try_from(request: &OrderRequest) -> Result<Self, Self::Error> {
		// Validate that signature is not empty
		if request.signature.is_empty() {
			return Err(crate::orders::OrderValidationError::InvalidSignature {
				reason: "Signature is required".to_string(),
			});
		}

		// Validate that quote_id is not empty
		if request.quote_response.quote_id.is_empty() {
			return Err(crate::orders::OrderValidationError::InvalidQuoteId {
				quote_id: "".to_string(),
			});
		}

		// Create the latest version PostOrderRequest
		let post_order_request = crate::oif::OifPostOrderRequestLatest {
			order: request.quote_response.order.clone(),
			signature: request.signature.clone(),
			quote_id: Some(request.quote_response.quote_id.clone()),
			origin_submission: None, // Not provided in OrderRequest
			metadata: request.metadata.clone(),
		};

		// Wrap in version-agnostic wrapper
		Ok(crate::oif::OifPostOrderRequest::new(post_order_request))
	}
}
