//! Metrics update job handler for processing solver performance data

use chrono::{Duration, Utc};
use futures::stream::{self, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::jobs::types::{JobError, JobResult, SolverMetricsUpdate};
use oif_storage::Storage;
use oif_types::{MetricsDataPoint, MetricsTimeSeries};

/// Rolling metrics update interval - only recalculate if staler than this
/// Set to 30 seconds to provide good performance while maintaining responsiveness
const ROLLING_METRICS_UPDATE_INTERVAL: Duration = Duration::seconds(30);

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
						"Processing metrics for solver '{}': operation='{}', success={}, response_time={}ms",
						solver_id, metrics_data.operation, metrics_data.was_successful, metrics_data.response_time_ms
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
			// Record failure with proper error categorization
			solver.metrics.record_categorized_failure(
				metrics_data.was_timeout,
				metrics_data.error_type.clone(),
			);
		}

		// Update last seen timestamp
		solver.mark_seen();

		// Load time-series data for intelligent status evaluation
		let timeseries = storage
			.get_metrics_timeseries(solver_id)
			.await
			.unwrap_or(None);

		// Evaluate and update solver status based on comprehensive metrics data
		Self::evaluate_and_update_solver_status(&mut solver, timeseries.as_ref());

		// Save updated solver back to storage
		storage
			.update_solver(solver)
			.await
			.map_err(|e| JobError::Storage(format!("Failed to update solver: {}", e)))?;

		debug!(
			"Updated current metrics for solver '{}': operation='{}', success={}, response_time={}ms",
			solver_id, metrics_data.operation, metrics_data.was_successful, metrics_data.response_time_ms
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
			error_type: metrics_data.error_type.clone(),
			operation: metrics_data
				.operation
				.parse()
				.unwrap_or(oif_types::Operation::Other),
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

		// Update rolling metrics only if they're stale (every 30 seconds)
		if should_update_rolling_metrics(&timeseries) {
			// Handle rolling metrics update failures properly
			if let Err(e) = timeseries.update_rolling_metrics() {
				// This is a critical failure that indicates potential data corruption or memory issues
				// Log as error and propagate up to caller for proper handling
				return Err(JobError::ProcessingFailed {
					message: format!(
						"Critical failure updating rolling metrics for solver '{}': {}",
						solver_id, e
					),
				});
			}
		}

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

	/// Intelligent status evaluation based on comprehensive metrics data
	/// This uses circuit breaker patterns and thresholds instead of immediate status changes
	fn evaluate_and_update_solver_status(
		solver: &mut oif_types::solvers::Solver,
		timeseries: Option<&MetricsTimeSeries>,
	) {
		use oif_types::solvers::SolverStatus;

		let solver_id = &solver.solver_id;

		// Circuit breaker thresholds - configurable in the future
		const CONSECUTIVE_FAILURE_THRESHOLD: u32 = 3;
		const MIN_REQUESTS_FOR_ERROR_RATE: u64 = 5;
		const ERROR_RATE_THRESHOLD: f64 = 0.7; // 70% error rate to mark as Error

		let current_status = solver.status.clone();
		let consecutive_failures = solver.metrics.consecutive_failures;
		let total_requests = solver.metrics.total_requests;

		// Calculate current error rate
		let current_error_rate = if total_requests > 0 {
			let failed_requests = total_requests - solver.metrics.successful_requests;
			failed_requests as f64 / total_requests as f64
		} else {
			0.0
		};

		// Get recent health status
		let is_currently_healthy = solver
			.metrics
			.health_status
			.as_ref()
			.map(|h| h.is_healthy)
			.unwrap_or(false);

		// Status evaluation logic
		let new_status = match current_status {
			SolverStatus::Active => {
				// From Active to Error: Need strong evidence of problems
				if consecutive_failures >= CONSECUTIVE_FAILURE_THRESHOLD
					|| (total_requests >= MIN_REQUESTS_FOR_ERROR_RATE
						&& current_error_rate >= ERROR_RATE_THRESHOLD)
				{
					SolverStatus::Error
				} else {
					// Stay Active - single failures don't immediately affect status
					SolverStatus::Active
				}
			},

			SolverStatus::Error => {
				// From Error to Active: Need evidence of recovery
				if (consecutive_failures == 0 && is_currently_healthy)
					|| (consecutive_failures <= 1
						&& current_error_rate < 0.3
						&& total_requests >= MIN_REQUESTS_FOR_ERROR_RATE)
				{
					SolverStatus::Active
				} else {
					// Stay Error - need sustained recovery to change status
					SolverStatus::Error
				}
			},

			SolverStatus::Inactive | SolverStatus::Initializing => {
				// From Inactive/Initializing to Active: Health check success
				if is_currently_healthy && consecutive_failures == 0 {
					SolverStatus::Active
				} else {
					current_status.clone() // Keep current status
				}
			},

			SolverStatus::Maintenance => {
				// Maintenance status is manually set - don't change automatically
				// Only move to Active if explicitly healthy and no failures
				if is_currently_healthy && consecutive_failures == 0 {
					SolverStatus::Active
				} else {
					current_status.clone() // Stay in Maintenance
				}
			},
		};

		// Update status if changed
		if new_status != current_status {
			solver.status = new_status.clone();
		}

		// Optional: Use time-series data for additional insights (future enhancement)
		if let Some(ts) = timeseries {
			if let Some(rolling) = &ts.rolling_metrics.last_hour {
				debug!(
					"Time-series context for '{}': last_hour success_rate={:.3}, p95_response={}ms",
					solver_id, rolling.success_rate, rolling.p95_response_time_ms
				);
			}
		}
	}
}

/// Check if rolling metrics should be updated based on staleness
fn should_update_rolling_metrics(timeseries: &MetricsTimeSeries) -> bool {
	let now = Utc::now();
	let last_update = timeseries.rolling_metrics.last_updated;

	// Always update if rolling metrics have never been properly calculated
	// (all aggregates are None means no calculations have been done yet)
	if timeseries.rolling_metrics.last_hour.is_none()
		&& timeseries.rolling_metrics.last_day.is_none()
		&& timeseries.rolling_metrics.last_week.is_none()
	{
		return true;
	}

	// Otherwise, only update if stale enough
	now - last_update >= ROLLING_METRICS_UPDATE_INTERVAL
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
			status_code: None,
			error_type: None,
			operation: "get_quotes".to_string(),
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
		// failed_requests is now calculated as: total_requests - successful_requests
		let failed_requests =
			updated_solver.metrics.total_requests - updated_solver.metrics.successful_requests;
		assert_eq!(failed_requests, 0);

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
			status_code: None,
			error_type: Some(oif_types::ErrorType::ServiceError),
			operation: "get_quotes".to_string(),
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
		// failed_requests is now calculated as: total_requests - successful_requests
		let failed_requests =
			updated_solver.metrics.total_requests - updated_solver.metrics.successful_requests;
		assert_eq!(failed_requests, 1);
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
			status_code: None,
			error_type: None,
			operation: "health_check".to_string(),
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
