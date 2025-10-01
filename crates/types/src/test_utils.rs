//! Test utilities for creating common test objects
//!
//! This module provides builder patterns and utility functions for creating
//! test instances of OIF types, particularly QuoteRequest objects.

use crate::{
	oif::common::{
		FailureHandlingMode, Input, IntentType, OriginSubmission, Output,
		QuotePreference as OifQuotePreference, SwapType,
	},
	oif::v0::{GetQuoteRequest as OifGetQuoteRequest, IntentRequest},
	quotes::request::{QuoteRequest, SolverOptions},
	InteropAddress, U256,
};

/// Builder for creating test QuoteRequest objects with sensible defaults
#[derive(Debug, Clone)]
pub struct QuoteRequestBuilder {
	user: InteropAddress,
	intent_type: IntentType,
	inputs: Vec<Input>,
	outputs: Vec<Output>,
	swap_type: Option<SwapType>,
	min_valid_until: Option<u64>,
	preference: Option<OifQuotePreference>,
	origin_submission: Option<OriginSubmission>,
	failure_handling: Option<Vec<FailureHandlingMode>>,
	partial_fill: Option<bool>,
	supported_types: Vec<String>,
	solver_options: Option<SolverOptions>,
	metadata: Option<serde_json::Value>,
}

#[cfg(test)]
impl Default for QuoteRequestBuilder {
	fn default() -> Self {
		Self {
			user: InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
				.expect("Valid test address"),
			intent_type: IntentType::OifSwap,
			inputs: vec![],
			outputs: vec![],
			swap_type: Some(SwapType::ExactInput),
			min_valid_until: Some(300), // 5 minutes
			preference: Some(OifQuotePreference::Price),
			origin_submission: None,
			failure_handling: None,
			partial_fill: None,
			supported_types: vec!["oif-escrow-v0".to_string()],
			solver_options: None,
			metadata: None,
		}
	}
}

impl QuoteRequestBuilder {
	/// Create a new builder with default values
	pub fn new() -> Self {
		Self {
			user: InteropAddress::from_chain_and_address(
				1,
				"0x742d35Cc6634C0532925a3b8D38BA2297C33A9D7",
			)
			.unwrap(),
			intent_type: IntentType::OifSwap,
			inputs: Vec::new(),
			outputs: Vec::new(),
			swap_type: Some(SwapType::ExactInput),
			preference: None,
			min_valid_until: None,
			origin_submission: None,
			failure_handling: None,
			partial_fill: None,
			metadata: None,
			solver_options: None,
			supported_types: vec!["oif-escrow-v0".to_string()],
		}
	}

	/// Set the user address
	pub fn user(mut self, user: InteropAddress) -> Self {
		self.user = user;
		self
	}

	/// Set the user from a text address
	pub fn user_from_text(mut self, address: &str) -> Self {
		self.user = InteropAddress::from_text(address).expect("Valid address format");
		self
	}

	/// Add a single input
	pub fn add_input(mut self, input: Input) -> Self {
		self.inputs.push(input);
		self
	}

	/// Add multiple inputs
	pub fn inputs(mut self, inputs: Vec<Input>) -> Self {
		self.inputs = inputs;
		self
	}

	/// Add a simple input with default values
	pub fn add_simple_input(
		mut self,
		asset_chain_id: u64,
		asset_address: &str,
		amount: &str,
	) -> Self {
		let input = Input {
			user: self.user.clone(),
			asset: InteropAddress::from_text(&format!(
				"eip155:{}:{}",
				asset_chain_id, asset_address
			))
			.expect("Valid asset address"),
			amount: Some(U256::new(amount.to_string())),
			lock: None,
		};
		self.inputs.push(input);
		self
	}

	/// Add a single output
	pub fn add_output(mut self, output: Output) -> Self {
		self.outputs.push(output);
		self
	}

	/// Add multiple outputs
	pub fn outputs(mut self, outputs: Vec<Output>) -> Self {
		self.outputs = outputs;
		self
	}

	/// Add a simple output with default values
	pub fn add_simple_output(
		mut self,
		asset_chain_id: u64,
		asset_address: &str,
		amount: &str,
	) -> Self {
		let output = Output {
			receiver: self.user.clone(),
			asset: InteropAddress::from_text(&format!(
				"eip155:{}:{}",
				asset_chain_id, asset_address
			))
			.expect("Valid asset address"),
			amount: Some(U256::new(amount.to_string())),
			calldata: None,
		};
		self.outputs.push(output);
		self
	}

	/// Set swap type
	pub fn swap_type(mut self, swap_type: SwapType) -> Self {
		self.swap_type = Some(swap_type);
		self
	}

	/// Set preference
	pub fn preference(mut self, preference: OifQuotePreference) -> Self {
		self.preference = Some(preference);
		self
	}

	/// Set minimum valid until
	pub fn min_valid_until(mut self, seconds: u64) -> Self {
		self.min_valid_until = Some(seconds);
		self
	}

	/// Set supported types
	pub fn supported_types(mut self, types: Vec<String>) -> Self {
		self.supported_types = types;
		self
	}

	/// Set solver options
	pub fn solver_options(mut self, options: SolverOptions) -> Self {
		self.solver_options = Some(options);
		self
	}

	/// Build the QuoteRequest
	pub fn build(self) -> QuoteRequest {
		QuoteRequest {
			quote_request: OifGetQuoteRequest {
				user: self.user,
				intent: IntentRequest {
					intent_type: self.intent_type,
					inputs: self.inputs,
					outputs: self.outputs,
					swap_type: self.swap_type,
					min_valid_until: self.min_valid_until,
					preference: self.preference,
					origin_submission: self.origin_submission,
					failure_handling: self.failure_handling,
					partial_fill: self.partial_fill,
					metadata: self.metadata.clone(),
				},
				supported_types: self.supported_types,
			},
			solver_options: self.solver_options,
			metadata: self.metadata,
		}
	}
}

/// Common test QuoteRequest factory functions
pub struct TestQuoteRequests;

impl TestQuoteRequests {
	/// Create a valid ETH -> USDC swap request (exact input)
	pub fn eth_to_usdc_exact_input() -> QuoteRequest {
		QuoteRequestBuilder::new()
            .add_simple_input(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "1000000000000000000") // 1 ETH
            .add_simple_output(1, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "2500000000") // 2500 USDC
            .swap_type(SwapType::ExactInput)
            .build()
	}

	/// Create a valid USDC -> ETH swap request (exact output)
	pub fn usdc_to_eth_exact_output() -> QuoteRequest {
		QuoteRequestBuilder::new()
            .add_simple_input(1, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "4000000000") // 4000 USDC
            .add_simple_output(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "2000000000000000000") // 2 ETH
            .swap_type(SwapType::ExactOutput)
            .build()
	}

	/// Create a cross-chain request (Ethereum -> Polygon)
	pub fn cross_chain_eth_polygon() -> QuoteRequest {
		QuoteRequestBuilder::new()
            .add_simple_input(1, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "1000000000") // USDC on Ethereum
            .add_simple_output(137, "0x2791bca1f2de4661ed88a30c99a7a9449aa84174", "1000000000") // USDC on Polygon
            .build()
	}

	/// Create a minimal valid request (single input/output)
	pub fn minimal_valid() -> QuoteRequest {
		QuoteRequestBuilder::new()
			.add_simple_input(
				1,
				"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
				"1000000000000000000",
			)
			.add_simple_output(
				1,
				"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
				"2500000000",
			)
			.supported_types(vec!["oif-escrow-v0".to_string()])
			.build()
	}

	/// Create a request with empty inputs (for testing validation failures)
	pub fn empty_inputs() -> QuoteRequest {
		QuoteRequestBuilder::new()
            .inputs(vec![]) // Empty inputs
            .add_simple_output(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "1000000000")
            .build()
	}

	/// Create a request with empty outputs (for testing validation failures)
	pub fn empty_outputs() -> QuoteRequest {
		QuoteRequestBuilder::new()
            .add_simple_input(1, "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0", "1000000000")
            .outputs(vec![]) // Empty outputs
            .build()
	}

	/// Create a request with zero amount (for testing validation failures)
	pub fn zero_amount_input() -> QuoteRequest {
		QuoteRequestBuilder::new()
            .add_simple_input(1, "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0", "0") // Zero amount
            .add_simple_output(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "1000000000")
            .build()
	}

	/// Create a request with custom assets for compatibility testing
	pub fn with_assets(
		input_assets: Vec<(u64, &str)>,
		output_assets: Vec<(u64, &str)>,
	) -> QuoteRequest {
		let mut builder = QuoteRequestBuilder::new();

		for (chain_id, address) in input_assets {
			builder = builder.add_simple_input(chain_id, address, "1000000000000000000");
		}

		for (chain_id, address) in output_assets {
			builder = builder.add_simple_output(chain_id, address, "1000000000");
		}

		builder.build()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_quote_request_builder() {
		let request = QuoteRequestBuilder::new()
			.add_simple_input(
				1,
				"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
				"1000000000",
			)
			.add_simple_output(
				1,
				"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
				"2000000000",
			)
			.preference(OifQuotePreference::Speed)
			.build();

		assert_eq!(request.quote_request.intent.inputs.len(), 1);
		assert_eq!(request.quote_request.intent.outputs.len(), 1);
		assert_eq!(
			request.quote_request.intent.preference,
			Some(OifQuotePreference::Speed)
		);
		assert!(request.validate().is_ok());
	}

	#[test]
	fn test_factory_methods() {
		let eth_usdc = TestQuoteRequests::eth_to_usdc_exact_input();
		assert!(eth_usdc.validate().is_ok());
		assert_eq!(
			eth_usdc.quote_request.intent.swap_type,
			Some(SwapType::ExactInput)
		);

		let usdc_eth = TestQuoteRequests::usdc_to_eth_exact_output();
		assert!(usdc_eth.validate().is_ok());
		assert_eq!(
			usdc_eth.quote_request.intent.swap_type,
			Some(SwapType::ExactOutput)
		);

		let cross_chain = TestQuoteRequests::cross_chain_eth_polygon();
		assert!(cross_chain.validate().is_ok());
	}

	#[test]
	fn test_validation_failures() {
		let empty_inputs = TestQuoteRequests::empty_inputs();
		assert!(empty_inputs.validate().is_err());

		let empty_outputs = TestQuoteRequests::empty_outputs();
		assert!(empty_outputs.validate().is_err());

		let zero_amount = TestQuoteRequests::zero_amount_input();
		assert!(zero_amount.validate().is_err());
	}
}
