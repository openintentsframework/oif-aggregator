//! Order request models and validation

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::QuoteResponse;

/// API request body for submitting orders
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrdersRequest {
	/// Associated quote ID (if using a specific quote)
	pub quote_id: Option<String>,

	/// Quote data (if not using quote_id)
	pub quote_response: QuoteResponse,

	/// User's wallet address
	pub user_address: String,

	/// User's signature for authorization
	pub signature: Option<String>,
}
