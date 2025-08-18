//! Quote request model and validation

use crate::{models::InteropAddress, AvailableInput, QuotePreference, RequestedOutput};
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use super::{QuoteValidationError, QuoteValidationResult};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum SolverSelection {
	All,
	Sampled,
	Priority,
}

/// API request body for /v1/quotes endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverOptions {
	/// Solver IDs to include (overrides circuit breaker Open state with warning)
	pub include_solvers: Option<Vec<String>>,
	/// Solver IDs to exclude from query
	pub exclude_solvers: Option<Vec<String>>,
	/// Overall request timeout (ms) for POST /quotes
	pub timeout: Option<u64>,
	/// Per-solver timeout (ms) for individual quote responses
	pub solver_timeout: Option<u64>,
	/// Minimum number of valid quotes required before responding
	pub min_quotes: Option<u64>,
	/// Solver selection strategy
	pub solver_selection: Option<SolverSelection>,
	/// Max solvers to query in 'sampled' mode
	pub sample_size: Option<u64>,
	/// Min solver confidence threshold
	pub priority_threshold: Option<u64>,
}

/// API request body for /v1/quotes endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuoteRequest {
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
	/// Solver options
	pub solver_options: Option<SolverOptions>,
}

impl QuoteRequest {
	/// Validate the ERC-7930 quotes request
	pub fn validate(&self) -> QuoteValidationResult<()> {
		// Validate we have at least one input and one output
		if self.available_inputs.is_empty() {
			return Err(QuoteValidationError::MissingRequiredField {
				field: "availableInputs".to_string(),
			});
		}

		if self.requested_outputs.is_empty() {
			return Err(QuoteValidationError::MissingRequiredField {
				field: "requestedOutputs".to_string(),
			});
		}

		// Validate user address
		self.user
			.validate()
			.map_err(|e| QuoteValidationError::InvalidTokenAddress {
				field: format!("user: {}", e),
			})?;

		// Validate all available inputs
		for (i, input) in self.available_inputs.iter().enumerate() {
			input
				.user
				.validate()
				.map_err(|e| QuoteValidationError::InvalidTokenAddress {
					field: format!("availableInputs[{}].user: {}", i, e),
				})?;
			input
				.asset
				.validate()
				.map_err(|e| QuoteValidationError::InvalidTokenAddress {
					field: format!("availableInputs[{}].asset: {}", i, e),
				})?;

			if input.amount.is_zero() {
				return Err(QuoteValidationError::InvalidAmount {
					field: format!("availableInputs[{}].amount", i),
					reason: "Amount must be greater than zero".to_string(),
				});
			}
		}

		// Validate all requested outputs
		for (i, output) in self.requested_outputs.iter().enumerate() {
			output
				.receiver
				.validate()
				.map_err(|e| QuoteValidationError::InvalidTokenAddress {
					field: format!("requestedOutputs[{}].receiver: {}", i, e),
				})?;
			output
				.asset
				.validate()
				.map_err(|e| QuoteValidationError::InvalidTokenAddress {
					field: format!("requestedOutputs[{}].asset: {}", i, e),
				})?;

			if output.amount.is_zero() {
				return Err(QuoteValidationError::InvalidAmount {
					field: format!("requestedOutputs[{}].amount", i),
					reason: "Amount must be greater than zero".to_string(),
				});
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::U256;

	use super::*;

	fn create_valid_erc7930_request() -> QuoteRequest {
		QuoteRequest {
			user: InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
				.unwrap(),
			available_inputs: vec![AvailableInput {
				user: InteropAddress::from_text(
					"eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
				)
				.unwrap(),
				asset: InteropAddress::from_text(
					"eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
				)
				.unwrap(),
				amount: U256::new("1000000000000000000".to_string()), // 1 ETH in wei
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				receiver: InteropAddress::from_text(
					"eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
				)
				.unwrap(),
				asset: InteropAddress::from_text(
					"eip155:1:0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
				)
				.unwrap(),
				amount: U256::new("2500000000".to_string()), // 2500 USDC
				calldata: None,
			}],
			min_valid_until: Some(300), // 5 minutes
			preference: Some(QuotePreference::Price),
			solver_options: None,
		}
	}

	#[test]
	fn test_erc7930_request_validation() {
		let request = create_valid_erc7930_request();
		assert!(request.validate().is_ok());
	}

	#[test]
	fn test_erc7930_empty_inputs() {
		let mut request = create_valid_erc7930_request();
		request.available_inputs.clear();
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_erc7930_empty_outputs() {
		let mut request = create_valid_erc7930_request();
		request.requested_outputs.clear();
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_erc7930_zero_amount() {
		let mut request = create_valid_erc7930_request();
		request.available_inputs[0].amount = U256::new("0".to_string());
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_erc7930_address_extraction() {
		let addr = InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
			.unwrap();

		let chain_id = addr.extract_chain_id().unwrap();
		assert_eq!(chain_id, 1);

		let address = addr.extract_address();
		assert_eq!(address, "0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0");
	}

	#[test]
	fn test_erc7930_invalid_address_format() {
		// Missing eip155 prefix
		assert!(InteropAddress::from_text("1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0").is_err());

		// Invalid format
		assert!(InteropAddress::from_text("invalid-address").is_err());
	}
}
