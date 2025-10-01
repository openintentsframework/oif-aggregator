//! Common OIF models shared across all versions
//!
//! This module contains foundational models that are stable across OIF specification
//! versions. These models represent core concepts that are unlikely to change between
//! v0, v1, v2, etc.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::{InteropAddress, U256};

/// OIF specification version for future compatibility
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum OifVersion {
	V0,
}

/// Asset amount representation using ERC-7930 interoperable address format.
///
/// This is a fundamental model used across all OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetAmount {
	/// Asset address in ERC-7930 interoperable format
	pub asset: InteropAddress,
	/// Amount as a big integer
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amount: Option<U256>,
}

/// Settlement mechanism types
///
/// Core settlement mechanisms that are stable across OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum SettlementType {
	Escrow,
	ResourceLock,
}

/// Settlement information for an order
///
/// Settlement structure that remains consistent across OIF versions.
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

/// Status of an order in the solver system.
///
/// Order lifecycle status that is fundamental across OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
	/// Order has been created but not yet prepared.
	/// Next: Prepare transaction will be sent, moving to Pending.
	Created,
	/// Order has a pending prepare transaction.
	/// Next: When prepare confirms, moves to Executing.
	Pending,
	/// Order is currently executing - prepare confirmed, fill transaction in progress.
	/// Next: When fill confirms, moves to Executed.
	Executing,
	/// Order has been executed - fill transaction confirmed.
	/// Next: Either PostFilled (if post-fill tx needed) or Settled (if no post-fill).
	Executed,
	/// Order has been settled and is ready to be claimed.
	/// Next: Either PreClaimed (if pre-claim tx needed) or Finalized (after claim).
	Settled,
	/// Order is settling and is ready to be claimed
	Settling,
	/// Order is finalized and complete - claim transaction confirmed.
	/// Terminal state: No further transitions.
	Finalized,
	/// Order execution failed with specific transaction type.
	/// Terminal state: No further transitions.
	Failed,
	/// Order has been refunded.
	Refunded,
}

impl fmt::Display for OrderStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			OrderStatus::Created => write!(f, "Created"),
			OrderStatus::Pending => write!(f, "Pending"),
			OrderStatus::Executing => write!(f, "Executing"),
			OrderStatus::Executed => write!(f, "Executed"),
			OrderStatus::Settled => write!(f, "Settled"),
			OrderStatus::Finalized => write!(f, "Finalized"),
			OrderStatus::Failed => write!(f, "Failed"),
			OrderStatus::Refunded => write!(f, "Refunded"),
			OrderStatus::Settling => write!(f, "Settling"),
		}
	}
}

/// Supported signature types
///
/// Core signature mechanisms that remain stable across OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SignatureType {
	Eip712,
	Eip3009,
}

/// Reference to a lock in a locking system
/// Corresponds to AssetLockReference in the OIF spec
///
/// Core locking mechanism that is stable across versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AssetLockReference {
	/// Lock type identifier
	pub kind: LockKind,
	/// Lock-specific parameters
	pub params: Option<serde_json::Value>,
}

/// Supported lock mechanisms
///
/// Core lock types that remain stable across OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum LockKind {
	#[serde(rename = "the-compact")]
	TheCompact,
	#[serde(rename = "rhinestone")]
	Rhinestone,
}

/// Alias for backward compatibility
pub type Lock = AssetLockReference;

/// Intent type identifier
///
/// Core intent types that are stable across OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum IntentType {
	#[serde(rename = "oif-swap")]
	OifSwap,
}

/// Quote optimization preferences following UII standard
///
/// Core preferences that remain stable across OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum QuotePreference {
	Price,
	Speed,
	InputPriority,
	TrustMinimization,
}

/// Order interpretation for quote requests
///
/// Core swap types that remain stable across OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum SwapType {
	ExactInput,
	ExactOutput,
}

/// Origin submission preference
///
/// Core origin submission mechanism that is stable across versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OriginSubmission {
	pub mode: OriginMode,
	pub schemes: Option<Vec<AuthScheme>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum OriginMode {
	User,
	Protocol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum AuthScheme {
	Erc4337,
	Permit2,
	Erc20Permit,
	Eip3009,
}

/// Failure handling policy
/// Defines how to handle transaction failures or partial fills
///
/// Core failure handling that remains stable across OIF versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum FailureHandlingMode {
	#[serde(rename = "refund-automatic")]
	RefundAutomatic,
	#[serde(rename = "refund-claim")]
	RefundClaim,
	#[serde(rename = "needs-new-signature")]
	NeedsNewSignature,
}

/// Available input from a user
/// Specifies assets that a user is willing to provide as input for a swap or transfer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Input {
	/// User address providing the input assets
	pub user: InteropAddress,
	/// Asset address being provided as input
	pub asset: InteropAddress,
	/// Amount available. For quote requests: exact-input (exact amount) or exact-output (minimum amount/optional)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amount: Option<U256>,
	/// Optional lock reference for securing the input assets
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lock: Option<AssetLockReference>,
}

/// Requested output for a receiver
/// Specifies the desired assets and destination for a swap or transfer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Output {
	/// Receiver address that will receive the output assets
	pub receiver: InteropAddress,
	/// Asset address to be received as output
	pub asset: InteropAddress,
	/// Amount requested. For quote requests: exact-input (minimum/optional) or exact-output (exact amount)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amount: Option<U256>,
	/// Optional calldata describing how the receiver will consume the output
	#[serde(skip_serializing_if = "Option::is_none")]
	pub calldata: Option<String>,
}

/// Quote preview for a swap or transfer
/// Specifies the desired assets and destination for a swap or transfer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct QuotePreview {
	/// Inputs for the preview
	pub inputs: Vec<Input>,
	/// Outputs for the preview
	pub outputs: Vec<Output>,
}

/// EIP-712 type property
/// Single field definition used inside the EIP-712 `types` map
/// Example: { name: "amount", type: "uint256" }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EIP712TypeProperty {
	/// Field name
	pub name: String,
	/// Solidity/EVM type
	#[serde(rename = "type")]
	pub type_name: String,
}

/// EIP-712 types mapping
/// Map from type name to its field definitions, per EIP-712
/// Example: {
///   EIP712Domain: [
///     { name: "name", type: "string" },
///     { name: "version", type: "string" },
///     { name: "chainId", type: "uint256" },
///     { name: "verifyingContract", type: "address" }
///   ],
///   BatchCompact: [
///     { name: "arbiter", type: "address" },
///     { name: "sponsor", type: "address" },
///     { name: "nonce", type: "uint256" },
///     { name: "expires", type: "uint256" },
///     { name: "commitments", type: "Lock[]" },
///     { name: "mandate", type: "Mandate" }
///   ]
/// }
pub type EIP712Types = HashMap<String, Vec<EIP712TypeProperty>>;

/// Status enum for order submission responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PostOrderResponseStatus {
	/// Order received and passed basic validation, queued for full validation
	Received,
	/// Order rejected due to validation failure
	Rejected,
	/// Order processing encountered an error
	Error,
}

impl fmt::Display for PostOrderResponseStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			PostOrderResponseStatus::Received => write!(f, "Received"),
			PostOrderResponseStatus::Rejected => write!(f, "Rejected"),
			PostOrderResponseStatus::Error => write!(f, "Error"),
		}
	}
}
