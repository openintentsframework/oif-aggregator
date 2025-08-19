//! API request/response fixtures for e2e and integration tests

use oif_aggregator::{api::routes::AppState, AggregatorBuilder};
use oif_service::IntegrityService;
use oif_types::{
	chrono,
	quotes::QuoteResponse,
	serde_json::{json, Value},
	AvailableInput, InteropAddress, Quote, QuoteDetails, QuoteOrder, RequestedOutput, SecretString,
	SignatureType, U256,
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
			"availableInputs": [
				{
					"user": user_addr.to_hex(),
					"asset": eth_addr.to_hex(),
					"amount": TestConstants::ONE_ETH_WEI,
					"lock": null
				}
			],
			"requestedOutputs": [
				{
					"asset": usdc_addr.to_hex(),
					"amount": TestConstants::TWO_THOUSAND_USDC,
					"receiver": user_addr.to_hex(),
					"calldata": null
				}
			],
			"minValidUntil": 300,
			"preference": null,
			"solverOptions": null
		})
	}

	/// Valid quote request with minimal fields
	pub fn minimal_quote_request() -> Value {
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();
		let eth_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS).unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS).unwrap();

		json!({
			"user": user_addr.to_hex(),
			"availableInputs": [
				{
					"user": user_addr.to_hex(),
					"asset": eth_addr.to_hex(),
					"amount": TestConstants::ONE_ETH_WEI,
					"lock": null
				}
			],
			"requestedOutputs": [
				{
					"asset": usdc_addr.to_hex(),
					"amount": TestConstants::TWO_THOUSAND_USDC,
					"receiver": user_addr.to_hex(),
					"calldata": null
				}
			]
		})
	}

	/// Invalid quote request - missing required field
	pub fn invalid_quote_request_missing_user() -> Value {
		let eth_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS).unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS).unwrap();

		json!({
			"availableInputs": [
				{
					"asset": eth_addr.to_hex(),
					"amount": TestConstants::ONE_ETH_WEI,
					"lock": null
				}
			],
			"requestedOutputs": [
				{
					"asset": usdc_addr.to_hex(),
					"amount": TestConstants::TWO_THOUSAND_USDC,
					"calldata": null
				}
			]
		})
	}

	/// Valid order request (with quote response)
	pub fn valid_order_request() -> Value {
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();

		json!({
			"userAddress": user_addr.to_hex(),
			"quoteResponse": {
				"quoteId": "test-quote-123",
				"solverId": "mock-demo-solver",
				"orders": [],
				"details": {
					"availableInputs": [],
					"requestedOutputs": []
				},
				"validUntil": 1700000000,
				"eta": 30,
				"provider": "Test Provider",
				"integrityChecksum": "test-checksum"
			},
			"signature": null
		})
	}

	/// Invalid order request - missing user address
	pub fn invalid_order_request_missing_user() -> Value {
		json!({
			"quoteResponse": {
				"quoteId": "test-quote-123"
			}
		})
	}

	/// Large payload for testing body size limits
	pub fn large_quote_request() -> Value {
		let large_payload = "x".repeat(2 * 1024 * 1024); // 2MB
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();

		json!({
			"user": user_addr.to_hex(),
			"availableInputs": [
				{
					"user": user_addr.to_hex(),
					"asset": large_payload,
					"amount": TestConstants::ONE_ETH_WEI,
					"lock": null
				}
			],
			"requestedOutputs": []
		})
	}

	/// Large quote request with custom payload for body size limit testing
	pub fn large_quote_request_with_payload(large_payload: String) -> Value {
		let user_addr =
			InteropAddress::from_chain_and_address(1, "0x1234567890123456789012345678901234567890")
				.unwrap();
		let eth_addr =
			InteropAddress::from_chain_and_address(1, "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
				.unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		json!({
			"user": user_addr.to_hex(),
			"availableInputs": [
				{
					"user": user_addr.to_hex(),
					"asset": eth_addr.to_hex(),
					"amount": "1000000000000000000",
					"lock": null,
					"largeData": large_payload  // Embed large payload here
				}
			],
			"requestedOutputs": [
				{
					"asset": usdc_addr.to_hex(),
					"amount": "2000000000",
					"receiver": user_addr.to_hex(),
					"calldata": null
				}
			],
			"minValidUntil": 300
		})
	}

	/// Malformed JSON string for testing error handling
	pub fn malformed_json() -> &'static str {
		"{ invalid json structure"
	}

	/// Create a properly signed order request for testing with integrity verification
	pub fn valid_order_request_with_integrity() -> Value {
		// Use the EXACT same secret as the TestServer
		let test_secret = SecretString::from(INTEGRITY_SECRET);
		let integrity_service = IntegrityService::new(test_secret);

		// Create test addresses using constants
		let user_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::TEST_USER_ADDRESS).unwrap();
		let eth_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::WETH_ADDRESS).unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, TestConstants::USDC_ADDRESS).unwrap();

		// Create quote details
		let details = QuoteDetails {
			available_inputs: vec![AvailableInput {
				user: user_addr.clone(),
				asset: eth_addr.clone(),
				amount: U256::new(TestConstants::ONE_ETH_WEI.to_string()), // 1 ETH
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				asset: usdc_addr.clone(),
				amount: U256::new(TestConstants::TWO_THOUSAND_USDC.to_string()), // 2000 USDC
				receiver: user_addr.clone(),
				calldata: None,
			}],
		};

		// Create quote order
		let quote_order = QuoteOrder {
			signature_type: SignatureType::Eip712,
			domain: user_addr.clone(),
			primary_type: "Order".to_string(),
			message: json!({
				"orderType": "swap",
				"inputAsset": eth_addr.to_hex(),
				"outputAsset": usdc_addr.to_hex(),
				"mockProvider": "Mock Demo Adapter"
			}),
		};

		// Create quote (without checksum first)
		let mut quote = Quote::new(
			"mock-demo-solver".to_string(), // Match the solver ID used in TestServer
			vec![quote_order],
			details,
			"Mock Demo Adapter Provider".to_string(), // Match MockDemoAdapter format
			String::new(),                            // Empty checksum initially
		);

		// Set other fields
		quote.valid_until = Some(chrono::Utc::now().timestamp() as u64 + 300); // 5 minutes from now
		quote.eta = Some(30);

		// Generate the integrity checksum
		let integrity_checksum = integrity_service.generate_checksum(&quote).unwrap();
		quote.integrity_checksum = integrity_checksum;

		// Convert to QuoteResponse
		let quote_response = QuoteResponse::try_from(quote).unwrap();

		// Create OrderRequest
		json!({
			"userAddress": user_addr.to_hex(),
			"quoteResponse": quote_response
		})
	}

	pub fn invalid_order_request_missing_quote() -> Value {
		// Create a request missing quote data
		let user_addr =
			InteropAddress::from_chain_and_address(1, "0x1234567890123456789012345678901234567890")
				.unwrap();

		json!({
			"userAddress": user_addr.to_hex()
			// Missing both quoteResponse and quoteId
		})
	}

	pub fn order_request_with_invalid_quote_id() -> Value {
		let user_addr =
			InteropAddress::from_chain_and_address(1, "0x1234567890123456789012345678901234567890")
				.unwrap();

		json!({
			"userAddress": user_addr.to_hex(),
			"quoteId": "non-existent-quote-id"
		})
	}
}

/// Application state builders for tests
#[allow(dead_code)]
pub struct AppStateBuilder;

#[allow(dead_code)]
impl AppStateBuilder {
	/// Create minimal test app state with dependencies
	pub async fn minimal() -> Result<AppState, Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var("INTEGRITY_SECRET", INTEGRITY_SECRET);

		let (_app, state) = AggregatorBuilder::default().start().await?;
		Ok(state)
	}

	/// Create app state with mock solvers
	pub async fn with_mock_solvers(
		_solver_count: usize,
	) -> Result<AppState, Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var("INTEGRITY_SECRET", INTEGRITY_SECRET);

		// Create minimal test settings without loading config that includes broken OIF adapter
		let mut settings = oif_config::Settings::default();
		settings.security.integrity_secret =
			oif_config::ConfigurableValue::from_env("INTEGRITY_SECRET");

		// Use the correct mock solver that matches the MockDemoAdapter ID
		let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
		let mock_solver = oif_aggregator::mocks::mock_solver();

		let (_app, state) = AggregatorBuilder::default()
			.with_settings(settings)
			.with_adapter(Box::new(mock_adapter))
			.with_solver(mock_solver)
			.start()
			.await?;
		Ok(state)
	}

	/// Create app state for testing with custom settings
	pub async fn with_custom_settings() -> Result<AppState, Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var("INTEGRITY_SECRET", INTEGRITY_SECRET);

		let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
		let mock_solver = oif_aggregator::mocks::mock_solver();

		let (_app, state) = AggregatorBuilder::default()
			.with_adapter(Box::new(mock_adapter))
			.with_solver(mock_solver)
			.start()
			.await?;
		Ok(state)
	}
}

/// Helper function to validate metadata in successful responses
#[allow(dead_code)]
pub fn assert_metadata_present_and_valid(body: &serde_json::Value) {
	let metadata = body["metadata"]
		.as_object()
		.expect("Metadata should be present in response");

	// Verify all required metadata fields are present
	assert!(metadata.contains_key("totalDurationMs"));
	assert!(metadata.contains_key("solverTimeoutMs"));
	assert!(metadata.contains_key("globalTimeoutMs"));
	assert!(metadata.contains_key("earlyTermination"));
	assert!(metadata.contains_key("totalSolversAvailable"));
	assert!(metadata.contains_key("solversQueried"));
	assert!(metadata.contains_key("solversRespondedSuccess"));
	assert!(metadata.contains_key("solversRespondedError"));
	assert!(metadata.contains_key("solversTimedOut"));
	assert!(metadata.contains_key("minQuotesRequired"));
	assert!(metadata.contains_key("solverSelectionMode"));

	// Verify basic constraints
	let total_available = metadata["totalSolversAvailable"].as_u64().unwrap();
	let queried = metadata["solversQueried"].as_u64().unwrap();
	let success = metadata["solversRespondedSuccess"].as_u64().unwrap();
	let error = metadata["solversRespondedError"].as_u64().unwrap();
	let timeout = metadata["solversTimedOut"].as_u64().unwrap();

	assert!(
		queried <= total_available,
		"Queried solvers cannot exceed available solvers"
	);
	assert!(
		success + error + timeout <= queried,
		"Response counts cannot exceed queried solvers"
	);
}
