//! Mock adapters for examples and testing
//!
//! This module provides simple, working mock adapters that can be used
//! in examples and tests without complex dependencies.

use std::collections::HashMap;

use async_trait::async_trait;
use oif_types::chrono::Utc;

use oif_types::oif::common::PostOrderResponseStatus;
use oif_types::serde_json::{json, Value};
use oif_types::{
	adapters::{
		traits::SolverAdapter, AdapterError, AdapterResult, AdapterValidationError,
		SupportedAssetsData,
	},
	oif::{
		common::{AssetAmount, OrderStatus, Settlement, SettlementType},
		v0::{GetOrderResponse, PostOrderResponse as SubmitOrderResponse},
	},
};
use oif_types::{Asset, InteropAddress, SolverRuntimeConfig, U256};
use oif_types::{
	OifGetOrderResponse, OifGetQuoteRequest, OifGetQuoteResponse, OifPostOrderRequest,
	OifPostOrderResponse, Solver,
};

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
		let id = "mock-demo-v1".to_string();
		let name = "Mock Demo Adapter".to_string();
		Self {
			adapter: oif_types::Adapter {
				adapter_id: id.clone(),
				name: name.clone(),
				description: Some("Mock Demo Adapter".to_string()),
				version: "1.0.0".to_string(),
				configuration: HashMap::new(),
			},
			id,
			name,
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
	fn adapter_info(&self) -> &oif_types::Adapter {
		&self.adapter
	}

	async fn get_quotes(
		&self,
		request: &OifGetQuoteRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<OifGetQuoteResponse> {
		// Create mock quote based on request
		let quote_id = format!("mock-quote-{}", Utc::now().timestamp());

		// Use the input from request or provide defaults
		let available_input =
			request
				.inputs()
				.first()
				.cloned()
				.unwrap_or_else(|| oif_types::Input {
					user: InteropAddress::from_chain_and_address(
						1,
						"0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03",
					)
					.unwrap(),
					asset: InteropAddress::from_chain_and_address(
						1,
						"0x0000000000000000000000000000000000000000",
					)
					.unwrap(),
					amount: Some(U256::new("1000000000000000000".to_string())),
					lock: None,
				});

		// Use the output from request or provide defaults
		let requested_output =
			request
				.outputs()
				.first()
				.cloned()
				.unwrap_or_else(|| oif_types::Output {
					asset: InteropAddress::from_chain_and_address(
						1,
						"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0",
					)
					.unwrap(),
					amount: Some(U256::new("1000000".to_string())),
					receiver: InteropAddress::from_chain_and_address(
						1,
						"0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03",
					)
					.unwrap(),
					calldata: None,
				});

		let quote = oif_types::oif::v0::Quote {
			preview: oif_types::oif::common::QuotePreview {
				inputs: vec![],
				outputs: vec![],
			},
			quote_id: Some(quote_id.clone()),
			order: oif_types::oif::v0::Order::OifEscrowV0 {
				payload: oif_types::oif::v0::OrderPayload {
					signature_type: oif_types::SignatureType::Eip712,
					domain: json!({
						"name": "MockAdapter",
						"version": "1",
						"chainId": 1
					}),
					primary_type: "MockOrder".to_string(),
					message: json!({
						"orderType": "swap",
						"inputAsset": available_input.asset.to_hex(),
						"outputAsset": requested_output.asset.to_hex(),
						"mockProvider": self.name
					}),
					types: std::collections::HashMap::new(),
				},
			},
			valid_until: Some(Utc::now().timestamp() as u64 + 300),
			eta: Some(30),
			provider: Some(format!("{} Provider", self.name)),
			failure_handling: None,
			partial_fill: false,
			metadata: Some(json!({
				"inputs": vec![available_input],
				"outputs": vec![requested_output],
			})),
		};

		let response = oif_types::GetQuoteResponse {
			quotes: vec![quote],
		};
		Ok(OifGetQuoteResponse::new(response))
	}

	async fn submit_order(
		&self,
		_request: &OifPostOrderRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<OifPostOrderResponse> {
		let order_id = format!("mock-order-{}", Utc::now().timestamp());

		let response = SubmitOrderResponse {
			status: PostOrderResponseStatus::Received,
			order_id: Some(order_id.clone()),
			message: Some("Order submitted successfully".to_string()),
			order: None,
			metadata: None,
		};
		Ok(OifPostOrderResponse::new(response))
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<OifGetOrderResponse> {
		let response = GetOrderResponse {
			id: order_id.to_string(),
			status: OrderStatus::Finalized,
			created_at: Utc::now().timestamp() as u64 - 60,
			updated_at: Utc::now().timestamp() as u64,
			quote_id: Some("mock-quote-123".to_string()),
			input_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(
					1,
					"0x0000000000000000000000000000000000000000",
				)
				.unwrap(),
				amount: Some(U256::new("1000000000000000000".to_string())),
			}],
			output_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(
					1,
					"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0",
				)
				.unwrap(),
				amount: Some(U256::new("1000000".to_string())),
			}],
			settlement: Settlement {
				settlement_type: SettlementType::Escrow,
				data: json!({"escrow_address": "0x1234567890123456789012345678901234567890"}),
			},
			fill_transaction: None,
		};
		Ok(OifGetOrderResponse::new(response))
	}

	async fn get_supported_assets(
		&self,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<SupportedAssetsData> {
		// Mock assets for testing (using assets mode)
		// Support assets that match the test fixtures
		let assets = vec![
			// Native ETH on Ethereum
			Asset::from_chain_and_address(
				1,
				"0x0000000000000000000000000000000000000000".to_string(),
				"ETH".to_string(),
				"Ethereum".to_string(),
				18,
			)
			.map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Invalid mock asset: {}", e),
			})?,
			// WETH on Ethereum (matches test fixtures)
			Asset::from_chain_and_address(
				1,
				"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
				"WETH".to_string(),
				"Wrapped Ethereum".to_string(),
				18,
			)
			.map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Invalid mock asset: {}", e),
			})?,
			// USDC on Ethereum (matches test fixtures)
			Asset::from_chain_and_address(
				1,
				"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
			)
			.map_err(|e| AdapterError::InvalidResponse {
				reason: format!("Invalid mock asset: {}", e),
			})?,
		];

		Ok(SupportedAssetsData::Assets(assets))
	}

	async fn health_check(&self, _config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		Ok(true)
	}
}

/// Simple test adapter that can be configured to succeed or fail
#[derive(Debug, Clone)]
pub struct MockTestAdapter {
	pub id: String,
	pub name: String,
	pub should_fail: bool,
	pub adapter_info: oif_types::Adapter,
}

impl MockTestAdapter {
	pub fn new() -> Self {
		let id = "mock-test-v1".to_string();
		let name = "Mock Test Adapter".to_string();

		Self {
			adapter_info: oif_types::Adapter::new(
				id.clone(),
				"Mock Test Adapter for testing".to_string(),
				name.clone(),
				"1.0.0".to_string(),
			),
			id,
			name,
			should_fail: false,
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
	fn adapter_info(&self) -> &oif_types::Adapter {
		&self.adapter_info
	}

	async fn get_quotes(
		&self,
		_request: &OifGetQuoteRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<OifGetQuoteResponse> {
		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: "Mock adapter configured to fail".to_string(),
				},
			));
		}

		let response = oif_types::GetQuoteResponse { quotes: vec![] };
		Ok(OifGetQuoteResponse::new(response))
	}

	async fn submit_order(
		&self,
		_request: &OifPostOrderRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<OifPostOrderResponse> {
		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: "Mock adapter configured to fail".to_string(),
				},
			));
		}

		let order_id = format!("test-order-{}", Utc::now().timestamp());
		let response = SubmitOrderResponse {
			status: PostOrderResponseStatus::Received,
			order_id: Some(order_id.clone()),
			message: Some("Order submitted successfully".to_string()),
			order: None,
			metadata: None,
		};
		Ok(OifPostOrderResponse::new(response))
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<OifGetOrderResponse> {
		let response = GetOrderResponse {
			id: order_id.to_string(),
			status: OrderStatus::Created,
			created_at: Utc::now().timestamp() as u64,
			updated_at: Utc::now().timestamp() as u64,
			quote_id: None,
			input_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(
					1,
					"0x0000000000000000000000000000000000000000",
				)
				.unwrap(),
				amount: Some(U256::new("0".to_string())),
			}],
			output_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(
					1,
					"0x0000000000000000000000000000000000000000",
				)
				.unwrap(),
				amount: Some(U256::new("0".to_string())),
			}],
			settlement: Settlement {
				settlement_type: SettlementType::Escrow,
				data: json!({}),
			},
			fill_transaction: None,
		};
		Ok(OifGetOrderResponse::new(response))
	}

	async fn get_supported_assets(
		&self,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<SupportedAssetsData> {
		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: "Mock adapter configured to fail".to_string(),
				},
			));
		}

		// Return empty assets mode for test adapter
		Ok(SupportedAssetsData::Assets(vec![]))
	}

	async fn health_check(&self, _config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		Ok(!self.should_fail)
	}
}

#[allow(dead_code)]
pub fn mock_solver() -> Solver {
	// Create assets that match the test fixtures for immediate compatibility
	let assets = vec![
		// Native ETH on Ethereum
		Asset::from_chain_and_address(
			1,
			"0x0000000000000000000000000000000000000000".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
		)
		.expect("Valid ETH asset"),
		// WETH on Ethereum (matches test fixtures)
		Asset::from_chain_and_address(
			1,
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
			"WETH".to_string(),
			"Wrapped Ethereum".to_string(),
			18,
		)
		.expect("Valid WETH asset"),
		// USDC on Ethereum (matches test fixtures)
		Asset::from_chain_and_address(
			1,
			"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
		)
		.expect("Valid USDC asset"),
	];

	Solver::new(
		"mock-demo-solver".to_string(),
		"mock-demo-v1".to_string(),
		"http://localhost:8080".to_string(),
	)
	.with_assets(assets)
}

#[allow(dead_code)]
pub fn mock_quote_request() -> (Value, InteropAddress, InteropAddress, InteropAddress) {
	let user_addr =
		InteropAddress::from_chain_and_address(1, "0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03")
			.unwrap();
	let eth_addr =
		InteropAddress::from_chain_and_address(1, "0x0000000000000000000000000000000000000000")
			.unwrap();
	let usdc_addr =
		InteropAddress::from_chain_and_address(1, "0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0")
			.unwrap();

	let quote_request = json!({
		"user": user_addr.to_hex(),
		"intent": {
			"intentType": "oif-swap",
			"inputs": [
				{
					"user": user_addr.to_hex(),
					"asset": eth_addr.to_hex(), // ETH
					"amount": "1000000000000000000", // 1 ETH
					"lock": null
				}
			],
			"outputs": [
				{
					"asset": usdc_addr.to_hex(), // USDC
					"amount": "1000000", // 1 USDC
					"receiver": user_addr.to_hex(),
					"calldata": null
				}
			],
			"swapType": "exact-input",
			"minValidUntil": 300,
			"preference": "speed",
			"partialFill": false
		},
		"supportedTypes": ["oif-escrow-v0"],
		"solverOptions": {
			"timeout": 4000,
			"solverTimeout": 2000
		}
	});

	(quote_request, user_addr, eth_addr, usdc_addr)
}
