//! OIF v0 specification models
//!
//! This module contains v0-specific models and re-exports common models.
//! See: https://github.com/openintentsframework/oif-specs/

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::{
	oif::common::{
		AssetAmount, EIP712Types, FailureHandlingMode, Input, IntentType, OrderStatus,
		OriginSubmission, Output, PostOrderResponseStatus, QuotePreference, QuotePreview,
		Settlement, SignatureType, SwapType,
	},
	InteropAddress,
};

/// Intent request structure - the nested intent object in GetQuoteRequest
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct IntentRequest {
	/// Type of intent - currently only "oif-swap" is supported
	pub intent_type: IntentType,
	/// Available inputs (order significant if preference is 'input-priority')
	pub inputs: Vec<Input>,
	/// Requested outputs
	pub outputs: Vec<Output>,
	/// Swap type for the quote - determines which amounts are fixed vs quoted
	#[serde(skip_serializing_if = "Option::is_none")]
	pub swap_type: Option<SwapType>,
	/// Minimum validity timestamp in seconds
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_valid_until: Option<u64>,
	/// Quote preference
	#[serde(skip_serializing_if = "Option::is_none")]
	pub preference: Option<QuotePreference>,
	/// Explicit preference for submission responsibility and acceptable auth schemes
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin_submission: Option<OriginSubmission>,
	/// Failure handling policies that the integrator supports (array)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failure_handling: Option<Vec<FailureHandlingMode>>,
	/// Whether the integrator supports partial fills
	#[serde(skip_serializing_if = "Option::is_none")]
	pub partial_fill: Option<bool>,
	/// Metadata for the order, never required, potentially contains provider specific data
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

/// Request for generating quotes
/// Request payload for obtaining swap quotes from providers. Contains all necessary
/// information for providers to calculate and return executable quotes.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GetQuoteRequest {
	/// User requesting the quote and recipient of refund inputs in case of failures
	pub user: InteropAddress,
	/// The intent object containing swap details
	pub intent: IntentRequest,
	/// Order types supported by the provider
	pub supported_types: Vec<String>,
}

impl GetQuoteRequest {
	/// Validate the OIF GetQuoteRequest
	///
	/// Applied validations:
	/// - **User address**: Valid InteropAddress format
	/// - **Inputs**: At least one input with valid addresses and amounts
	/// - **Outputs**: At least one output with valid addresses and amounts
	/// - **Intent type**: Currently only "oif-swap" is supported
	///
	/// This ensures the request is well-formed before processing.
	pub fn validate(&self) -> Result<(), String> {
		// Validate intent type
		if self.intent.intent_type != IntentType::OifSwap {
			return Err("intent type must be oif-swap".to_string());
		}

		// Validate swap type
		let swap_type = self
			.intent
			.swap_type
			.as_ref()
			.unwrap_or(&SwapType::ExactInput);

		// Validate we have at least one input and one output
		if swap_type == &SwapType::ExactInput && self.intent.inputs.is_empty() {
			return Err("inputs cannot be empty for exact-input".to_string());
		}

		if swap_type == &SwapType::ExactOutput && self.intent.outputs.is_empty() {
			return Err("outputs cannot be empty for exact-output".to_string());
		}

		// Validate user address
		self.user
			.validate()
			.map_err(|e| format!("user address invalid: {}", e))?;

		// Validate all inputs
		for (i, input) in self.intent.inputs.iter().enumerate() {
			input
				.user
				.validate()
				.map_err(|e| format!("inputs[{}].user invalid: {}", i, e))?;
			input
				.asset
				.validate()
				.map_err(|e| format!("inputs[{}].asset invalid: {}", i, e))?;

			if swap_type == &SwapType::ExactInput && input.amount.is_none() {
				return Err(format!(
					"inputs[{}].amount must be specified for exact-input",
					i
				));
			}

			// For exact-input, amount must be specified and > 0
			// For exact-output, amount is optional but if provided must be > 0
			if let Some(amount) = &input.amount {
				if amount.is_zero() {
					return Err(format!("inputs[{}].amount must be greater than zero", i));
				}
			}

			// Validate lock field requirement for resource-lock order types
			if self
				.supported_types
				.contains(&"oif-resource-lock-v0".to_string())
			{
				if input.lock.is_none() {
					return Err(format!(
						"inputs[{}].lock field is required when supportedTypes includes 'oif-resource-lock-v0'",
						i
					));
				}
			}
		}

		// Validate all outputs
		for (i, output) in self.intent.outputs.iter().enumerate() {
			output
				.receiver
				.validate()
				.map_err(|e| format!("outputs[{}].receiver invalid: {}", i, e))?;
			output
				.asset
				.validate()
				.map_err(|e| format!("outputs[{}].asset invalid: {}", i, e))?;

			if swap_type == &SwapType::ExactOutput && output.amount.is_none() {
				return Err(format!(
					"outputs[{}].amount must be specified for exact-output",
					i
				));
			}

			// For exact-output, amount must be specified and > 0
			// For exact-input, amount is optional but if provided must be > 0
			if let Some(amount) = &output.amount {
				if amount.is_zero() {
					return Err(format!("outputs[{}].amount must be greater than zero", i));
				}
			}
		}

		// Validate supported types is not empty
		if self.supported_types.is_empty() {
			return Err("supportedTypes cannot be empty".to_string());
		}

		Ok(())
	}

	/// Convenience method to get available inputs
	/// Adapters can use this to access inputs directly without nested access
	pub fn inputs(&self) -> &[Input] {
		&self.intent.inputs
	}

	/// Convenience method to get requested outputs  
	/// Adapters can use this to access outputs directly without nested access
	pub fn outputs(&self) -> &[Output] {
		&self.intent.outputs
	}

	/// Convenience method to get the preference
	pub fn preference(&self) -> Option<&QuotePreference> {
		self.intent.preference.as_ref()
	}

	/// Convenience method to get minimum valid until
	pub fn min_valid_until(&self) -> Option<u64> {
		self.intent.min_valid_until
	}
}

/// OIF Order union type with versioning support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum Order {
	#[serde(rename = "oif-escrow-v0")]
	OifEscrowV0 { payload: OrderPayload },
	#[serde(rename = "oif-resource-lock-v0")]
	OifResourceLockV0 { payload: OrderPayload },
	#[serde(rename = "oif-3009-v0")]
	Oif3009V0 {
		payload: OrderPayload,
		metadata: serde_json::Value,
	},
	#[serde(rename = "oif-generic-v0")]
	OifGenericV0 {
		payload: serde_json::Value, // More flexible for generic orders
	},
	/// TODO Across order type
	Across { payload: serde_json::Value },
}

impl Order {
	/// Returns the order type as a string
	pub fn order_type(&self) -> &'static str {
		match self {
			Order::OifEscrowV0 { .. } => "oif-escrow-v0",
			Order::OifResourceLockV0 { .. } => "oif-resource-lock-v0",
			Order::Oif3009V0 { .. } => "oif-3009-v0",
			Order::OifGenericV0 { .. } => "oif-generic-v0",
			Order::Across { .. } => "across",
		}
	}

	/// Returns the payload for orders that have a standard OrderPayload structure
	pub fn payload(&self) -> Option<&OrderPayload> {
		match self {
			Order::OifEscrowV0 { payload } => Some(payload),
			Order::OifResourceLockV0 { payload } => Some(payload),
			Order::Oif3009V0 { payload, .. } => Some(payload),
			_ => None,
		}
	}
}

/// Standard order payload structure for most order types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct OrderPayload {
	pub signature_type: SignatureType,
	pub domain: serde_json::Value,
	pub primary_type: String,
	pub message: serde_json::Value,
	pub types: EIP712Types,
}

/// Quote response model
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Quote {
	/// Unique identifier for the quote
	#[serde(skip_serializing_if = "Option::is_none")]
	pub quote_id: Option<String>,

	/// Order
	pub order: Order,

	/// Quote validity timestamp
	#[serde(skip_serializing_if = "Option::is_none")]
	pub valid_until: Option<u64>,

	/// Estimated time to completion in seconds
	#[serde(skip_serializing_if = "Option::is_none")]
	pub eta: Option<u64>,

	/// Provider identifier
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider: Option<String>,

	/// Failure handling policy for execution
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failure_handling: Option<FailureHandlingMode>,

	/// Whether the quote supports partial fill
	pub partial_fill: bool,

	/// Preview showing exact input/output amounts
	pub preview: QuotePreview,

	/// Metadata for the order, never required, potentially contains provider specific data
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GetQuoteResponse {
	pub quotes: Vec<Quote>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PostOrderRequest {
	pub order: Order,

	pub signature: String,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub quote_id: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin_submission: Option<OriginSubmission>,

	/// Adapter-specific metadata that can store order data, sponsor info, and other custom data
	/// This allows each adapter to include the specific information it needs for order execution
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PostOrderResponse {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub order_id: Option<String>,

	pub status: PostOrderResponseStatus,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub order: Option<serde_json::Value>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GetOrderRequest {
	pub id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GetOrderResponse {
	///  Unique identifier for this order
	pub id: String,

	/// Current order status
	pub status: OrderStatus,

	/// Timestamp when this order was created
	pub created_at: u64,

	/// Timestamp when this order was last updated
	pub updated_at: u64,

	/// Associated quote ID if available
	#[serde(skip_serializing_if = "Option::is_none")]
	pub quote_id: Option<String>,

	/// Input amount
	pub input_amounts: Vec<AssetAmount>,

	/// Output amount
	pub output_amounts: Vec<AssetAmount>,

	/// Settlement information
	pub settlement: Settlement,

	/// Transaction details if order has been executed
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fill_transaction: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::oif::common::{AssetLockReference, LockKind};

	#[test]
	fn test_validate_resource_lock_requires_lock_field() {
		// Create a request with resource-lock in supported types but no lock field in input
		let request = GetQuoteRequest {
			user: InteropAddress::from_text("eip155:1:0x742d35Cc6634C0532925a3b844D400c9569c9D3B")
				.unwrap(),
			intent: IntentRequest {
				intent_type: IntentType::OifSwap,
				inputs: vec![Input {
					user: InteropAddress::from_text(
						"eip155:1:0x742d35Cc6634C0532925a3b844D400c9569c9D3B",
					)
					.unwrap(),
					asset: InteropAddress::from_text(
						"eip155:1:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
					)
					.unwrap(),
					amount: Some(crate::U256::new("1000000".to_string())),
					lock: None, // Missing lock field!
				}],
				outputs: vec![Output {
					receiver: InteropAddress::from_text(
						"eip155:10:0x742d35Cc6634C0532925a3b844D400c9569c9D3B",
					)
					.unwrap(),
					asset: InteropAddress::from_text(
						"eip155:10:0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
					)
					.unwrap(),
					amount: Some(crate::U256::new("1000000".to_string())),
					calldata: None,
				}],
				swap_type: Some(SwapType::ExactInput),
				min_valid_until: None,
				preference: None,
				origin_submission: None,
				failure_handling: None,
				partial_fill: None,
				metadata: None,
			},
			supported_types: vec!["oif-resource-lock-v0".to_string()],
		};

		// Should fail validation
		let result = request.validate();
		assert!(result.is_err());
		assert!(result.unwrap_err().contains(
			"lock field is required when supportedTypes includes 'oif-resource-lock-v0'"
		));
	}

	#[test]
	fn test_validate_resource_lock_with_lock_field_succeeds() {
		// Create a request with resource-lock in supported types AND lock field in input
		let request = GetQuoteRequest {
			user: InteropAddress::from_text("eip155:1:0x742d35Cc6634C0532925a3b844D400c9569c9D3B")
				.unwrap(),
			intent: IntentRequest {
				intent_type: IntentType::OifSwap,
				inputs: vec![Input {
					user: InteropAddress::from_text(
						"eip155:1:0x742d35Cc6634C0532925a3b844D400c9569c9D3B",
					)
					.unwrap(),
					asset: InteropAddress::from_text(
						"eip155:1:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
					)
					.unwrap(),
					amount: Some(crate::U256::new("1000000".to_string())),
					lock: Some(AssetLockReference {
						kind: LockKind::TheCompact,
						params: None,
					}),
				}],
				outputs: vec![Output {
					receiver: InteropAddress::from_text(
						"eip155:10:0x742d35Cc6634C0532925a3b844D400c9569c9D3B",
					)
					.unwrap(),
					asset: InteropAddress::from_text(
						"eip155:10:0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
					)
					.unwrap(),
					amount: Some(crate::U256::new("1000000".to_string())),
					calldata: None,
				}],
				swap_type: Some(SwapType::ExactInput),
				min_valid_until: None,
				preference: None,
				origin_submission: None,
				failure_handling: None,
				partial_fill: None,
				metadata: None,
			},
			supported_types: vec!["oif-resource-lock-v0".to_string()],
		};

		// Should pass validation
		let result = request.validate();
		assert!(result.is_ok());
	}

	#[test]
	fn test_validate_without_resource_lock_no_lock_field_required() {
		// Create a request without resource-lock in supported types and no lock field
		let request = GetQuoteRequest {
			user: InteropAddress::from_text("eip155:1:0x742d35Cc6634C0532925a3b844D400c9569c9D3B")
				.unwrap(),
			intent: IntentRequest {
				intent_type: IntentType::OifSwap,
				inputs: vec![Input {
					user: InteropAddress::from_text(
						"eip155:1:0x742d35Cc6634C0532925a3b844D400c9569c9D3B",
					)
					.unwrap(),
					asset: InteropAddress::from_text(
						"eip155:1:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
					)
					.unwrap(),
					amount: Some(crate::U256::new("1000000".to_string())),
					lock: None, // No lock field, but that's OK since we're not using resource-lock
				}],
				outputs: vec![Output {
					receiver: InteropAddress::from_text(
						"eip155:10:0x742d35Cc6634C0532925a3b844D400c9569c9D3B",
					)
					.unwrap(),
					asset: InteropAddress::from_text(
						"eip155:10:0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
					)
					.unwrap(),
					amount: Some(crate::U256::new("1000000".to_string())),
					calldata: None,
				}],
				swap_type: Some(SwapType::ExactInput),
				min_valid_until: None,
				preference: None,
				origin_submission: None,
				failure_handling: None,
				partial_fill: None,
				metadata: None,
			},
			supported_types: vec!["oif-escrow-v0".to_string()],
		};

		// Should pass validation
		let result = request.validate();
		assert!(result.is_ok());
	}

	#[test]
	fn test_order_type_serialization() {
		// Test that Order serializes with correct "type" field
		let order = Order::OifResourceLockV0 {
			payload: OrderPayload {
				signature_type: SignatureType::Eip712,
				domain: serde_json::json!({
					"name": "TestDomain",
					"version": "1"
				}),
				primary_type: "TestType".to_string(),
				message: serde_json::json!({
					"test": "value"
				}),
				types: std::collections::HashMap::new(),
			},
		};

		// Serialize to JSON
		let json = serde_json::to_value(&order).unwrap();

		// Print the serialized JSON for verification
		println!("Serialized Order JSON:");
		println!("{}", serde_json::to_string_pretty(&json).unwrap());

		// Verify the "type" field is present and correct
		assert_eq!(json["type"], "oif-resource-lock-v0");
		assert!(json["payload"].is_object());

		// Verify order_type() method returns correct value
		assert_eq!(order.order_type(), "oif-resource-lock-v0");
	}

	#[test]
	fn test_order_type_deserialization() {
		// Test that Order deserializes correctly from JSON with "type" field
		let json = serde_json::json!({
			"type": "oif-resource-lock-v0",
			"payload": {
				"signatureType": "eip712",
				"domain": {
					"name": "TestDomain",
					"version": "1"
				},
				"primaryType": "TestType",
				"message": {
					"test": "value"
				},
				"types": {}
			}
		});

		let order: Order = serde_json::from_value(json).unwrap();

		// Verify it deserialized to the correct variant
		match order {
			Order::OifResourceLockV0 { .. } => {
				assert_eq!(order.order_type(), "oif-resource-lock-v0");
			},
			_ => panic!("Expected OifResourceLockV0 variant"),
		}
	}

	#[test]
	fn test_all_order_types() {
		// Test all order type strings
		let escrow = Order::OifEscrowV0 {
			payload: OrderPayload {
				signature_type: SignatureType::Eip712,
				domain: serde_json::json!({}),
				primary_type: "Test".to_string(),
				message: serde_json::json!({}),
				types: std::collections::HashMap::new(),
			},
		};
		assert_eq!(escrow.order_type(), "oif-escrow-v0");

		let resource_lock = Order::OifResourceLockV0 {
			payload: OrderPayload {
				signature_type: SignatureType::Eip712,
				domain: serde_json::json!({}),
				primary_type: "Test".to_string(),
				message: serde_json::json!({}),
				types: std::collections::HashMap::new(),
			},
		};
		assert_eq!(resource_lock.order_type(), "oif-resource-lock-v0");

		let oif_3009 = Order::Oif3009V0 {
			payload: OrderPayload {
				signature_type: SignatureType::Eip712,
				domain: serde_json::json!({}),
				primary_type: "Test".to_string(),
				message: serde_json::json!({}),
				types: std::collections::HashMap::new(),
			},
			metadata: serde_json::json!({}),
		};
		assert_eq!(oif_3009.order_type(), "oif-3009-v0");
	}

	#[test]
	fn test_post_order_request_serialization() {
		// Test complete PostOrderRequest matches expected format
		let request = PostOrderRequest {
			order: Order::OifResourceLockV0 {
				payload: OrderPayload {
					signature_type: SignatureType::Eip712,
					domain: serde_json::json!({
						"chainId": "11155420",
						"name": "TheCompact",
						"verifyingContract": "0x00000000000000171ede64904551eedf3c6c9788",
						"version": "1"
					}),
					primary_type: "BatchCompact".to_string(),
					message: serde_json::json!({
						"arbiter": "0x19593e48e7bff482244e31048cef9cd9547d6099",
						"sponsor": "0xde3e1d42253bc63be5085057132de89262b14237"
					}),
					types: std::collections::HashMap::new(),
				},
			},
			signature: "0x000000...".to_string(),
			quote_id: Some("c27076e9-389e-47e8-8270-148c6b4f99c0".to_string()),
			origin_submission: None,
			metadata: None,
		};

		// Serialize to JSON
		let json = serde_json::to_string_pretty(&request).unwrap();
		println!("PostOrderRequest JSON:");
		println!("{}", json);

		// Parse back and verify structure
		let value: serde_json::Value = serde_json::from_str(&json).unwrap();

		// Verify the order has the type field
		assert_eq!(value["order"]["type"], "oif-resource-lock-v0");
		assert!(value["order"]["payload"].is_object());
		assert_eq!(value["signature"], "0x000000...");
		assert_eq!(value["quoteId"], "c27076e9-389e-47e8-8270-148c6b4f99c0");

		println!("\n✓ PostOrderRequest correctly includes order.type field!");
	}
}
