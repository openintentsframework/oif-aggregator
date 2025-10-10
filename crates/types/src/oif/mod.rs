//! Version-agnostic OIF model wrappers
//!
//! This module provides version-agnostic wrappers around OIF specification models.
//! It allows the codebase to work with multiple OIF versions without hardcoding
//! version-specific imports throughout the application.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

pub mod common;
pub mod v0;

use crate::oif::common::{AssetAmount, OrderStatus, Settlement};

// ================================
// VERSION ALIASES - SINGLE SOURCE OF TRUTH
// ================================
//
// These type aliases define the "latest" supported OIF version.
// When a new OIF version is introduced, update these aliases to point to the new version.
// This ensures all code using these types automatically uses the latest version.
//
// EXAMPLE: When OIF v1 is introduced, simply change:
//   pub type OifOrderLatest = v0::Order;
// to:
//   pub type OifOrderLatest = v1::Order;
//
// This single change will automatically update ALL usage throughout the codebase!

/// Latest supported OIF Order type
/// Currently: v0::Order
/// When v1 is introduced, change to: v1::Order
pub type OifOrderLatest = v0::Order;

/// Latest supported OIF Quote type
/// Currently: v0::Quote
/// When v1 is introduced, change to: v1::Quote
pub type OifQuoteLatest = v0::Quote;

/// Latest supported OIF GetQuoteRequest type
/// Currently: v0::GetQuoteRequest
/// When v1 is introduced, change to: v1::GetQuoteRequest
pub type OifGetQuoteRequestLatest = v0::GetQuoteRequest;

/// Latest supported OIF PostOrderRequest type
/// Currently: v0::PostOrderRequest
/// When v1 is introduced, change to: v1::PostOrderRequest
pub type OifPostOrderRequestLatest = v0::PostOrderRequest;

/// Latest supported OIF PostOrderResponse type
/// Currently: v0::PostOrderResponse
/// When v1 is introduced, change to: v1::PostOrderResponse
pub type OifPostOrderResponseLatest = v0::PostOrderResponse;

/// Latest supported OIF GetOrderResponse type
/// Currently: v0::GetOrderResponse
/// When v1 is introduced, change to: v1::GetOrderResponse
pub type OifGetOrderResponseLatest = v0::GetOrderResponse;

/// Latest supported OIF IntentRequest type
/// Currently: v0::IntentRequest
/// When v1 is introduced, change to: v1::IntentRequest
pub type OifIntentRequestLatest = v0::IntentRequest;

/// Version-agnostic wrapper for OIF Quote models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "version")]
pub enum OifQuote {
	#[serde(rename = "v0")]
	V0(v0::Quote),
	// Future versions should be added here:
	// #[serde(rename = "v1")]
	// V1(v1::Quote),
}

impl OifQuote {
	/// Create a new quote wrapper using the latest supported OIF version
	///
	/// Uses the latest version defined by OifQuoteLatest type alias.
	/// When newer versions are added, update the type alias to automatically use the new version.
	pub fn new(quote: OifQuoteLatest) -> Self {
		Self::new_v0(quote)
	}

	/// Create a new V0 quote wrapper
	pub fn new_v0(quote: v0::Quote) -> Self {
		Self::V0(quote)
	}

	/// Get the quote ID (version-agnostic)
	pub fn quote_id(&self) -> Option<&String> {
		match self {
			Self::V0(quote) => quote.quote_id.as_ref(),
		}
	}

	/// Get the order for v0 quotes
	/// Returns None if this is not a v0 quote
	pub fn order_v0(&self) -> Option<&crate::oif::v0::Order> {
		match self {
			Self::V0(quote) => Some(&quote.order),
		}
	}

	/// Get the order (version-agnostic)
	/// Returns the order in the latest supported format
	/// Use order_v0() or order_v1() for version-specific access
	pub fn order(&self) -> &crate::oif::OifOrderLatest {
		match self {
			Self::V0(quote) => &quote.order,
		}
	}

	/// Check if this quote is v0
	pub fn is_v0(&self) -> bool {
		matches!(self, Self::V0(_))
	}

	/// Get the underlying OIF version
	pub fn version(&self) -> &'static str {
		match self {
			Self::V0(_) => "v0",
		}
	}

	/// Get validity timestamp (version-agnostic)
	pub fn valid_until(&self) -> Option<u64> {
		match self {
			Self::V0(quote) => quote.valid_until,
		}
	}

	/// Get estimated time to completion (version-agnostic)
	pub fn eta(&self) -> Option<u64> {
		match self {
			Self::V0(quote) => quote.eta,
		}
	}

	/// Get provider identifier (version-agnostic)
	pub fn provider(&self) -> Option<&String> {
		match self {
			Self::V0(quote) => quote.provider.as_ref(),
		}
	}

	/// Get failure handling policy (version-agnostic)
	pub fn failure_handling(&self) -> Option<&crate::oif::common::FailureHandlingMode> {
		match self {
			Self::V0(quote) => quote.failure_handling.as_ref(),
		}
	}

	/// Get partial fill support (version-agnostic)
	pub fn partial_fill(&self) -> bool {
		match self {
			Self::V0(quote) => quote.partial_fill,
		}
	}

	/// Get metadata (version-agnostic)
	pub fn metadata(&self) -> Option<&serde_json::Value> {
		match self {
			Self::V0(quote) => quote.metadata.as_ref(),
		}
	}

	/// Check if this quote supports a specific version
	pub fn supports_version(&self, version: &str) -> bool {
		self.version() == version
	}

	pub fn preview(&self) -> &crate::oif::common::QuotePreview {
		match self {
			Self::V0(quote) => &quote.preview,
		}
	}
}

/// Version-agnostic wrapper for OIF GetOrderResponse models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "version")]
pub enum OifGetOrderResponse {
	#[serde(rename = "v0")]
	V0(v0::GetOrderResponse),
	// Future versions will be added here:
	// #[serde(rename = "v1")]
	// V1(v1::GetOrderResponse),
}

impl OifGetOrderResponse {
	/// Create a new order response wrapper using the latest supported OIF version
	///
	/// Uses the latest version defined by OifGetOrderResponseLatest type alias.
	/// When newer versions are added, update the type alias to automatically use the new version.
	pub fn new(response: OifGetOrderResponseLatest) -> Self {
		Self::new_v0(response)
	}

	/// Create a new V0 order response wrapper
	pub fn new_v0(response: v0::GetOrderResponse) -> Self {
		Self::V0(response)
	}

	/// Get the order ID (version-agnostic)
	pub fn id(&self) -> &str {
		match self {
			Self::V0(order) => &order.id,
		}
	}

	/// Get the order status (version-agnostic)
	pub fn status(&self) -> &OrderStatus {
		match self {
			Self::V0(order) => &order.status,
		}
	}

	/// Get created timestamp as u64 (version-agnostic)
	pub fn created_at_timestamp(&self) -> u64 {
		match self {
			Self::V0(order) => order.created_at,
		}
	}

	/// Get updated timestamp as u64 (version-agnostic)
	pub fn updated_at_timestamp(&self) -> u64 {
		match self {
			Self::V0(order) => order.updated_at,
		}
	}

	/// Get created timestamp as DateTime<Utc> (convenience method)
	pub fn created_at(&self) -> DateTime<Utc> {
		let timestamp = self.created_at_timestamp();
		let safe_timestamp = i64::try_from(timestamp).unwrap_or(i64::MAX);
		chrono::Utc
			.timestamp_opt(safe_timestamp, 0)
			.single()
			.unwrap_or_default()
	}

	/// Get updated timestamp as DateTime<Utc> (convenience method)
	pub fn updated_at(&self) -> DateTime<Utc> {
		let timestamp = self.updated_at_timestamp();
		let safe_timestamp = i64::try_from(timestamp).unwrap_or(i64::MAX);
		chrono::Utc
			.timestamp_opt(safe_timestamp, 0)
			.single()
			.unwrap_or_default()
	}

	/// Get the quote ID (version-agnostic)
	pub fn quote_id(&self) -> Option<&String> {
		match self {
			Self::V0(order) => order.quote_id.as_ref(),
		}
	}

	/// Get input amounts (version-agnostic) - returns all input amounts
	pub fn input_amounts(&self) -> &Vec<AssetAmount> {
		match self {
			Self::V0(order) => &order.input_amounts,
		}
	}

	/// Get output amounts (version-agnostic) - returns all output amounts  
	pub fn output_amounts(&self) -> &Vec<AssetAmount> {
		match self {
			Self::V0(order) => &order.output_amounts,
		}
	}

	/// Get settlement information (version-agnostic)
	pub fn settlement(&self) -> &Settlement {
		match self {
			Self::V0(order) => &order.settlement,
		}
	}

	/// Get fill transaction (version-agnostic)
	pub fn fill_transaction(&self) -> Option<&serde_json::Value> {
		match self {
			Self::V0(order) => order.fill_transaction.as_ref(),
		}
	}

	/// Get the underlying OIF version
	pub fn version(&self) -> &'static str {
		match self {
			Self::V0(_) => "v0",
		}
	}

	/// Check if this is a v0 order response (for adapter compatibility checking)
	pub fn is_v0(&self) -> bool {
		matches!(self, Self::V0(_))
	}

	/// Check if this order response supports a specific version
	pub fn supports_version(&self, version: &str) -> bool {
		self.version() == version
	}

	/// Get the inner v0 response (for compatibility)
	/// Returns None if this is not a v0 response
	pub fn as_v0(&self) -> Option<&v0::GetOrderResponse> {
		match self {
			Self::V0(response) => Some(response),
		}
	}

	pub fn as_latest(&self) -> &crate::oif::OifGetOrderResponseLatest {
		match self {
			Self::V0(response) => response,
		}
	}
}

/// Version-agnostic wrapper for OIF PostOrderRequest models  
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "version")]
pub enum OifPostOrderRequest {
	#[serde(rename = "v0")]
	V0(v0::PostOrderRequest),
	// Future versions will be added here:
	// #[serde(rename = "v1")]
	// V1(v1::PostOrderRequest),
}

impl OifPostOrderRequest {
	/// Create a new order request wrapper using the latest supported OIF version
	///
	/// Uses the latest version defined by OifPostOrderRequestLatest type alias.
	/// When newer versions are added, update the type alias to automatically use the new version.
	pub fn new(request: OifPostOrderRequestLatest) -> Self {
		Self::new_v0(request)
	}

	/// Create a new V0 order request wrapper
	pub fn new_v0(request: v0::PostOrderRequest) -> Self {
		Self::V0(request)
	}

	/// Get the order (version-agnostic)
	/// Returns the order in the latest supported format
	/// Use order_v0() or order_v1() for version-specific access
	pub fn order(&self) -> &crate::oif::OifOrderLatest {
		match self {
			Self::V0(request) => &request.order,
		}
	}

	/// Get the signature (version-agnostic)
	pub fn signature(&self) -> &str {
		match self {
			Self::V0(request) => &request.signature,
		}
	}

	/// Get the quote ID (version-agnostic)
	pub fn quote_id(&self) -> Option<&String> {
		match self {
			Self::V0(request) => request.quote_id.as_ref(),
		}
	}

	/// Get metadata (version-agnostic)
	pub fn metadata(&self) -> Option<&serde_json::Value> {
		match self {
			Self::V0(request) => request.metadata.as_ref(),
		}
	}

	/// Get the underlying OIF version
	pub fn version(&self) -> &'static str {
		match self {
			Self::V0(_) => "v0",
		}
	}

	/// Check if this is a v0 order request (for adapter compatibility checking)
	pub fn is_v0(&self) -> bool {
		matches!(self, Self::V0(_))
	}

	/// Check if this order request supports a specific version
	pub fn supports_version(&self, version: &str) -> bool {
		self.version() == version
	}

	/// Get the inner v0 request (for adapter compatibility)
	/// Returns None if this is not a v0 request
	pub fn as_v0(&self) -> Option<&v0::PostOrderRequest> {
		match self {
			Self::V0(request) => Some(request),
		}
	}

	pub fn as_latest(&self) -> &crate::oif::OifPostOrderRequestLatest {
		match self {
			Self::V0(request) => request,
		}
	}
}

/// Version-agnostic wrapper for OIF GetQuoteRequest models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "version")]
pub enum OifGetQuoteRequest {
	#[serde(rename = "v0")]
	V0(v0::GetQuoteRequest),
	// Future versions will be added here:
	// #[serde(rename = "v1")]
	// V1(v1::GetQuoteRequest),
}

impl OifGetQuoteRequest {
	/// Create a new quote request wrapper using the latest supported OIF version
	///
	/// Uses the latest version defined by OifGetQuoteRequestLatest type alias.
	/// When newer versions are added, update the type alias to automatically use the new version.
	pub fn new(request: OifGetQuoteRequestLatest) -> Self {
		Self::new_v0(request)
	}

	/// Create a new V0 quote request wrapper
	pub fn new_v0(request: v0::GetQuoteRequest) -> Self {
		Self::V0(request)
	}

	/// Get the user (version-agnostic)
	pub fn user(&self) -> &crate::InteropAddress {
		match self {
			Self::V0(request) => &request.user,
		}
	}

	/// Get the intent (version-agnostic)
	pub fn intent(&self) -> &OifIntentRequestLatest {
		match self {
			Self::V0(request) => &request.intent,
		}
	}

	/// Get supported types (version-agnostic)
	pub fn supported_types(&self) -> &[String] {
		match self {
			Self::V0(request) => &request.supported_types,
		}
	}

	/// Get the underlying OIF version
	pub fn version(&self) -> &'static str {
		match self {
			Self::V0(_) => "v0",
		}
	}

	/// Check if this is a v0 quote request (for adapter compatibility checking)
	pub fn is_v0(&self) -> bool {
		matches!(self, Self::V0(_))
	}

	/// Check if this quote request supports a specific version
	pub fn supports_version(&self, version: &str) -> bool {
		self.version() == version
	}

	/// Get the inner v0 request
	/// Returns None if this is not a v0 request
	pub fn as_v0(&self) -> Option<&v0::GetQuoteRequest> {
		match self {
			Self::V0(request) => Some(request),
		}
	}

	pub fn as_latest(&self) -> &crate::oif::OifGetQuoteRequestLatest {
		match self {
			Self::V0(request) => request,
		}
	}

	/// Get inputs (version-agnostic convenience method)
	pub fn inputs(&self) -> &[crate::oif::common::Input] {
		match self {
			Self::V0(request) => request.inputs(),
		}
	}

	/// Get outputs (version-agnostic convenience method)
	pub fn outputs(&self) -> &[crate::oif::common::Output] {
		match self {
			Self::V0(request) => request.outputs(),
		}
	}
}

/// Version-agnostic wrapper for OIF GetQuoteResponse models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "version")]
pub enum OifGetQuoteResponse {
	#[serde(rename = "v0")]
	V0(v0::GetQuoteResponse),
	// Future versions will be added here:
	// #[serde(rename = "v1")]
	// V1(v1::GetQuoteResponse),
}

impl OifGetQuoteResponse {
	/// Create a new quote response wrapper using the latest supported OIF version
	pub fn new(response: v0::GetQuoteResponse) -> Self {
		Self::new_v0(response)
	}

	/// Create a new V0 quote response wrapper
	pub fn new_v0(response: v0::GetQuoteResponse) -> Self {
		Self::V0(response)
	}

	/// Get the quotes (version-agnostic)
	/// Returns quotes in the latest supported format
	pub fn quotes(&self) -> &[crate::oif::OifQuoteLatest] {
		match self {
			Self::V0(response) => &response.quotes,
		}
	}

	/// Get the underlying OIF version
	pub fn version(&self) -> &'static str {
		match self {
			Self::V0(_) => "v0",
		}
	}

	/// Check if this is a v0 quote response (for adapter compatibility checking)
	pub fn is_v0(&self) -> bool {
		matches!(self, Self::V0(_))
	}

	/// Check if this quote response supports a specific version
	pub fn supports_version(&self, version: &str) -> bool {
		self.version() == version
	}
}

/// Version-agnostic wrapper for OIF PostOrderResponse models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "version")]
pub enum OifPostOrderResponse {
	#[serde(rename = "v0")]
	V0(v0::PostOrderResponse),
	// Future versions will be added here:
	// #[serde(rename = "v1")]
	// V1(v1::PostOrderResponse),
}

impl OifPostOrderResponse {
	/// Create a new post order response wrapper using the latest supported OIF version
	///
	/// Uses the latest version defined by OifPostOrderResponseLatest type alias.
	/// When newer versions are added, update the type alias to automatically use the new version.
	pub fn new(response: OifPostOrderResponseLatest) -> Self {
		Self::new_v0(response)
	}

	/// Create a new V0 post order response wrapper
	pub fn new_v0(response: v0::PostOrderResponse) -> Self {
		Self::V0(response)
	}

	/// Get the order ID (version-agnostic)
	pub fn order_id(&self) -> Option<&String> {
		match self {
			Self::V0(response) => response.order_id.as_ref(),
		}
	}

	/// Get the status (version-agnostic)
	pub fn status(&self) -> &crate::oif::common::PostOrderResponseStatus {
		match self {
			Self::V0(response) => &response.status,
		}
	}

	/// Get the message (version-agnostic)
	pub fn message(&self) -> Option<&String> {
		match self {
			Self::V0(response) => response.message.as_ref(),
		}
	}

	/// Get the order (version-agnostic)
	/// Returns the order in the latest supported format
	/// Use order_v0() or order_v1() for version-specific access
	pub fn order(&self) -> Option<&serde_json::Value> {
		match self {
			Self::V0(response) => response.order.as_ref(),
		}
	}

	/// Get metadata (version-agnostic)
	pub fn metadata(&self) -> Option<&serde_json::Value> {
		match self {
			Self::V0(response) => response.metadata.as_ref(),
		}
	}

	/// Get the underlying OIF version
	pub fn version(&self) -> &'static str {
		match self {
			Self::V0(_) => "v0",
		}
	}

	/// Check if this is a v0 post order response
	pub fn is_v0(&self) -> bool {
		matches!(self, Self::V0(_))
	}

	/// Check if this post order response supports a specific version
	pub fn supports_version(&self, version: &str) -> bool {
		self.version() == version
	}

	/// Get the inner v0 response
	/// Returns None if this is not a v0 response
	pub fn as_v0(&self) -> Option<&v0::PostOrderResponse> {
		match self {
			Self::V0(response) => Some(response),
		}
	}

	/// Get the response in the latest supported format
	pub fn as_latest(&self) -> &crate::oif::OifPostOrderResponseLatest {
		match self {
			Self::V0(response) => response,
		}
	}
}
