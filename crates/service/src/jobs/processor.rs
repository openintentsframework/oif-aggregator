//! Background job processor implementation

use async_trait::async_trait;
use futures::FutureExt;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, MissedTickBehavior};
use tracing::{debug, error, info, warn};

use super::types::{BackgroundJob, JobError, JobResult};

/// Trait for handling different types of background jobs
#[async_trait]
pub trait JobHandler: Send + Sync {
	/// Handle a background job
	async fn handle(&self, job: BackgroundJob) -> JobResult;
}

/// Retry policy configuration for jobs
#[derive(Debug, Clone)]
pub struct RetryPolicy {
	/// Maximum number of retry attempts
	pub max_retries: u32,
	/// Delay between retries in seconds
	pub retry_delay_seconds: u64,
	/// Exponential backoff multiplier (1.0 = fixed delay, >1.0 = exponential)
	pub backoff_multiplier: f32,
}

impl Default for RetryPolicy {
	fn default() -> Self {
		Self {
			max_retries: 1,
			retry_delay_seconds: 2,
			backoff_multiplier: 1.0,
		}
	}
}

/// Job execution status
#[derive(Debug, Clone)]
pub enum JobStatus {
	/// Job is queued but not yet started
	Pending,
	/// Job is currently being executed
	Running,
	/// Job completed successfully
	Completed,
	/// Job failed (no more retries)
	Failed { error: String },
	/// Job failed but will be retried
	Retrying {
		attempt: u32,
		next_retry_at: SystemTime,
		last_error: String,
	},
}

/// Complete information about a job
#[derive(Debug, Clone)]
pub struct JobInfo {
	/// Unique job identifier
	pub id: String,
	/// The job definition
	pub job: BackgroundJob,
	/// Current status
	pub status: JobStatus,
	/// When the job was submitted
	pub submitted_at: SystemTime,
	/// When the job started execution (if it has started)
	pub started_at: Option<SystemTime>,
	/// When the job completed (if it has completed)
	pub completed_at: Option<SystemTime>,
	/// Retry policy (if any)
	pub retry_policy: Option<RetryPolicy>,
}

/// Statistics about job info memory usage
#[derive(Debug, Clone)]
pub struct JobInfoStats {
	/// Current number of job info entries
	pub total_entries: usize,
	/// Maximum allowed entries before LRU eviction
	pub max_entries: usize,
	/// Breakdown by job status
	pub by_status: HashMap<String, usize>,
	/// TTL in minutes for completed/failed jobs
	pub ttl_minutes: u64,
	/// Cleanup interval in minutes
	pub cleanup_interval_minutes: u64,
}

/// A scheduled job with its configuration
#[derive(Debug, Clone)]
pub struct ScheduledJob {
	/// Unique identifier for this scheduled job (used for both unscheduling AND deduplication)
	pub id: String,
	/// Interval in minutes
	pub interval_minutes: u64,
	/// The job to execute
	pub job: BackgroundJob,
	/// Human-readable description
	pub description: String,
	/// Optional retry policy for this scheduled job
	pub retry_policy: Option<RetryPolicy>,
}

/// Job schedule for scheduling loop
#[derive(Debug, Clone)]
struct JobSchedule {
	/// The scheduled job configuration
	job: ScheduledJob,
	/// Type of job scheduling
	job_type: JobType,
	/// When this job should execute next
	next_execution: SystemTime,
	/// When this job last executed (for recurring jobs)
	last_execution: Option<SystemTime>,
}

/// Type of job scheduling
#[derive(Debug, Clone)]
enum JobType {
	/// Recurring job that runs every interval
	Recurring {
		interval: Duration,
		jitter: Duration,
	},
	/// One-time delayed job
	Delayed,
}

/// Configuration for the job processor
#[derive(Debug, Clone)]
pub struct JobProcessorConfig {
	/// Maximum number of jobs that can be queued
	pub queue_capacity: usize,
	/// Number of worker tasks to spawn
	pub worker_count: usize,
	/// Maximum number of concurrent scheduled jobs
	pub max_scheduled_jobs: usize,
	/// Maximum number of job info entries to keep in memory (LRU eviction)
	pub max_job_info_entries: usize,
	/// Automatic cleanup interval in minutes (0 = disabled)
	pub cleanup_interval_minutes: u64,
	/// Job info TTL in minutes - entries older than this are eligible for cleanup
	pub job_info_ttl_minutes: u64,
}

impl Default for JobProcessorConfig {
	fn default() -> Self {
		Self {
			queue_capacity: 1000,
			worker_count: 4,
			max_scheduled_jobs: 100,
			max_job_info_entries: 10000,  // Keep up to 10K job records
			cleanup_interval_minutes: 30, // Cleanup every 30 minutes
			job_info_ttl_minutes: 1440,   // Keep job info for 24 hours
		}
	}
}

/// Internal job execution request
#[derive(Debug, Clone)]
struct JobRequest {
	job: BackgroundJob,
	job_id: Option<String>,
	retry_policy: Option<RetryPolicy>,
	attempt: u32,
	original_job_info_id: Option<String>, // For tracking in job_info
}

/// Background job processor that handles job execution and scheduling
///
/// This processor uses tokio mpsc internally but hides the implementation
/// details to allow for future backend changes (e.g., Redis, database queues).
/// It also supports scheduling jobs to run at regular intervals.
///
/// Job deduplication is supported through optional job IDs to prevent
/// duplicate work from retries or concurrent submissions.
///
/// Retry policies and job status tracking are supported for observability.
pub struct JobProcessor {
	// Job execution
	sender: mpsc::Sender<JobRequest>,
	workers: Vec<JoinHandle<()>>,
	shutdown_sender: mpsc::Sender<()>,

	// Job scheduling
	config: JobProcessorConfig,
	job_schedules: Arc<RwLock<HashMap<String, JobSchedule>>>,
	scheduler_handle: Option<JoinHandle<()>>,
	scheduler_notify: Arc<tokio::sync::Notify>,
	next_job_id: AtomicU64,

	// Job deduplication
	active_job_ids: Arc<RwLock<HashSet<String>>>,

	// Job status tracking and memory management
	job_info: Arc<RwLock<HashMap<String, JobInfo>>>,
	next_job_info_id: AtomicU64,
	cleanup_handle: Option<JoinHandle<()>>,
}

impl JobProcessor {
	/// Create a new job processor with the given handler and configuration
	pub fn new(handler: Arc<dyn JobHandler>, config: JobProcessorConfig) -> JobResult<Self> {
		let (job_sender, job_receiver) = mpsc::channel::<JobRequest>(config.queue_capacity);
		let (shutdown_sender, _shutdown_receiver) = mpsc::channel::<()>(1);

		// Create shared job receiver
		let job_receiver = Arc::new(tokio::sync::Mutex::new(job_receiver));

		let active_job_ids = Arc::new(RwLock::new(HashSet::new()));
		let job_info = Arc::new(RwLock::new(HashMap::new()));

		// Spawn worker tasks
		let mut workers = Vec::new();
		for worker_id in 0..config.worker_count {
			let handler = Arc::clone(&handler);
			let job_receiver = Arc::clone(&job_receiver);

			let active_job_ids = Arc::clone(&active_job_ids);
			let job_info = Arc::clone(&job_info);
			let sender = job_sender.clone(); // For retry submissions

			let worker = tokio::spawn(async move {
				Self::worker_loop(
					worker_id,
					handler,
					job_receiver,
					active_job_ids,
					job_info,
					sender,
				)
				.await;
			});

			workers.push(worker);
		}

		// Start automatic cleanup task if enabled
		let cleanup_handle = if config.cleanup_interval_minutes > 0 {
			let job_info_clone = Arc::clone(&job_info);
			let cleanup_config = config.clone();

			let handle = tokio::spawn(async move {
				Self::cleanup_loop(job_info_clone, cleanup_config).await;
			});

			info!(
				"Started automatic job info cleanup every {} minutes (TTL: {} minutes)",
				config.cleanup_interval_minutes, config.job_info_ttl_minutes
			);

			Some(handle)
		} else {
			None
		};

		info!(
			"Started job processor with {} workers, scheduling support, and retry policies",
			config.worker_count
		);

		let job_schedules = Arc::new(RwLock::new(HashMap::new()));
		let scheduler_notify = Arc::new(tokio::sync::Notify::new());

		// Start the unified scheduler
		let scheduler_handle = {
			let sender = job_sender.clone();
			let schedules = Arc::clone(&job_schedules);
			let active_ids = Arc::clone(&active_job_ids);
			let notify = Arc::clone(&scheduler_notify);

			Some(tokio::spawn(async move {
				Self::scheduler_loop(sender, schedules, active_ids, notify).await;
			}))
		};

		Ok(Self {
			sender: job_sender,
			workers,
			shutdown_sender,
			config: config.clone(),
			job_schedules,
			scheduler_handle,
			scheduler_notify,
			next_job_id: AtomicU64::new(1),
			active_job_ids,
			job_info,
			next_job_info_id: AtomicU64::new(1),
			cleanup_handle,
		})
	}

	/// Scheduler loop that handles both recurring and delayed jobs
	async fn scheduler_loop(
		sender: mpsc::Sender<JobRequest>,
		job_schedules: Arc<RwLock<HashMap<String, JobSchedule>>>,
		active_job_ids: Arc<RwLock<HashSet<String>>>,
		notify: Arc<tokio::sync::Notify>,
	) {
		debug!("Starting scheduler loop");

		loop {
			let next_wake_time = {
				let schedules = job_schedules.read().await;
				if schedules.is_empty() {
					// No jobs scheduled, sleep for a reasonable interval
					SystemTime::now() + Duration::from_secs(5)
				} else {
					// Find the earliest execution time
					schedules
						.values()
						.map(|schedule| schedule.next_execution)
						.min()
						.unwrap_or_else(|| SystemTime::now() + Duration::from_secs(5))
				}
			};

			// Sleep until the next job needs to run OR until notified of new jobs
			let now = SystemTime::now();
			if next_wake_time > now {
				let sleep_duration = next_wake_time
					.duration_since(now)
					.unwrap_or(Duration::from_secs(1));

				// Wait for either timeout or notification
				tokio::select! {
					_ = tokio::time::sleep(sleep_duration) => {}
					_ = notify.notified() => {}
				}
			}

			// Execute all jobs that are due
			let jobs_to_execute = {
				let mut schedules = job_schedules.write().await;
				let now = SystemTime::now();
				let mut jobs = Vec::new();
				let mut jobs_to_remove = Vec::new();

				for (id, schedule) in schedules.iter_mut() {
					if schedule.next_execution <= now {
						// This job is due for execution
						jobs.push((id.clone(), schedule.job.clone(), schedule.job_type.clone()));

						match &schedule.job_type {
							JobType::Recurring { interval, jitter } => {
								// Update next execution time for recurring jobs
								schedule.last_execution = Some(now);
								schedule.next_execution = now + *interval + *jitter;
							},
							JobType::Delayed => {
								// Mark delayed jobs for removal
								jobs_to_remove.push(id.clone());
							},
						}
					}
				}

				// Remove completed delayed jobs
				for id in jobs_to_remove {
					schedules.remove(&id);
				}

				jobs
			};

			// Execute all due jobs
			for (schedule_id, scheduled_job, _job_type) in jobs_to_execute {
				debug!(
					"Executing scheduled job '{}' ({})",
					schedule_id, scheduled_job.description
				);

				// Check deduplication
				{
					let mut ids = active_job_ids.write().await;
					if !ids.insert(schedule_id.clone()) {
						debug!(
							"Scheduled job '{}' ({}) skipped - already running (dedup)",
							schedule_id, scheduled_job.description
						);
						continue;
					}
				}

				// Submit the job
				let job_request = JobRequest {
					job: scheduled_job.job.clone(),
					job_id: Some(schedule_id.clone()),
					retry_policy: scheduled_job.retry_policy.clone(),
					attempt: 0,
					original_job_info_id: None,
				};

				match tokio::time::timeout(Duration::from_millis(250), sender.send(job_request))
					.await
				{
					Ok(Ok(())) => {
						debug!(
							"Successfully submitted scheduled job '{}' ({})",
							schedule_id, scheduled_job.description
						);
					},
					Ok(Err(e)) => {
						error!(
							"Queue closed for scheduled job '{}' ({}): {}",
							schedule_id, scheduled_job.description, e
						);
						// Remove from active set since we failed to submit
						let mut ids = active_job_ids.write().await;
						ids.remove(&schedule_id);
					},
					Err(_) => {
						warn!(
							"Queue busy; dropping scheduled job '{}' ({})",
							schedule_id, scheduled_job.description
						);
						// Remove from active set since we failed to submit
						let mut ids = active_job_ids.write().await;
						ids.remove(&schedule_id);
					},
				}
			}
		}
	}

	/// Submit a job for background processing with retry policy and optional job ID
	pub async fn submit(
		&self,
		job: BackgroundJob,
		job_id: Option<String>,
		retry_policy: Option<RetryPolicy>,
	) -> JobResult<String> {
		debug!(
			"Submitting job: {} (ID: {:?}, retries: {:?})",
			job.description(),
			job_id,
			retry_policy
		);

		// Check for duplicate job ID if provided and register it atomically
		if let Some(ref id) = job_id {
			let mut active_jobs = self.active_job_ids.write().await;
			if active_jobs.contains(id) {
				return Err(JobError::Duplicate { id: id.clone() });
			}
			// Reserve the ID immediately to prevent race conditions
			active_jobs.insert(id.clone());
		}

		// Always create job info for tracking
		let info_id = format!(
			"job-{}",
			self.next_job_info_id.fetch_add(1, Ordering::SeqCst)
		);

		let job_info = JobInfo {
			id: info_id.clone(),
			job: job.clone(),
			status: JobStatus::Pending,
			submitted_at: SystemTime::now(),
			started_at: None,
			completed_at: None,
			retry_policy: retry_policy.clone(),
		};

		let mut info_map = self.job_info.write().await;

		// Check if we need to evict old entries (LRU)
		if info_map.len() >= self.config.max_job_info_entries {
			self.evict_oldest_entries(&mut info_map);
		}

		info_map.insert(info_id.clone(), job_info);
		drop(info_map);

		// Clone job_id for cleanup purposes (before it gets moved into JobRequest)
		let job_id_clone = job_id.clone();

		let job_request = JobRequest {
			job: job.clone(),
			job_id,
			retry_policy,
			attempt: 0,
			original_job_info_id: Some(info_id.clone()),
		};

		match self.sender.send(job_request).await {
			Ok(()) => {
				debug!("Job submitted successfully: {}", job.description());
				Ok(info_id)
			},
			Err(_) => {
				warn!("Failed to submit job: {}", job.description());
				// Clean up the reserved job ID since submission failed
				if let Some(ref id) = job_id_clone {
					let mut active_jobs = self.active_job_ids.write().await;
					active_jobs.remove(id);
					debug!(
						"Removed job ID '{}' from active set due to submission failure",
						id
					);
				}
				Err(JobError::QueueFull)
			},
		}
	}

	/// Submit a job to be executed after a delay using the unified scheduler
	pub async fn submit_with_delay(
		&self,
		job: BackgroundJob,
		delay: Duration,
		job_id: Option<String>,
		retry_policy: Option<RetryPolicy>,
	) -> JobResult<String> {
		debug!(
			"Submitting delayed job: {} (delay: {:?}, ID: {:?}, retries: {:?})",
			job.description(),
			delay,
			job_id,
			retry_policy
		);

		// Generate a unique schedule ID for this delayed job
		let schedule_id = job_id.unwrap_or_else(|| {
			format!(
				"delayed-job-{}",
				self.next_job_id.fetch_add(1, Ordering::SeqCst)
			)
		});

		// Check if schedule ID already exists
		{
			let schedules = self.job_schedules.read().await;
			if schedules.contains_key(&schedule_id) {
				return Err(JobError::Duplicate { id: schedule_id });
			}
		}

		// Create the scheduled job
		let scheduled_job = ScheduledJob {
			id: schedule_id.clone(),
			interval_minutes: 0, // Not used for delayed jobs
			job: job.clone(),
			description: format!("Delayed job: {}", job.description()),
			retry_policy: retry_policy.clone(),
		};

		// Create the job schedule for delayed execution
		let job_schedule = JobSchedule {
			job: scheduled_job,
			job_type: JobType::Delayed,
			next_execution: SystemTime::now() + delay,
			last_execution: None,
		};

		// Add to the unified scheduler
		{
			let mut schedules = self.job_schedules.write().await;
			schedules.insert(schedule_id.clone(), job_schedule);
		}

		// Notify the scheduler that a new job has been added
		self.scheduler_notify.notify_one();

		debug!("Delayed job scheduled successfully: {}", job.description());
		Ok(schedule_id)
	}

	/// Submit a job without waiting (fire and forget) with retry policy and optional job ID
	pub fn submit_nowait(
		&self,
		job: BackgroundJob,
		job_id: Option<String>,
		retry_policy: Option<RetryPolicy>,
	) -> JobResult<String> {
		debug!(
			"Submitting job (no wait): {} (ID: {:?}, retries: {:?})",
			job.description(),
			job_id,
			retry_policy
		);

		// Check for duplicate job ID if provided and register it atomically
		if let Some(ref id) = job_id {
			if let Ok(mut active_jobs) = self.active_job_ids.try_write() {
				if active_jobs.contains(id) {
					return Err(JobError::Duplicate { id: id.clone() });
				}
				// Reserve the ID immediately to prevent race conditions
				active_jobs.insert(id.clone());
			}
			// If we can't get the lock, proceed anyway to avoid blocking
		}

		// Always create job info for tracking
		let info_id = format!(
			"job-{}",
			self.next_job_info_id.fetch_add(1, Ordering::SeqCst)
		);

		let job_info = JobInfo {
			id: info_id.clone(),
			job: job.clone(),
			status: JobStatus::Pending,
			submitted_at: SystemTime::now(),
			started_at: None,
			completed_at: None,
			retry_policy: retry_policy.clone(),
		};

		// Try to add to job info (non-blocking)
		if let Ok(mut info_map) = self.job_info.try_write() {
			// Check if we need to evict old entries (LRU)
			if info_map.len() >= self.config.max_job_info_entries {
				self.evict_oldest_entries(&mut info_map);
			}
			info_map.insert(info_id.clone(), job_info);
		}

		// Clone job_id for cleanup purposes (before it gets moved into JobRequest)
		let job_id_clone = job_id.clone();

		let job_request = JobRequest {
			job: job.clone(),
			job_id,
			retry_policy,
			attempt: 0,
			original_job_info_id: Some(info_id.clone()),
		};

		match self.sender.try_send(job_request) {
			Ok(()) => {
				debug!(
					"Job submitted successfully (no wait): {}",
					job.description()
				);
				Ok(info_id)
			},
			Err(e) => {
				warn!("Failed to submit job: {} - {}", job.description(), e);

				// Clean up the reserved job ID since submission failed
				if let Some(ref id) = job_id_clone {
					// Use try_write to avoid blocking in the nowait method
					if let Ok(mut active_jobs) = self.active_job_ids.try_write() {
						active_jobs.remove(id);
						debug!(
							"Removed job ID '{}' from active set due to submission failure",
							id
						);
					}
				}

				match e {
					mpsc::error::TrySendError::Full(_) => Err(JobError::QueueFull),
					mpsc::error::TrySendError::Closed(_) => Err(JobError::ShuttingDown),
				}
			},
		}
	}

	/// Get the current queue capacity (number of jobs that can be queued)
	pub fn queue_capacity(&self) -> usize {
		self.sender.capacity()
	}

	/// Get job information by job info ID
	pub async fn get_job_info(&self, job_info_id: &str) -> Option<JobInfo> {
		self.job_info.read().await.get(job_info_id).cloned()
	}

	/// Get all tracked job information
	pub async fn get_all_job_info(&self) -> Vec<JobInfo> {
		self.job_info.read().await.values().cloned().collect()
	}

	/// Get job information for jobs with a specific status
	pub async fn get_jobs_by_status(&self, status_filter: JobStatus) -> Vec<JobInfo> {
		self.job_info
			.read()
			.await
			.values()
			.filter(|info| {
				std::mem::discriminant(&info.status) == std::mem::discriminant(&status_filter)
			})
			.cloned()
			.collect()
	}

	/// Remove completed or failed job information (cleanup)
	pub async fn cleanup_completed_jobs(&self, older_than: SystemTime) -> usize {
		let mut info_map = self.job_info.write().await;
		let original_count = info_map.len();

		info_map.retain(|_, info| {
			match &info.status {
				JobStatus::Completed | JobStatus::Failed { .. } => info
					.completed_at
					.map_or(true, |completed| completed > older_than),
				_ => true, // Keep pending, running, retrying jobs
			}
		});

		original_count - info_map.len()
	}

	/// Get job info memory usage statistics
	pub async fn get_job_info_stats(&self) -> JobInfoStats {
		let info_map = self.job_info.read().await;
		let total_entries = info_map.len();
		let mut by_status = HashMap::new();

		for info in info_map.values() {
			let status_key = match &info.status {
				JobStatus::Pending => "pending",
				JobStatus::Running => "running",
				JobStatus::Completed => "completed",
				JobStatus::Failed { .. } => "failed",
				JobStatus::Retrying { .. } => "retrying",
			};
			*by_status.entry(status_key.to_string()).or_insert(0) += 1;
		}

		JobInfoStats {
			total_entries,
			max_entries: self.config.max_job_info_entries,
			by_status,
			ttl_minutes: self.config.job_info_ttl_minutes,
			cleanup_interval_minutes: self.config.cleanup_interval_minutes,
		}
	}

	/// Enhanced cleanup with TTL-based and status-based removal
	pub async fn cleanup_old_job_info(&self) -> usize {
		let ttl_cutoff =
			SystemTime::now() - Duration::from_secs(self.config.job_info_ttl_minutes * 60);
		let mut info_map = self.job_info.write().await;
		let original_count = info_map.len();

		info_map.retain(|_, info| {
			// Always keep active jobs (pending, running, retrying)
			match &info.status {
				JobStatus::Pending | JobStatus::Running => true,
				JobStatus::Retrying { .. } => true,
				JobStatus::Completed | JobStatus::Failed { .. } => {
					// For completed/failed jobs, check TTL
					info.submitted_at > ttl_cutoff
				},
			}
		});

		let removed = original_count - info_map.len();
		if removed > 0 {
			debug!(
				"Cleaned up {} old job info entries (TTL: {} minutes)",
				removed, self.config.job_info_ttl_minutes
			);
		}
		removed
	}

	/// Optimized LRU eviction when capacity limits are reached
	fn evict_oldest_entries(&self, info_map: &mut HashMap<String, JobInfo>) {
		Self::evict_lru_entries_optimized(
			info_map,
			self.config.max_job_info_entries,
			Some("capacity limit"),
		)
	}

	/// Schedule a job to run at the specified interval
	///
	/// # Parameters
	/// - `interval_minutes`: How often to run the job
	/// - `job`: The job to execute
	/// - `description`: Human-readable description
	/// - `schedule_id`: Optional ID for this schedule. Used for both scheduling management AND deduplication. If provided, prevents both duplicate schedules and overlapping executions. Auto-generated if None.
	/// - `run_immediately`: If true, runs the job immediately before starting the recurring schedule
	/// - `retry_policy`: Optional retry policy for this scheduled job
	pub async fn schedule_job(
		&self,
		interval_minutes: u64,
		job: BackgroundJob,
		description: String,
		schedule_id: Option<String>,
		run_immediately: bool,
		retry_policy: Option<RetryPolicy>,
	) -> Result<String, String> {
		// Check if we've reached the maximum number of scheduled jobs
		let jobs_count = self.job_schedules.read().await.len();
		if jobs_count >= self.config.max_scheduled_jobs {
			return Err(format!(
				"Cannot schedule more jobs. Maximum limit of {} reached",
				self.config.max_scheduled_jobs
			));
		}

		// Generate schedule ID if not provided
		let schedule_id = schedule_id
			.unwrap_or_else(|| format!("job-{}", self.next_job_id.fetch_add(1, Ordering::SeqCst)));

		// Check if schedule ID already exists
		{
			let schedules = self.job_schedules.read().await;
			if schedules.contains_key(&schedule_id) {
				return Err(format!(
					"Scheduled job with ID '{}' already exists",
					schedule_id
				));
			}
		}

		let scheduled_job = ScheduledJob {
			id: schedule_id.clone(),
			interval_minutes,
			job: job.clone(),
			description: description.clone(),
			retry_policy: retry_policy.clone(),
		};

		// Calculate interval and jitter
		let interval_duration = Duration::from_secs(interval_minutes * 60);
		let max_jitter_seconds = if interval_minutes <= 10 {
			// Short intervals: small jitter (5 seconds max)
			(interval_duration.as_secs() / 12).min(5)
		} else {
			// Long intervals: larger jitter for load distribution (20 seconds max)
			(interval_duration.as_secs() / 12).min(20)
		};
		let jitter = Self::calculate_jitter(&schedule_id, max_jitter_seconds);

		// Calculate next execution time
		let next_execution = if run_immediately {
			SystemTime::now() // Run immediately
		} else {
			SystemTime::now() + interval_duration + jitter
		};

		// Create the job schedule for recurring execution
		let job_schedule = JobSchedule {
			job: scheduled_job,
			job_type: JobType::Recurring {
				interval: interval_duration,
				jitter,
			},
			next_execution,
			last_execution: None,
		};

		// Add to the scheduler
		{
			let mut schedules = self.job_schedules.write().await;
			schedules.insert(schedule_id.clone(), job_schedule);
		}

		info!(
			"Scheduled job '{}' ({}) to run every {} minutes (ID used for both scheduling and deduplication)",
			schedule_id, description, interval_minutes
		);

		Ok(schedule_id)
	}

	/// Cancel a delayed job by its ID
	pub async fn cancel_delayed_job(&self, schedule_id: &str) -> Result<(), String> {
		// Remove from scheduled jobs
		let removed_schedule = {
			let mut schedules = self.job_schedules.write().await;
			schedules.remove(schedule_id)
		};

		if let Some(schedule) = removed_schedule {
			// Verify it was actually a delayed job
			match schedule.job_type {
				JobType::Delayed => {
					info!("Cancelled delayed job '{}'", schedule_id);
					Ok(())
				},
				JobType::Recurring { .. } => {
					// Put it back if it wasn't a delayed job
					let mut schedules = self.job_schedules.write().await;
					schedules.insert(schedule_id.to_string(), schedule);
					Err(format!("Job '{}' is a recurring job, not a delayed job. Use unschedule_job() instead.", schedule_id))
				},
			}
		} else {
			Err(format!("Delayed job with ID '{}' not found", schedule_id))
		}
	}

	/// Unschedule a job by its ID
	pub async fn unschedule_job(&self, schedule_id: &str) -> Result<(), String> {
		// Remove from scheduled jobs
		let removed_schedule = {
			let mut schedules = self.job_schedules.write().await;
			schedules.remove(schedule_id)
		};

		if removed_schedule.is_some() {
			info!("Unscheduled job '{}'", schedule_id);
			Ok(())
		} else {
			Err(format!("Job with ID '{}' not found", schedule_id))
		}
	}

	/// Get all scheduled jobs
	pub async fn get_scheduled_jobs(&self) -> Vec<ScheduledJob> {
		self.job_schedules
			.read()
			.await
			.values()
			.map(|schedule| schedule.job.clone())
			.collect()
	}

	/// Get a specific scheduled job by ID
	pub async fn get_scheduled_job(&self, schedule_id: &str) -> Option<ScheduledJob> {
		self.job_schedules
			.read()
			.await
			.get(schedule_id)
			.map(|schedule| schedule.job.clone())
	}

	/// Background cleanup loop that runs periodically
	async fn cleanup_loop(
		job_info: Arc<RwLock<HashMap<String, JobInfo>>>,
		config: JobProcessorConfig,
	) {
		let mut interval = interval(Duration::from_secs(config.cleanup_interval_minutes * 60));
		interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

		info!(
			"Started job info cleanup loop (interval: {} minutes, TTL: {} minutes)",
			config.cleanup_interval_minutes, config.job_info_ttl_minutes
		);

		loop {
			interval.tick().await;

			let ttl_cutoff =
				SystemTime::now() - Duration::from_secs(config.job_info_ttl_minutes * 60);
			let mut info_map = job_info.write().await;

			let original_count = info_map.len();
			info_map.retain(|_, info| {
				// Always keep active jobs (pending, running, retrying)
				match &info.status {
					JobStatus::Pending | JobStatus::Running => true,
					JobStatus::Retrying { .. } => true,
					JobStatus::Completed | JobStatus::Failed { .. } => {
						// For completed/failed jobs, check TTL
						info.submitted_at > ttl_cutoff
					},
				}
			});

			let removed = original_count - info_map.len();
			if removed > 0 {
				info!(
					"Automatic cleanup removed {} old job info entries (kept {}, TTL: {} minutes)",
					removed,
					info_map.len(),
					config.job_info_ttl_minutes
				);
			} else {
				debug!(
					"Automatic cleanup: no old entries to remove (current: {} entries)",
					info_map.len()
				);
			}

			// Additional check: if we're still over capacity, do LRU eviction
			if info_map.len() > config.max_job_info_entries {
				Self::evict_lru_entries_optimized(
					&mut info_map,
					config.max_job_info_entries,
					Some("cleanup over capacity"),
				);
			}
		}
	}

	/// Optimized consolidated LRU eviction method
	///
	/// Fixes performance issues:
	/// - Uses partial sorting (select_nth_unstable) instead of full sort
	/// - Uses HashSet for O(1) lookups instead of Vec contains()
	/// - Consolidates duplicate logic between methods
	fn evict_lru_entries_optimized(
		info_map: &mut HashMap<String, JobInfo>,
		max_entries: usize,
		reason: Option<&str>,
	) {
		if info_map.len() <= max_entries {
			return;
		}

		// Calculate target removal count
		let target_size = if info_map.len() == max_entries {
			(max_entries * 9) / 10 // Remove 10% when at exact capacity
		} else {
			max_entries // Remove down to max when over capacity
		};
		let remove_count = info_map.len() - target_size;

		if remove_count == 0 {
			return;
		}

		// Collect entries and find the oldest ones efficiently
		let mut entries: Vec<_> = info_map.iter().collect();

		// Use partial sort for better performance - only sort the portion we need
		if remove_count < entries.len() {
			entries.select_nth_unstable_by_key(remove_count, |(_, info)| info.submitted_at);
		} else {
			entries.sort_unstable_by_key(|(_, info)| info.submitted_at);
		}

		// Use HashSet for O(1) lookups instead of Vec contains()
		let mut to_remove = HashSet::new();
		let mut removed = 0;

		// First pass: prioritize completed/failed jobs from oldest entries
		for (id, info) in entries.iter().take(remove_count * 2) {
			// Look at 2x to find completed jobs
			if removed >= remove_count {
				break;
			}
			match info.status {
				JobStatus::Completed | JobStatus::Failed { .. } => {
					to_remove.insert((*id).clone());
					removed += 1;
				},
				_ => continue,
			}
		}

		// Second pass: if we still need to remove more, take oldest regardless of status
		if removed < remove_count {
			for (id, _) in entries.iter().take(remove_count) {
				if removed >= remove_count {
					break;
				}
				if !to_remove.contains(*id) {
					to_remove.insert((*id).clone());
					removed += 1;
				}
			}
		}

		// Remove the selected entries
		for id in &to_remove {
			info_map.remove(id);
		}

		if removed > 0 {
			let reason_str = reason.unwrap_or("LRU eviction");
			debug!(
				"Evicted {} oldest job info entries ({}): {} -> {} entries",
				removed,
				reason_str,
				info_map.len() + removed,
				info_map.len()
			);
		}
	}

	/// Gracefully shutdown the job processor
	pub async fn shutdown(mut self) -> JobResult {
		info!("Shutting down job processor...");

		// Cancel the scheduler
		if let Some(scheduler_handle) = self.scheduler_handle.take() {
			debug!("Cancelling scheduler");
			scheduler_handle.abort();
		}

		// Cancel cleanup task if running
		if let Some(cleanup_handle) = self.cleanup_handle.take() {
			debug!("Cancelling automatic cleanup task");
			cleanup_handle.abort();
		}

		// Clear scheduled jobs and active job IDs
		{
			let mut schedules = self.job_schedules.write().await;
			schedules.clear();
		}
		{
			let mut active_ids = self.active_job_ids.write().await;
			active_ids.clear();
		}

		// Signal all workers to shutdown
		drop(self.shutdown_sender);

		// Close the job queue
		drop(self.sender);

		// Wait for all workers to complete
		for (i, worker) in self.workers.drain(..).enumerate() {
			if let Err(e) = worker.await {
				error!("Worker {} failed to shutdown cleanly: {}", i, e);
			} else {
				debug!("Worker {} shutdown cleanly", i);
			}
		}

		info!("Job processor shutdown complete");
		Ok(())
	}

	/// Calculate stable jitter based on schedule ID to prevent thundering herd
	fn calculate_jitter(schedule_id: &str, max_jitter_seconds: u64) -> Duration {
		let mut hasher = DefaultHasher::new();
		schedule_id.hash(&mut hasher);
		let hash = hasher.finish();

		// Use modulo to get a stable value within the jitter window
		let jitter_seconds = hash % (max_jitter_seconds + 1);
		Duration::from_secs(jitter_seconds)
	}

	/// Worker loop that processes jobs
	async fn worker_loop(
		worker_id: usize,
		handler: Arc<dyn JobHandler>,
		job_receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<JobRequest>>>,
		active_job_ids: Arc<RwLock<HashSet<String>>>,
		job_info: Arc<RwLock<HashMap<String, JobInfo>>>,
		sender: mpsc::Sender<JobRequest>, // For retry submissions
	) {
		debug!("Worker {} started", worker_id);

		// Process jobs until the queue is closed and drained
		while let Some(job_request) = job_receiver.lock().await.recv().await {
			debug!(
				"Worker {} processing job: {} (ID: {:?}, attempt: {})",
				worker_id,
				job_request.job.description(),
				job_request.job_id,
				job_request.attempt
			);

			// Note: Job deduplication is already handled in submit() method
			// where job IDs are registered atomically to prevent race conditions
			let should_process = true;

			if should_process {
				// Update job status to Running with proper error handling
				if let Some(ref info_id) = job_request.original_job_info_id {
					match job_info.try_write() {
						Ok(mut info_map) => {
							if let Some(info) = info_map.get_mut(info_id) {
								info.status = JobStatus::Running;
								info.started_at = Some(SystemTime::now());
							} else {
								warn!(
									"Job info entry '{}' not found when updating to Running status",
									info_id
								);
							}
						},
						Err(_) => {
							debug!("Could not acquire job_info lock to update job '{}' to Running status", info_id);
						},
					}
				}

				let start_time = std::time::Instant::now();

				// Use panic protection to prevent one bad job from killing the worker
				let fut = handler.handle(job_request.job.clone());
				let result = match AssertUnwindSafe(fut).catch_unwind().await {
					Ok(Ok(())) => Ok(()),
					Ok(Err(e)) => Err(e),
					Err(_) => {
						error!(
							"Job handler panicked for job: {}",
							job_request.job.description()
						);
						Err(JobError::ProcessingFailed {
							message: "Job handler panicked".to_string(),
						})
					},
				};

				let duration = start_time.elapsed();
				let completion_time = SystemTime::now();
				let job_succeeded = result.is_ok();

				match result {
					Ok(()) => {
						debug!(
							"Worker {} completed job: {} (took {:?})",
							worker_id,
							job_request.job.description(),
							duration
						);

						// Update job status to Completed with proper error handling
						if let Some(ref info_id) = job_request.original_job_info_id {
							match job_info.try_write() {
								Ok(mut info_map) => {
									if let Some(info) = info_map.get_mut(info_id) {
										info.status = JobStatus::Completed;
										info.completed_at = Some(completion_time);
									} else {
										warn!("Job info entry '{}' not found when updating to Completed status", info_id);
									}
								},
								Err(_) => {
									debug!("Could not acquire job_info lock to update job '{}' to Completed status", info_id);
								},
							}
						}
					},
					Err(e) => {
						error!(
							"Worker {} failed to process job: {} - {} (attempt {})",
							worker_id,
							job_request.job.description(),
							e,
							job_request.attempt + 1
						);

						// Handle retry logic
						let should_retry = if let Some(ref retry_policy) = job_request.retry_policy
						{
							job_request.attempt < retry_policy.max_retries
						} else {
							false
						};

						if should_retry {
							let retry_policy = job_request.retry_policy.as_ref().unwrap();
							let next_attempt = job_request.attempt + 1;

							// Calculate retry delay with backoff
							let delay_seconds = (retry_policy.retry_delay_seconds as f32
								* retry_policy
									.backoff_multiplier
									.powi(job_request.attempt as i32)) as u64;

							let next_retry_at =
								SystemTime::now() + Duration::from_secs(delay_seconds);

							info!(
								"Worker {} scheduling retry for job: {} (attempt {}/{}) in {}s",
								worker_id,
								job_request.job.description(),
								next_attempt + 1,
								retry_policy.max_retries + 1,
								delay_seconds
							);

							// Update job status to Retrying with proper error handling
							if let Some(ref info_id) = job_request.original_job_info_id {
								match job_info.try_write() {
									Ok(mut info_map) => {
										if let Some(info) = info_map.get_mut(info_id) {
											info.status = JobStatus::Retrying {
												attempt: next_attempt,
												next_retry_at,
												last_error: e.to_string(),
											};
										} else {
											warn!("Job info entry '{}' not found when updating to Retrying status", info_id);
										}
									},
									Err(_) => {
										debug!("Could not acquire job_info lock to update job '{}' to Retrying status", info_id);
									},
								}
							}

							// Schedule retry
							let retry_sender = sender.clone();
							let retry_request = JobRequest {
								job: job_request.job.clone(),
								job_id: job_request.job_id.clone(),
								retry_policy: job_request.retry_policy.clone(),
								attempt: next_attempt,
								original_job_info_id: job_request.original_job_info_id.clone(),
							};

							// Clone references for the retry task
							let active_job_ids_retry = Arc::clone(&active_job_ids);
							let job_info_retry = Arc::clone(&job_info);

							tokio::spawn(async move {
								sleep(Duration::from_secs(delay_seconds)).await;

								if let Err(retry_err) =
									retry_sender.send(retry_request.clone()).await
								{
									error!("Failed to submit retry: {}", retry_err);

									// Remove job ID since retry couldn't be queued
									if let Some(ref id) = retry_request.job_id {
										let mut active_ids = active_job_ids_retry.write().await;
										active_ids.remove(id);
										debug!("Removed job ID '{}' from active set due to retry submission failure", id);
									}

									// Update job status to Failed since retry failed
									if let Some(ref info_id) = retry_request.original_job_info_id {
										if let Ok(mut info_map) = job_info_retry.try_write() {
											if let Some(info) = info_map.get_mut(info_id) {
												info.status = JobStatus::Failed {
													error: format!(
														"Retry submission failed: {}",
														retry_err
													),
												};
												info.completed_at = Some(SystemTime::now());
											}
										}
									}
								}
							});
						} else {
							// No more retries, mark as failed with proper error handling
							if let Some(ref info_id) = job_request.original_job_info_id {
								match job_info.try_write() {
									Ok(mut info_map) => {
										if let Some(info) = info_map.get_mut(info_id) {
											info.status = JobStatus::Failed {
												error: e.to_string(),
											};
											info.completed_at = Some(completion_time);
										} else {
											warn!("Job info entry '{}' not found when updating to Failed status", info_id);
										}
									},
									Err(_) => {
										debug!("Could not acquire job_info lock to update job '{}' to Failed status", info_id);
									},
								}
							}
						}
					},
				}

				// Remove job ID from active set when done (unless retrying)
				if let Some(ref id) = job_request.job_id {
					let should_remove = job_succeeded
						|| job_request
							.retry_policy
							.as_ref()
							.map_or(true, |p| job_request.attempt >= p.max_retries);

					if should_remove {
						let mut active_ids = active_job_ids.write().await;
						active_ids.remove(id);
						debug!(
							"Worker {} removed job ID '{}' from active set",
							worker_id, id
						);
					}
				}
			}
		}

		debug!("Worker {} stopped", worker_id);
	}
}
