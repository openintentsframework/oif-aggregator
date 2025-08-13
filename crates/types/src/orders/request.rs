//! Order request models and validation

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::QuoteResponse;

/// API request body for submitting orders
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrderRequest {
	/// Quote data
	pub quote_response: QuoteResponse,

	/// User's wallet address
	pub user_address: String,

	/// User's signature for authorization
	pub signature: Option<String>,
}
