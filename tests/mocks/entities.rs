//! Domain entity mocks for testing

use oif_types::{
	adapters::models::{
		AssetAmount, AvailableInput, QuoteDetails, QuoteOrder, RequestedOutput, Settlement,
		SettlementType, SignatureType,
	},
	chrono::{Duration, Utc},
	orders::{Order, OrderStatus},
	quotes::{Quote, QuoteRequest},
	serde_json::json,
	solvers::{Solver, SolverStatus},
	InteropAddress, U256,
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
		let orders = vec![QuoteOrder {
			signature_type: SignatureType::Eip712,
			domain: InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS)
				.unwrap(),
			primary_type: "Order".to_string(),
			message: json!({
				"orderType": "swap",
				"inputAsset": TestConstants::WETH_ADDRESS,
				"outputAsset": TestConstants::USDC_ADDRESS
			}),
		}];

		let details = QuoteDetails {
			available_inputs: vec![AvailableInput {
				user: InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS)
					.unwrap(),
				asset: InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS)
					.unwrap(),
				amount: U256::new(TestConstants::ONE_ETH_WEI.to_string()),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS)
					.unwrap(),
				amount: U256::new(TestConstants::TWO_THOUSAND_USDC.to_string()),
				receiver: InteropAddress::from_chain_and_address(
					1,
					TestConstants::TEST_USER_ADDRESS,
				)
				.unwrap(),
				calldata: None,
			}],
		};

		Quote::new(
			"test-solver-1".to_string(),
			orders,
			details,
			"Test Provider".to_string(),
			"test-checksum".to_string(),
		)
	}

	/// Create a quote with custom parameters
	pub fn quote_with_solver(solver_id: &str, provider: &str) -> Quote {
		let mut quote = Self::quote();
		quote.solver_id = solver_id.to_string();
		quote.provider = provider.to_string();
		quote
	}

	/// Create a quote that expires soon (for TTL testing)
	pub fn expiring_quote(ttl_seconds: i64) -> Quote {
		let mut quote = Self::quote();
		quote.valid_until = Some(Utc::now().timestamp() as u64 + ttl_seconds as u64);
		quote
	}

	/// Create an expired quote
	pub fn expired_quote() -> Quote {
		let mut quote = Self::quote();
		quote.valid_until = Some((Utc::now() - Duration::minutes(1)).timestamp() as u64);
		quote
	}

	/// Create a basic test order using current Order structure
	pub fn order() -> Order {
		Order {
			order_id: "test-order-123".to_string(),
			quote_id: Some("test-quote-123".to_string()),
			solver_id: "test-solver-123".to_string(),
			status: OrderStatus::Created,
			created_at: Utc::now(),
			updated_at: Utc::now(),
			input_amount: AssetAmount {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS)
					.unwrap(),
				amount: U256::new("1000000000000000000".to_string()),
			},
			output_amount: AssetAmount {
				asset: InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS)
					.unwrap(),
				amount: U256::new("1000000".to_string()),
			},
			settlement: Settlement {
				settlement_type: SettlementType::Escrow,
				data: json!({}),
			},
			fill_transaction: None,
			quote_details: None,
		}
	}

	/// Create an order with custom amounts
	pub fn order_with_amounts(input_amount: &str, output_amount: &str) -> Order {
		let mut order = Self::order();
		order.input_amount.amount = U256::new(input_amount.to_string());
		order.output_amount.amount = U256::new(output_amount.to_string());
		order
	}

	/// Create an order with specific status
	pub fn order_with_status(status: OrderStatus) -> Order {
		let mut order = Self::order();
		order.status = status;
		order
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
		solver.status = SolverStatus::Inactive;
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

		QuoteRequest {
			user: user_addr.clone(),
			available_inputs: vec![AvailableInput {
				user: user_addr.clone(),
				asset: eth_addr,
				amount: U256::new(TestConstants::ONE_ETH_WEI.to_string()),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				asset: usdc_addr,
				amount: U256::new(TestConstants::TWO_THOUSAND_USDC.to_string()),
				receiver: user_addr,
				calldata: None,
			}],
			min_valid_until: Some(300),
			preference: None,
			solver_options: None,
		}
	}

	/// Create quote request with custom min_valid_until
	pub fn quote_request_with_min_valid_until(min_valid_until: u64) -> QuoteRequest {
		let mut request = Self::quote_request();
		request.min_valid_until = Some(min_valid_until);
		request
	}

	/// Create quote request for different tokens
	pub fn quote_request_custom_tokens(token_in_addr: &str, token_out_addr: &str) -> QuoteRequest {
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();
		let in_addr = InteropAddress::from_chain_and_address(1, token_in_addr).unwrap();
		let out_addr = InteropAddress::from_chain_and_address(1, token_out_addr).unwrap();

		QuoteRequest {
			user: user_addr.clone(),
			available_inputs: vec![AvailableInput {
				user: user_addr.clone(),
				asset: in_addr,
				amount: U256::new(TestConstants::ONE_ETH_WEI.to_string()),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				asset: out_addr,
				amount: U256::new(TestConstants::TWO_THOUSAND_USDC.to_string()),
				receiver: user_addr,
				calldata: None,
			}],
			min_valid_until: Some(300),
			preference: None,
			solver_options: None,
		}
	}
}
