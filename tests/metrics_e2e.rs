//! End-to-end integration tests for metrics collection and processing
//!
//! Tests the complete metrics pipeline:
//! 1. MetricsUpdateHandler processes solver metrics data
//! 2. Data is stored in both SolverMetrics and MetricsTimeSeries
//! 3. Cleanup jobs work correctly with the stored data

use chrono::{Duration, Utc};
use oif_service::jobs::{
	handlers::{MetricsCleanupHandler, MetricsUpdateHandler},
	types::SolverMetricsUpdate,
};
use oif_storage::{traits::MetricsStorage, MemoryStore, Storage};
use oif_types::solvers::Solver;
use std::sync::Arc;

mod mocks;
use mocks::configs::MockConfigs;

/// Integration test helper to set up complete metrics testing environment
struct MetricsTestEnvironment {
	pub storage: Arc<MemoryStore>,
	pub metrics_handler: MetricsUpdateHandler,
	pub cleanup_handler: MetricsCleanupHandler,
}

impl MetricsTestEnvironment {
	/// Create a complete test environment with metrics collection enabled
	async fn new() -> Result<Self, Box<dyn std::error::Error>> {
		// Create test settings with metrics enabled
		let mut settings = MockConfigs::test_settings();
		settings.metrics = Some(oif_config::settings::MetricsSettings {
			collection_enabled: true,
			retention_hours: 24,             // 24 hour retention for testing
			cleanup_interval_hours: 1,       // 1 hour cleanup interval
			aggregation_interval_minutes: 5, // 5 minute aggregation
		});

		// Create storage
		let storage = Arc::new(MemoryStore::new());

		// Create handlers
		let metrics_handler = MetricsUpdateHandler::new(storage.clone());
		let cleanup_handler = MetricsCleanupHandler::new(storage.clone(), settings.clone());

		Ok(Self {
			storage,
			metrics_handler,
			cleanup_handler,
		})
	}

	/// Create and store test solvers
	async fn setup_test_solvers(&self) -> Result<Vec<Solver>, Box<dyn std::error::Error>> {
		let solver_configs = vec![
			("solver-1", "adapter-1", "http://solver1.test.com"),
			("solver-2", "adapter-2", "http://solver2.test.com"),
			("solver-3", "adapter-3", "http://solver3.test.com"),
		];

		let mut solvers = Vec::new();
		for (id, adapter, endpoint) in solver_configs {
			let solver = Solver::new(id.to_string(), adapter.to_string(), endpoint.to_string());
			self.storage.create_solver(solver.clone()).await?;
			solvers.push(solver);
		}

		Ok(solvers)
	}

	/// Simulate aggregation metrics data
	fn create_test_metrics_data(
		&self,
		successful: bool,
		response_time_ms: u64,
	) -> SolverMetricsUpdate {
		SolverMetricsUpdate {
			response_time_ms,
			was_successful: successful,
			was_timeout: false,
			timestamp: Utc::now(),
			error_message: None,
			status_code: None,
			error_type: None,
		}
	}
}

#[tokio::test]
async fn test_end_to_end_metrics_collection_flow() {
	let env = MetricsTestEnvironment::new().await.unwrap();
	let solvers = env.setup_test_solvers().await.unwrap();

	// Step 1: Schedule metrics update job (simulating what AggregatorService would do)
	// Step 2: Process the metrics update job
	let result = env
		.metrics_handler
		.handle_aggregation_metrics_update(
			"test-aggregation-1",
			vec![
				(
					"solver-1".to_string(),
					env.create_test_metrics_data(true, 150),
				),
				(
					"solver-2".to_string(),
					env.create_test_metrics_data(true, 200),
				),
				(
					"solver-3".to_string(),
					env.create_test_metrics_data(false, 5000),
				),
			],
		)
		.await;

	assert!(result.is_ok(), "Metrics update job should succeed");

	// Step 3: Verify solver metrics were updated
	for solver in &solvers {
		let updated_solver = env
			.storage
			.get_solver(&solver.solver_id)
			.await
			.unwrap()
			.unwrap();

		// Check that request count increased
		assert!(
			updated_solver.metrics.total_requests > 0,
			"Solver {} should have updated request count",
			solver.solver_id
		);

		// Check last_seen was updated
		assert!(
			updated_solver.last_seen.is_some(),
			"Solver {} should have updated last_seen",
			solver.solver_id
		);
	}

	// Step 4: Verify time-series metrics were created
	for solver in &solvers {
		let timeseries = env
			.storage
			.get_metrics_timeseries(&solver.solver_id)
			.await
			.unwrap();

		assert!(
			timeseries.is_some(),
			"Time-series should be created for solver {}",
			solver.solver_id
		);

		let ts = timeseries.unwrap();
		assert_eq!(ts.solver_id, solver.solver_id);

		// Check that buckets have data
		assert!(
			!ts.five_minute_buckets.aggregates.is_empty(),
			"5-minute buckets should have data for solver {}",
			solver.solver_id
		);
	}
}

#[tokio::test]
async fn test_parallel_metrics_processing_performance() {
	let env = MetricsTestEnvironment::new().await.unwrap();

	// Create many solvers to test parallel processing
	let solver_count = 20;
	let mut solvers = Vec::new();

	for i in 0..solver_count {
		let solver = Solver::new(
			format!("solver-{}", i),
			format!("adapter-{}", i),
			format!("http://solver{}.test.com", i),
		);
		env.storage.create_solver(solver.clone()).await.unwrap();
		solvers.push(solver);
	}

	// Create metrics data for all solvers
	let solver_metrics: Vec<(String, SolverMetricsUpdate)> = solvers
		.iter()
		.enumerate()
		.map(|(i, solver)| {
			(
				solver.solver_id.clone(),
				env.create_test_metrics_data(i % 3 != 0, 100 + (i as u64 * 10)),
			)
		})
		.collect();

	// Measure processing time
	let start_time = std::time::Instant::now();

	let result = env
		.metrics_handler
		.handle_aggregation_metrics_update("perf-test-aggregation", solver_metrics)
		.await;

	let elapsed = start_time.elapsed();

	assert!(result.is_ok(), "Parallel metrics processing should succeed");

	// Verify all solvers were processed
	for solver in &solvers {
		let updated_solver = env
			.storage
			.get_solver(&solver.solver_id)
			.await
			.unwrap()
			.unwrap();

		assert!(
			updated_solver.metrics.total_requests > 0,
			"Solver {} should have been processed",
			solver.solver_id
		);

		// Verify time-series data exists
		let timeseries = env
			.storage
			.get_metrics_timeseries(&solver.solver_id)
			.await
			.unwrap();
		assert!(
			timeseries.is_some(),
			"Time-series should exist for solver {}",
			solver.solver_id
		);
	}

	println!("Processed {} solvers in {:?}", solver_count, elapsed);

	// Performance assertion: should complete much faster than sequential processing
	// Sequential would be ~2 seconds (20 * 100ms), parallel should be much faster
	assert!(
		elapsed.as_millis() < 1000,
		"Parallel processing should complete in under 1 second, took {:?}",
		elapsed
	);
}

#[tokio::test]
async fn test_metrics_cleanup_integration() {
	let env = MetricsTestEnvironment::new().await.unwrap();
	let solvers = env.setup_test_solvers().await.unwrap();

	// Create initial metrics data
	for (i, solver) in solvers.iter().enumerate() {
		let metrics_data = env.create_test_metrics_data(true, 150);
		let result = env
			.metrics_handler
			.handle_aggregation_metrics_update(
				&format!("initial-agg-{}", i),
				vec![(solver.solver_id.clone(), metrics_data)],
			)
			.await;
		assert!(result.is_ok());
	}

	// Verify initial data exists
	let initial_count = env.storage.count_metrics_timeseries().await.unwrap();
	assert!(initial_count >= solvers.len());

	// Simulate old data by creating metrics with old timestamps
	for solver in &solvers {
		let mut old_timeseries = env
			.storage
			.get_metrics_timeseries(&solver.solver_id)
			.await
			.unwrap()
			.unwrap();

		// Make the time-series appear old by backdating its last update
		old_timeseries.last_updated = Utc::now() - Duration::hours(48); // 48 hours ago

		env.storage
			.update_metrics_timeseries(&solver.solver_id, old_timeseries)
			.await
			.unwrap();
	}

	// Run cleanup
	let cleanup_result = env.cleanup_handler.handle_metrics_cleanup().await;
	assert!(cleanup_result.is_ok(), "Cleanup should succeed");

	// Verify old data was cleaned up (with 24-hour retention)
	let final_count = env.storage.count_metrics_timeseries().await.unwrap();
	assert_eq!(
		final_count, 0,
		"All old time-series should have been cleaned up"
	);
}

#[tokio::test]
async fn test_metrics_error_handling_and_partial_success() {
	let env = MetricsTestEnvironment::new().await.unwrap();
	let valid_solvers = env.setup_test_solvers().await.unwrap();

	// Create metrics update with mix of valid and invalid solvers
	let mixed_metrics = vec![
		(
			valid_solvers[0].solver_id.clone(),
			env.create_test_metrics_data(true, 100),
		),
		(
			"nonexistent-solver".to_string(),
			env.create_test_metrics_data(true, 200),
		),
		(
			valid_solvers[1].solver_id.clone(),
			env.create_test_metrics_data(false, 5000),
		),
	];

	// Process the mixed batch
	let result = env
		.metrics_handler
		.handle_aggregation_metrics_update("mixed-aggregation", mixed_metrics)
		.await;

	// Should still succeed partially (not fail completely)
	// The handler processes each solver independently in parallel
	assert!(
		result.is_ok(),
		"Partial success should be handled gracefully"
	);

	// Verify valid solvers were processed
	for solver in &valid_solvers[..2] {
		let updated_solver = env
			.storage
			.get_solver(&solver.solver_id)
			.await
			.unwrap()
			.unwrap();

		assert!(
			updated_solver.metrics.total_requests > 0,
			"Valid solver {} should have been processed",
			solver.solver_id
		);
	}

	// Verify nonexistent solver didn't create spurious data
	let nonexistent_timeseries = env
		.storage
		.get_metrics_timeseries("nonexistent-solver")
		.await
		.unwrap();
	assert!(
		nonexistent_timeseries.is_none(),
		"Nonexistent solver should not have time-series data"
	);
}

#[tokio::test]
async fn test_metrics_storage_integration() {
	// Test integration between different storage methods
	let env = MetricsTestEnvironment::new().await.unwrap();
	let _solvers = env.setup_test_solvers().await.unwrap();

	// Verify that storage layer provides the expected interface
	let storage_count = env.storage.count_metrics_timeseries().await.unwrap();
	assert_eq!(storage_count, 0, "Should start with no time-series data");

	let solver_ids = env.storage.list_solvers_with_metrics().await.unwrap();
	assert!(
		solver_ids.is_empty(),
		"Should start with no solvers having metrics"
	);
}

#[tokio::test]
async fn test_rolling_metrics_calculation_integration() {
	let env = MetricsTestEnvironment::new().await.unwrap();
	let solvers = env.setup_test_solvers().await.unwrap();

	// Create multiple metrics data points over time
	for solver in &solvers {
		for i in 0..10 {
			let success = i % 3 != 0; // Mix of successes and failures
			let response_time = 100 + (i * 50); // Varying response times
			let is_timeout = i % 7 == 0; // Occasional timeouts
			let metrics_data = SolverMetricsUpdate {
				response_time_ms: response_time,
				was_successful: success,
				was_timeout: is_timeout,
				timestamp: Utc::now() - Duration::minutes(i as i64 * 2), // Spread over time
				error_message: if success {
					None
				} else {
					Some("Test error".to_string())
				},
				status_code: None,
				error_type: if success {
					None
				} else if is_timeout {
					Some(oif_types::ErrorType::ServiceError)
				} else {
					Some(oif_types::ErrorType::Unknown)
				},
			};

			let result = env
				.metrics_handler
				.handle_aggregation_metrics_update(
					&format!("rolling-agg-{}-{}", solver.solver_id, i),
					vec![(solver.solver_id.clone(), metrics_data)],
				)
				.await;
			assert!(result.is_ok());
		}
	}

	// Verify rolling metrics were calculated
	for solver in &solvers {
		let timeseries = env
			.storage
			.get_metrics_timeseries(&solver.solver_id)
			.await
			.unwrap()
			.unwrap();

		// Check that rolling metrics have reasonable values
		let rolling = &timeseries.rolling_metrics;

		if let Some(ref last_hour) = rolling.last_hour {
			assert!(
				last_hour.avg_response_time_ms > 0.0,
				"Rolling average should be calculated for solver {}",
				solver.solver_id
			);

			assert!(
				last_hour.success_rate >= 0.0 && last_hour.success_rate <= 1.0,
				"Success rate should be between 0 and 1 for solver {}",
				solver.solver_id
			);

			assert!(
				last_hour.total_requests > 0,
				"Total requests should be positive for solver {}",
				solver.solver_id
			);
		} else {
			panic!(
				"Rolling metrics should be available for solver {}",
				solver.solver_id
			);
		}

		// Verify different granularities have data
		assert!(
			!timeseries.five_minute_buckets.aggregates.is_empty(),
			"5-minute buckets should have data"
		);
		assert!(
			!timeseries.fifteen_minute_buckets.aggregates.is_empty(),
			"15-minute buckets should have data"
		);
	}
}
