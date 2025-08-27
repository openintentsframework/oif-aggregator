//! Comprehensive order status monitoring with exponential backoff

use std::sync::Arc;
use std::time::Duration;

use crate::jobs::scheduler::JobScheduler;
use crate::jobs::types::{BackgroundJob, JobError, JobResult};
use crate::order::OrderServiceTrait;
use oif_storage::Storage;
use oif_types::OrderStatus;

/// Tracing target for structured logging
const TRACING_TARGET: &str = "oif_aggregator::order_monitor";

/// Configuration for order monitoring behavior
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
	/// Backoff schedule in seconds: [5s, 10s, 15s, 30s, 60s, 120s, 300s]
	pub backoff_schedule: Vec<u64>,
	/// Maximum backoff time in seconds (5 minutes)
	pub max_backoff_seconds: u64,
	/// Maximum number of monitoring attempts before giving up
	pub max_attempts: Option<u32>,
	/// Timeout for individual order refresh operations
	pub refresh_timeout_seconds: u64,
}

impl Default for MonitoringConfig {
	fn default() -> Self {
		Self {
			backoff_schedule: vec![5, 10, 15, 30, 60, 120, 300],
			max_backoff_seconds: 300,
			max_attempts: None, // No limit by default
			refresh_timeout_seconds: 5,
		}
	}
}

/// Result of order status monitoring that indicates next action
#[derive(Debug, Clone, PartialEq)]
pub enum MonitoringResult {
	/// Order reached final status, stop monitoring
	Completed,
	/// Order needs continued monitoring after the specified delay
	ContinueMonitoring { next_attempt: u32 },
	/// Monitoring failed, should retry after delay
	RetryLater { next_attempt: u32 },
	/// Maximum attempts reached, stop monitoring
	MaxAttemptsReached,
}

/// Service for calculating monitoring delays and scheduling
pub struct MonitoringScheduler {
	config: MonitoringConfig,
	job_scheduler: Arc<dyn JobScheduler>,
}

impl MonitoringScheduler {
	pub fn new(config: MonitoringConfig, job_scheduler: Arc<dyn JobScheduler>) -> Self {
		Self {
			config,
			job_scheduler,
		}
	}

	/// Calculate the next backoff delay based on the current attempt
	pub fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
		let seconds = if (attempt as usize) < self.config.backoff_schedule.len() {
			self.config.backoff_schedule[attempt as usize]
		} else {
			self.config.max_backoff_seconds
		};

		Duration::from_secs(seconds)
	}

	/// Check if maximum attempts have been reached
	pub fn is_max_attempts_reached(&self, attempt: u32) -> bool {
		self.config
			.max_attempts
			.map(|max| attempt >= max)
			.unwrap_or(false)
	}

	/// Schedule the next monitoring attempt
	pub async fn schedule_next_monitoring(
		&self,
		order_id: &str,
		next_attempt: u32,
	) -> JobResult<()> {
		let delay = self.calculate_backoff_delay(next_attempt);
		let next_job_id = Self::generate_job_id(order_id);

		tracing::debug!(
			target: TRACING_TARGET,
			order_id = %order_id,
			attempt = next_attempt,
			delay_seconds = delay.as_secs(),
			"Scheduling next order monitoring"
		);

		self.job_scheduler
			.schedule_with_delay(
				BackgroundJob::OrderStatusMonitor {
					order_id: order_id.to_string(),
					attempt: next_attempt,
				},
				delay,
				Some(next_job_id),
			)
			.await
			.map_err(|e| JobError::ProcessingFailed {
				message: format!("Failed to schedule next monitoring: {}", e),
			})?;

		Ok(())
	}

	/// Generate job ID for a specific attempt
	pub fn generate_job_id(order_id: &str) -> String {
		format!("order-monitor-{}", order_id)
	}

	/// Get the monitoring configuration
	pub fn config(&self) -> &MonitoringConfig {
		&self.config
	}
}

/// Core order monitoring logic - focused only on checking order status
pub struct OrderMonitor {
	order_service: Arc<dyn OrderServiceTrait>,
	storage: Arc<dyn Storage>,
	config: MonitoringConfig,
}

impl OrderMonitor {
	pub fn new(
		order_service: Arc<dyn OrderServiceTrait>,
		storage: Arc<dyn Storage>,
		config: MonitoringConfig,
	) -> Self {
		Self {
			order_service,
			storage,
			config,
		}
	}

	/// Check if an order status is final (no more monitoring needed)
	pub fn is_final_status(status: &OrderStatus) -> bool {
		matches!(status, OrderStatus::Finalized | OrderStatus::Failed)
	}

	/// Monitor order status and return the next action to take
	pub async fn check_order_status(
		&self,
		order_id: &str,
		attempt: u32,
	) -> JobResult<MonitoringResult> {
		tracing::debug!(
			target: TRACING_TARGET,
			order_id = %order_id,
			attempt = attempt,
			"Checking order status"
		);

		// Get current order from storage
		let current_order = match self.storage.get_order(order_id).await {
			Ok(Some(order)) => order,
			Ok(None) => {
				tracing::warn!(
					target: TRACING_TARGET,
					order_id = %order_id,
					"Order not found in storage, stopping monitoring"
				);
				return Ok(MonitoringResult::Completed);
			},
			Err(e) => {
				tracing::error!(
					target: TRACING_TARGET,
					order_id = %order_id,
					attempt = attempt,
					error = %e,
					"Failed to get order from storage"
				);
				return Ok(MonitoringResult::RetryLater {
					next_attempt: attempt + 1,
				});
			},
		};

		// Check if order is already in final status
		if Self::is_final_status(&current_order.status) {
			tracing::debug!(
				target: TRACING_TARGET,
				order_id = %order_id,
				status = ?current_order.status,
				"Order already in final status, stopping monitoring"
			);
			return Ok(MonitoringResult::Completed);
		}

		// Refresh order status from solver with timeout
		let refresh_result = tokio::time::timeout(
			Duration::from_secs(self.config.refresh_timeout_seconds),
			self.order_service.refresh_order(order_id),
		)
		.await;

		match refresh_result {
			Ok(Ok(Some(updated_order))) => {
				if updated_order.status != current_order.status {
					tracing::debug!(
						target: TRACING_TARGET,
						order_id = %order_id,
						old_status = ?current_order.status,
						new_status = ?updated_order.status,
						"Order status updated"
					);
				}

				// Check if the updated order is now in final status
				if Self::is_final_status(&updated_order.status) {
					tracing::debug!(
						target: TRACING_TARGET,
						order_id = %order_id,
						final_status = ?updated_order.status,
						"Order reached final status, monitoring complete"
					);
					Ok(MonitoringResult::Completed)
				} else {
					// Continue monitoring with backoff
					tracing::debug!(
						target: TRACING_TARGET,
						order_id = %order_id,
						current_status = ?updated_order.status,
						"Order still active, continuing monitoring"
					);
					Ok(MonitoringResult::ContinueMonitoring {
						next_attempt: attempt + 1,
					})
				}
			},
			Ok(Ok(None)) => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					"Order not found during status refresh, stopping monitoring"
				);
				Ok(MonitoringResult::Completed)
			},
			Ok(Err(e)) => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					attempt = attempt,
					error = %e,
					"Failed to refresh order status"
				);
				Ok(MonitoringResult::RetryLater {
					next_attempt: attempt + 1,
				})
			},
			Err(_timeout) => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					attempt = attempt,
					timeout_seconds = self.config.refresh_timeout_seconds,
					"Order status refresh timed out"
				);
				Ok(MonitoringResult::RetryLater {
					next_attempt: attempt + 1,
				})
			},
		}
	}
}

/// High-level handler that coordinates monitoring and scheduling
pub struct OrderStatusMonitorHandler {
	monitor: OrderMonitor,
	scheduler: MonitoringScheduler,
}

impl OrderStatusMonitorHandler {
	/// Create a new order status monitor handler with the given dependencies and configuration
	pub fn new(
		order_service: Arc<dyn OrderServiceTrait>,
		storage: Arc<dyn Storage>,
		job_scheduler: Arc<dyn JobScheduler>,
		config: Option<MonitoringConfig>,
	) -> Self {
		let config = config.unwrap_or_default();

		Self {
			monitor: OrderMonitor::new(order_service, storage, config.clone()),
			scheduler: MonitoringScheduler::new(config, job_scheduler),
		}
	}

	/// Handle order status monitoring with potential rescheduling
	/// This is the main entry point that includes both monitoring and scheduling logic
	pub async fn handle_order_monitoring(&self, order_id: &str, attempt: u32) -> JobResult<()> {
		tracing::debug!(
			target: TRACING_TARGET,
			order_id = %order_id,
			attempt = attempt,
			"Starting order monitoring handler"
		);

		// Check if we've hit max attempts
		if self.scheduler.is_max_attempts_reached(attempt) {
			tracing::debug!(
				target: TRACING_TARGET,
				order_id = %order_id,
				attempt = attempt,
				max_attempts = ?self.scheduler.config().max_attempts,
				"Maximum monitoring attempts reached, stopping"
			);
			return Ok(());
		}

		// Monitor the order status
		let result = self.monitor.check_order_status(order_id, attempt).await?;

		tracing::debug!(
			target: TRACING_TARGET,
			order_id = %order_id,
			attempt = attempt,
			result = ?result,
			"Monitoring result received"
		);

		// Handle the result and potentially schedule the next monitoring
		match result {
			MonitoringResult::Completed => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					"Order monitoring completed successfully"
				);
				Ok(())
			},
			MonitoringResult::ContinueMonitoring { next_attempt } => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					next_attempt = next_attempt,
					"Scheduling continued monitoring"
				);
				self.scheduler
					.schedule_next_monitoring(order_id, next_attempt)
					.await
			},
			MonitoringResult::RetryLater { next_attempt } => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					next_attempt = next_attempt,
					"Scheduling retry monitoring after error"
				);
				self.scheduler
					.schedule_next_monitoring(order_id, next_attempt)
					.await
			},
			MonitoringResult::MaxAttemptsReached => {
				tracing::debug!(
					target: TRACING_TARGET,
					order_id = %order_id,
					"Maximum monitoring attempts reached, stopping"
				);
				Ok(())
			},
		}
	}

	/// Generate job ID for a specific attempt (delegated to scheduler)
	pub fn generate_job_id(order_id: &str) -> String {
		MonitoringScheduler::generate_job_id(order_id)
	}

	/// Get the monitoring configuration
	pub fn config(&self) -> &MonitoringConfig {
		self.scheduler.config()
	}

	/// Check if the monitor is properly configured and ready
	pub fn is_ready(&self) -> bool {
		// All dependencies are injected at construction time
		true
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_is_final_status() {
		assert!(OrderMonitor::is_final_status(&OrderStatus::Finalized));
		assert!(OrderMonitor::is_final_status(&OrderStatus::Failed));
		assert!(!OrderMonitor::is_final_status(&OrderStatus::Created));
		assert!(!OrderMonitor::is_final_status(&OrderStatus::Pending));
		assert!(!OrderMonitor::is_final_status(&OrderStatus::Executed));
		assert!(!OrderMonitor::is_final_status(&OrderStatus::Settled));
	}

	#[test]
	fn test_job_id_generation() {
		assert_eq!(
			MonitoringScheduler::generate_job_id("order-123"),
			"order-monitor-order-123"
		);
	}

	#[test]
	fn test_monitoring_result_equality() {
		assert_eq!(MonitoringResult::Completed, MonitoringResult::Completed);
		assert_eq!(
			MonitoringResult::ContinueMonitoring { next_attempt: 1 },
			MonitoringResult::ContinueMonitoring { next_attempt: 1 }
		);
		assert_ne!(
			MonitoringResult::ContinueMonitoring { next_attempt: 1 },
			MonitoringResult::ContinueMonitoring { next_attempt: 2 }
		);
	}

	#[test]
	fn test_config_defaults() {
		let config = MonitoringConfig::default();
		assert_eq!(config.backoff_schedule, vec![5, 10, 15, 30, 60, 120, 300]);
		assert_eq!(config.max_backoff_seconds, 300);
		assert_eq!(config.max_attempts, None);
		assert_eq!(config.refresh_timeout_seconds, 5);
	}
}
