//! End-to-end test utilities and shared fixtures

use axum::Router;
use oif_aggregator::{api::routes::create_router, AggregatorBuilder};

use tokio::task::JoinHandle;

pub mod timing_controlled_mocks;

// E2E test utilities - no longer need AppStateBuilder as we use AggregatorBuilder

/// Test server instance with configurable settings
pub struct TestServer {
	#[allow(dead_code)]
	pub base_url: String,
	pub handle: JoinHandle<()>,
}

impl TestServer {
	/// Spawn a test server with default settings
	#[allow(dead_code)]
	pub async fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
		Self::spawn_with_mock_adapter().await
	}

	/// Spawn a test server with mock adapter for testing
	pub async fn spawn_with_mock_adapter() -> Result<Self, Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var(
			"INTEGRITY_SECRET",
			"test-secret-for-e2e-tests-12345678901234567890",
		);

		// Create minimal test settings without loading config that includes broken OIF adapter
		let mut settings = oif_config::Settings::default();
		settings.security.integrity_secret =
			oif_config::ConfigurableValue::from_env("INTEGRITY_SECRET");

		let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
		let mock_solver = oif_aggregator::mocks::mock_solver();

		let (_router, state) = AggregatorBuilder::default()
			.with_settings(settings)
			.with_adapter(Box::new(mock_adapter))
			.with_solver(mock_solver)
			.start()
			.await?;

		let app: Router = create_router().with_state(state);

		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("bind test port");
		let addr = listener.local_addr().unwrap();
		let base_url = format!("http://{}:{}", addr.ip(), addr.port());

		let handle = tokio::spawn(async move {
			let _ = axum::serve(listener, app).await;
		});

		// Give server time to start
		tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

		Ok(Self { base_url, handle })
	}

	/// Spawn a test server with minimal configuration (no solvers)
	#[allow(dead_code)]
	pub async fn spawn_minimal() -> Result<Self, Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var(
			"INTEGRITY_SECRET",
			"test-secret-for-e2e-tests-12345678901234567890",
		);

		let (_router, state) = AggregatorBuilder::default().start().await?;
		let app: Router = create_router().with_state(state);

		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("bind test port");
		let addr = listener.local_addr().unwrap();
		let base_url = format!("http://{}:{}", addr.ip(), addr.port());

		let handle = tokio::spawn(async move {
			let _ = axum::serve(listener, app).await;
		});

		tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

		Ok(Self { base_url, handle })
	}

	/// Spawn a test server with multiple timing-controlled adapters for sophisticated testing
	#[allow(dead_code)]
	pub async fn spawn_with_timing_controlled_adapters() -> Result<
		(Self, Vec<timing_controlled_mocks::TimingControlledAdapter>),
		Box<dyn std::error::Error>,
	> {
		// Set required environment variable for tests
		std::env::set_var(
			"INTEGRITY_SECRET",
			"test-secret-for-e2e-tests-12345678901234567890",
		);

		let mut settings = oif_config::Settings::default();
		settings.security.integrity_secret =
			oif_config::ConfigurableValue::from_env("INTEGRITY_SECRET");

		// Create multiple adapters with different timing characteristics
		let fast_adapter = timing_controlled_mocks::TimingControlledAdapter::fast("fast");
		let slow_adapter = timing_controlled_mocks::TimingControlledAdapter::slow("slow");
		let timeout_adapter = timing_controlled_mocks::TimingControlledAdapter::timeout("timeout");
		let failing_adapter = timing_controlled_mocks::TimingControlledAdapter::failing("failing");

		// Create corresponding solvers
		let fast_solver = oif_types::Solver::new(
			"fast-solver".to_string(),
			"timing-fast".to_string(),
			"http://localhost:8080".to_string(),
			3000,
		);
		let slow_solver = oif_types::Solver::new(
			"slow-solver".to_string(),
			"timing-slow".to_string(),
			"http://localhost:8081".to_string(),
			3000,
		);
		let timeout_solver = oif_types::Solver::new(
			"timeout-solver".to_string(),
			"timing-timeout".to_string(),
			"http://localhost:8082".to_string(),
			3000,
		);
		let failing_solver = oif_types::Solver::new(
			"failing-solver".to_string(),
			"timing-failing".to_string(),
			"http://localhost:8083".to_string(),
			3000,
		);

		// Keep references to adapters for call tracking
		let adapters = vec![
			fast_adapter.clone(),
			slow_adapter.clone(),
			timeout_adapter.clone(),
			failing_adapter.clone(),
		];

		let (_router, state) = AggregatorBuilder::default()
			.with_settings(settings)
			.with_adapter(Box::new(fast_adapter))
			.with_adapter(Box::new(slow_adapter))
			.with_adapter(Box::new(timeout_adapter))
			.with_adapter(Box::new(failing_adapter))
			.with_solver(fast_solver)
			.with_solver(slow_solver)
			.with_solver(timeout_solver)
			.with_solver(failing_solver)
			.start()
			.await?;

		let app: Router = create_router().with_state(state);

		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("bind test port");
		let addr = listener.local_addr().unwrap();
		let base_url = format!("http://{}:{}", addr.ip(), addr.port());

		let handle = tokio::spawn(async move {
			let _ = axum::serve(listener, app).await;
		});

		// Give server time to start
		tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

		Ok((Self { base_url, handle }, adapters))
	}

	#[allow(dead_code)]
	pub fn abort(self) {
		self.handle.abort();
	}
}

/// Re-export API fixtures for backward compatibility
#[allow(dead_code)]
pub mod fixtures {
	use crate::mocks::api_fixtures::ApiFixtures;
	use oif_service::IntegrityService;
	use oif_types::{
		chrono, quotes::QuoteResponse, AvailableInput, Quote, QuoteDetails, QuoteOrder,
		RequestedOutput, SecretString, SignatureType, U256,
	};
	use oif_types::{serde_json::json, serde_json::Value, InteropAddress};

	/// Create a properly signed order request for testing with integrity verification
	pub fn valid_order_request_with_integrity() -> Value {
		// Use the EXACT same secret as the TestServer
		let test_secret = SecretString::from("test-secret-for-e2e-tests-12345678901234567890");
		let integrity_service = IntegrityService::new(test_secret);

		// Create test addresses
		let user_addr =
			InteropAddress::from_chain_and_address(1, "0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03")
				.unwrap();
		let eth_addr =
			InteropAddress::from_chain_and_address(1, "0x0000000000000000000000000000000000000000")
				.unwrap();
		let usdc_addr =
			InteropAddress::from_chain_and_address(1, "0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0")
				.unwrap();

		// Create quote details
		let details = QuoteDetails {
			available_inputs: vec![AvailableInput {
				user: user_addr.clone(),
				asset: eth_addr.clone(),
				amount: U256::new("1000000000000000000".to_string()), // 1 ETH
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				asset: usdc_addr.clone(),
				amount: U256::new("3000000000".to_string()), // 3000 USDC
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

	// Re-export key fixtures for backward compatibility
	pub fn valid_quote_request() -> Value {
		ApiFixtures::valid_quote_request()
	}

	pub fn invalid_quote_request_empty_token() -> Value {
		ApiFixtures::invalid_quote_request_missing_user()
	}

	pub fn valid_order_request_stateless() -> Value {
		ApiFixtures::valid_order_request()
	}

	pub fn invalid_order_request_missing_user() -> Value {
		ApiFixtures::invalid_order_request_missing_user()
	}

	pub fn invalid_order_request_missing_quote() -> Value {
		// Create a request missing quote data
		use oif_types::serde_json::json;
		let user_addr =
			InteropAddress::from_chain_and_address(1, "0x1234567890123456789012345678901234567890")
				.unwrap();

		json!({
			"userAddress": user_addr.to_hex()
			// Missing both quoteResponse and quoteId
		})
	}

	pub fn order_request_with_invalid_quote_id() -> Value {
		use oif_types::serde_json::json;
		let user_addr =
			InteropAddress::from_chain_and_address(1, "0x1234567890123456789012345678901234567890")
				.unwrap();

		json!({
			"userAddress": user_addr.to_hex(),
			"quoteId": "non-existent-quote-id"
		})
	}
}
