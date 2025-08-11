//! Tests for the Builder Pattern implementation

use oif_aggregator::{config::Settings, storage::MemoryStore, AggregatorBuilder, Solver};
use oif_types::Network;

use std::collections::HashMap;

/// Create a minimal test configuration
fn create_test_settings() -> Settings {
	use oif_aggregator::config::settings::*;

	Settings {
		server: ServerSettings {
			host: "127.0.0.1".to_string(),
			port: 3001, // Different port for testing
			workers: None,
		},
		solvers: HashMap::new(), // Empty for testing
		timeouts: TimeoutSettings {
			per_solver_ms: 2000,
			global_ms: 5000,
			request_ms: 1000,
		},
		environment: EnvironmentSettings {
			profile: EnvironmentProfile::Development,
			debug: true,
			metrics_enabled: false,
			rate_limiting: RateLimitSettings {
				enabled: false,
				requests_per_minute: 60,
				burst_size: 10,
			},
		},
		logging: LoggingSettings {
			level: "debug".to_string(),
			format: LogFormat::Compact,
			structured: false,
		},
	}
}

/// Create a test solver
fn create_test_solver() -> Solver {
	use oif_aggregator::SolverStatus;

	let mut solver = Solver::new(
		"test-solver".to_string(),
		"test-adapter".to_string(),
		"http://localhost:8080".to_string(),
		1000,
	);
	solver.metadata.name = Some("Test Solver".to_string());
	solver.metadata.description = Some("A test solver for unit tests".to_string());
	solver.metadata.version = Some("1.0.0".to_string());
	solver.metadata.supported_networks = vec![
		Network::new(1, "Ethereum".to_string(), false),
		Network::new(137, "Polygon".to_string(), false),
	];
	solver.metadata.max_retries = 3;
	solver.status = SolverStatus::Active;
	solver
}

#[tokio::test]
async fn test_builder_new() {
	let builder = AggregatorBuilder::new();
	assert!(builder.settings().is_none());
}

#[tokio::test]
async fn test_builder_default() {
	let builder = AggregatorBuilder::new();
	assert!(builder.settings().is_none());
}

#[tokio::test]
async fn test_builder_from_config() {
	let settings = create_test_settings();
	let builder = AggregatorBuilder::from_config(settings.clone()).await;

	assert!(builder.settings().is_some());
	assert_eq!(builder.settings().unwrap().server.port, 3001);
}

#[tokio::test]
async fn test_builder_with_solver() {
	let solver = create_test_solver();
	let builder = AggregatorBuilder::new().with_solver(solver.clone()).await;

	// Create router to verify solver was added
	let (router, app_state) = builder.start().await.unwrap();
	let stats = app_state.aggregator_service.get_stats();

	assert_eq!(stats.total_solvers, 1);
	// Just verify router creation succeeded
	drop(router);
}

#[tokio::test]
async fn test_builder_with_storage() {
	let custom_storage = MemoryStore::new();
	let builder = AggregatorBuilder::with_storage(custom_storage);

	// Test that we can create the app with custom storage
	let result = builder.start().await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_builder_chaining() {
	let solver = create_test_solver();
	let builder = AggregatorBuilder::new().with_solver(solver).await;

	// Test that method chaining works correctly
	let result = builder.start().await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_builder_storage_switching() {
	let solver = create_test_solver();
	let custom_storage = MemoryStore::new();

	// Test switching storage and adding solver
	let builder = AggregatorBuilder::with_storage(custom_storage)
		.with_solver(solver)
		.await;

	let (router, app_state) = builder.start().await.unwrap();
	let stats = app_state.aggregator_service.get_stats();

	assert_eq!(stats.total_solvers, 1);
	// Just verify router creation succeeded
	drop(router);
}

#[tokio::test]
async fn test_builder_with_config_and_solver() {
	let settings = create_test_settings();
	let solver = create_test_solver();

	let builder = AggregatorBuilder::from_config(settings)
		.await
		.with_solver(solver)
		.await;

	let (router, app_state) = builder.start().await.unwrap();
	let stats = app_state.aggregator_service.get_stats();

	// Should have solvers from config + manually added solver
	assert!(stats.total_solvers >= 1);
	// Just verify router creation succeeded
	drop(router);
}

#[tokio::test]
async fn test_builder_start_creates_router() {
	let builder = AggregatorBuilder::new();
	let result = builder.start().await;

	assert!(result.is_ok());
	let (router, app_state) = result.unwrap();

	// Verify router is functional
	// Just verify router creation succeeded
	drop(router);

	// Verify app state is properly initialized
	let stats = app_state.aggregator_service.get_stats();
	assert_eq!(stats.total_solvers, 0); // No solvers added
}

#[tokio::test]
async fn test_builder_with_multiple_solvers() {
	let solver1 = {
		let mut solver = create_test_solver();
		solver.solver_id = "solver-1".to_string();
		solver
	};

	let solver2 = {
		let mut solver = create_test_solver();
		solver.solver_id = "solver-2".to_string();
		solver.endpoint = "http://localhost:8081".to_string();
		solver
	};

	let builder = AggregatorBuilder::new()
		.with_solver(solver1)
		.await
		.with_solver(solver2)
		.await;

	let (_, app_state) = builder.start().await.unwrap();
	let stats = app_state.aggregator_service.get_stats();

	assert_eq!(stats.total_solvers, 2);
}

#[tokio::test]
async fn test_builder_defaults_handling() {
	// Test that builder works with just defaults
	let builder = AggregatorBuilder::new();
	let result = builder.start().await;

	assert!(result.is_ok());
	let (_, app_state) = result.unwrap();

	// Should work even with no configuration
	let stats = app_state.aggregator_service.get_stats();
	assert_eq!(stats.total_solvers, 0);
	assert_eq!(stats.initialized_adapters, 0);
}

#[tokio::test]
async fn test_builder_settings_access() {
	let settings = create_test_settings();
	let builder = AggregatorBuilder::from_config(settings.clone()).await;

	assert!(builder.settings().is_some());
	assert_eq!(
		builder.settings().unwrap().server.port,
		settings.server.port
	);

	let builder_no_settings = AggregatorBuilder::new();
	assert!(builder_no_settings.settings().is_none());
}
