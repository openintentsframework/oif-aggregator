//! API request/response fixtures for e2e and integration tests

use oif_aggregator::{
	api::routes::AppState,
	service::{aggregator::AggregatorService, OrderService, SolverService},
	storage::MemoryStore,
};
use serde_json::{json, Value};
use std::sync::Arc;

use super::entities::{MockEntities, TestConstants};

/// API test data fixtures
pub struct ApiFixtures;

impl ApiFixtures {
	/// Valid quote request with all fields
	pub fn valid_quote_request() -> Value {
		json!({
			"token_in": TestConstants::WETH_ADDRESS,
			"token_out": TestConstants::USDC_ADDRESS,
			"amount_in": TestConstants::ONE_ETH_WEI,
			"chain_id": TestConstants::ETHEREUM_CHAIN_ID,
			"slippage_tolerance": 0.005
		})
	}

	/// Valid quote request with minimal fields
	pub fn minimal_quote_request() -> Value {
		json!({
			"token_in": TestConstants::WETH_ADDRESS,
			"token_out": TestConstants::USDC_ADDRESS,
			"amount_in": TestConstants::ONE_ETH_WEI,
			"chain_id": TestConstants::ETHEREUM_CHAIN_ID
		})
	}

	/// Quote request with custom parameters
	pub fn quote_request_with_params(
		token_in: &str,
		token_out: &str,
		amount_in: &str,
		chain_id: u64,
	) -> Value {
		json!({
			"token_in": token_in,
			"token_out": token_out,
			"amount_in": amount_in,
			"chain_id": chain_id
		})
	}

	/// Invalid quote request - empty token_in
	pub fn invalid_quote_request_empty_token() -> Value {
		json!({
			"token_in": "",
			"token_out": TestConstants::USDC_ADDRESS,
			"amount_in": TestConstants::ONE_ETH_WEI,
			"chain_id": TestConstants::ETHEREUM_CHAIN_ID
		})
	}

	/// Invalid quote request - missing required field
	pub fn invalid_quote_request_missing_token_out() -> Value {
		json!({
			"token_in": TestConstants::WETH_ADDRESS,
			"amount_in": TestConstants::ONE_ETH_WEI,
			"chain_id": TestConstants::ETHEREUM_CHAIN_ID
		})
	}

	/// Invalid quote request - invalid chain ID
	pub fn invalid_quote_request_invalid_chain() -> Value {
		json!({
			"token_in": TestConstants::WETH_ADDRESS,
			"token_out": TestConstants::USDC_ADDRESS,
			"amount_in": TestConstants::ONE_ETH_WEI,
			"chain_id": 999999 // Invalid chain
		})
	}

	/// Valid order request (stateless with quote_response)
	pub fn valid_order_request_stateless() -> Value {
		json!({
			"user_address": TestConstants::TEST_USER_ADDRESS,
			"quote_response": {
				"token_in": TestConstants::WETH_ADDRESS,
				"token_out": TestConstants::USDC_ADDRESS,
				"amount_in": TestConstants::ONE_ETH_WEI,
				"amount_out": TestConstants::TWO_THOUSAND_USDC,
				"chain_id": TestConstants::ETHEREUM_CHAIN_ID,
				"price_impact": 0.02,
				"estimated_gas": 150000
			},
			"slippage_tolerance": 0.01,
			"deadline": (chrono::Utc::now().timestamp() + 3600)
		})
	}

	/// Valid order request using quote_id
	pub fn valid_order_request_with_quote_id(quote_id: &str) -> Value {
		json!({
			"user_address": TestConstants::TEST_USER_ADDRESS,
			"quote_id": quote_id,
			"slippage_tolerance": 0.01,
			"deadline": (chrono::Utc::now().timestamp() + 3600)
		})
	}

	/// Order request with custom user address
	pub fn order_request_for_user(user_address: &str) -> Value {
		json!({
			"user_address": user_address,
			"quote_response": {
				"token_in": TestConstants::WETH_ADDRESS,
				"token_out": TestConstants::USDC_ADDRESS,
				"amount_in": TestConstants::ONE_ETH_WEI,
				"amount_out": TestConstants::TWO_THOUSAND_USDC,
				"chain_id": TestConstants::ETHEREUM_CHAIN_ID,
				"price_impact": 0.02,
				"estimated_gas": 150000
			},
			"slippage_tolerance": 0.01
		})
	}

	/// Invalid order request - missing user_address
	pub fn invalid_order_request_missing_user() -> Value {
		json!({
			"quote_id": "test-quote-id"
		})
	}

	/// Invalid order request - missing quote data
	pub fn invalid_order_request_missing_quote() -> Value {
		json!({
			"user_address": TestConstants::TEST_USER_ADDRESS
		})
	}

	/// Order request with non-existent quote_id
	pub fn order_request_with_invalid_quote_id() -> Value {
		json!({
			"user_address": TestConstants::TEST_USER_ADDRESS,
			"quote_id": "non-existent-quote-id"
		})
	}

	/// Large payload for testing body size limits
	pub fn large_quote_request() -> Value {
		let large_payload = "x".repeat(2 * 1024 * 1024); // 2MB
		json!({
			"token_in": large_payload,
			"token_out": TestConstants::USDC_ADDRESS,
			"amount_in": TestConstants::ONE_ETH_WEI,
			"chain_id": TestConstants::ETHEREUM_CHAIN_ID
		})
	}

	/// Malformed JSON string for testing error handling
	pub fn malformed_json() -> &'static str {
		"{ invalid json structure"
	}
}

/// Application state builders for tests
pub struct AppStateBuilder;

impl AppStateBuilder {
	/// Create minimal test app state (no solvers)
	pub fn minimal() -> AppState {
		let aggregator_service = AggregatorService::new(vec![], 5000);
		let storage = Arc::new(MemoryStore::new());
		AppState {
			aggregator_service: Arc::new(aggregator_service),
			storage: storage.clone(),
			order_service: Arc::new(OrderService::new(storage.clone())),
			solver_service: Arc::new(SolverService::new(storage.clone())),
		}
	}

	/// Create app state with test solvers
	pub fn with_solvers(solver_count: usize) -> AppState {
		let solvers = MockEntities::multiple_solvers(solver_count);
		let aggregator_service = AggregatorService::new(solvers, 5000);
		let storage = Arc::new(MemoryStore::new());
		AppState {
			aggregator_service: Arc::new(aggregator_service),
			storage: storage.clone(),
			order_service: Arc::new(OrderService::new(storage.clone())),
			solver_service: Arc::new(SolverService::new(storage.clone())),
		}
	}

	/// Create app state with custom timeout
	pub fn with_timeout(timeout_ms: u64) -> AppState {
		let aggregator_service = AggregatorService::new(vec![], timeout_ms);
		let storage = Arc::new(MemoryStore::new());
		AppState {
			aggregator_service: Arc::new(aggregator_service),
			storage: storage.clone(),
			order_service: Arc::new(OrderService::new(storage.clone())),
			solver_service: Arc::new(SolverService::new(storage.clone())),
		}
	}
}
