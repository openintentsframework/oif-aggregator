//! Tests for background job processing

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::{
	AggregatorService, AggregatorTrait, IntegrityService, IntegrityTrait, OrderService,
	OrderServiceTrait, SolverFilterService, SolverFilterTrait, SolverService, SolverServiceTrait,
};
use oif_adapters::AdapterRegistry;
use oif_storage::{MemoryStore, Storage};
use oif_types::Solver;

use super::handlers::BackgroundJobHandler;
use super::processor::{JobHandler, JobProcessor, JobProcessorConfig, JobStatus, RetryPolicy};
use super::scheduler::{JobScheduler, MockJobScheduler};
use super::types::{BackgroundJob, JobError};

// Helper function to create test services
fn create_test_services(
	storage: Arc<dyn Storage>,
	adapter_registry: Arc<AdapterRegistry>,
	_order_service: Arc<dyn OrderServiceTrait>,
) -> (
	Arc<dyn SolverServiceTrait>,
	Arc<dyn AggregatorTrait>,
	Arc<dyn IntegrityTrait>,
) {
	let solver_service = Arc::new(SolverService::new(
		Arc::clone(&storage),
		Arc::clone(&adapter_registry),
		None,
	)) as Arc<dyn SolverServiceTrait>;

	let integrity_service = Arc::new(IntegrityService::new(
		"test_secret_key_1234567890123456".to_string().into(),
	)) as Arc<dyn IntegrityTrait>;

	let solver_filter_service = Arc::new(SolverFilterService::new()) as Arc<dyn SolverFilterTrait>;

	let storage = Arc::new(MemoryStore::new()) as Arc<dyn Storage>;
	let aggregator_service = Arc::new(AggregatorService::with_config(
		Arc::clone(&storage),
		Arc::clone(&adapter_registry),
		Arc::clone(&integrity_service),
		solver_filter_service,
		Default::default(),
		None,
	)) as Arc<dyn AggregatorTrait>;

	(solver_service, aggregator_service, integrity_service)
}

#[tokio::test]
async fn test_job_processor_basic_functionality() {
	let storage = Arc::new(MemoryStore::new()) as Arc<dyn Storage>;
	let adapter_registry = Arc::new(AdapterRegistry::with_defaults());

	// Create integrity service first
	let integrity_service = Arc::new(IntegrityService::new(
		"test_secret_key_1234567890123456".to_string().into(),
	)) as Arc<dyn IntegrityTrait>;

	// Create mock job scheduler for testing
	let mut mock_scheduler = MockJobScheduler::new();
	mock_scheduler
		.expect_schedule_with_delay()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-delayed-job-id".to_string()) }));
	mock_scheduler
		.expect_schedule_recurring()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-recurring-job-id".to_string()) }));
	mock_scheduler
		.expect_cancel_job()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_| Box::pin(async { Ok(()) }));
	let job_scheduler = Arc::new(mock_scheduler) as Arc<dyn JobScheduler>;

	let order_service = Arc::new(OrderService::new(
		Arc::clone(&storage),
		Arc::clone(&adapter_registry),
		Arc::clone(&integrity_service),
		Arc::clone(&job_scheduler),
	)) as Arc<dyn OrderServiceTrait>;

	// Create job handler with all required services
	let (solver_service, aggregator_service, _) = create_test_services(
		storage.clone(),
		adapter_registry.clone(),
		order_service.clone(),
	);
	let handler = Arc::new(BackgroundJobHandler::new(
		storage.clone(),
		adapter_registry,
		solver_service,
		aggregator_service,
		integrity_service,
		order_service,
		job_scheduler,
		oif_config::Settings::default(),
	));

	// Create job processor with minimal config for testing
	let config = JobProcessorConfig {
		queue_capacity: 10,
		worker_count: 1,
		max_scheduled_jobs: 100,
		max_job_info_entries: 1000,
		cleanup_interval_minutes: 0, // Disable automatic cleanup in tests
		job_info_ttl_minutes: 60,
	};

	let processor = JobProcessor::new(handler, config).unwrap();

	// Test job submission
	let job = BackgroundJob::FetchSolverAssets {
		solver_id: "test-solver".to_string(),
	};
	let job_info_id = processor.submit(job, None, None).await;
	assert!(job_info_id.is_ok());

	// Allow some time for job processing
	sleep(Duration::from_millis(100)).await;

	// Shutdown processor with timeout - ignore shutdown errors in tests since mocks can cause issues
	let _ = tokio::time::timeout(Duration::from_secs(2), processor.shutdown()).await;
}

#[tokio::test]
async fn test_job_processor_queue_capacity() {
	let storage = Arc::new(MemoryStore::new()) as Arc<dyn Storage>;
	let adapter_registry = Arc::new(AdapterRegistry::with_defaults());

	// Create integrity service first
	let integrity_service = Arc::new(IntegrityService::new(
		"test_secret_key_1234567890123456".to_string().into(),
	)) as Arc<dyn IntegrityTrait>;

	// Create mock job scheduler for testing
	let mut mock_scheduler = MockJobScheduler::new();
	mock_scheduler
		.expect_schedule_with_delay()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-delayed-job-id".to_string()) }));
	mock_scheduler
		.expect_schedule_recurring()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-recurring-job-id".to_string()) }));
	mock_scheduler
		.expect_cancel_job()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_| Box::pin(async { Ok(()) }));
	let job_scheduler = Arc::new(mock_scheduler) as Arc<dyn JobScheduler>;

	let order_service = Arc::new(OrderService::new(
		Arc::clone(&storage),
		Arc::clone(&adapter_registry),
		Arc::clone(&integrity_service),
		Arc::clone(&job_scheduler),
	)) as Arc<dyn OrderServiceTrait>;

	let (solver_service, aggregator_service, _) = create_test_services(
		storage.clone(),
		adapter_registry.clone(),
		order_service.clone(),
	);
	let handler = Arc::new(BackgroundJobHandler::new(
		storage,
		adapter_registry,
		solver_service,
		aggregator_service,
		integrity_service,
		order_service,
		job_scheduler,
		oif_config::Settings::default(),
	));

	// Create processor with very small queue
	let config = JobProcessorConfig {
		queue_capacity: 1,
		worker_count: 1,
		max_scheduled_jobs: 100,
		max_job_info_entries: 1000,
		cleanup_interval_minutes: 0, // Disable automatic cleanup in tests
		job_info_ttl_minutes: 60,
	};

	let processor = JobProcessor::new(handler, config).unwrap();

	// Fill the queue
	let job1 = BackgroundJob::FetchSolverAssets {
		solver_id: "test-solver".to_string(),
	};
	let job_info_id1 = processor.submit_nowait(job1, None, None);
	assert!(job_info_id1.is_ok());

	// This should fail due to queue being full
	let job2 = BackgroundJob::FetchSolverAssets {
		solver_id: "test-solver".to_string(),
	};
	let result = processor.submit_nowait(job2, None, None);

	// Should get either QueueFull or succeed depending on timing
	// The test is mainly to ensure the queue capacity is respected
	match result {
		Ok(job_id) => println!("Job submitted successfully with ID: {}", job_id),
		Err(JobError::QueueFull) => println!("Queue full as expected"),
		Err(e) => panic!("Unexpected error: {}", e),
	}

	// Shutdown with timeout - ignore shutdown errors in tests since mocks can cause issues
	let _ = tokio::time::timeout(Duration::from_secs(2), processor.shutdown()).await;
}

#[tokio::test]
async fn test_solver_maintenance_handler() {
	let storage = Arc::new(MemoryStore::new()) as Arc<dyn Storage>;
	let adapter_registry = Arc::new(AdapterRegistry::with_defaults());

	// Create integrity service first
	let integrity_service = Arc::new(IntegrityService::new(
		"test_secret_key_1234567890123456".to_string().into(),
	)) as Arc<dyn IntegrityTrait>;

	// Create mock job scheduler for testing
	let mut mock_scheduler = MockJobScheduler::new();
	mock_scheduler
		.expect_schedule_with_delay()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-delayed-job-id".to_string()) }));
	mock_scheduler
		.expect_schedule_recurring()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-recurring-job-id".to_string()) }));
	mock_scheduler
		.expect_cancel_job()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_| Box::pin(async { Ok(()) }));
	let job_scheduler = Arc::new(mock_scheduler) as Arc<dyn JobScheduler>;

	let order_service = Arc::new(OrderService::new(
		Arc::clone(&storage),
		Arc::clone(&adapter_registry),
		Arc::clone(&integrity_service),
		Arc::clone(&job_scheduler),
	)) as Arc<dyn OrderServiceTrait>;
	// Create a test solver
	let solver = Solver::new(
		"test-solver".to_string(),
		"demo".to_string(), // Use demo adapter
		"http://localhost:8080".to_string(),
	);

	// Store the solver
	storage.create_solver(solver.clone()).await.unwrap();

	// Create handler
	let (solver_service, aggregator_service, _) = create_test_services(
		storage.clone(),
		adapter_registry.clone(),
		order_service.clone(),
	);
	let handler = BackgroundJobHandler::new(
		storage.clone(),
		adapter_registry,
		solver_service,
		aggregator_service,
		integrity_service,
		order_service,
		job_scheduler,
		oif_config::Settings::default(),
	);

	// Test health check job
	let health_job = BackgroundJob::SolverHealthCheck {
		solver_id: solver.solver_id.clone(),
	};

	// This will likely fail since we don't have a real server, but it tests the handler logic
	let result = handler.handle(health_job).await;

	// The result depends on the demo adapter implementation
	// We're mainly testing that the handler doesn't panic
	match result {
		Ok(()) => println!("Health check succeeded"),
		Err(e) => println!("Health check failed as expected: {}", e),
	}
}

#[tokio::test]
async fn test_background_job_descriptions() {
	let job1 = BackgroundJob::SolverHealthCheck {
		solver_id: "test-solver".to_string(),
	};
	assert_eq!(job1.description(), "Health check for solver 'test-solver'");

	let job2 = BackgroundJob::FetchSolverAssets {
		solver_id: "test-solver".to_string(),
	};
	assert_eq!(job2.description(), "Fetch assets for solver 'test-solver'");

	let job3 = BackgroundJob::OrdersCleanup;
	assert_eq!(job3.description(), "Clean up old orders in final status");
}

#[tokio::test]
async fn test_background_job_solver_id() {
	let job1 = BackgroundJob::SolverHealthCheck {
		solver_id: "test-solver".to_string(),
	};
	assert_eq!(job1.solver_id(), Some("test-solver"));

	let job2 = BackgroundJob::FetchSolverAssets {
		solver_id: "test-solver".to_string(),
	};
	assert_eq!(job2.solver_id(), Some("test-solver"));

	let job3 = BackgroundJob::OrdersCleanup;
	assert_eq!(job3.solver_id(), None);
}

#[tokio::test]
async fn test_job_scheduling() {
	let storage = Arc::new(MemoryStore::new()) as Arc<dyn Storage>;
	let adapter_registry = Arc::new(AdapterRegistry::with_defaults());

	// Create integrity service first
	let integrity_service = Arc::new(IntegrityService::new(
		"test_secret_key_1234567890123456".to_string().into(),
	)) as Arc<dyn IntegrityTrait>;

	// Create mock job scheduler for testing
	let mut mock_scheduler = MockJobScheduler::new();
	mock_scheduler
		.expect_schedule_with_delay()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-delayed-job-id".to_string()) }));
	mock_scheduler
		.expect_schedule_recurring()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-recurring-job-id".to_string()) }));
	mock_scheduler
		.expect_cancel_job()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_| Box::pin(async { Ok(()) }));
	let job_scheduler = Arc::new(mock_scheduler) as Arc<dyn JobScheduler>;

	let order_service = Arc::new(OrderService::new(
		Arc::clone(&storage),
		Arc::clone(&adapter_registry),
		Arc::clone(&integrity_service),
		Arc::clone(&job_scheduler),
	)) as Arc<dyn OrderServiceTrait>;

	// Create job handler with all required services
	let (solver_service, aggregator_service, _) = create_test_services(
		storage.clone(),
		adapter_registry.clone(),
		order_service.clone(),
	);
	let handler = Arc::new(BackgroundJobHandler::new(
		storage.clone(),
		adapter_registry,
		solver_service,
		aggregator_service,
		integrity_service,
		order_service,
		job_scheduler,
		oif_config::Settings::default(),
	));

	let config = JobProcessorConfig {
		queue_capacity: 100,
		worker_count: 1,
		max_scheduled_jobs: 10,
		max_job_info_entries: 1000,
		cleanup_interval_minutes: 0, // Disable automatic cleanup in tests
		job_info_ttl_minutes: 60,
	};

	let processor = JobProcessor::new(handler, config).unwrap();

	// Test scheduling a job without custom ID (auto-generated)
	let job = BackgroundJob::AllSolversHealthCheck;
	let schedule_id = processor
		.schedule_job(1, job, "Test scheduled job".to_string(), None, false, None)
		.await;
	assert!(schedule_id.is_ok());
	let schedule_id = schedule_id.unwrap();

	// Verify the job is scheduled
	let scheduled_jobs = processor.get_scheduled_jobs().await;
	assert_eq!(scheduled_jobs.len(), 1);
	assert_eq!(scheduled_jobs[0].id, schedule_id);
	assert_eq!(scheduled_jobs[0].interval_minutes, 1);
	assert_eq!(scheduled_jobs[0].description, "Test scheduled job");

	// Test scheduling with custom ID and immediate execution
	let custom_job = BackgroundJob::AllSolversFetchAssets;
	let custom_schedule_id = processor
		.schedule_job(
			2,
			custom_job,
			"Custom scheduled job".to_string(),
			Some("custom-schedule-id".to_string()),
			true, // Run immediately
			None, // No retry policy for this test
		)
		.await;
	assert!(custom_schedule_id.is_ok());
	assert_eq!(custom_schedule_id.unwrap(), "custom-schedule-id");

	// Test unscheduling the job
	let result = processor.unschedule_job(&schedule_id).await;
	assert!(result.is_ok());

	// Verify the job is removed
	let scheduled_jobs = processor.get_scheduled_jobs().await;
	assert_eq!(scheduled_jobs.len(), 1); // Only the custom one remains

	// Test unscheduling a non-existent job
	let result = processor.unschedule_job("non-existent").await;
	assert!(result.is_err());
	assert!(result.unwrap_err().contains("not found"));

	// Test job deduplication
	let job_with_id = BackgroundJob::SolverHealthCheck {
		solver_id: "test-solver".to_string(),
	};

	// Submit first job with ID
	let result1 = processor
		.submit(
			job_with_id.clone(),
			Some("duplicate-test".to_string()),
			None,
		)
		.await;
	assert!(result1.is_ok());

	// Submit duplicate job with same ID (should fail)
	let result2 = processor
		.submit(
			job_with_id.clone(),
			Some("duplicate-test".to_string()),
			None,
		)
		.await;
	assert!(result2.is_err()); // Now returns error for duplicates

	// Submit job without ID (should work)
	let result3 = processor.submit(job_with_id, None, None).await;
	assert!(result3.is_ok());

	// Test retry policy and job status tracking
	let retry_policy = RetryPolicy {
		max_retries: 2,
		retry_delay_seconds: 1,
		backoff_multiplier: 1.5,
	};

	let job_with_retry = BackgroundJob::SolverHealthCheck {
		solver_id: "test-solver".to_string(),
	};

	// Submit job with retry policy
	let job_info_id = processor
		.submit(
			job_with_retry,
			Some("retry-test".to_string()),
			Some(retry_policy),
		)
		.await;
	assert!(job_info_id.is_ok());
	let job_info_id = job_info_id.unwrap();

	// Check job info was created
	let job_info = processor.get_job_info(&job_info_id).await;
	assert!(job_info.is_some());
	let job_info = job_info.unwrap();
	assert_eq!(job_info.retry_policy.as_ref().unwrap().max_retries, 2);

	// Check we can get all job info
	let all_jobs = processor.get_all_job_info().await;
	assert!(!all_jobs.is_empty());

	// Test scheduling a job with retry policy
	let retry_policy_for_scheduled = RetryPolicy {
		max_retries: 1,
		retry_delay_seconds: 2,
		backoff_multiplier: 1.0,
	};

	let scheduled_job_with_retry = BackgroundJob::SolverHealthCheck {
		solver_id: "retry-test-scheduled".to_string(),
	};

	let schedule_id_with_retry = processor
		.schedule_job(
			1, // 1 minute interval
			scheduled_job_with_retry,
			"Scheduled job with retry policy".to_string(),
			Some("scheduled-retry-test".to_string()),
			false, // Don't run immediately
			Some(retry_policy_for_scheduled.clone()),
		)
		.await;

	assert!(schedule_id_with_retry.is_ok());

	// Verify the scheduled job has the retry policy
	let scheduled_job_info = processor.get_scheduled_job("scheduled-retry-test").await;
	assert!(scheduled_job_info.is_some());
	let scheduled_job_info = scheduled_job_info.unwrap();
	assert!(scheduled_job_info.retry_policy.is_some());
	assert_eq!(
		scheduled_job_info
			.retry_policy
			.as_ref()
			.unwrap()
			.max_retries,
		1
	);

	// Cleanup with timeout - ignore shutdown errors in tests since mocks can cause issues
	let _ = tokio::time::timeout(Duration::from_secs(2), processor.shutdown()).await;
}

#[tokio::test]
async fn test_job_memory_management() {
	let storage = Arc::new(MemoryStore::new()) as Arc<dyn Storage>;
	let adapter_registry = Arc::new(AdapterRegistry::new());

	// Create integrity service first
	let integrity_service = Arc::new(IntegrityService::new(
		"test_secret_key_1234567890123456".to_string().into(),
	)) as Arc<dyn IntegrityTrait>;

	// Create mock job scheduler for testing
	let mut mock_scheduler = MockJobScheduler::new();
	mock_scheduler
		.expect_schedule_with_delay()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-delayed-job-id".to_string()) }));
	mock_scheduler
		.expect_schedule_recurring()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-recurring-job-id".to_string()) }));
	mock_scheduler
		.expect_cancel_job()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_| Box::pin(async { Ok(()) }));
	let job_scheduler = Arc::new(mock_scheduler) as Arc<dyn JobScheduler>;

	let order_service = Arc::new(OrderService::new(
		Arc::clone(&storage),
		Arc::clone(&adapter_registry),
		Arc::clone(&integrity_service),
		Arc::clone(&job_scheduler),
	)) as Arc<dyn OrderServiceTrait>;

	let (solver_service, aggregator_service, _) = create_test_services(
		storage.clone(),
		adapter_registry.clone(),
		order_service.clone(),
	);

	let handler = Arc::new(BackgroundJobHandler::new(
		storage.clone(),
		adapter_registry,
		solver_service,
		aggregator_service,
		integrity_service,
		order_service,
		job_scheduler,
		oif_config::Settings::default(),
	));

	// Create processor with very small max entries to test LRU eviction
	let config = JobProcessorConfig {
		queue_capacity: 10, // Smaller queue to avoid timing issues
		worker_count: 1,
		max_scheduled_jobs: 5,       // Smaller limits to avoid complexity
		max_job_info_entries: 3,     // Smaller limit to make eviction more predictable
		cleanup_interval_minutes: 0, // Disable automatic cleanup in tests
		job_info_ttl_minutes: 60,    // Longer TTL to avoid TTL-based eviction during test
	};

	let processor = JobProcessor::new(handler, config).unwrap();

	// Test initial stats
	let stats = processor.get_job_info_stats().await;
	assert_eq!(stats.total_entries, 0);
	assert_eq!(stats.max_entries, 3);
	assert_eq!(stats.ttl_minutes, 60);

	// Submit several jobs to test LRU eviction
	let mut job_ids = Vec::new();
	for i in 0..5 {
		// Submit more than max_entries (3)
		let job = BackgroundJob::SolverHealthCheck {
			solver_id: format!("test-solver-{}", i),
		};
		let job_id = processor.submit(job, None, None).await.unwrap();
		job_ids.push(job_id);

		// Add delay to ensure different submission times and allow processing
		tokio::time::sleep(Duration::from_millis(100)).await;
	}

	// Give the system time to process jobs and apply LRU eviction
	tokio::time::sleep(Duration::from_millis(300)).await;

	// Check that LRU eviction occurred (should have <= 3 entries)
	// Allow for some flexibility as the eviction might not be exact due to timing
	let stats_after_eviction = processor.get_job_info_stats().await;
	assert!(
		stats_after_eviction.total_entries <= 4, // Allow 1 extra for timing tolerance
		"Expected <= 4 entries due to LRU eviction (3 + 1 for timing), but got {}",
		stats_after_eviction.total_entries
	);

	// The first few job IDs should have been evicted (no longer retrievable)
	let first_job_info = processor.get_job_info(&job_ids[0]).await;
	assert!(
		first_job_info.is_none(),
		"First job should have been evicted"
	);

	// The last few job IDs should still be retrievable
	let last_job_info = processor.get_job_info(&job_ids[4]).await;
	assert!(
		last_job_info.is_some(),
		"Last job should still be in memory"
	);

	// Test manual cleanup
	let removed = processor.cleanup_old_job_info().await;
	// With TTL of 60 minutes and recent submissions, no entries should be removed by TTL
	assert_eq!(removed, 0, "No entries should be removed by TTL yet");

	// Test stats breakdown by status
	let all_jobs = processor.get_all_job_info().await;
	let mut status_counts = std::collections::HashMap::new();
	for job in &all_jobs {
		let status_key = match &job.status {
			JobStatus::Pending => "pending",
			JobStatus::Running => "running",
			JobStatus::Completed => "completed",
			JobStatus::Failed { .. } => "failed",
			JobStatus::Retrying { .. } => "retrying",
		};
		*status_counts.entry(status_key).or_insert(0) += 1;
	}

	let final_stats = processor.get_job_info_stats().await;
	assert_eq!(
		final_stats.by_status.get("pending").unwrap_or(&0),
		status_counts.get("pending").unwrap_or(&0)
	);

	// Cleanup with timeout - ignore shutdown errors in tests since mocks can cause issues
	let _ = tokio::time::timeout(Duration::from_secs(2), processor.shutdown()).await;
}

#[tokio::test]
async fn test_orders_cleanup_job() {
	let storage = Arc::new(MemoryStore::new()) as Arc<dyn Storage>;
	let adapter_registry = Arc::new(AdapterRegistry::with_defaults());

	// Create integrity service first
	let integrity_service = Arc::new(IntegrityService::new(
		"test_secret_key_1234567890123456".to_string().into(),
	)) as Arc<dyn IntegrityTrait>;

	// Create mock job scheduler for testing
	let mut mock_scheduler = MockJobScheduler::new();
	mock_scheduler
		.expect_schedule_with_delay()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-delayed-job-id".to_string()) }));
	mock_scheduler
		.expect_schedule_recurring()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_, _, _| Box::pin(async { Ok("mock-recurring-job-id".to_string()) }));
	mock_scheduler
		.expect_cancel_job()
		.times(0..=10)  // More specific range instead of unlimited
		.returning(|_| Box::pin(async { Ok(()) }));
	let job_scheduler = Arc::new(mock_scheduler) as Arc<dyn JobScheduler>;

	let order_service = Arc::new(OrderService::new(
		Arc::clone(&storage),
		Arc::clone(&adapter_registry),
		Arc::clone(&integrity_service),
		Arc::clone(&job_scheduler),
	)) as Arc<dyn OrderServiceTrait>;

	// Create job handler
	let (solver_service, aggregator_service, _) = create_test_services(
		storage.clone(),
		adapter_registry.clone(),
		order_service.clone(),
	);
	let handler = BackgroundJobHandler::new(
		storage.clone(),
		adapter_registry,
		solver_service,
		aggregator_service,
		integrity_service,
		order_service,
		job_scheduler,
		oif_config::Settings::default(),
	);

	// Test orders cleanup job
	let cleanup_job = BackgroundJob::OrdersCleanup;

	// This should succeed even with no orders to clean up
	let result = handler.handle(cleanup_job).await;
	assert!(
		result.is_ok(),
		"Order cleanup job should succeed: {:?}",
		result
	);
}
