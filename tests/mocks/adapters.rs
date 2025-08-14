//! Mock adapters for examples and testing
//!
//! This module provides simple, working mock adapters that can be used
//! in examples and tests without complex dependencies.

use std::collections::HashMap;

use async_trait::async_trait;
use oif_types::chrono::Utc;

use oif_types::adapters::{
	models::{
		AdapterQuote, AssetAmount, AvailableInput, GetOrderResponse, GetQuoteRequest,
		GetQuoteResponse, OrderResponse, OrderStatus, QuoteDetails, QuoteOrder, RequestedOutput,
		Settlement, SettlementType, SignatureType, SubmitOrderRequest,
	},
	traits::SolverAdapter,
	AdapterResult, AdapterValidationError,
};
use oif_types::serde_json::json;
use oif_types::{models::Asset, models::Network, InteropAddress, SolverRuntimeConfig, U256};

/// Simple mock adapter for examples and testing
#[derive(Debug, Clone)]
pub struct MockDemoAdapter {
	pub id: String,
	pub name: String,
	pub adapter: oif_types::Adapter,
}

impl MockDemoAdapter {
	/// Create a new mock demo adapter
	pub fn new() -> Self {
		Self {
			id: "mock-demo-v1".to_string(),
			name: "Mock Demo Adapter".to_string(),
			adapter: oif_types::Adapter {
				adapter_id: "mock-demo-v1".to_string(),
				name: "Mock Demo Adapter".to_string(),
				description: Some("Mock Demo Adapter".to_string()),
				version: "1.0.0".to_string(),
				configuration: HashMap::new(),
			},
		}
	}

	/// Create a mock adapter with custom ID and name
	#[allow(dead_code)]
	pub fn with_config(id: String, name: String) -> Self {
		Self {
			id: id.clone(),
			name: name.clone(),
			adapter: oif_types::Adapter {
				adapter_id: id,
				name: name.clone(),
				description: Some(name.clone()),
				version: "1.0.0".to_string(),
				configuration: HashMap::new(),
			},
		}
	}
}

impl Default for MockDemoAdapter {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl SolverAdapter for MockDemoAdapter {
	fn adapter_id(&self) -> &str {
		&self.id
	}

	fn adapter_name(&self) -> &str {
		&self.name
	}

	fn adapter_info(&self) -> &oif_types::Adapter {
		&self.adapter
	}

	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetQuoteResponse> {
		// Create mock quote based on request
		let quote_id = format!("mock-quote-{}", Utc::now().timestamp());

		// Use the input from request or provide defaults
		let available_input = request
			.available_inputs
			.first()
			.cloned()
			.unwrap_or_else(|| AvailableInput {
				user: InteropAddress::from_hex("0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03")
					.unwrap(),
				asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
					.unwrap(),
				amount: U256::new("1000000000000000000".to_string()),
				lock: None,
			});

		// Use the output from request or provide defaults
		let requested_output = request
			.requested_outputs
			.first()
			.cloned()
			.unwrap_or_else(|| RequestedOutput {
				asset: InteropAddress::from_hex("0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0")
					.unwrap(),
				amount: U256::new("1000000".to_string()),
				receiver: InteropAddress::from_hex("0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03")
					.unwrap(),
				calldata: None,
			});

		let quote = AdapterQuote {
			quote_id: quote_id.clone(),
			orders: vec![QuoteOrder {
				signature_type: SignatureType::Eip712,
				domain: available_input.user.clone(),
				primary_type: "Order".to_string(),
				message: json!({
					"orderType": "swap",
					"inputAsset": available_input.asset.to_hex(),
					"outputAsset": requested_output.asset.to_hex(),
					"mockProvider": self.name
				}),
			}],
			details: QuoteDetails {
				available_inputs: vec![available_input],
				requested_outputs: vec![requested_output],
			},
			valid_until: Some(Utc::now().timestamp() as u64 + 300),
			eta: Some(30),
			provider: format!("{} Provider", self.name),
		};

		Ok(GetQuoteResponse {
			quotes: vec![quote],
		})
	}

	async fn submit_order(
		&self,
		_request: &SubmitOrderRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		let order_id = format!("mock-order-{}", Utc::now().timestamp());

		Ok(GetOrderResponse {
			order: OrderResponse {
				id: order_id.clone(),
				quote_id: Some("mock-quote-123".to_string()),
				status: OrderStatus::Created,
				created_at: Utc::now().timestamp() as u64,
				updated_at: Utc::now().timestamp() as u64,
				input_amount: AssetAmount {
					asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
						.unwrap(),
					amount: U256::new("1000000000000000000".to_string()),
				},
				output_amount: AssetAmount {
					asset: InteropAddress::from_hex("0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0")
						.unwrap(),
					amount: U256::new("1000000".to_string()),
				},
				settlement: Settlement {
					settlement_type: SettlementType::Escrow,
					data: json!({"escrow_address": "0x1234567890123456789012345678901234567890"}),
				},
				fill_transaction: None,
			},
		})
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		Ok(GetOrderResponse {
			order: OrderResponse {
				id: order_id.to_string(),
				quote_id: Some("mock-quote-123".to_string()),
				status: OrderStatus::Pending,
				created_at: Utc::now().timestamp() as u64 - 60,
				updated_at: Utc::now().timestamp() as u64,
				input_amount: AssetAmount {
					asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
						.unwrap(),
					amount: U256::new("1000000000000000000".to_string()),
				},
				output_amount: AssetAmount {
					asset: InteropAddress::from_hex("0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0")
						.unwrap(),
					amount: U256::new("1000000".to_string()),
				},
				settlement: Settlement {
					settlement_type: SettlementType::Escrow,
					data: json!({"escrow_address": "0x1234567890123456789012345678901234567890"}),
				},
				fill_transaction: None,
			},
		})
	}

	async fn get_supported_assets(&self, _network: &Network) -> AdapterResult<Vec<Asset>> {
		Ok(vec![
			Asset::new(
				"0x0000000000000000000000000000000000000000".to_string(),
				"ETH".to_string(),
				"Ethereum".to_string(),
				18,
				1,
			),
			Asset::new(
				"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0".to_string(),
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
				1,
			),
		])
	}

	async fn health_check(&self, _config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		Ok(true)
	}

	async fn get_supported_networks(&self) -> AdapterResult<Vec<Network>> {
		Ok(vec![
			Network::new(1, "Ethereum".to_string(), false),
			Network::new(137, "Polygon".to_string(), false),
		])
	}
}

/// Simple test adapter that can be configured to succeed or fail
#[derive(Debug, Clone)]
pub struct MockTestAdapter {
	pub id: String,
	pub name: String,
	pub should_fail: bool,
	pub adapter: oif_types::Adapter,
}

impl MockTestAdapter {
	pub fn new() -> Self {
		Self {
			id: "mock-test-v1".to_string(),
			name: "Mock Test Adapter".to_string(),
			should_fail: false,
			adapter: oif_types::Adapter {
				adapter_id: "mock-test-v1".to_string(),
				name: "Mock Test Adapter".to_string(),
				description: Some("Mock Test Adapter".to_string()),
				version: "1.0.0".to_string(),
				configuration: HashMap::new(),
			},
		}
	}

	#[allow(dead_code)]
	pub fn with_failure() -> Self {
		Self {
			id: "mock-test-v1".to_string(),
			name: "Mock Test Adapter".to_string(),
			should_fail: true,
			adapter: oif_types::Adapter {
				adapter_id: "mock-test-v1".to_string(),
				name: "Mock Test Adapter".to_string(),
				description: Some("Mock Test Adapter".to_string()),
				version: "1.0.0".to_string(),
				configuration: HashMap::new(),
			},
		}
	}
}

impl Default for MockTestAdapter {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl SolverAdapter for MockTestAdapter {
	fn adapter_id(&self) -> &str {
		&self.id
	}

	fn adapter_name(&self) -> &str {
		&self.name
	}

	fn adapter_info(&self) -> &oif_types::Adapter {
		&self.adapter
	}

	async fn get_quotes(
		&self,
		_request: &GetQuoteRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetQuoteResponse> {
		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: "Mock adapter configured to fail".to_string(),
				},
			));
		}

		Ok(GetQuoteResponse { quotes: vec![] })
	}

	async fn submit_order(
		&self,
		_request: &SubmitOrderRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: "Mock adapter configured to fail".to_string(),
				},
			));
		}

		let order_id = format!("test-order-{}", Utc::now().timestamp());
		Ok(GetOrderResponse {
			order: OrderResponse {
				id: order_id,
				quote_id: None,
				status: OrderStatus::Created,
				created_at: Utc::now().timestamp() as u64,
				updated_at: Utc::now().timestamp() as u64,
				input_amount: AssetAmount {
					asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
						.unwrap(),
					amount: U256::new("0".to_string()),
				},
				output_amount: AssetAmount {
					asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
						.unwrap(),
					amount: U256::new("0".to_string()),
				},
				settlement: Settlement {
					settlement_type: SettlementType::Escrow,
					data: json!({}),
				},
				fill_transaction: None,
			},
		})
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: "Mock adapter configured to fail".to_string(),
				},
			));
		}

		Ok(GetOrderResponse {
			order: OrderResponse {
				id: order_id.to_string(),
				quote_id: None,
				status: OrderStatus::Created,
				created_at: Utc::now().timestamp() as u64,
				updated_at: Utc::now().timestamp() as u64,
				input_amount: AssetAmount {
					asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
						.unwrap(),
					amount: U256::new("0".to_string()),
				},
				output_amount: AssetAmount {
					asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
						.unwrap(),
					amount: U256::new("0".to_string()),
				},
				settlement: Settlement {
					settlement_type: SettlementType::Escrow,
					data: json!({}),
				},
				fill_transaction: None,
			},
		})
	}

	async fn get_supported_assets(&self, _network: &Network) -> AdapterResult<Vec<Asset>> {
		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: "Mock adapter configured to fail".to_string(),
				},
			));
		}
		Ok(vec![])
	}

	async fn health_check(&self, _config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		Ok(!self.should_fail)
	}

	async fn get_supported_networks(&self) -> AdapterResult<Vec<Network>> {
		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: "Mock adapter configured to fail".to_string(),
				},
			));
		}
		Ok(vec![])
	}
}
