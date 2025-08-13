//! Shared models for adapter communication
//! Used by SolverAdapter trait implementations for request/response data

use crate::models::{InteropAddress, Lock, U256};
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

// ================================
// REQUEST MODELS
// ================================

/// Request to get quotes from a solver adapter
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetQuoteRequest {
	/// User making the request in ERC-7930 interoperable format
	pub user: InteropAddress,
	/// Available inputs (order significant if preference is 'input-priority')
	pub available_inputs: Vec<AvailableInput>,
	/// Requested outputs
	pub requested_outputs: Vec<RequestedOutput>,
	/// Minimum quote validity duration in seconds
	pub min_valid_until: Option<u64>,
	/// User preference for optimization
	pub preference: Option<QuotePreference>,
}

// ================================
// SHARED INPUT/OUTPUT MODELS
// ================================

/// Available input with lock information and user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AvailableInput {
	/// User address in ERC-7930 interoperable format
	pub user: InteropAddress,
	/// Asset address in ERC-7930 interoperable format
	pub asset: InteropAddress,
	/// Amount as a big integer
	pub amount: U256,
	/// Lock information if asset is already locked
	pub lock: Option<Lock>,
}

/// Requested output with receiver and optional calldata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RequestedOutput {
	/// Receiver address in ERC-7930 interoperable format
	pub receiver: InteropAddress,
	/// Asset address in ERC-7930 interoperable format
	pub asset: InteropAddress,
	/// Amount as a big integer
	pub amount: U256,
	/// Optional calldata for the output
	pub calldata: Option<String>,
}

/// Quote optimization preferences following UII standard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum QuotePreference {
	Price,
	Speed,
	InputPriority,
	TrustMinimization,
}

// ================================
// RESPONSE MODELS
// ================================

/// Response containing quote options following UII standard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetQuoteResponse {
	/// Available quotes
	pub quotes: Vec<AdapterQuote>,
}

/// A quote option following UII standard (adapter-specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterQuote {
	/// Array of EIP-712 compliant orders
	pub orders: Vec<QuoteOrder>,
	/// Quote details matching request structure
	pub details: QuoteDetails,
	/// Quote validity timestamp
	pub valid_until: Option<u64>,
	/// Estimated time to completion in seconds
	pub eta: Option<u64>,
	/// Unique quote identifier
	pub quote_id: String,

	pub provider: String,
}

/// Quote details matching the request structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuoteDetails {
	/// Requested outputs for this quote
	pub requested_outputs: Vec<RequestedOutput>,
	/// Available inputs for this quote
	pub available_inputs: Vec<AvailableInput>,
}

/// EIP-712 compliant order structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuoteOrder {
	/// Signature type (eip-712 or erc-3009)
	pub signature_type: SignatureType,
	/// ERC-7930 interoperable address of the domain
	pub domain: InteropAddress,
	/// Primary type for EIP-712 signing
	pub primary_type: String,
	/// Message object to be signed and submitted
	pub message: serde_json::Value,
}

/// Supported signature types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum SignatureType {
	Eip712,
	Erc3009,
}

/// Settlement mechanism types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum SettlementType {
	Escrow,
	ResourceLock,
}

/// Order
/// Settlement information for an order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settlement {
	/// Settlement mechanism type
	pub settlement_type: SettlementType,
	/// Settlement-specific data
	pub data: serde_json::Value,
}

/// Types of transactions in the solver system.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum TransactionType {
	/// Transaction that prepares an off-chain order on-chain (e.g., openFor).
	Prepare,
	/// Transaction that fills an order on the destination chain.
	Fill,
	/// Transaction that claims rewards on the origin chain.
	Claim,
}

/// Status of an order in the solver system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum OrderStatus {
	/// Order has been created but not yet prepared.
	Created,
	/// Order is pending execution.
	Pending,
	/// Order has been executed.
	Executed,
	/// Order has been settled and is ready to be claimed.
	Settled,
	/// Order is finalized and complete (after claim confirmation).
	Finalized,
	/// Order execution failed with specific transaction type.
	Failed(TransactionType),
}

/// Asset amount representation using ERC-7930 interoperable address format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetAmount {
	/// Asset address in ERC-7930 interoperable format
	pub asset: InteropAddress,
	/// Amount as a big integer
	pub amount: U256,
}

/// Order response for API endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrderResponse {
	/// Unique identifier for this order
	pub id: String,
	/// Current order status
	pub status: OrderStatus,
	/// Timestamp when this order was created
	pub created_at: u64,
	/// Timestamp when this order was last updated
	pub updated_at: u64,
	/// Associated quote ID if available
	pub quote_id: Option<String>,
	/// Input asset and amount
	pub input_amount: AssetAmount,
	/// Output asset and amount
	pub output_amount: AssetAmount,
	/// Settlement information
	pub settlement: Settlement,
	/// Transaction details if order has been executed
	pub fill_transaction: Option<serde_json::Value>,
}

/// Response containing order details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetOrderResponse {
	/// Order details
	pub order: OrderResponse,
}

// ================================
// IMPLEMENTATION METHODS
// ================================

impl GetQuoteRequest {
	/// Create a new GetQuoteRequest
	pub fn new(
		user: InteropAddress,
		available_inputs: Vec<AvailableInput>,
		requested_outputs: Vec<RequestedOutput>,
	) -> Self {
		Self {
			user,
			available_inputs,
			requested_outputs,
			min_valid_until: None,
			preference: None,
		}
	}

	/// Set minimum validity duration
	pub fn with_min_valid_until(mut self, min_valid_until: u64) -> Self {
		self.min_valid_until = Some(min_valid_until);
		self
	}

	/// Set user preference
	pub fn with_preference(mut self, preference: QuotePreference) -> Self {
		self.preference = Some(preference);
		self
	}

	/// Validate the request
	pub fn validate(&self) -> Result<(), String> {
		if self.available_inputs.is_empty() {
			return Err("At least one available input is required".to_string());
		}

		if self.requested_outputs.is_empty() {
			return Err("At least one requested output is required".to_string());
		}

		// Validate all inputs
		for input in &self.available_inputs {
			input.validate()?;
		}

		// Validate all outputs
		for output in &self.requested_outputs {
			output.validate()?;
		}

		Ok(())
	}
}

impl AvailableInput {
	/// Create a new AvailableInput
	pub fn new(user: InteropAddress, asset: InteropAddress, amount: U256) -> Self {
		Self {
			user,
			asset,
			amount,
			lock: None,
		}
	}

	/// Set lock information
	pub fn with_lock(mut self, lock: Lock) -> Self {
		self.lock = Some(lock);
		self
	}

	/// Validate the input
	pub fn validate(&self) -> Result<(), String> {
		self.user.validate().map_err(|e| e.to_string())?;
		self.asset.validate().map_err(|e| e.to_string())?;
		self.amount.validate()?;

		if self.amount.is_zero() {
			return Err("Amount cannot be zero".to_string());
		}

		Ok(())
	}
}

impl RequestedOutput {
	/// Create a new RequestedOutput
	pub fn new(receiver: InteropAddress, asset: InteropAddress, amount: U256) -> Self {
		Self {
			receiver,
			asset,
			amount,
			calldata: None,
		}
	}

	/// Set calldata
	pub fn with_calldata(mut self, calldata: String) -> Self {
		self.calldata = Some(calldata);
		self
	}

	/// Validate the output
	pub fn validate(&self) -> Result<(), String> {
		self.receiver.validate().map_err(|e| e.to_string())?;
		self.asset.validate().map_err(|e| e.to_string())?;
		self.amount.validate()?;

		if self.amount.is_zero() {
			return Err("Amount cannot be zero".to_string());
		}

		Ok(())
	}
}

impl GetQuoteResponse {
	/// Create a new empty response
	pub fn new() -> Self {
		Self { quotes: Vec::new() }
	}

	/// Create response with quotes
	pub fn with_quotes(quotes: Vec<AdapterQuote>) -> Self {
		Self { quotes }
	}

	/// Add a quote to the response
	pub fn add_quote(&mut self, quote: AdapterQuote) {
		self.quotes.push(quote);
	}

	/// Check if response has any quotes
	pub fn is_empty(&self) -> bool {
		self.quotes.is_empty()
	}

	/// Get number of quotes
	pub fn len(&self) -> usize {
		self.quotes.len()
	}
}

impl Default for GetQuoteResponse {
	fn default() -> Self {
		Self::new()
	}
}

// ================================
// CONVERSIONS
// ================================

impl TryFrom<crate::QuoteRequest> for GetQuoteRequest {
	type Error = String;

	fn try_from(quote_request: crate::QuoteRequest) -> Result<Self, Self::Error> {
		// Validate the source request first
		quote_request
			.validate()
			.map_err(|e| format!("Invalid QuoteRequest: {}", e))?;

		// Convert to GetQuoteRequest (solver_options are ignored for adapter communication)
		Ok(GetQuoteRequest {
			user: quote_request.user,
			available_inputs: quote_request.available_inputs,
			requested_outputs: quote_request.requested_outputs,
			min_valid_until: quote_request.min_valid_until,
			preference: quote_request.preference,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_quote_request_creation() {
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		let input = AvailableInput::new(user.clone(), asset.clone(), U256::from(1000u64));
		let output = RequestedOutput::new(user.clone(), asset, U256::from(500u64));

		let request = GetQuoteRequest::new(user, vec![input], vec![output])
			.with_min_valid_until(300)
			.with_preference(QuotePreference::Price);

		assert!(request.validate().is_ok());
		assert_eq!(request.min_valid_until, Some(300));
		assert!(matches!(request.preference, Some(QuotePreference::Price)));
	}

	#[test]
	fn test_get_quote_response_operations() {
		let response = GetQuoteResponse::new();
		assert!(response.is_empty());
		assert_eq!(response.len(), 0);

		// Note: Creating a full AdapterQuote would require extensive setup
		// This test focuses on the response container operations
	}

	#[test]
	fn test_available_input_validation() {
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		// Valid input
		let input = AvailableInput::new(user.clone(), asset.clone(), U256::from(1000u64));
		assert!(input.validate().is_ok());

		// Zero amount should fail
		let zero_input = AvailableInput::new(user, asset, U256::from(0u64));
		assert!(zero_input.validate().is_err());
	}

	#[test]
	fn test_requested_output_validation() {
		let receiver =
			InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
				.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		// Valid output
		let output = RequestedOutput::new(receiver.clone(), asset.clone(), U256::from(500u64));
		assert!(output.validate().is_ok());

		// Zero amount should fail
		let zero_output = RequestedOutput::new(receiver, asset, U256::from(0u64));
		assert!(zero_output.validate().is_err());
	}

	#[test]
	fn test_serde_round_trip() {
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		let input = AvailableInput::new(user.clone(), asset.clone(), U256::from(1000u64));
		let output = RequestedOutput::new(user.clone(), asset, U256::from(500u64));

		let request = GetQuoteRequest::new(user, vec![input], vec![output]);

		// Test serialization
		let json = serde_json::to_string(&request).unwrap();

		// Test deserialization
		let deserialized: GetQuoteRequest = serde_json::from_str(&json).unwrap();

		assert_eq!(
			request.available_inputs.len(),
			deserialized.available_inputs.len()
		);
		assert_eq!(
			request.requested_outputs.len(),
			deserialized.requested_outputs.len()
		);
	}

	#[test]
	fn test_try_from_quote_request() {
		// Create a QuoteRequest (API model)
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		let input = AvailableInput::new(user.clone(), asset.clone(), U256::from(1000u64));
		let output = RequestedOutput::new(user.clone(), asset, U256::from(500u64));

		let quote_request = crate::QuoteRequest {
			user: user.clone(),
			available_inputs: vec![input],
			requested_outputs: vec![output],
			min_valid_until: Some(300),
			preference: Some(QuotePreference::Price),
			solver_options: None, // This field is ignored in conversion
		};

		// Convert to GetQuoteRequest (adapter model)
		let get_quote_request: GetQuoteRequest = quote_request.try_into().unwrap();

		// Verify conversion worked correctly
		assert_eq!(get_quote_request.user, user);
		assert_eq!(get_quote_request.available_inputs.len(), 1);
		assert_eq!(get_quote_request.requested_outputs.len(), 1);
		assert_eq!(get_quote_request.min_valid_until, Some(300));
		assert!(matches!(
			get_quote_request.preference,
			Some(QuotePreference::Price)
		));

		// Test that validation works
		assert!(get_quote_request.validate().is_ok());
	}

	#[test]
	fn test_try_from_invalid_quote_request() {
		// Create an invalid QuoteRequest with empty inputs
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();

		let invalid_quote_request = crate::QuoteRequest {
			user,
			available_inputs: vec![], // Empty - should fail validation
			requested_outputs: vec![],
			min_valid_until: None,
			preference: None,
			solver_options: None,
		};

		// Conversion should fail due to validation
		let result: Result<GetQuoteRequest, String> = invalid_quote_request.try_into();
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("Invalid QuoteRequest"));
	}
}
