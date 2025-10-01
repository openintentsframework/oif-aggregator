//! API request/response fixtures for e2e and integration tests

use oif_aggregator::{api::routes::AppState, AggregatorBuilder};
use oif_types::{
	chrono,
	oif::v0::{Order as V0Order, OrderPayload, Quote as V0Quote},
	serde_json::{json, Value},
	InteropAddress, Quote, SignatureType,
};

use super::entities::TestConstants;

/// API test data fixtures
#[allow(dead_code)]
pub struct ApiFixtures;

pub static INTEGRITY_SECRET: &str = "test-secret-for-e2e-tests-12345678901234567890";

#[allow(dead_code)]
impl ApiFixtures {
	/// Valid ERC-7930 compliant quote request
	pub fn valid_quote_request() -> Value {
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();
		let eth_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS).unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS).unwrap();

		json!({
			"user": user_addr.to_hex(),
			"intent": {
				"intentType": "oif-swap",
				"inputs": [
					{
						"user": user_addr.to_hex(),
						"asset": eth_addr.to_hex(), // ETH
						"amount": TestConstants::ONE_ETH_WEI
					}
				],
				"outputs": [
					{
						"asset": usdc_addr.to_hex(), // USDC
						"amount": TestConstants::TWO_THOUSAND_USDC,
						"receiver": user_addr.to_hex()
					}
				],
				"minValidUntil": 300
			},
			"supportedTypes": ["oif-escrow-v0"],
			"solverOptions": {
				"timeout": 4000,
				"solverTimeout": 2000
			}
		})
	}

	/// Invalid quote request (missing required fields)
	pub fn invalid_quote_request() -> Value {
		json!({
			"invalidField": "This should fail validation"
		})
	}

	/// Valid order submission request
	pub fn valid_order_request() -> Value {
		let quote_response = Self::sample_quote_response();
		json!({
			"quoteResponse": quote_response,
			"signature": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12",
			"metadata": {
				"order": "0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
				"sponsor": "0x70997970c51812dc3a010c7d01b50e0d17dc79c8"
			}
		})
	}

	/// Sample quote response for testing
	pub fn sample_quote_response() -> Value {
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();
		let eth_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS).unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS).unwrap();

		json!({
			"quoteId": "6a22e92f-3e5d-4f05-ab5f-007b01e58b21",
			"solverId": "example-solver",
			"order": {
				"type": "oif-escrow-v0",
				"payload": {
					"signatureType": "eip712",
					"domain": {
						"name": "TestOrder",
						"version": "1",
						"chainId": 1
					},
					"primaryType": "PermitBatchWitnessTransferFrom",
					"message": {
						"digest": "0xdfbfeb9aed6340d513ef52f716cef5b50b677118d364c8448bff1c9ea9fd0b14"
					},
					"types": {}
				}
			},
			"validUntil": chrono::Utc::now().timestamp() as u64 + 300,
			"eta": 30,
			"provider": "Example Solver v1.0",
			"partialFill": false,
			"integrityChecksum": "hmac-sha256:a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890",
			"oifMetadata": {
				"inputs": [{
					"user": user_addr.to_hex(),
					"asset": eth_addr.to_hex(),
					"amount": TestConstants::ONE_ETH_WEI
				}],
				"outputs": [{
					"asset": usdc_addr.to_hex(),
					"amount": TestConstants::TWO_THOUSAND_USDC,
					"receiver": user_addr.to_hex()
				}]
			},
			"metadata": {
				"test": "additional context"
			}
		})
	}

	/// Create test app state with mock adapters for e2e testing
	pub async fn test_app_state() -> AppState {
		// Use the integrity secret for testing
		std::env::set_var("INTEGRITY_SECRET", INTEGRITY_SECRET);

		let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
		let mock_solver = oif_aggregator::mocks::mock_solver();

		let (_, state) = AggregatorBuilder::default()
			.with_adapter(Box::new(mock_adapter))
			.with_solver(mock_solver)
			.start()
			.await
			.expect("Failed to create test app state");

		state
	}

	/// Create a quote for TTL testing that expires soon
	pub fn expiring_quote() -> Quote {
		let v0_quote = V0Quote {
			preview: oif_types::oif::common::QuotePreview {
				inputs: vec![],
				outputs: vec![],
			},
			quote_id: Some("test-expiring-quote".to_string()),
			order: V0Order::OifEscrowV0 {
				payload: OrderPayload {
					signature_type: SignatureType::Eip712,
					domain: json!({
						"name": "TestQuote",
						"version": "1",
						"chainId": 1
					}),
					primary_type: "TestOrder".to_string(),
					message: json!({
							"orderType": "swap",
							"test": "expiring"
					}),
					types: std::collections::HashMap::new(),
				},
			},
			valid_until: Some(chrono::Utc::now().timestamp() as u64 + 5), // Expires in 5 seconds
			eta: Some(30),
			provider: Some("Test Provider".to_string()),
			failure_handling: None,
			partial_fill: false,
			metadata: Some(json!({
				"test": "expiring_quote"
			})),
		};

		Quote::new(
			"test-solver".to_string(),
			oif_types::oif::OifQuote::new(v0_quote),
			"test-checksum".to_string(),
		)
	}

	/// Create a quote for TTL testing
	pub fn quote_with_ttl(ttl_seconds: i64) -> Quote {
		let v0_quote = V0Quote {
			preview: oif_types::oif::common::QuotePreview {
				inputs: vec![],
				outputs: vec![],
			},
			quote_id: Some("test-ttl-quote".to_string()),
			order: V0Order::OifEscrowV0 {
				payload: OrderPayload {
					signature_type: SignatureType::Eip712,
					domain: json!({
						"name": "TestQuote",
						"version": "1",
						"chainId": 1
					}),
					primary_type: "TestOrder".to_string(),
					message: json!({
							"orderType": "swap",
							"ttl": ttl_seconds
					}),
					types: std::collections::HashMap::new(),
				},
			},
			valid_until: Some(chrono::Utc::now().timestamp() as u64 + ttl_seconds as u64),
			eta: Some(30),
			provider: Some("Test Provider".to_string()),
			failure_handling: None,
			partial_fill: false,
			metadata: Some(json!({
				"test": "ttl_quote",
				"ttl_seconds": ttl_seconds
			})),
		};

		Quote::new(
			"test-solver".to_string(),
			oif_types::oif::OifQuote::new(v0_quote),
			"test-checksum".to_string(),
		)
	}

	/// Valid solver configuration for tests
	pub fn valid_solver_config() -> Value {
		json!({
			"solverId": "test-solver",
			"adapterId": "test-adapter",
			"endpoint": "http://localhost:8080",
			"enabled": true,
			"configuration": {
				"timeout": 5000,
				"retries": 3
			}
		})
	}

	/// API health check response format
	pub fn health_response() -> Value {
		json!({
			"status": "healthy",
			"timestamp": chrono::Utc::now().timestamp(),
			"version": "test"
		})
	}

	/// Common test headers for API requests
	pub fn test_headers() -> Vec<(&'static str, &'static str)> {
		vec![
			("Content-Type", "application/json"),
			("User-Agent", "oif-aggregator-test/1.0.0"),
		]
	}

	/// Rate limit exceeded error response
	pub fn rate_limit_error() -> Value {
		json!({
			"error": "rate_limit_exceeded",
			"message": "Too many requests. Please try again later.",
			"timestamp": chrono::Utc::now().timestamp()
		})
	}

	/// Authentication required error response
	pub fn auth_required_error() -> Value {
		json!({
			"error": "authentication_required",
			"message": "Valid authentication credentials are required",
			"timestamp": chrono::Utc::now().timestamp()
		})
	}

	/// Generic validation error response format
	pub fn validation_error(field: &str, message: &str) -> Value {
		json!({
			"error": "validation_failed",
			"field": field,
			"message": message,
			"timestamp": chrono::Utc::now().timestamp()
		})
	}

	/// Solver not found error response
	pub fn solver_not_found_error(solver_id: &str) -> Value {
		json!({
			"error": "solver_not_found",
			"message": format!("Solver '{}' not found", solver_id),
			"solver_id": solver_id,
			"timestamp": chrono::Utc::now().timestamp()
		})
	}

	/// No quotes available error response
	pub fn no_quotes_error() -> Value {
		json!({
			"error": "no_quotes_available",
			"message": "No valid quotes were returned from any solver",
			"timestamp": chrono::Utc::now().timestamp()
		})
	}

	/// Internal server error response
	pub fn internal_error() -> Value {
		json!({
			"error": "internal_server_error",
			"message": "An unexpected error occurred",
			"timestamp": chrono::Utc::now().timestamp()
		})
	}

	/// Test pagination parameters
	pub fn pagination_params() -> Vec<(&'static str, &'static str)> {
		vec![("page", "1"), ("limit", "10")]
	}

	/// Solver status response for health checks
	pub fn solver_status_response(solver_id: &str, healthy: bool) -> Value {
		json!({
			"solver_id": solver_id,
			"status": if healthy { "healthy" } else { "unhealthy" },
			"last_check": chrono::Utc::now().timestamp(),
			"response_time_ms": if healthy { json!(150) } else { json!(null) }
		})
	}

	/// Invalid quote request missing user field
	pub fn invalid_quote_request_missing_user() -> Value {
		json!({
			"intent": {
				"intentType": "oif-swap",
				"inputs": [{
					"asset": "0x0000000000000000000000000000000000000000",
					"amount": "1000000000000000000"
				}],
				"outputs": [{
					"asset": "0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0",
					"amount": "1000000",
					"receiver": "0x1234567890123456789012345678901234567890"
				}]
			}
		})
	}

	/// Invalid order request missing user
	pub fn invalid_order_request_missing_user() -> Value {
		json!({
			"quoteResponse": {
				"quoteId": "test-quote-id",
				"solverId": "test-solver"
			},
			"signature": "0x1234567890abcdef"
		})
	}

	/// Invalid order request missing quote
	pub fn invalid_order_request_missing_quote() -> Value {
		json!({
			"signature": "0x1234567890abcdef",
			"metadata": {
				"test": "data"
			}
		})
	}

	/// Order request with invalid quote ID
	pub fn order_request_with_invalid_quote_id() -> Value {
		let mut request = Self::valid_order_request();
		if let Some(quote_response) = request.get_mut("quoteResponse") {
			quote_response["quoteId"] = json!("invalid-quote-id");
		}
		request
	}

	/// Valid order request with integrity
	pub fn valid_order_request_with_integrity() -> Value {
		Self::valid_order_request() // For now, same as valid order request
	}

	/// Large quote request with payload for middleware testing
	pub fn large_quote_request_with_payload(payload: Value) -> Value {
		let mut request = Self::valid_quote_request();
		request["largePayload"] = payload;
		request
	}
}

/// Assert that metadata is present and valid
pub fn assert_metadata_present_and_valid(response: &Value) {
	assert!(
		response.get("metadata").is_some(),
		"Metadata should be present in response"
	);
}

/// App state builder for test configuration
pub struct AppStateBuilder;

impl AppStateBuilder {
	pub fn new() -> Self {
		Self
	}

	pub async fn build() -> oif_aggregator::api::routes::AppState {
		ApiFixtures::test_app_state().await
	}

	pub async fn minimal() -> oif_aggregator::api::routes::AppState {
		ApiFixtures::test_app_state().await
	}
}
