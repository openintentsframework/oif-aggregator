//! Metrics update job handler for processing solver performance data

use futures::stream::{self, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::jobs::types::{JobError, JobResult, SolverMetricsUpdate};
use oif_storage::Storage;
use oif_types::{MetricsDataPoint, MetricsTimeSeries};

/// Handler for metrics update jobs
pub struct MetricsUpdateHandler {
	storage: Arc<dyn Storage>,
}

impl MetricsUpdateHandler {
	/// Create a new metrics update handler
	pub fn new(storage: Arc<dyn Storage>) -> Self {
		Self { storage }
	}

	/// Handle aggregation metrics update job with multiple solvers
	pub async fn handle_aggregation_metrics_update(
		&self,
		aggregation_id: &str,
		solver_metrics: Vec<(String, SolverMetricsUpdate)>,
	) -> JobResult<()> {
		debug!(
			"Processing aggregation metrics update '{}' for {} solvers",
			aggregation_id,
			solver_metrics.len()
		);

		// Process solver metrics in parallel for better performance
		debug!(
			"Processing metrics for {} solvers in parallel",
			solver_metrics.len()
		);

		// Use atomic counters for thread-safe result aggregation
		let success_count = Arc::new(AtomicUsize::new(0));
		let error_count = Arc::new(AtomicUsize::new(0));

		// Process solvers in parallel with controlled concurrency (max 8 concurrent)
		let concurrent_limit = 8;

		stream::iter(solver_metrics.into_iter())
			.for_each_concurrent(concurrent_limit, |(solver_id, metrics_data)| {
				let storage = Arc::clone(&self.storage);
				let success_count = Arc::clone(&success_count);
				let error_count = Arc::clone(&error_count);

				async move {
					debug!(
						"Processing metrics for solver '{}': success={}, response_time={}ms",
						solver_id, metrics_data.was_successful, metrics_data.response_time_ms
					);

					// Run both updates in parallel for each solver
					let (current_result, timeseries_result) = tokio::join!(
						Self::update_current_solver_metrics_static(
							&storage,
							&solver_id,
							&metrics_data
						),
						Self::update_timeseries_metrics_static(&storage, &solver_id, &metrics_data)
					);

					// Track results - count errors per operation, success only if both succeed
					let mut solver_errors = 0;

					// Handle current metrics update result
					if let Err(e) = current_result {
						warn!(
							"Failed to update current metrics for solver '{}': {}",
							solver_id, e
						);
						solver_errors += 1;
					}

					// Handle time-series metrics update result
					if let Err(e) = timeseries_result {
						warn!(
							"Failed to update time-series metrics for solver '{}': {}",
							solver_id, e
						);
						solver_errors += 1;
					}

					// Update counters based on results
					if solver_errors > 0 {
						error_count.fetch_add(solver_errors, Ordering::Relaxed);
					} else {
						success_count.fetch_add(1, Ordering::Relaxed);
					}
				}
			})
			.await;

		// Extract final counts
		let success_count = success_count.load(Ordering::Relaxed);
		let error_count = error_count.load(Ordering::Relaxed);

		if error_count > 0 && success_count == 0 {
			return Err(JobError::ProcessingFailed {
				message: format!(
					"Failed to update metrics for all {} solvers in aggregation '{}'",
					error_count, aggregation_id
				),
			});
		}

		debug!(
			"Successfully processed aggregation metrics update '{}': {} succeeded, {} failed",
			aggregation_id, success_count, error_count
		);
		Ok(())
	}

	/// Static version of update_current_solver_metrics for parallel processing
	async fn update_current_solver_metrics_static(
		storage: &Arc<dyn Storage>,
		solver_id: &str,
		metrics_data: &SolverMetricsUpdate,
	) -> JobResult<()> {
		// Get current solver
		let mut solver = match storage.get_solver(solver_id).await {
			Ok(Some(solver)) => solver,
			Ok(None) => {
				return Err(JobError::InvalidConfig(format!(
					"Solver '{}' not found",
					solver_id
				)));
			},
			Err(e) => {
				return Err(JobError::Storage(format!(
					"Failed to get solver '{}': {}",
					solver_id, e
				)));
			},
		};

		// Update the solver's current metrics
		if metrics_data.was_successful {
			solver.metrics.record_success(metrics_data.response_time_ms);
		} else {
			solver.metrics.record_failure(metrics_data.was_timeout);
		}

		// Update last seen timestamp
		solver.mark_seen();

		// Save updated solver back to storage
		storage
			.update_solver(solver)
			.await
			.map_err(|e| JobError::Storage(format!("Failed to update solver: {}", e)))?;

		debug!(
			"Updated current metrics for solver '{}': success={}, response_time={}ms",
			solver_id, metrics_data.was_successful, metrics_data.response_time_ms
		);

		Ok(())
	}

	/// Static version of update_timeseries_metrics for parallel processing
	async fn update_timeseries_metrics_static(
		storage: &Arc<dyn Storage>,
		solver_id: &str,
		metrics_data: &SolverMetricsUpdate,
	) -> JobResult<()> {
		// Verify solver exists before creating/updating time-series
		match storage.get_solver(solver_id).await {
			Ok(Some(_)) => {
				// Solver exists, continue with metrics update
			},
			Ok(None) => {
				return Err(JobError::InvalidConfig(format!(
					"Cannot update time-series metrics for nonexistent solver '{}'",
					solver_id
				)));
			},
			Err(e) => {
				return Err(JobError::Storage(format!(
					"Failed to verify solver '{}' existence: {}",
					solver_id, e
				)));
			},
		}

		// Create metrics data point from the update
		let data_point = MetricsDataPoint {
			timestamp: metrics_data.timestamp,
			response_time_ms: metrics_data.response_time_ms,
			was_successful: metrics_data.was_successful,
			was_timeout: metrics_data.was_timeout,
			error_type: metrics_data.error_message.as_ref().map(|_| {
				// Simple error classification - could be improved
				if metrics_data.was_timeout {
					oif_types::ErrorType::Timeout
				} else {
					oif_types::ErrorType::Unknown
				}
			}),
		};

		// Get or create time-series for this solver
		let mut timeseries = match storage.get_metrics_timeseries(solver_id).await {
			Ok(Some(ts)) => ts,
			Ok(None) => {
				debug!("Creating new time-series for solver '{}'", solver_id);
				MetricsTimeSeries::new(solver_id.to_string())
			},
			Err(e) => {
				return Err(JobError::Storage(format!(
					"Failed to get time-series for solver '{}': {}",
					solver_id, e
				)));
			},
		};

		// Add the data point to the time-series
		timeseries.add_data_point(data_point);

		// Update rolling metrics (this can be expensive, consider doing it less frequently)
		timeseries.update_rolling_metrics();

		// Save updated time-series back to storage
		storage
			.update_metrics_timeseries(solver_id, timeseries)
			.await
			.map_err(|e| {
				JobError::Storage(format!(
					"Failed to update time-series for solver '{}': {}",
					solver_id, e
				))
			})?;

		debug!(
			"Updated time-series metrics for solver '{}': {} buckets updated",
			solver_id,
			4 // We update 4 bucket types (5min, 15min, hourly, daily)
		);

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::Utc;
	use oif_storage::{traits::MetricsStorage, MemoryStore};
	use oif_types::Solver;

	/// Helper to create test services
	async fn create_test_services() -> Arc<MemoryStore> {
		Arc::new(MemoryStore::new())
	}

	/// Create a test solver
	async fn create_test_solver(storage: &Arc<MemoryStore>) -> Solver {
		let solver = Solver::new(
			"test-solver".to_string(),
			"test-adapter".to_string(),
			"http://test.example.com".to_string(),
		);
		storage.create_solver(solver.clone()).await.unwrap();
		solver
	}

	#[tokio::test]
	async fn test_metrics_update_handler_creation() {
		let storage = create_test_services().await;
		let handler = MetricsUpdateHandler::new(storage as Arc<dyn Storage>);

		// Verify we can create the handler
		assert!(std::ptr::addr_of!(handler).is_aligned());
	}

	#[tokio::test]
	async fn test_handle_successful_metrics_update() {
		let storage = create_test_services().await;
		let _solver = create_test_solver(&storage).await;
		let handler = MetricsUpdateHandler::new(storage.clone() as Arc<dyn Storage>);

		let metrics_data = SolverMetricsUpdate {
			response_time_ms: 250,
			was_successful: true,
			was_timeout: false,
			timestamp: Utc::now(),
			error_message: None,
		};

		let result = handler
			.handle_aggregation_metrics_update(
				"test-agg-1",
				vec![("test-solver".to_string(), metrics_data)],
			)
			.await;
		assert!(result.is_ok());

		// Verify solver metrics were updated
		let updated_solver = storage.get_solver("test-solver").await.unwrap().unwrap();
		assert_eq!(updated_solver.metrics.total_requests, 1);
		assert_eq!(updated_solver.metrics.successful_requests, 1);
		assert_eq!(updated_solver.metrics.failed_requests, 0);

		// Verify time-series was created
		let timeseries = storage
			.get_metrics_timeseries("test-solver")
			.await
			.unwrap()
			.unwrap();
		assert_eq!(timeseries.solver_id, "test-solver");
		assert!(!timeseries.five_minute_buckets.is_empty());
	}

	#[tokio::test]
	async fn test_handle_failed_metrics_update() {
		let storage = create_test_services().await;
		let _solver = create_test_solver(&storage).await;
		let handler = MetricsUpdateHandler::new(storage.clone() as Arc<dyn Storage>);

		let metrics_data = SolverMetricsUpdate {
			response_time_ms: 5000,
			was_successful: false,
			was_timeout: true,
			timestamp: Utc::now(),
			error_message: Some("Connection timeout".to_string()),
		};

		let result = handler
			.handle_aggregation_metrics_update(
				"test-agg-2",
				vec![("test-solver".to_string(), metrics_data)],
			)
			.await;
		assert!(result.is_ok());

		// Verify solver metrics were updated
		let updated_solver = storage.get_solver("test-solver").await.unwrap().unwrap();
		assert_eq!(updated_solver.metrics.total_requests, 1);
		assert_eq!(updated_solver.metrics.successful_requests, 0);
		assert_eq!(updated_solver.metrics.failed_requests, 1);
		assert_eq!(updated_solver.metrics.timeout_requests, 1);
	}

	#[tokio::test]
	async fn test_handle_metrics_update_nonexistent_solver() {
		let storage = create_test_services().await;
		let handler = MetricsUpdateHandler::new(storage as Arc<dyn Storage>);

		let metrics_data = SolverMetricsUpdate {
			response_time_ms: 250,
			was_successful: true,
			was_timeout: false,
			timestamp: Utc::now(),
			error_message: None,
		};

		let result = handler
			.handle_aggregation_metrics_update(
				"test-agg-3",
				vec![("nonexistent-solver".to_string(), metrics_data)],
			)
			.await;

		// With the batch processing, this will return an error since all solvers failed
		assert!(result.is_err());
		if let Err(JobError::ProcessingFailed { message }) = result {
			assert!(message.contains("Failed to update metrics for all"));
		} else {
			panic!("Expected ProcessingFailed error");
		}
	}
}
