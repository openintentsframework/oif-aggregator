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
	/// Quote provider identifier
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
	#[serde(rename = "type")]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetAmount {
	/// Asset address in ERC-7930 interoperable format
	pub asset: InteropAddress,
	/// Amount as a big integer
	pub amount: U256,
}

impl Default for AssetAmount {
	fn default() -> Self {
		Self {
			asset: "".to_string(),
			amount: 0u64.into(),
		}
	}
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

/// Response containing order details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StandardOrder {
	/// Expiration timestamp as Unix timestamp (corresponds to uint32, but u64 for safety)
	pub expires: u64,
	/// Fill deadline timestamp as Unix timestamp (corresponds to uint32, but u64 for safety)
	pub fill_deadline: u64,
	/// Input oracle address in hex format (corresponds to address)
	pub input_oracle: String,
	/// Array of input pairs in hex format (corresponds to uint256[2][])
	pub inputs: Vec<Vec<String>>,
	/// Nonce in hex format (corresponds to uint256)
	pub nonce: String,
	/// Origin chain ID in hex format (corresponds to uint256)
	pub origin_chain_id: String,
	/// Output mandates
	pub outputs: Vec<SolMandateOutput>,
	/// User address in hex format (corresponds to address)
	pub user: String,
}

impl StandardOrder {
	/// Parse nonce from hex string to u128 (safe subset of uint256)
	pub fn parse_nonce(&self) -> Result<u128, String> {
		if let Some(hex) = self.nonce.strip_prefix("0x") {
			u128::from_str_radix(hex, 16).map_err(|e| format!("Invalid nonce hex: {}", e))
		} else {
			Err("Nonce must start with 0x".to_string())
		}
	}

	/// Parse origin_chain_id from hex string to u64
	pub fn parse_origin_chain_id(&self) -> Result<u64, String> {
		if let Some(hex) = self.origin_chain_id.strip_prefix("0x") {
			u64::from_str_radix(hex, 16).map_err(|e| format!("Invalid origin_chain_id hex: {}", e))
		} else {
			Err("Origin chain ID must start with 0x".to_string())
		}
	}

	/// Parse inputs from hex strings to numeric values
	pub fn parse_inputs(&self) -> Result<Vec<[u128; 2]>, String> {
		let mut parsed_inputs = Vec::new();
		for input_pair in &self.inputs {
			if input_pair.len() != 2 {
				return Err("Each input must have exactly 2 elements".to_string());
			}

			let mut parsed_pair = [0u128; 2];
			for (i, hex_str) in input_pair.iter().enumerate() {
				if let Some(hex) = hex_str.strip_prefix("0x") {
					parsed_pair[i] = u128::from_str_radix(hex, 16)
						.map_err(|e| format!("Invalid input[{}] hex: {}", i, e))?;
				} else {
					return Err(format!("Input[{}] must start with 0x", i));
				}
			}
			parsed_inputs.push(parsed_pair);
		}
		Ok(parsed_inputs)
	}

	/// Validate that addresses are proper 20-byte Ethereum addresses
	pub fn validate_addresses(&self) -> Result<(), String> {
		let addresses = [("user", &self.user), ("input_oracle", &self.input_oracle)];

		for (name, addr) in addresses {
			if let Some(hex) = addr.strip_prefix("0x") {
				if hex.len() != 40 {
					return Err(format!("{} must be 40 hex characters (20 bytes)", name));
				}
				if hex::decode(hex).is_err() {
					return Err(format!("{} contains invalid hex characters", name));
				}
			} else {
				return Err(format!("{} must start with 0x", name));
			}
		}
		Ok(())
	}

	/// Validate timestamp fields are within uint32 range
	pub fn validate_timestamps(&self) -> Result<(), String> {
		if self.expires > u32::MAX as u64 {
			return Err("expires exceeds uint32 range".to_string());
		}
		if self.fill_deadline > u32::MAX as u64 {
			return Err("fill_deadline exceeds uint32 range".to_string());
		}
		Ok(())
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolMandateOutput {
	/// Amount in hex format (corresponds to uint256)
	pub amount: String,
	/// Call data in hex format (corresponds to bytes)
	pub call: String,
	/// Chain ID in hex format (corresponds to uint256)
	pub chain_id: String,
	/// Context data in hex format (corresponds to bytes)
	pub context: String,
	/// Oracle address in hex format (corresponds to bytes32)
	pub oracle: String,
	/// Recipient address in hex format (corresponds to bytes32)
	pub recipient: String,
	/// Settler address in hex format (corresponds to bytes32)
	pub settler: String,
	/// Token address in hex format (corresponds to bytes32)
	pub token: String,
}

impl SolMandateOutput {
	/// Parse amount from hex string to u128 (safe subset of uint256)
	pub fn parse_amount(&self) -> Result<u128, String> {
		if let Some(hex) = self.amount.strip_prefix("0x") {
			u128::from_str_radix(hex, 16).map_err(|e| format!("Invalid amount hex: {}", e))
		} else {
			Err("Amount must start with 0x".to_string())
		}
	}

	/// Parse chain_id from hex string to u64
	pub fn parse_chain_id(&self) -> Result<u64, String> {
		if let Some(hex) = self.chain_id.strip_prefix("0x") {
			u64::from_str_radix(hex, 16).map_err(|e| format!("Invalid chain_id hex: {}", e))
		} else {
			Err("Chain ID must start with 0x".to_string())
		}
	}

	/// Parse call data from hex string to bytes
	pub fn parse_call_data(&self) -> Result<Vec<u8>, String> {
		if let Some(hex) = self.call.strip_prefix("0x") {
			if hex.is_empty() {
				return Ok(Vec::new());
			}
			hex::decode(hex).map_err(|e| format!("Invalid call data hex: {}", e))
		} else {
			Err("Call data must start with 0x".to_string())
		}
	}

	/// Parse context from hex string to bytes
	pub fn parse_context(&self) -> Result<Vec<u8>, String> {
		if let Some(hex) = self.context.strip_prefix("0x") {
			if hex.is_empty() {
				return Ok(Vec::new());
			}
			hex::decode(hex).map_err(|e| format!("Invalid context hex: {}", e))
		} else {
			Err("Context must start with 0x".to_string())
		}
	}

	/// Validate that addresses are proper 20-byte Ethereum addresses
	pub fn validate_addresses(&self) -> Result<(), String> {
		let addresses = [
			("oracle", &self.oracle),
			("recipient", &self.recipient),
			("settler", &self.settler),
			("token", &self.token),
		];

		for (name, addr) in addresses {
			if let Some(hex) = addr.strip_prefix("0x") {
				if hex.len() != 40 {
					return Err(format!("{} must be 40 hex characters (20 bytes)", name));
				}
				if hex::decode(hex).is_err() {
					return Err(format!("{} contains invalid hex characters", name));
				}
			} else {
				return Err(format!("{} must start with 0x", name));
			}
		}
		Ok(())
	}
}

/// Response containing order details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitOrderResponse {
	/// Status
	pub status: String,
	/// Order ID
	pub order_id: Option<String>,
	// /// Order details
	pub order: StandardOrder,
	/// Message
	pub message: Option<String>,
}

/// Response containing order details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitOrderRequest {
	pub order: String,

	/// User's wallet address
	pub sponsor: String,

	/// User's signature for authorization
	pub signature: String,
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

impl TryFrom<crate::orders::OrderRequest> for SubmitOrderRequest {
	type Error = String;

	fn try_from(req: crate::orders::OrderRequest) -> Result<Self, Self::Error> {
		Ok(Self {
			order: req.order,
			sponsor: req.sponsor,
			signature: req.signature,
		})
	}
}
