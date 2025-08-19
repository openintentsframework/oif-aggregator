//! Test server for integration tests
//!
//! Provides different types of test servers for various testing scenarios

use axum::Router;
use oif_aggregator::{api::routes::create_router, AggregatorBuilder};
use tokio::task::JoinHandle;

use super::adapters::{MockDemoAdapter, TimingControlledAdapter};

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

		let mock_adapter = MockDemoAdapter::new();
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

	/// Spawn a test server with multiple timing-controlled adapters for sophisticated testing
	#[allow(dead_code)]
	pub async fn spawn_with_timing_controlled_adapters(
	) -> Result<(Self, Vec<TimingControlledAdapter>), Box<dyn std::error::Error>> {
		// Set required environment variable for tests
		std::env::set_var("INTEGRITY_SECRET", super::api_fixtures::INTEGRITY_SECRET);

		let mut settings = oif_config::Settings::default();
		settings.security.integrity_secret =
			oif_config::ConfigurableValue::from_env("INTEGRITY_SECRET");

		// Create multiple adapters with different timing characteristics
		let fast_adapter = TimingControlledAdapter::fast("fast");
		let slow_adapter = TimingControlledAdapter::slow("slow");
		let timeout_adapter = TimingControlledAdapter::timeout("timeout");
		let failing_adapter = TimingControlledAdapter::failing("failing");

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
