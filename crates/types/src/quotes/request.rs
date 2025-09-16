//! Quote request model and validation

use crate::constants::limits::{
	MAX_GLOBAL_TIMEOUT_MS, MAX_PRIORITY_THRESHOLD, MAX_SOLVER_TIMEOUT_MS, MIN_SOLVER_TIMEOUT_MS,
};
use crate::{models::InteropAddress, AvailableInput, QuotePreference, RequestedOutput};
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
#[allow(unused_imports)]
use serde_json::json;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use super::{QuoteValidationError, QuoteValidationResult};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum SolverSelection {
	#[default]
	All,
	Sampled,
	Priority,
}

/// API request body for /v1/quotes endpoint
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "includeSolvers": ["example-solver"],
    "timeout": 4000,
    "solverTimeout": 2000,
    "minQuotes": 1,
    "solverSelection": "all"
})))]
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

impl SolverOptions {
	/// Validate solver options to prevent abuse and logical errors
	///
	/// Applied validations:
	/// - **minQuotes**: Must be ≥ 1 (cannot aggregate 0 quotes)
	/// - **sampleSize**: Must be ≥ 1 when using sampled selection
	/// - **priorityThreshold**: Must be 0-100 (percentage range)
	/// - **timeout** (global): Must be within MIN_SOLVER_TIMEOUT_MS to MAX_GLOBAL_TIMEOUT_MS range
	/// - **solverTimeout** (per-solver): Must be within MIN_SOLVER_TIMEOUT_MS to MAX_SOLVER_TIMEOUT_MS range
	/// - **Logical consistency**: Global timeout must be ≥ per-solver timeout
	///
	/// These validations ensure reasonable resource usage and prevent configuration errors
	/// that could lead to poor user experience or system abuse.
	pub fn validate(&self) -> QuoteValidationResult<()> {
		// Validate minQuotes - must be positive
		if let Some(min_quotes) = self.min_quotes {
			if min_quotes == 0 {
				return Err(QuoteValidationError::InvalidSolverOptions {
					field: "minQuotes".to_string(),
					reason: "minQuotes must be at least 1 (cannot aggregate 0 quotes)".to_string(),
				});
			}
		}

		// Validate sampleSize - must be positive when specified
		if let Some(sample_size) = self.sample_size {
			if sample_size == 0 {
				return Err(QuoteValidationError::InvalidSolverOptions {
					field: "sampleSize".to_string(),
					reason: "sampleSize must be at least 1 when using sampled selection"
						.to_string(),
				});
			}
		}

		// Validate priorityThreshold - should be reasonable range (0-100)
		if let Some(threshold) = self.priority_threshold {
			if threshold > MAX_PRIORITY_THRESHOLD {
				return Err(QuoteValidationError::InvalidSolverOptions {
					field: "priorityThreshold".to_string(),
					reason: format!(
						"priorityThreshold must be between 0-{}",
						MAX_PRIORITY_THRESHOLD
					),
				});
			}
		}

		// Validate timeout values
		// Validate global timeout (allow more generous limits for aggregation)
		if let Some(timeout) = self.timeout {
			if timeout < MIN_SOLVER_TIMEOUT_MS {
				return Err(QuoteValidationError::InvalidSolverOptions {
					field: "timeout".to_string(),
					reason: format!(
						"Global timeout {}ms is too low (minimum: {}ms)",
						timeout, MIN_SOLVER_TIMEOUT_MS
					),
				});
			}

			// Allow up to 2 minutes for global timeout (longer than individual solver timeout)
			if timeout > MAX_GLOBAL_TIMEOUT_MS {
				return Err(QuoteValidationError::InvalidSolverOptions {
					field: "timeout".to_string(),
					reason: format!(
						"Global timeout {}ms is too high (maximum: {}ms)",
						timeout, MAX_GLOBAL_TIMEOUT_MS
					),
				});
			}
		}

		// Validate per-solver timeout
		if let Some(solver_timeout) = self.solver_timeout {
			if solver_timeout < MIN_SOLVER_TIMEOUT_MS {
				return Err(QuoteValidationError::InvalidSolverOptions {
					field: "solverTimeout".to_string(),
					reason: format!(
						"Per-solver timeout {}ms is too low (minimum: {}ms)",
						solver_timeout, MIN_SOLVER_TIMEOUT_MS
					),
				});
			}

			if solver_timeout > MAX_SOLVER_TIMEOUT_MS {
				return Err(QuoteValidationError::InvalidSolverOptions {
					field: "solverTimeout".to_string(),
					reason: format!(
						"Per-solver timeout {}ms is too high (maximum: {}ms)",
						solver_timeout, MAX_SOLVER_TIMEOUT_MS
					),
				});
			}
		}

		// Validate logical consistency: global timeout should be >= per-solver timeout
		if let (Some(global), Some(per_solver)) = (self.timeout, self.solver_timeout) {
			if global < per_solver {
				return Err(QuoteValidationError::InvalidSolverOptions {
					field: "timeout".to_string(),
					reason: format!(
						"Global timeout ({}ms) should not be less than per-solver timeout ({}ms)",
						global, per_solver
					),
				});
			}
		}

		Ok(())
	}
}

/// API request body for /v1/quotes endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "user": "0x01000002147a6970997970C51812dc3A010C7d01b50e0d17dc79C8",
    "availableInputs": [
        {
            "user": "0x01000002147a6970997970C51812dc3A010C7d01b50e0d17dc79C8",
            "asset": "0x01000002147a695FbDB2315678afecb367f032d93F642f64180aa3",
            "amount": "1000000000000000000"
        }
    ],
    "requestedOutputs": [
        {
            "receiver": "0x01000002147a6a3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
            "asset": "0x01000002147a6a5FbDB2315678afecb367f032d93F642f64180aa3",
            "amount": "1000000000000000000"
        }
    ],
    "preference": "speed",
    "minValidUntil": 600,
    "solverOptions": {
        "timeout": 4000,
        "solverTimeout": 2000
    }
})))]
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
	/// Optional metadata for custom adapter use
	/// This allows clients to include adapter-specific information in quote requests
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

impl QuoteRequest {
	/// Validate the ERC-7930 quotes request
	///
	/// Applied validations:
	/// - **Required fields**: Must have at least one available input and one requested output
	/// - **User address**: Must be a valid ERC-7930 InteropAddress format
	/// - **Available inputs**:
	///   - User address must be valid InteropAddress
	///   - Asset address must be valid InteropAddress  
	///   - Amount must be > 0
	/// - **Requested outputs**:
	///   - Receiver address must be valid InteropAddress
	///   - Asset address must be valid InteropAddress
	///   - Amount must be > 0
	/// - **Solver options**: Validates all solver option parameters (see SolverOptions::validate)
	///
	/// This ensures the request is well-formed and contains valid addresses, amounts, and configuration
	/// before processing, preventing invalid requests from reaching the solver network.
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

		// Validate solver options if provided
		if let Some(options) = &self.solver_options {
			options.validate()?;
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
			metadata: None,
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
