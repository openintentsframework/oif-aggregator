//! Job scheduling abstraction for clean dependency injection

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::types::{BackgroundJob, JobError};

/// Trait for scheduling background jobs - focused interface for dependency injection
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait JobScheduler: Send + Sync {
	/// Schedule a job to run after a delay
	async fn schedule_with_delay(
		&self,
		job: BackgroundJob,
		delay: Duration,
		job_id: Option<String>,
	) -> Result<String, JobError>;

	/// Schedule a recurring job
	async fn schedule_recurring(
		&self,
		interval_minutes: u64,
		job: BackgroundJob,
		description: String,
	) -> Result<String, JobError>;

	/// Cancel a scheduled job
	async fn cancel_job(&self, job_id: &str) -> Result<(), JobError>;

	/// Allow downcasting to concrete implementations
	fn as_any(&self) -> &dyn std::any::Any;
}

/// Simple JobScheduler that gets upgraded with a JobProcessor during initialization
/// No jobs are submitted before upgrade, so no queueing logic needed
#[derive(Default)]
pub struct UpgradableJobScheduler {
	/// The actual JobProcessor (None initially, Some after upgrade)
	processor: RwLock<Option<Arc<super::processor::JobProcessor>>>,
}

impl UpgradableJobScheduler {
	/// Create a new upgradable job scheduler (starts without a processor)
	pub fn new() -> Self {
		Self {
			processor: RwLock::new(None),
		}
	}

	/// Upgrade the scheduler with a JobProcessor during initialization
	pub async fn initialize_processor(&self, processor: Arc<super::processor::JobProcessor>) {
		let mut proc_guard = self.processor.write().await;
		*proc_guard = Some(processor);
		tracing::info!("JobScheduler upgraded with JobProcessor");
	}

	/// Get the processor (assumes it's been upgraded - for internal use)
	async fn get_processor(&self) -> Result<Arc<super::processor::JobProcessor>, JobError> {
		let proc_guard = self.processor.read().await;
		proc_guard
			.as_ref()
			.cloned()
			.ok_or_else(|| JobError::ProcessingFailed {
				message:
					"JobScheduler not upgraded with JobProcessor - this is a bug in initialization"
						.to_string(),
			})
	}
}

#[async_trait]
impl JobScheduler for UpgradableJobScheduler {
	async fn schedule_with_delay(
		&self,
		job: BackgroundJob,
		delay: Duration,
		job_id: Option<String>,
	) -> Result<String, JobError> {
		let processor = self.get_processor().await?;
		processor
			.submit_with_delay(job, delay, job_id, None)
			.await
			.map_err(|e| JobError::ProcessingFailed {
				message: format!("Failed to schedule job: {}", e),
			})
	}

	async fn schedule_recurring(
		&self,
		interval_minutes: u64,
		job: BackgroundJob,
		description: String,
	) -> Result<String, JobError> {
		let processor = self.get_processor().await?;
		processor
			.schedule_job(interval_minutes, job, description, None, false, None)
			.await
			.map_err(|e| JobError::ProcessingFailed {
				message: format!("Failed to schedule recurring job: {}", e),
			})
	}

	async fn cancel_job(&self, job_id: &str) -> Result<(), JobError> {
		let processor = self.get_processor().await?;
		processor
			.cancel_delayed_job(job_id)
			.await
			.map_err(|e| JobError::ProcessingFailed {
				message: format!("Failed to cancel job: {}", e),
			})
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

// Make the generated mock Send + Sync for testing
#[cfg(test)]
unsafe impl Send for MockJobScheduler {}
#[cfg(test)]
unsafe impl Sync for MockJobScheduler {}
