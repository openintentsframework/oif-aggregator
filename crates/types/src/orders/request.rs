//! Order request models and validation

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
#[allow(unused_imports)]
use serde_json::json;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::QuoteResponse;

/// API request body for submitting orders
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
    "sponsor": "0x70997970c51812dc3a010c7d01b50e0d17dc79c8",
    "signature": "0x1234567890abcdef...",
    "order": "0xfedcba0987654321..."
})))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrderRequest {
	/// Quote data
	pub quote_response: QuoteResponse,

	/// User's wallet address
	pub sponsor: String,

	/// User's signature for authorization
	pub signature: String,

	/// Order data
	pub order: String,
}
