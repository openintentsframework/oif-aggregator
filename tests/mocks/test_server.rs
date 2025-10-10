//! Test server for integration tests
//!
//! Provides different types of test servers for various testing scenarios

use axum::Router;
use oif_aggregator::{api::routes::create_router, AggregatorBuilder};
use tokio::task::JoinHandle;

use super::{
	adapters::{
		create_failing_timing_adapter, create_fast_adapter, create_mock_adapter,
		create_slow_adapter, create_timeout_adapter, MockAdapter,
	},
	configs,
};
use oif_config::CircuitBreakerSettings;
use oif_types::{adapters::traits::SolverAdapter, Solver};

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
		std::env::set_var("INTEGRITY_SECRET", super::api_fixtures::INTEGRITY_SECRET);

		// Create minimal test settings without loading config that includes broken OIF adapter
		let mut settings = oif_config::Settings::default();
		settings.security.integrity_secret =
			oif_config::ConfigurableValue::from_env("INTEGRITY_SECRET");

		let mock_adapter = create_mock_adapter();
		let mock_solver = oif_aggregator::mocks::mock_solver();

		let (_router, state) = AggregatorBuilder::default()
			.with_settings(settings)
			.with_adapter(Box::new(mock_adapter))
			.with_solver(mock_solver)
			.start()
			.await?;

		let app: Router = create_router().with_state(state);

		Self::spawn_server_with_app(app).await
	}

	/// Spawn a test server with minimal configuration (no solvers)
	#[allow(dead_code)]
	pub async fn spawn_minimal() -> Result<Self, Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var("INTEGRITY_SECRET", super::api_fixtures::INTEGRITY_SECRET);

		let (_router, state) = AggregatorBuilder::default().start().await?;
		let app: Router = create_router().with_state(state);

		Self::spawn_server_with_app(app).await
	}

	/// Spawn a test server with circuit breaker configuration and specified adapters
	#[allow(dead_code)]
	pub async fn spawn_with_circuit_breaker_config(
		cb_config: CircuitBreakerSettings,
		adapters_and_solvers: Vec<(String, impl SolverAdapter + 'static)>,
	) -> Result<Self, Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var("INTEGRITY_SECRET", super::api_fixtures::INTEGRITY_SECRET);

		let settings = configs::create_test_settings_with_circuit_breaker(cb_config);

		let mut builder = AggregatorBuilder::new().with_settings(settings);

		// Add each adapter and corresponding solver
		for (solver_id, adapter) in adapters_and_solvers {
			let adapter_id = adapter.adapter_info().adapter_id.clone();
			let mut solver =
				Solver::new(solver_id, adapter_id, "http://test.example.com".to_string());

			// Add test assets to make solver compatible with test fixtures
			// This matches what mock_solver() does to ensure compatibility
			use oif_types::models::Asset;
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
					"0xA0b86a33E6417a77C9A0C65f8E69b8B6e2B0C4A0".to_string(),
					"USDC".to_string(),
					"USD Coin".to_string(),
					6,
				)
				.expect("Valid USDC asset"),
			];
			solver = solver.with_assets(assets);

			builder = builder.with_adapter(Box::new(adapter)).with_solver(solver);
		}

		let (_router, state) = builder.start().await?;
		let app: Router = create_router().with_state(state);

		Self::spawn_server_with_app(app).await
	}

	/// Spawn a test server with multiple timing-controlled adapters for sophisticated testing
	#[allow(dead_code)]
	pub async fn spawn_with_timing_controlled_adapters(
	) -> Result<(Self, Vec<MockAdapter>), Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var("INTEGRITY_SECRET", super::api_fixtures::INTEGRITY_SECRET);

		let mut settings = oif_config::Settings::default();
		// Ensure aggregation settings exist and set include_unknown_compatibility for tests
		if let Some(ref mut agg) = settings.aggregation {
			agg.include_unknown_compatibility = Some(true);
		} else {
			settings.aggregation = Some(oif_config::AggregationSettings {
				global_timeout_ms: None,
				per_solver_timeout_ms: None,
				max_concurrent_solvers: None,
				max_retries_per_solver: None,
				retry_delay_ms: None,
				include_unknown_compatibility: Some(true),
			});
		}

		// Explicitly disable circuit breaker for these tests to avoid interference
		settings.circuit_breaker = None;
		settings.security.integrity_secret =
			oif_config::ConfigurableValue::from_env("INTEGRITY_SECRET");

		// Create multiple adapters with different timing characteristics
		let fast_adapter = create_fast_adapter("fast");
		let slow_adapter = create_slow_adapter("slow");
		let timeout_adapter = create_timeout_adapter("timeout");
		let failing_adapter = create_failing_timing_adapter("failing");

		// Create test assets that match test fixtures for compatibility
		use oif_types::models::Asset;
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
				"0xA0b86a33E6417a77C9A0C65f8E69b8B6e2B0C4A0".to_string(),
				"USDC".to_string(),
				"USD Coin".to_string(),
				6,
			)
			.expect("Valid USDC asset"),
		];

		// Create corresponding solvers with assets
		let fast_solver = oif_types::Solver::new(
			"fast-solver".to_string(),
			"timing-fast".to_string(),
			"http://localhost:8080".to_string(),
		)
		.with_assets(assets.clone());

		let slow_solver = oif_types::Solver::new(
			"slow-solver".to_string(),
			"timing-slow".to_string(),
			"http://localhost:8081".to_string(),
		)
		.with_assets(assets.clone());

		let timeout_solver = oif_types::Solver::new(
			"timeout-solver".to_string(),
			"timing-timeout".to_string(),
			"http://localhost:8082".to_string(),
		)
		.with_assets(assets.clone());

		let failing_solver = oif_types::Solver::new(
			"failing-solver".to_string(),
			"timing-failing".to_string(),
			"http://localhost:8083".to_string(),
		)
		.with_assets(assets.clone());

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
		let server = Self::spawn_server_with_app(app).await?;

		Ok((server, adapters))
	}

	/// Common server spawning logic
	async fn spawn_server_with_app(app: Router) -> Result<Self, Box<dyn std::error::Error>> {
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

	#[allow(dead_code)]
	pub fn abort(self) {
		self.handle.abort();
	}
}
