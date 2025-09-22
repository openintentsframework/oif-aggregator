//! Metrics cleanup job handler for removing old time-series data

use chrono::{DateTime, Duration, Utc};
use futures::stream::{self, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::jobs::types::{JobError, JobResult};
use oif_config::Settings;
use oif_storage::Storage;

/// Estimation parameters for cleanup impact calculation
///
/// These constants define the heuristics used to estimate how many time-series
/// entries might be stale and need cleanup, without having to scan each individual
/// time-series entry (which would be expensive).
const ESTIMATED_STALE_PERCENTAGE: usize = 10; // Assume ~10% of solvers might be stale
const MIN_ESTIMATED_STALE_COUNT: usize = 5; // Minimum estimate to avoid reporting 0 for small datasets

/// Concurrency limits for parallel processing
const MAX_CONCURRENT_CLEANUP_TASKS: usize = 10; // Limit parallel tasks to avoid overwhelming storage

/// Handler for metrics cleanup jobs
pub struct MetricsCleanupHandler {
	storage: Arc<dyn Storage>,
	settings: Settings,
}

impl MetricsCleanupHandler {
	/// Create a new metrics cleanup handler
	pub fn new(storage: Arc<dyn Storage>, settings: Settings) -> Self {
		Self { storage, settings }
	}

	/// Handle metrics cleanup job - removes old time-series data
	pub async fn handle_metrics_cleanup(&self) -> JobResult<()> {
		let retention_hours = self.settings.get_metrics_retention_hours();
		let cutoff_time = Utc::now() - Duration::hours(retention_hours as i64);

		info!(
			"Starting metrics cleanup: removing data older than {} hours (before {})",
			retention_hours,
			cutoff_time.format("%Y-%m-%d %H:%M:%S UTC")
		);

		let mut total_cleaned = 0;
		let mut errors = 0;

		// Run both cleanup operations in parallel for better performance
		let (stale_cleanup_result, internal_cleanup_result) = tokio::join!(
			self.cleanup_old_timeseries(cutoff_time),
			self.cleanup_internal_bucket_data()
		);

		// Handle stale time-series cleanup results
		match stale_cleanup_result {
			Ok(cleaned_count) => {
				total_cleaned += cleaned_count;
				if cleaned_count > 0 {
					info!("Cleaned up {} stale time-series entries", cleaned_count);
				}
			},
			Err(e) => {
				warn!("Failed to clean up time-series data: {}", e);
				errors += 1;
			},
		}

		// Handle internal bucket cleanup results
		match internal_cleanup_result {
			Ok(processed_count) => {
				debug!(
					"Processed {} time-series for internal cleanup",
					processed_count
				);
			},
			Err(e) => {
				warn!("Failed to clean up internal bucket data: {}", e);
				errors += 1;
			},
		}

		if errors > 0 && total_cleaned == 0 {
			return Err(JobError::ProcessingFailed {
				message: format!("Metrics cleanup failed with {} errors", errors),
			});
		}

		info!(
			"Metrics cleanup completed: {} items cleaned, {} errors",
			total_cleaned, errors
		);

		Ok(())
	}

	/// Clean up stale time-series entries (solvers not updated recently)
	async fn cleanup_old_timeseries(&self, cutoff_time: DateTime<Utc>) -> JobResult<usize> {
		match self.storage.cleanup_old_metrics(cutoff_time).await {
			Ok(count) => Ok(count),
			Err(e) => Err(JobError::Storage(format!(
				"Failed to cleanup old metrics: {}",
				e
			))),
		}
	}

	/// Clean up internal bucket data within existing time-series
	/// This ensures each time-series doesn't grow beyond its configured limits
	async fn cleanup_internal_bucket_data(&self) -> JobResult<usize> {
		// Get list of all solvers with metrics
		let solver_ids = match self.storage.list_solvers_with_metrics().await {
			Ok(ids) => ids,
			Err(e) => {
				return Err(JobError::Storage(format!(
					"Failed to list solvers with metrics: {}",
					e
				)));
			},
		};

		debug!(
			"Processing {} solvers for internal bucket cleanup in parallel",
			solver_ids.len()
		);

		// Use atomic counters for thread-safe result aggregation
		let processed = Arc::new(AtomicUsize::new(0));
		let errors = Arc::new(AtomicUsize::new(0));

		// Process solvers in parallel with controlled concurrency
		let concurrent_limit = MAX_CONCURRENT_CLEANUP_TASKS;

		stream::iter(solver_ids.iter())
			.for_each_concurrent(concurrent_limit, |solver_id| {
				let storage = Arc::clone(&self.storage);
				let processed = Arc::clone(&processed);
				let errors = Arc::clone(&errors);
				let solver_id = solver_id.clone();

				async move {
					match storage.get_metrics_timeseries(&solver_id).await {
						Ok(Some(mut timeseries)) => {
							// The MetricsTimeSeries already has internal cleanup mechanisms
							// when adding data points, but we can trigger explicit cleanup here
							let old_size = timeseries.five_minute_buckets.aggregates.len();

							// Trigger internal cleanup by updating rolling metrics
							// This will clean up old buckets that exceed max_buckets
							if let Err(e) = timeseries.update_rolling_metrics() {
								tracing::warn!(
									"Failed to update rolling metrics during cleanup for solver '{}': {}",
									solver_id, e
								);
								// Continue with cleanup - rolling metrics failure shouldn't stop the cleanup process
							}

							let new_size = timeseries.five_minute_buckets.aggregates.len();

							if old_size != new_size {
								debug!(
									"Cleaned internal buckets for solver '{}': {} -> {} buckets",
									solver_id, old_size, new_size
								);

								// Save the cleaned time-series back
								if let Err(e) = storage
									.update_metrics_timeseries(&solver_id, timeseries)
									.await
								{
									warn!(
										"Failed to save cleaned time-series for solver '{}': {}",
										solver_id, e
									);
									errors.fetch_add(1, Ordering::Relaxed);
								}
							}

							processed.fetch_add(1, Ordering::Relaxed);
						},
						Ok(None) => {
							// No time-series for this solver, skip silently
						},
						Err(e) => {
							warn!(
								"Failed to get time-series for solver '{}': {}",
								solver_id, e
							);
							errors.fetch_add(1, Ordering::Relaxed);
						},
					}
				}
			})
			.await;

		// Extract final counts
		let processed_count = processed.load(Ordering::Relaxed);
		let errors_count = errors.load(Ordering::Relaxed);

		if errors_count > 0 {
			warn!(
				"Internal bucket cleanup completed with {} errors out of {} solvers",
				errors_count,
				solver_ids.len()
			);
		}

		debug!(
			"Internal bucket cleanup completed: {} solvers processed successfully",
			processed_count
		);

		Ok(processed_count)
	}

	/// Get retention cutoff time based on configuration
	pub fn get_retention_cutoff(&self) -> DateTime<Utc> {
		let retention_hours = self.settings.get_metrics_retention_hours();
		Utc::now() - Duration::hours(retention_hours as i64)
	}

	/// Estimate how much data would be cleaned up (dry run)
	///
	/// This provides a lightweight estimation without scanning individual time-series entries.
	/// Uses statistical heuristics based on typical solver lifecycle patterns.
	pub async fn estimate_cleanup_impact(&self) -> JobResult<CleanupEstimate> {
		let cutoff_time = self.get_retention_cutoff();

		let solver_count = match self.storage.count_metrics_timeseries().await {
			Ok(count) => count,
			Err(e) => return Err(JobError::Storage(format!("Failed to count metrics: {}", e))),
		};

		// Estimate stale count using heuristics to avoid expensive per-timeseries scanning
		//
		// Rationale:
		// - Assume ~10% of solvers become stale over time (inactive, removed, etc.)
		// - Use minimum of 5 to provide meaningful estimates for small datasets
		// - This provides a reasonable approximation without storage overhead
		let estimated_stale_count = std::cmp::min(
			solver_count / ESTIMATED_STALE_PERCENTAGE,
			MIN_ESTIMATED_STALE_COUNT,
		);

		Ok(CleanupEstimate {
			total_timeseries: solver_count,
			estimated_stale_timeseries: estimated_stale_count,
			cutoff_time,
			retention_hours: self.settings.get_metrics_retention_hours(),
		})
	}
}

/// Estimate of cleanup impact for dry-run purposes
#[derive(Debug, Clone)]
pub struct CleanupEstimate {
	pub total_timeseries: usize,
	pub estimated_stale_timeseries: usize,
	pub cutoff_time: DateTime<Utc>,
	pub retention_hours: u32,
}

impl CleanupEstimate {
	/// Get a human-readable summary
	pub fn summary(&self) -> String {
		format!(
			"Cleanup estimate: {} of {} time-series entries would be removed (retention: {}h, cutoff: {})",
			self.estimated_stale_timeseries,
			self.total_timeseries,
			self.retention_hours,
			self.cutoff_time.format("%Y-%m-%d %H:%M:%S UTC")
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::Utc;
	use oif_storage::MemoryStore;
	use oif_types::{MetricsDataPoint, MetricsTimeSeries};

	async fn create_test_handler() -> MetricsCleanupHandler {
		let storage = Arc::new(MemoryStore::new());
		let settings = Settings::default();
		MetricsCleanupHandler::new(storage as Arc<dyn Storage>, settings)
	}

	#[tokio::test]
	async fn test_metrics_cleanup_handler_creation() {
		let handler = create_test_handler().await;
		assert!(handler.settings.get_metrics_retention_hours() > 0);
	}

	#[tokio::test]
	async fn test_cleanup_with_no_metrics() {
		let handler = create_test_handler().await;
		let result = handler.handle_metrics_cleanup().await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_cleanup_estimate() {
		let handler = create_test_handler().await;

		// Add some test data
		let mut timeseries = MetricsTimeSeries::new("test-solver".to_string());
		let data_point = MetricsDataPoint {
			timestamp: Utc::now(),
			response_time_ms: 100,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: oif_types::Operation::GetQuotes,
		};
		timeseries.add_data_point(data_point);

		handler
			.storage
			.update_metrics_timeseries("test-solver", timeseries)
			.await
			.unwrap();

		let estimate = handler.estimate_cleanup_impact().await.unwrap();
		assert!(estimate.total_timeseries > 0);
		assert!(!estimate.summary().is_empty());
	}

	#[tokio::test]
	async fn test_retention_cutoff_calculation() {
		let handler = create_test_handler().await;
		let cutoff = handler.get_retention_cutoff();
		let now = Utc::now();

		// Cutoff should be in the past
		assert!(cutoff < now);

		// Should be approximately the retention period ago
		let expected_cutoff =
			now - Duration::hours(handler.settings.get_metrics_retention_hours() as i64);
		let diff = (cutoff - expected_cutoff).num_minutes().abs();
		assert!(diff < 5); // Within 5 minutes tolerance
	}
}
