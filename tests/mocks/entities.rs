//! Domain entity mocks for testing

use oif_types::chrono::{Duration, Utc};
use oif_types::{
	orders::{Order, OrderStatus},
	quotes::{Quote, QuoteRequest},
	solvers::{Solver, SolverStatus},
};

/// Common test addresses and tokens
pub struct TestConstants;

impl TestConstants {
	pub const WETH_ADDRESS: &'static str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
	pub const USDC_ADDRESS: &'static str = "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0";
	pub const TEST_USER_ADDRESS: &'static str = "0x1234567890123456789012345678901234567890";
	pub const ETHEREUM_CHAIN_ID: u64 = 1;
	pub const ONE_ETH_WEI: &'static str = "1000000000000000000";
	pub const TWO_THOUSAND_USDC: &'static str = "2000000000"; // 2000 USDC (6 decimals)
}

/// Entity builders for tests
pub struct MockEntities;

impl MockEntities {
	/// Create a basic test quote
	pub fn quote() -> Quote {
		Quote::new(
			"test-solver-1".to_string(),
			"request-123".to_string(),
			TestConstants::WETH_ADDRESS.to_string(),
			TestConstants::USDC_ADDRESS.to_string(),
			TestConstants::ONE_ETH_WEI.to_string(),
			TestConstants::TWO_THOUSAND_USDC.to_string(),
			TestConstants::ETHEREUM_CHAIN_ID,
		)
	}

	/// Create a quote with custom parameters
	pub fn quote_with_params(
		solver_id: &str,
		request_id: &str,
		token_in: &str,
		token_out: &str,
		amount_in: &str,
		amount_out: &str,
		chain_id: u64,
	) -> Quote {
		Quote::new(
			solver_id.to_string(),
			request_id.to_string(),
			token_in.to_string(),
			token_out.to_string(),
			amount_in.to_string(),
			amount_out.to_string(),
			chain_id,
		)
	}

	/// Create a quote that expires soon (for TTL testing)
	pub fn expiring_quote(ttl_seconds: i64) -> Quote {
		let mut quote = Self::quote();
		quote.expires_at = Utc::now() + Duration::seconds(ttl_seconds);
		quote
	}

	/// Create an expired quote
	pub fn expired_quote() -> Quote {
		let mut quote = Self::quote();
		quote.expires_at = Utc::now() - Duration::minutes(1);
		quote
	}

	/// Create a quote with all optional fields populated
	pub fn complete_quote() -> Quote {
		Self::quote()
			.with_estimated_gas(150000)
			.with_price_impact(0.02)
	}

	/// Create a basic test order
	pub fn order() -> Order {
		Order::new(
			TestConstants::TEST_USER_ADDRESS.to_string(),
			0.01,                            // 1% slippage
			Utc::now() + Duration::hours(1), // 1 hour deadline
		)
	}

	/// Create an order with custom user address
	pub fn order_for_user(user_address: &str) -> Order {
		Order::new(
			user_address.to_string(),
			0.01,                            // 1% slippage
			Utc::now() + Duration::hours(1), // 1 hour deadline
		)
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
			1000,
		);
		solver.status = SolverStatus::Active;
		solver
	}

	/// Create solver with custom parameters
	pub fn solver_with_params(
		solver_id: &str,
		adapter_id: &str,
		endpoint: &str,
		timeout_ms: u64,
	) -> Solver {
		let mut solver = Solver::new(
			solver_id.to_string(),
			adapter_id.to_string(),
			endpoint.to_string(),
			timeout_ms,
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
					1000 + (i as u64) * 100,
				)
			})
			.collect()
	}

	/// Create a basic quote request
	pub fn quote_request() -> QuoteRequest {
		QuoteRequest {
			request_id: "test-request-123".to_string(),
			token_in: TestConstants::WETH_ADDRESS.to_string(),
			token_out: TestConstants::USDC_ADDRESS.to_string(),
			amount_in: TestConstants::ONE_ETH_WEI.to_string(),
			chain_id: TestConstants::ETHEREUM_CHAIN_ID,
			slippage_tolerance: Some(0.005),
			deadline: None,
			recipient: None,
			referrer: None,
		}
	}

	/// Create quote request with custom slippage
	pub fn quote_request_with_slippage(slippage: f64) -> QuoteRequest {
		QuoteRequest {
			slippage_tolerance: Some(slippage),
			..Self::quote_request()
		}
	}

	/// Create quote request with deadline
	pub fn quote_request_with_deadline(deadline_seconds: i64) -> QuoteRequest {
		QuoteRequest {
			deadline: Some(Utc::now() + Duration::seconds(deadline_seconds)),
			..Self::quote_request()
		}
	}
}
