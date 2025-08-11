//! Order request models and validation

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// API request body for submitting orders
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrdersRequest {
	/// Associated quote ID (if using a specific quote)
	pub quote_id: Option<String>,

	/// Quote data (if not using quote_id)
	pub quote_response: Option<QuoteResponseRequest>,

	/// User's wallet address
	pub user_address: String,

	/// User's signature for authorization
	pub signature: Option<String>,
}

/// Quote data in API request format
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct QuoteResponseRequest {}

#[cfg(test)]
mod tests {
	use super::*;

	fn create_valid_request() -> OrdersRequest {
		OrdersRequest {
			quote_id: Some("quote-123".to_string()),
			quote_response: None,
			user_address: "0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e".to_string(),
			signature: Some("0x".to_string() + &"a".repeat(130)),
		}
	}
}
