//! Domain entity mocks for testing

use oif_types::{
	chrono::{Duration, Utc},
	oif::{
		common::{
			AssetAmount, IntentType, OrderStatus as OifOrderStatus, Settlement, SettlementType,
			SwapType,
		},
		v0::{
			GetOrderResponse, GetQuoteRequest, IntentRequest, Order as V0Order, OrderPayload,
			Quote as V0Quote,
		},
		OifGetOrderResponse,
	},
	orders::Order,
	quotes::{Quote, QuoteRequest},
	serde_json::json,
	solvers::{Solver, SolverStatus},
	Input, InteropAddress, Output, SignatureType, U256,
};

/// Common test addresses and tokens
pub struct TestConstants;

#[allow(dead_code)]
impl TestConstants {
	pub const WETH_ADDRESS: &'static str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
	pub const USDC_ADDRESS: &'static str = "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0";
	pub const TEST_USER_ADDRESS: &'static str = "0x1234567890123456789012345678901234567890";
	pub const ETHEREUM_CHAIN_ID: u64 = 1;
	pub const ONE_ETH_WEI: &'static str = "1000000000000000000";
	pub const TWO_THOUSAND_USDC: &'static str = "2000000000"; // 2000 USDC (6 decimals)
}

/// Entity builders for tests
#[allow(dead_code)]
pub struct MockEntities;

#[allow(dead_code)]
impl MockEntities {
	/// Create a basic test quote using current Quote::new signature
	pub fn quote() -> Quote {
		let v0_quote = V0Quote {
			preview: oif_types::oif::common::QuotePreview {
				inputs: vec![],
				outputs: vec![],
			},
			quote_id: Some("test-quote-123".to_string()),
			order: V0Order::OifEscrowV0 {
				payload: OrderPayload {
					signature_type: SignatureType::Eip712,
					domain: json!({
						"name": "TestQuote",
						"version": "1",
						"chainId": 1
					}),
					primary_type: "Order".to_string(),
					message: json!({
						"orderType": "swap",
						"inputAsset": TestConstants::WETH_ADDRESS,
						"outputAsset": TestConstants::USDC_ADDRESS
					}),
					types: std::collections::HashMap::new(),
				},
			},
			valid_until: Some(Utc::now().timestamp() as u64 + 300),
			eta: Some(30),
			provider: Some("Test Provider".to_string()),
			failure_handling: None,
			partial_fill: false,
			metadata: Some(json!({
				"test": "data"
			})),
		};

		Quote::new(
			"test-solver-1".to_string(),
			oif_types::oif::OifQuote::new(v0_quote),
			"test-checksum".to_string(),
		)
	}

	/// Create a quote with custom parameters
	pub fn quote_with_solver(solver_id: &str, provider: &str) -> Quote {
		let v0_quote = V0Quote {
			preview: oif_types::oif::common::QuotePreview {
				inputs: vec![],
				outputs: vec![],
			},
			quote_id: Some("test-quote-123".to_string()),
			order: V0Order::OifEscrowV0 {
				payload: OrderPayload {
					signature_type: SignatureType::Eip712,
					domain: json!({
						"name": "TestQuote",
						"version": "1",
						"chainId": 1
					}),
					primary_type: "Order".to_string(),
					message: json!({
						"orderType": "swap",
						"inputAsset": TestConstants::WETH_ADDRESS,
						"outputAsset": TestConstants::USDC_ADDRESS
					}),
					types: std::collections::HashMap::new(),
				},
			},
			valid_until: Some(Utc::now().timestamp() as u64 + 300),
			eta: Some(30),
			provider: Some(provider.to_string()),
			failure_handling: None,
			partial_fill: false,
			metadata: Some(json!({
				"test": "data"
			})),
		};

		Quote::new(
			solver_id.to_string(),
			oif_types::oif::OifQuote::new(v0_quote),
			"test-checksum".to_string(),
		)
	}

	/// Create a quote that expires soon (for TTL testing)
	pub fn expiring_quote(ttl_seconds: i64) -> Quote {
		let v0_quote = V0Quote {
			preview: oif_types::oif::common::QuotePreview {
				inputs: vec![],
				outputs: vec![],
			},
			quote_id: Some("test-quote-expiring".to_string()),
			order: V0Order::OifEscrowV0 {
				payload: OrderPayload {
					signature_type: SignatureType::Eip712,
					domain: json!({
						"name": "TestQuote",
						"version": "1",
						"chainId": 1
					}),
					primary_type: "Order".to_string(),
					message: json!({
						"orderType": "swap",
						"inputAsset": TestConstants::WETH_ADDRESS,
						"outputAsset": TestConstants::USDC_ADDRESS
					}),
					types: std::collections::HashMap::new(),
				},
			},
			valid_until: Some(Utc::now().timestamp() as u64 + ttl_seconds as u64),
			eta: Some(30),
			provider: Some("Test Provider".to_string()),
			failure_handling: None,
			partial_fill: false,
			metadata: Some(json!({
				"test": "data"
			})),
		};

		Quote::new(
			"test-solver-1".to_string(),
			oif_types::oif::OifQuote::new(v0_quote),
			"test-checksum".to_string(),
		)
	}

	/// Create an expired quote
	pub fn expired_quote() -> Quote {
		let v0_quote = V0Quote {
			preview: oif_types::oif::common::QuotePreview {
				inputs: vec![],
				outputs: vec![],
			},
			quote_id: Some("test-quote-expired".to_string()),
			order: V0Order::OifEscrowV0 {
				payload: OrderPayload {
					signature_type: SignatureType::Eip712,
					domain: json!({
						"name": "TestQuote",
						"version": "1",
						"chainId": 1
					}),
					primary_type: "Order".to_string(),
					message: json!({
						"orderType": "swap",
						"inputAsset": TestConstants::WETH_ADDRESS,
						"outputAsset": TestConstants::USDC_ADDRESS
					}),
					types: std::collections::HashMap::new(),
				},
			},
			valid_until: Some((Utc::now() - Duration::minutes(1)).timestamp() as u64),
			eta: Some(30),
			provider: Some("Test Provider".to_string()),
			failure_handling: None,
			partial_fill: false,
			metadata: Some(json!({
				"test": "data"
			})),
		};

		Quote::new(
			"test-solver-1".to_string(),
			oif_types::oif::OifQuote::new(v0_quote),
			"test-checksum".to_string(),
		)
	}

	/// Create a basic test order using current Order structure
	pub fn order() -> Order {
		let order_response = GetOrderResponse {
			id: "test-order-123".to_string(),
			status: OifOrderStatus::Created,
			created_at: Utc::now().timestamp() as u64,
			updated_at: Utc::now().timestamp() as u64,
			quote_id: Some("test-quote-123".to_string()),
			input_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS)
					.unwrap(),
				amount: Some(U256::new("1000000000000000000".to_string())),
			}],
			output_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS)
					.unwrap(),
				amount: Some(U256::new("1000000".to_string())),
			}],
			settlement: Settlement {
				settlement_type: SettlementType::Escrow,
				data: json!({}),
			},
			fill_transaction: None,
		};

		Order::new(
			"test-solver-123".to_string(),
			OifGetOrderResponse::new(order_response),
			None,
		)
	}

	/// Create an order with custom amounts
	pub fn order_with_amounts(input_amount: &str, output_amount: &str) -> Order {
		let order_response = GetOrderResponse {
			id: "test-order-123".to_string(),
			status: OifOrderStatus::Created,
			created_at: Utc::now().timestamp() as u64,
			updated_at: Utc::now().timestamp() as u64,
			quote_id: Some("test-quote-123".to_string()),
			input_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS)
					.unwrap(),
				amount: Some(U256::new(input_amount.to_string())),
			}],
			output_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS)
					.unwrap(),
				amount: Some(U256::new(output_amount.to_string())),
			}],
			settlement: Settlement {
				settlement_type: SettlementType::Escrow,
				data: json!({}),
			},
			fill_transaction: None,
		};

		Order::new(
			"test-solver-123".to_string(),
			OifGetOrderResponse::new(order_response),
			None,
		)
	}

	/// Create an order with specific status
	pub fn order_with_status(status: OifOrderStatus) -> Order {
		let order_response = GetOrderResponse {
			id: "test-order-123".to_string(),
			status,
			created_at: Utc::now().timestamp() as u64,
			updated_at: Utc::now().timestamp() as u64,
			quote_id: Some("test-quote-123".to_string()),
			input_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS)
					.unwrap(),
				amount: Some(U256::new("1000000000000000000".to_string())),
			}],
			output_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS)
					.unwrap(),
				amount: Some(U256::new("1000000".to_string())),
			}],
			settlement: Settlement {
				settlement_type: SettlementType::Escrow,
				data: json!({}),
			},
			fill_transaction: None,
		};

		Order::new(
			"test-solver-123".to_string(),
			OifGetOrderResponse::new(order_response),
			None,
		)
	}

	/// Create a test solver
	pub fn solver() -> Solver {
		let mut solver = Solver::new(
			"test-solver".to_string(),
			"test-adapter".to_string(),
			"http://localhost:8080".to_string(),
		);
		solver.status = SolverStatus::Active;
		solver
	}

	/// Create solver with custom parameters
	pub fn solver_with_params(solver_id: &str, adapter_id: &str, endpoint: &str) -> Solver {
		let mut solver = Solver::new(
			solver_id.to_string(),
			adapter_id.to_string(),
			endpoint.to_string(),
		);
		solver.status = SolverStatus::Active;
		solver
	}

	/// Create inactive solver
	pub fn inactive_solver() -> Solver {
		let mut solver = Self::solver();
		solver.status = SolverStatus::Disabled;
		solver
	}

	/// Create multiple test solvers
	pub fn multiple_solvers(count: usize) -> Vec<Solver> {
		(1..=count)
			.map(|i| {
				Self::solver_with_params(
					&format!("test-solver-{}", i),
					&format!("test-adapter-{}", i),
					&format!("http://localhost:808{}", i),
				)
			})
			.collect()
	}

	/// Create a basic ERC-7930 compliant quote request
	pub fn quote_request() -> QuoteRequest {
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();
		let eth_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS).unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS).unwrap();

		let oif_request = GetQuoteRequest {
			user: user_addr.clone(),
			intent: IntentRequest {
				intent_type: IntentType::OifSwap,
				inputs: vec![Input {
					user: user_addr.clone(),
					asset: eth_addr,
					amount: Some(U256::new(TestConstants::ONE_ETH_WEI.to_string())),
					lock: None,
				}],
				outputs: vec![Output {
					asset: usdc_addr,
					amount: Some(U256::new(TestConstants::TWO_THOUSAND_USDC.to_string())),
					receiver: user_addr,
					calldata: None,
				}],
				swap_type: Some(SwapType::ExactInput),
				min_valid_until: Some(300),
				preference: None,
				origin_submission: None,
				failure_handling: None,
				partial_fill: None,
				metadata: None,
			},
			supported_types: vec!["oif-escrow-v0".to_string()],
		};

		QuoteRequest {
			quote_request: oif_request,
			solver_options: None,
			metadata: None,
		}
	}

	/// Create quote request with custom min_valid_until
	pub fn quote_request_with_min_valid_until(min_valid_until: u64) -> QuoteRequest {
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();
		let eth_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS).unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS).unwrap();

		let oif_request = GetQuoteRequest {
			user: user_addr.clone(),
			intent: IntentRequest {
				intent_type: IntentType::OifSwap,
				inputs: vec![Input {
					user: user_addr.clone(),
					asset: eth_addr,
					amount: Some(U256::new(TestConstants::ONE_ETH_WEI.to_string())),
					lock: None,
				}],
				outputs: vec![Output {
					asset: usdc_addr,
					amount: Some(U256::new(TestConstants::TWO_THOUSAND_USDC.to_string())),
					receiver: user_addr,
					calldata: None,
				}],
				swap_type: Some(SwapType::ExactInput),
				min_valid_until: Some(min_valid_until),
				preference: None,
				origin_submission: None,
				failure_handling: None,
				partial_fill: None,
				metadata: None,
			},
			supported_types: vec!["oif-escrow-v0".to_string()],
		};

		QuoteRequest {
			quote_request: oif_request,
			solver_options: None,
			metadata: None,
		}
	}

	/// Create quote request for different tokens
	pub fn quote_request_custom_tokens(token_in_addr: &str, token_out_addr: &str) -> QuoteRequest {
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();
		let in_addr = InteropAddress::from_chain_and_address(1, token_in_addr).unwrap();
		let out_addr = InteropAddress::from_chain_and_address(1, token_out_addr).unwrap();

		let oif_request = GetQuoteRequest {
			user: user_addr.clone(),
			intent: IntentRequest {
				intent_type: IntentType::OifSwap,
				inputs: vec![Input {
					user: user_addr.clone(),
					asset: in_addr,
					amount: Some(U256::new(TestConstants::ONE_ETH_WEI.to_string())),
					lock: None,
				}],
				outputs: vec![Output {
					asset: out_addr,
					amount: Some(U256::new(TestConstants::TWO_THOUSAND_USDC.to_string())),
					receiver: user_addr,
					calldata: None,
				}],
				swap_type: Some(SwapType::ExactInput),
				min_valid_until: Some(300),
				preference: None,
				origin_submission: None,
				failure_handling: None,
				partial_fill: None,
				metadata: None,
			},
			supported_types: vec!["oif-escrow-v0".to_string()],
		};

		QuoteRequest {
			quote_request: oif_request,
			solver_options: None,
			metadata: None,
		}
	}
}
