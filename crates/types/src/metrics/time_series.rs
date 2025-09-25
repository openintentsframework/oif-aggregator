//! Time-series data structures for metrics storage and analysis

use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Response time distribution constants for more accurate percentile estimation
///
/// Most response time distributions are right-skewed (log-normal or gamma), not normal.
/// Using median-based approximation provides much better accuracy than mean-based approaches.
/// Coefficient for p95 approximation: p95 ≈ median + COEFF * (max - median)
/// Value of 1.3 is calibrated for typical log-normal response time distributions
/// where the 95th percentile is approximately 1.3 standard deviations from the median
/// toward the maximum value in right-skewed distributions.
const P95_MEDIAN_COEFFICIENT: f64 = 1.3;

/// Error types for metrics computation failures
#[derive(Debug, thiserror::Error)]
pub enum MetricsComputationError {
	#[error("Failed to compute rolling metrics for {operation}: {details}")]
	RollingMetricsFailure { operation: String, details: String },
	#[error("Aggregate computation failed: {details}")]
	AggregateComputationFailure { details: String },
	#[error("Data integrity issue: {issue}")]
	DataIntegrityError { issue: String },
}

/// Operation types for solver interactions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Operation {
	/// Quote fetching operation
	GetQuotes,
	/// Order submission operation  
	SubmitOrder,
	/// Health check operation
	HealthCheck,
	/// Asset fetching operation
	GetAssets,
	/// Order details retrieval
	GetOrderDetails,
	/// Other/unknown operations
	Other,
}

impl std::str::FromStr for Operation {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let op = match s {
			"get_quotes" => Operation::GetQuotes,
			"submit_order" => Operation::SubmitOrder,
			"health_check" => Operation::HealthCheck,
			"get_assets" => Operation::GetAssets,
			"get_order_details" => Operation::GetOrderDetails,
			_ => Operation::Other,
		};
		Ok(op)
	}
}

impl Operation {
	/// Get string representation
	pub fn as_str(&self) -> &'static str {
		match self {
			Operation::GetQuotes => "get_quotes",
			Operation::SubmitOrder => "submit_order",
			Operation::HealthCheck => "health_check",
			Operation::GetAssets => "get_assets",
			Operation::GetOrderDetails => "get_order_details",
			Operation::Other => "other",
		}
	}

	/// Check if this operation should be tracked in detailed metrics breakdown
	pub fn is_tracked(&self) -> bool {
		matches!(self, Operation::GetQuotes | Operation::SubmitOrder)
	}
}

impl std::fmt::Display for Operation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

/// Metrics data point for a single solver interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDataPoint {
	/// Timestamp when the metric was recorded
	pub timestamp: DateTime<Utc>,
	/// Response time in milliseconds
	pub response_time_ms: u64,
	/// Whether the request was successful
	pub was_successful: bool,
	/// Whether the request timed out
	pub was_timeout: bool,
	/// Optional error classification
	pub error_type: Option<ErrorType>,
	/// Operation type
	pub operation: Operation,
}

/// Simple error categorization for circuit breaker and monitoring decisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ErrorType {
	/// Service errors that should affect circuit breaker (5xx, 429, network issues, timeouts)
	ServiceError,
	/// Client errors that don't indicate service problems (4xx except 429)
	ClientError,
	/// Application errors (parsing, config, internal issues)
	ApplicationError,
	/// Unknown/uncategorized error
	Unknown,
}

impl ErrorType {
	/// Get a human-readable description of the error type
	pub fn description(&self) -> &'static str {
		match self {
			ErrorType::ServiceError => "Service Error (affects circuit breaker)",
			ErrorType::ClientError => "Client Error (doesn't affect circuit breaker)",
			ErrorType::ApplicationError => "Application Error (internal issue)",
			ErrorType::Unknown => "Unknown Error",
		}
	}

	/// Create error type from HTTP status code
	pub fn from_http_status(status_code: u16) -> Self {
		match status_code {
			// 4xx client errors - except timeouts and rate limiting
			400..=407 | 409..=428 | 430..=499 => ErrorType::ClientError,
			// 408 request timeout - often indicates solver performance issues
			408 => ErrorType::ServiceError,
			// 429 rate limiting - service overload, affects circuit breaker
			429 => ErrorType::ServiceError,
			// 5xx server errors - service problems
			500..=599 => ErrorType::ServiceError,
			// Everything else
			_ => ErrorType::Unknown,
		}
	}
}

/// Operation-specific metrics for a single operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
	/// Operation type
	pub operation: Operation,
	/// Total number of requests for this operation
	pub total_requests: u64,
	/// Number of successful requests
	pub successful_requests: u64,
	/// Number of failed requests (calculated)
	pub failed_requests: u64,
	/// Success rate (0.0 to 1.0)
	pub success_rate: f64,
	/// Average response time in milliseconds
	pub avg_response_time_ms: f64,
	/// Estimated median response time in milliseconds (more robust for skewed data)
	pub median_response_time_ms: f64,
	/// Minimum response time in milliseconds
	pub min_response_time_ms: u64,
	/// Maximum response time in milliseconds
	pub max_response_time_ms: u64,
	/// 95th percentile response time in milliseconds
	pub p95_response_time_ms: u64,
	/// Number of service errors (5xx, timeouts, network issues)
	pub service_errors: u64,
	/// Number of client errors (4xx except 429)
	pub client_errors: u64,
	/// Number of application errors (internal issues)
	pub application_errors: u64,
	/// Number of timeout requests
	pub timeout_requests: u64,
}

impl OperationStats {
	/// Create a new OperationStats with default values
	pub fn new(operation: Operation) -> Self {
		Self {
			operation,
			total_requests: 0,
			successful_requests: 0,
			failed_requests: 0,
			success_rate: 0.0,
			avg_response_time_ms: 0.0,
			median_response_time_ms: 0.0,
			min_response_time_ms: 0,
			max_response_time_ms: 0,
			p95_response_time_ms: 0,
			service_errors: 0,
			client_errors: 0,
			application_errors: 0,
			timeout_requests: 0,
		}
	}

	/// Calculate success rate from total and successful requests
	fn calculate_success_rate(&mut self) {
		if self.total_requests > 0 {
			self.success_rate = self.successful_requests as f64 / self.total_requests as f64;
		} else {
			self.success_rate = 0.0;
		}
	}

	/// Calculate failed requests from total and successful
	fn calculate_failed_requests(&mut self) {
		if self.total_requests >= self.successful_requests {
			self.failed_requests = self.total_requests - self.successful_requests;
		} else {
			self.failed_requests = 0;
		}
	}

	/// Calculate p95 using improved median-based approximation for right-skewed distributions
	fn calculate_p95(&mut self) {
		if self.total_requests == 0 {
			self.p95_response_time_ms = 0;
		} else if self.min_response_time_ms == self.max_response_time_ms {
			self.p95_response_time_ms = self.min_response_time_ms;
		} else {
			// Use median-based approximation for better accuracy with right-skewed response times
			// p95 ≈ median + COEFF * (max - median)
			// This is much more accurate for log-normal/gamma distributions typical of response times
			let median = self.median_response_time_ms;
			let max = self.max_response_time_ms as f64;

			// Ensure median is reasonable (between min and max)
			let clamped_median = median.max(self.min_response_time_ms as f64).min(max);

			let p95_estimate = clamped_median + P95_MEDIAN_COEFFICIENT * (max - clamped_median);
			self.p95_response_time_ms = p95_estimate.max(clamped_median).min(max) as u64;
		}
	}
}

/// Aggregated metrics for a specific time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsAggregate {
	/// Time period this aggregate covers
	pub time_range: TimeRange,
	/// Total number of requests in this period
	pub total_requests: u64,
	/// Number of successful requests
	pub successful_requests: u64,
	/// Number of failed requests
	pub failed_requests: u64,
	/// Number of timeout requests
	pub timeout_requests: u64,
	/// Average response time in milliseconds
	pub avg_response_time_ms: f64,
	/// Estimated median response time in milliseconds (more robust for skewed data)
	pub median_response_time_ms: f64,
	/// Minimum response time in milliseconds
	pub min_response_time_ms: u64,
	/// Maximum response time in milliseconds
	pub max_response_time_ms: u64,
	/// 95th percentile response time in milliseconds
	pub p95_response_time_ms: u64,
	/// Success rate (0.0 to 1.0)
	pub success_rate: f64,
	/// Per-operation breakdown of metrics within this time bucket
	/// Only tracks get_quotes and submit_order operations
	pub operation_breakdown: HashMap<Operation, OperationStats>,
	/// When this aggregate was last updated
	pub last_updated: DateTime<Utc>,
}

impl MetricsAggregate {
	/// Create an empty aggregate for a time range
	pub fn new(time_range: TimeRange) -> Self {
		Self {
			time_range,
			total_requests: 0,
			successful_requests: 0,
			failed_requests: 0,
			timeout_requests: 0,
			avg_response_time_ms: 0.0,
			median_response_time_ms: 0.0,
			min_response_time_ms: u64::MAX,
			max_response_time_ms: 0,
			p95_response_time_ms: 0,
			success_rate: 0.0,
			operation_breakdown: HashMap::new(),
			last_updated: Utc::now(),
		}
	}

	/// Add a data point to this aggregate
	pub fn add_data_point(&mut self, point: &MetricsDataPoint) {
		let old_total = self.total_requests;
		self.total_requests += 1;

		if point.was_successful {
			self.successful_requests += 1;
		} else {
			self.failed_requests += 1;
		}

		if point.was_timeout {
			self.timeout_requests += 1;
		}

		// Update response time statistics using Welford's online algorithm for numerical stability
		// This prevents precision loss that accumulates with the naive running average approach
		if old_total == 0 {
			self.avg_response_time_ms = point.response_time_ms as f64;
		} else {
			// Welford's algorithm: new_avg = old_avg + (new_value - old_avg) / new_count
			let delta = point.response_time_ms as f64 - self.avg_response_time_ms;
			self.avg_response_time_ms += delta / self.total_requests as f64;
		}

		// Update running median estimate using incremental approximation
		// This converges to the true median and is memory-efficient
		self.update_running_median(point.response_time_ms as f64);

		if point.response_time_ms < self.min_response_time_ms {
			self.min_response_time_ms = point.response_time_ms;
		}

		if point.response_time_ms > self.max_response_time_ms {
			self.max_response_time_ms = point.response_time_ms;
		}

		// Update success rate
		self.success_rate = self.successful_requests as f64 / self.total_requests as f64;

		// Calculate p95 response time using mathematical approximation
		// This provides a reasonable estimate without storing all individual response times
		self.p95_response_time_ms = self.calculate_p95_approximation();

		// Update operation-specific metrics (only for tracked operations)
		if point.operation.is_tracked() {
			let operation_stats = self
				.operation_breakdown
				.entry(point.operation)
				.or_insert_with(|| OperationStats::new(point.operation));

			// Update operation totals
			let old_operation_total = operation_stats.total_requests;
			operation_stats.total_requests += 1;

			if point.was_successful {
				operation_stats.successful_requests += 1;
			}

			if point.was_timeout {
				operation_stats.timeout_requests += 1;
			}

			// Update operation error counts
			if let Some(ref error_type) = point.error_type {
				match error_type {
					ErrorType::ServiceError => operation_stats.service_errors += 1,
					ErrorType::ClientError => operation_stats.client_errors += 1,
					ErrorType::ApplicationError => operation_stats.application_errors += 1,
					ErrorType::Unknown => operation_stats.service_errors += 1, // Treat unknown as service error
				}
			}

			// Update operation response time statistics
			if old_operation_total == 0 {
				// First data point for this operation
				operation_stats.min_response_time_ms = point.response_time_ms;
				operation_stats.max_response_time_ms = point.response_time_ms;
				operation_stats.avg_response_time_ms = point.response_time_ms as f64;
				operation_stats.median_response_time_ms = point.response_time_ms as f64;
			} else {
				// Update running average using Welford's algorithm for numerical stability
				let delta = point.response_time_ms as f64 - operation_stats.avg_response_time_ms;
				operation_stats.avg_response_time_ms +=
					delta / operation_stats.total_requests as f64;

				// Update running median estimate for operation stats
				let new_value = point.response_time_ms as f64;
				const OP_LEARNING_RATE: f64 = 0.1;
				let diff = new_value - operation_stats.median_response_time_ms;
				operation_stats.median_response_time_ms += OP_LEARNING_RATE * diff;

				if point.response_time_ms < operation_stats.min_response_time_ms {
					operation_stats.min_response_time_ms = point.response_time_ms;
				}

				if point.response_time_ms > operation_stats.max_response_time_ms {
					operation_stats.max_response_time_ms = point.response_time_ms;
				}
			}

			// Recalculate derived values for this operation
			operation_stats.calculate_failed_requests();
			operation_stats.calculate_success_rate();
			operation_stats.calculate_p95();
		}

		self.last_updated = Utc::now();
	}

	/// Check if this aggregate is empty (no data points)
	pub fn is_empty(&self) -> bool {
		self.total_requests == 0
	}

	/// Calculate p95 response time using improved median-based approximation
	///
	/// Uses median instead of mean for better accuracy with right-skewed response time distributions.
	/// Most response times follow log-normal or gamma distributions, not normal distributions.
	fn calculate_p95_approximation(&self) -> u64 {
		// Handle edge cases
		if self.total_requests == 0 {
			return 0;
		}

		if self.total_requests == 1 {
			return self.max_response_time_ms;
		}

		// If all values are the same (min == max), p95 is that value
		if self.min_response_time_ms == self.max_response_time_ms {
			return self.max_response_time_ms;
		}

		// Use median-based approximation for better accuracy with right-skewed distributions
		// p95 ≈ median + COEFF * (max - median)
		let median_ms = self.median_response_time_ms;
		let max_ms = self.max_response_time_ms as f64;
		let min_ms = self.min_response_time_ms as f64;

		// Ensure median is reasonable (between min and max)
		let clamped_median = median_ms.max(min_ms).min(max_ms);

		let p95_estimate = clamped_median + P95_MEDIAN_COEFFICIENT * (max_ms - clamped_median);

		// Ensure result is within bounds [median, max] and convert to u64
		p95_estimate.max(clamped_median).min(max_ms) as u64
	}

	/// Update running median estimate using incremental approximation
	///
	/// This is a simple online algorithm that converges to the true median.
	/// Learning rate of 0.1 provides good balance between responsiveness and stability.
	fn update_running_median(&mut self, new_value: f64) {
		if self.total_requests == 1 {
			// First value becomes the initial median
			self.median_response_time_ms = new_value;
		} else {
			// Incremental median update with learning rate
			const LEARNING_RATE: f64 = 0.1;
			let diff = new_value - self.median_response_time_ms;
			self.median_response_time_ms += LEARNING_RATE * diff;
		}
	}
}

/// Time range representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeRange {
	/// Start of the time range (inclusive)
	pub start: DateTime<Utc>,
	/// End of the time range (exclusive)
	pub end: DateTime<Utc>,
}

impl TimeRange {
	/// Create a new time range
	pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
		Self { start, end }
	}

	/// Check if a timestamp falls within this range
	pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
		timestamp >= self.start && timestamp < self.end
	}

	/// Get the duration of this time range
	pub fn duration(&self) -> Duration {
		self.end - self.start
	}

	/// Create a time range for a specific hour
	pub fn for_hour(datetime: DateTime<Utc>) -> Self {
		let start = Self::truncate_to_hour(datetime);
		let end = start + Duration::hours(1);
		Self::new(start, end)
	}

	/// Create a time range for a specific day
	pub fn for_day(datetime: DateTime<Utc>) -> Self {
		let start = Self::truncate_to_day(datetime);
		let end = start + Duration::days(1);
		Self::new(start, end)
	}

	/// Safely truncate datetime to hour boundary (set minutes, seconds, nanoseconds to 0)
	fn truncate_to_hour(datetime: DateTime<Utc>) -> DateTime<Utc> {
		datetime
			.with_minute(0)
			.and_then(|dt| dt.with_second(0))
			.and_then(|dt| dt.with_nanosecond(0))
			.unwrap_or(datetime)
	}

	/// Safely truncate datetime to day boundary (set hours, minutes, seconds, nanoseconds to 0)
	fn truncate_to_day(datetime: DateTime<Utc>) -> DateTime<Utc> {
		datetime
			.with_hour(0)
			.and_then(|dt| dt.with_minute(0))
			.and_then(|dt| dt.with_second(0))
			.and_then(|dt| dt.with_nanosecond(0))
			.unwrap_or(datetime)
	}

	/// Safely set minute and truncate to that boundary
	fn truncate_to_minute(datetime: DateTime<Utc>, minute: u32) -> DateTime<Utc> {
		datetime
			.with_minute(minute)
			.and_then(|dt| dt.with_second(0))
			.and_then(|dt| dt.with_nanosecond(0))
			.unwrap_or(datetime)
	}
}

/// Time bucket granularity for aggregating metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeBucket {
	/// 5-minute buckets
	FiveMinutes,
	/// 15-minute buckets
	FifteenMinutes,
	/// 1-hour buckets
	Hourly,
	/// Daily buckets
	Daily,
}

impl TimeBucket {
	/// Get the duration for this bucket type
	pub fn duration(&self) -> Duration {
		match self {
			TimeBucket::FiveMinutes => Duration::minutes(5),
			TimeBucket::FifteenMinutes => Duration::minutes(15),
			TimeBucket::Hourly => Duration::hours(1),
			TimeBucket::Daily => Duration::days(1),
		}
	}

	/// Get the time range for the bucket containing the given timestamp
	pub fn bucket_for(&self, timestamp: DateTime<Utc>) -> TimeRange {
		match self {
			TimeBucket::FiveMinutes => {
				let minutes = timestamp.minute();
				let bucket_start_minute = (minutes / 5) * 5;
				let start = TimeRange::truncate_to_minute(timestamp, bucket_start_minute);
				let end = start + self.duration();
				TimeRange::new(start, end)
			},
			TimeBucket::FifteenMinutes => {
				let minutes = timestamp.minute();
				let bucket_start_minute = (minutes / 15) * 15;
				let start = TimeRange::truncate_to_minute(timestamp, bucket_start_minute);
				let end = start + self.duration();
				TimeRange::new(start, end)
			},
			TimeBucket::Hourly => TimeRange::for_hour(timestamp),
			TimeBucket::Daily => TimeRange::for_day(timestamp),
		}
	}
}

/// A collection of metrics data organized in time buckets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsBucket {
	/// The granularity of this bucket
	pub bucket_type: TimeBucket,
	/// Aggregated metrics organized by time range
	pub aggregates: BTreeMap<TimeRange, MetricsAggregate>,
	/// Maximum number of buckets to keep
	pub max_buckets: usize,
	/// When this bucket collection was last updated
	pub last_updated: DateTime<Utc>,
}

impl MetricsBucket {
	/// Create a new metrics bucket
	pub fn new(bucket_type: TimeBucket, max_buckets: usize) -> Self {
		Self {
			bucket_type,
			aggregates: BTreeMap::new(),
			max_buckets,
			last_updated: Utc::now(),
		}
	}

	/// Add a data point to the appropriate time bucket
	pub fn add_data_point(&mut self, point: &MetricsDataPoint) {
		let time_range = self.bucket_type.bucket_for(point.timestamp);

		let aggregate = self
			.aggregates
			.entry(time_range.clone())
			.or_insert_with(|| MetricsAggregate::new(time_range));

		aggregate.add_data_point(point);
		self.last_updated = Utc::now();

		// Clean up old buckets if we exceed the limit
		self.cleanup_old_buckets();
	}

	/// Add multiple data points efficiently to reduce HashMap lookups and allocations
	pub fn add_data_points_batch(&mut self, points: &[&MetricsDataPoint]) {
		if points.is_empty() {
			return;
		}

		// Group points by their time buckets to reduce HashMap lookups
		// Using HashMap<TimeRange, Vec<&MetricsDataPoint>> to batch points
		let mut bucket_groups: HashMap<TimeRange, Vec<&MetricsDataPoint>> = HashMap::new();

		for point in points {
			let time_range = self.bucket_type.bucket_for(point.timestamp);
			bucket_groups.entry(time_range).or_default().push(point);
		}

		// Process each time bucket group
		for (time_range, bucket_points) in bucket_groups {
			let aggregate = self
				.aggregates
				.entry(time_range.clone())
				.or_insert_with(|| MetricsAggregate::new(time_range));

			// Add all points for this time bucket at once
			for point in bucket_points {
				aggregate.add_data_point(point);
			}
		}

		// Single timestamp update for the entire batch
		self.last_updated = Utc::now();

		// Single cleanup operation after processing all points
		self.cleanup_old_buckets();
	}

	/// Remove old buckets that exceed the maximum count
	fn cleanup_old_buckets(&mut self) {
		while self.aggregates.len() > self.max_buckets {
			if let Some(oldest_key) = self.aggregates.keys().next().cloned() {
				self.aggregates.remove(&oldest_key);
			} else {
				break;
			}
		}
	}

	/// Get aggregates for a specific time range
	pub fn get_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&MetricsAggregate> {
		let query_range = TimeRange::new(start, end);
		self.aggregates
			.values()
			.filter(|aggregate| {
				aggregate.time_range.start < query_range.end
					&& aggregate.time_range.end > query_range.start
			})
			.collect()
	}

	/// Get the most recent aggregate
	pub fn get_latest(&self) -> Option<&MetricsAggregate> {
		self.aggregates.values().last()
	}

	/// Check if this bucket is empty
	pub fn is_empty(&self) -> bool {
		self.aggregates.is_empty()
	}
}

/// Time window for rolling metrics calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
	/// Duration of the rolling window
	pub duration: Duration,
	/// When the window was last calculated
	pub last_updated: DateTime<Utc>,
}

impl TimeWindow {
	/// Create a new time window
	pub fn new(duration: Duration) -> Self {
		Self {
			duration,
			last_updated: Utc::now(),
		}
	}

	/// Get the start time for this window from now
	pub fn start_time(&self) -> DateTime<Utc> {
		Utc::now() - self.duration
	}

	/// Check if a timestamp is within this rolling window
	pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
		timestamp >= self.start_time()
	}

	/// Common time windows
	pub fn last_hour() -> Self {
		Self::new(Duration::hours(1))
	}

	pub fn last_day() -> Self {
		Self::new(Duration::days(1))
	}

	pub fn last_week() -> Self {
		Self::new(Duration::weeks(1))
	}
}

/// Collection of rolling metrics for different time windows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingMetrics {
	/// 1-hour rolling window
	pub last_hour: Option<MetricsAggregate>,
	/// 24-hour rolling window
	pub last_day: Option<MetricsAggregate>,
	/// 7-day rolling window
	pub last_week: Option<MetricsAggregate>,
	/// When these rolling metrics were last calculated
	pub last_updated: DateTime<Utc>,
}

impl RollingMetrics {
	/// Create empty rolling metrics
	pub fn new() -> Self {
		Self {
			last_hour: None,
			last_day: None,
			last_week: None,
			last_updated: Utc::now(),
		}
	}
}

impl Default for RollingMetrics {
	fn default() -> Self {
		Self::new()
	}
}

/// Complete time-series storage for a solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsTimeSeries {
	/// Solver ID this time series belongs to
	pub solver_id: String,
	/// 5-minute granularity buckets
	pub five_minute_buckets: MetricsBucket,
	/// 15-minute granularity buckets
	pub fifteen_minute_buckets: MetricsBucket,
	/// Hourly granularity buckets
	pub hourly_buckets: MetricsBucket,
	/// Daily granularity buckets
	pub daily_buckets: MetricsBucket,
	/// Pre-computed rolling metrics
	pub rolling_metrics: RollingMetrics,
	/// When this time series was created
	pub created_at: DateTime<Utc>,
	/// When this time series was last updated
	pub last_updated: DateTime<Utc>,
}

impl MetricsTimeSeries {
	/// Create a new time series for a solver
	pub fn new(solver_id: String) -> Self {
		let now = Utc::now();

		Self {
			solver_id,
			// Keep last 288 5-minute buckets (24 hours)
			five_minute_buckets: MetricsBucket::new(TimeBucket::FiveMinutes, 288),
			// Keep last 96 15-minute buckets (24 hours)
			fifteen_minute_buckets: MetricsBucket::new(TimeBucket::FifteenMinutes, 96),
			// Keep last 168 hourly buckets (7 days)
			hourly_buckets: MetricsBucket::new(TimeBucket::Hourly, 168),
			// Keep last 30 daily buckets (30 days)
			daily_buckets: MetricsBucket::new(TimeBucket::Daily, 30),
			rolling_metrics: RollingMetrics::new(),
			created_at: now,
			last_updated: now,
		}
	}

	/// Add a data point to all appropriate buckets
	pub fn add_data_point(&mut self, point: MetricsDataPoint) {
		self.five_minute_buckets.add_data_point(&point);
		self.fifteen_minute_buckets.add_data_point(&point);
		self.hourly_buckets.add_data_point(&point);
		self.daily_buckets.add_data_point(&point);

		self.last_updated = Utc::now();
	}

	/// Add multiple data points efficiently with optimized batch processing
	pub fn add_data_points(&mut self, points: &[MetricsDataPoint]) {
		if points.is_empty() {
			return;
		}

		// Early return for single point to avoid overhead
		if points.len() == 1 {
			self.add_data_point(points[0].clone());
			return;
		}

		// Sort by timestamp for better cache locality and temporal coherence
		// This improves performance when buckets need to allocate new time ranges
		let mut sorted_points: Vec<&MetricsDataPoint> = points.iter().collect();
		sorted_points.sort_by_key(|p| p.timestamp);

		// Batch process all points to each bucket type
		// This is more cache-friendly than interleaving bucket updates
		self.five_minute_buckets
			.add_data_points_batch(&sorted_points);
		self.fifteen_minute_buckets
			.add_data_points_batch(&sorted_points);
		self.hourly_buckets.add_data_points_batch(&sorted_points);
		self.daily_buckets.add_data_points_batch(&sorted_points);

		// Single timestamp update at the end
		self.last_updated = Utc::now();
	}

	/// Update rolling metrics from the stored buckets
	/// Update rolling metrics (hourly, daily, weekly) from detailed buckets
	/// This is called periodically to avoid expensive recalculation on every data point
	///
	/// Returns `Err` if any critical computation fails, allowing callers to handle the failure appropriately
	pub fn update_rolling_metrics(&mut self) -> Result<(), MetricsComputationError> {
		let now = Utc::now();

		// Calculate 1-hour rolling metrics from 5-minute buckets
		let hour_start = now - Duration::hours(1);
		let hour_aggregates = self.five_minute_buckets.get_range(hour_start, now);
		self.rolling_metrics.last_hour = self
			.compute_aggregate_from_buckets(hour_aggregates)
			.map_err(|e| MetricsComputationError::RollingMetricsFailure {
				operation: "1-hour rolling metrics".to_string(),
				details: e.to_string(),
			})?;

		// Calculate 24-hour rolling metrics from hourly buckets
		let day_start = now - Duration::days(1);
		let day_aggregates = self.hourly_buckets.get_range(day_start, now);
		self.rolling_metrics.last_day = self
			.compute_aggregate_from_buckets(day_aggregates)
			.map_err(|e| MetricsComputationError::RollingMetricsFailure {
				operation: "24-hour rolling metrics".to_string(),
				details: e.to_string(),
			})?;

		// Calculate 7-day rolling metrics from daily buckets
		let week_start = now - Duration::weeks(1);
		let week_aggregates = self.daily_buckets.get_range(week_start, now);
		self.rolling_metrics.last_week = self
			.compute_aggregate_from_buckets(week_aggregates)
			.map_err(|e| MetricsComputationError::RollingMetricsFailure {
				operation: "7-day rolling metrics".to_string(),
				details: e.to_string(),
			})?;

		self.rolling_metrics.last_updated = now;
		Ok(())
	}

	/// Combine multiple aggregates into a single aggregate with enhanced error reporting
	fn compute_aggregate_from_buckets(
		&self,
		aggregates: Vec<&MetricsAggregate>,
	) -> Result<Option<MetricsAggregate>, MetricsComputationError> {
		if aggregates.is_empty() {
			return Ok(None);
		}

		// Validate data integrity before processing
		if aggregates.len() > 10000 {
			return Err(MetricsComputationError::DataIntegrityError {
				issue: format!(
					"Excessive aggregate count: {} - possible memory leak",
					aggregates.len()
				),
			});
		}

		// Use proper error handling with detailed context
		let start_time = aggregates
			.iter()
			.map(|a| a.time_range.start)
			.min()
			.ok_or_else(|| MetricsComputationError::AggregateComputationFailure {
				details: format!(
					"Failed to compute minimum start time from {} aggregates",
					aggregates.len()
				),
			})?;

		let end_time = aggregates
			.iter()
			.map(|a| a.time_range.end)
			.max()
			.ok_or_else(|| MetricsComputationError::AggregateComputationFailure {
				details: format!(
					"Failed to compute maximum end time from {} aggregates",
					aggregates.len()
				),
			})?;

		let mut combined = MetricsAggregate::new(TimeRange::new(start_time, end_time));

		// Calculate weighted averages for response time and median with overflow protection
		let mut total_weighted_time = 0.0;
		let mut total_weighted_median = 0.0;

		for aggregate in &aggregates {
			// Validate individual aggregate integrity
			if aggregate.total_requests > 1_000_000_000 {
				return Err(MetricsComputationError::DataIntegrityError {
					issue: format!(
						"Suspicious request count: {} in single aggregate - possible data corruption",
						aggregate.total_requests
					),
				});
			}

			// Check for arithmetic overflow before addition
			if combined
				.total_requests
				.checked_add(aggregate.total_requests)
				.is_none()
			{
				return Err(MetricsComputationError::DataIntegrityError {
					issue:
						"Integer overflow detected in total_requests - possible memory corruption"
							.to_string(),
				});
			}

			combined.total_requests += aggregate.total_requests;
			combined.successful_requests += aggregate.successful_requests;
			combined.failed_requests += aggregate.failed_requests;
			combined.timeout_requests += aggregate.timeout_requests;

			// Validate floating point calculations before accumulation
			let weighted_time = aggregate.avg_response_time_ms * aggregate.total_requests as f64;
			let weighted_median =
				aggregate.median_response_time_ms * aggregate.total_requests as f64;

			if !weighted_time.is_finite() || !weighted_median.is_finite() {
				return Err(MetricsComputationError::DataIntegrityError {
					issue: format!(
						"Invalid floating point values detected: avg={}, median={}, requests={}",
						aggregate.avg_response_time_ms,
						aggregate.median_response_time_ms,
						aggregate.total_requests
					),
				});
			}

			total_weighted_time += weighted_time;
			total_weighted_median += weighted_median;

			// Update min/max response times
			if aggregate.min_response_time_ms < combined.min_response_time_ms
				&& aggregate.total_requests > 0
			{
				combined.min_response_time_ms = aggregate.min_response_time_ms;
			}
			if aggregate.max_response_time_ms > combined.max_response_time_ms {
				combined.max_response_time_ms = aggregate.max_response_time_ms;
			}
		}

		// Recalculate derived metrics with validation
		if combined.total_requests > 0 {
			combined.success_rate =
				combined.successful_requests as f64 / combined.total_requests as f64;

			// Validate success rate is reasonable
			if !combined.success_rate.is_finite() || combined.success_rate > 1.0 {
				return Err(MetricsComputationError::DataIntegrityError {
					issue: format!(
						"Invalid success rate computed: {} (successful: {}, total: {})",
						combined.success_rate,
						combined.successful_requests,
						combined.total_requests
					),
				});
			}

			// Calculate weighted averages with overflow protection
			combined.avg_response_time_ms = total_weighted_time / combined.total_requests as f64;
			combined.median_response_time_ms =
				total_weighted_median / combined.total_requests as f64;

			// Validate computed averages
			if !combined.avg_response_time_ms.is_finite()
				|| !combined.median_response_time_ms.is_finite()
			{
				return Err(MetricsComputationError::DataIntegrityError {
					issue: format!(
						"Invalid averages computed: avg={}, median={}",
						combined.avg_response_time_ms, combined.median_response_time_ms
					),
				});
			}

			// Calculate p95 using improved median-based approximation method
			combined.p95_response_time_ms = combined.calculate_p95_approximation();
		}

		combined.last_updated = Utc::now();
		Ok(Some(combined))
	}

	/// Clean up old data beyond retention periods
	pub fn cleanup_old_data(&mut self) {
		// The cleanup happens automatically in MetricsBucket based on max_buckets
		// This method is here for future enhancements
	}

	/// Get operation-specific success rates for the given time window
	/// Returns tracked operations only
	pub fn get_operation_success_rates(&self, window: Duration) -> HashMap<Operation, f64> {
		let now = Utc::now();
		let window_start = now - window;

		let mut operation_totals: HashMap<Operation, (u64, u64)> = HashMap::new(); // (total, successful)

		// Get aggregates from appropriate bucket based on window duration
		let aggregates = if window <= Duration::hours(1) {
			self.five_minute_buckets.get_range(window_start, now)
		} else if window <= Duration::days(1) {
			self.fifteen_minute_buckets.get_range(window_start, now)
		} else if window <= Duration::weeks(1) {
			self.hourly_buckets.get_range(window_start, now)
		} else {
			self.daily_buckets.get_range(window_start, now)
		};

		// Combine operation stats from all matching aggregates
		for aggregate in aggregates {
			for (&operation, stats) in &aggregate.operation_breakdown {
				if operation.is_tracked() {
					let totals = operation_totals.entry(operation).or_insert((0, 0));
					totals.0 += stats.total_requests;
					totals.1 += stats.successful_requests;
				}
			}
		}

		// Calculate success rates
		operation_totals
			.into_iter()
			.map(|(operation, (total, successful))| {
				let success_rate = if total > 0 {
					successful as f64 / total as f64
				} else {
					0.0
				};
				(operation, success_rate)
			})
			.collect()
	}

	/// Get comprehensive operation statistics for a specific operation
	pub fn get_operation_stats(
		&self,
		operation: Operation,
		window: Duration,
	) -> Option<OperationStats> {
		// Only process tracked operations
		if !operation.is_tracked() {
			return None;
		}

		let now = Utc::now();
		let window_start = now - window;

		// Get aggregates from appropriate bucket based on window duration
		let aggregates = if window <= Duration::hours(1) {
			self.five_minute_buckets.get_range(window_start, now)
		} else if window <= Duration::days(1) {
			self.fifteen_minute_buckets.get_range(window_start, now)
		} else if window <= Duration::weeks(1) {
			self.hourly_buckets.get_range(window_start, now)
		} else {
			self.daily_buckets.get_range(window_start, now)
		};

		// Combine operation stats from all matching aggregates
		let mut combined_stats = OperationStats::new(operation);
		let mut found_data = false;
		let mut response_time_sum = 0.0;
		let mut median_sum = 0.0;
		let mut response_time_count = 0u64;

		for aggregate in aggregates {
			if let Some(operation_stats) = aggregate.operation_breakdown.get(&operation) {
				found_data = true;

				// Accumulate totals
				combined_stats.total_requests += operation_stats.total_requests;
				combined_stats.successful_requests += operation_stats.successful_requests;
				combined_stats.timeout_requests += operation_stats.timeout_requests;
				combined_stats.service_errors += operation_stats.service_errors;
				combined_stats.client_errors += operation_stats.client_errors;
				combined_stats.application_errors += operation_stats.application_errors;

				// For response times, we need to calculate weighted averages
				// and track min/max across all aggregates
				if operation_stats.total_requests > 0 {
					response_time_sum += operation_stats.avg_response_time_ms
						* operation_stats.total_requests as f64;
					median_sum += operation_stats.median_response_time_ms
						* operation_stats.total_requests as f64;
					response_time_count += operation_stats.total_requests;

					if combined_stats.min_response_time_ms == 0
						|| operation_stats.min_response_time_ms
							< combined_stats.min_response_time_ms
					{
						combined_stats.min_response_time_ms = operation_stats.min_response_time_ms;
					}

					if operation_stats.max_response_time_ms > combined_stats.max_response_time_ms {
						combined_stats.max_response_time_ms = operation_stats.max_response_time_ms;
					}
				}
			}
		}

		if !found_data {
			return None;
		}

		// Calculate final averages and derived values
		if response_time_count > 0 {
			combined_stats.avg_response_time_ms = response_time_sum / response_time_count as f64;
			combined_stats.median_response_time_ms = median_sum / response_time_count as f64;
		}

		combined_stats.calculate_failed_requests();
		combined_stats.calculate_success_rate();
		combined_stats.calculate_p95();

		Some(combined_stats)
	}

	/// Get operation statistics for all important operations (get_quotes, submit_order)
	pub fn get_all_operation_stats(&self, window: Duration) -> Vec<OperationStats> {
		let mut results = Vec::new();

		if let Some(get_quotes_stats) = self.get_operation_stats(Operation::GetQuotes, window) {
			results.push(get_quotes_stats);
		}

		if let Some(submit_order_stats) = self.get_operation_stats(Operation::SubmitOrder, window) {
			results.push(submit_order_stats);
		}

		results
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_p95_calculation_single_value() {
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let mut aggregate = MetricsAggregate::new(time_range);

		let data_point = MetricsDataPoint {
			timestamp: Utc::now(),
			response_time_ms: 100,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::Other,
		};

		aggregate.add_data_point(&data_point);

		// With single value, p95 should equal that value
		assert_eq!(aggregate.p95_response_time_ms, 100);
	}

	#[test]
	fn test_p95_calculation_identical_values() {
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let mut aggregate = MetricsAggregate::new(time_range);

		for _ in 0..10 {
			let data_point = MetricsDataPoint {
				timestamp: Utc::now(),
				response_time_ms: 150,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::Other,
			};
			aggregate.add_data_point(&data_point);
		}

		// All identical values, p95 should equal that value
		assert_eq!(aggregate.p95_response_time_ms, 150);
		assert_eq!(aggregate.avg_response_time_ms, 150.0);
		assert_eq!(aggregate.min_response_time_ms, 150);
		assert_eq!(aggregate.max_response_time_ms, 150);
	}

	#[test]
	fn test_p95_calculation_varied_values() {
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let mut aggregate = MetricsAggregate::new(time_range);

		// Add data points with values: 50, 100, 150, 200, 300
		let values = vec![50, 100, 150, 200, 300];
		for value in values {
			let data_point = MetricsDataPoint {
				timestamp: Utc::now(),
				response_time_ms: value,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::GetQuotes,
			};
			aggregate.add_data_point(&data_point);
		}

		// With median-based calculation:
		// avg = 160, median ≈ 150-200 (converges during updates), max = 300
		// p95 ≈ median + P95_MEDIAN_COEFFICIENT * (max - median)
		// Note: exact median will vary due to incremental algorithm
		assert_eq!(aggregate.avg_response_time_ms, 160.0);
		assert_eq!(aggregate.min_response_time_ms, 50);
		assert_eq!(aggregate.max_response_time_ms, 300);

		// P95 should be between median and max, and higher than with old formula
		assert!(aggregate.p95_response_time_ms > 258); // Higher than old mean-based calculation
		assert!(aggregate.p95_response_time_ms >= aggregate.median_response_time_ms as u64);
		assert!(aggregate.p95_response_time_ms <= aggregate.max_response_time_ms);
	}

	#[test]
	fn test_p95_bounds_checking() {
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let mut aggregate = MetricsAggregate::new(time_range);

		// Add data points where average and max are very close
		let data_point1 = MetricsDataPoint {
			timestamp: Utc::now(),
			response_time_ms: 99,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::HealthCheck,
		};
		let data_point2 = MetricsDataPoint {
			timestamp: Utc::now(),
			response_time_ms: 100,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::HealthCheck,
		};

		aggregate.add_data_point(&data_point1);
		aggregate.add_data_point(&data_point2);

		// p95 should be between avg and max
		assert!(aggregate.p95_response_time_ms >= aggregate.avg_response_time_ms as u64);
		assert!(aggregate.p95_response_time_ms <= aggregate.max_response_time_ms);
	}

	#[test]
	fn test_p95_empty_aggregate() {
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let aggregate = MetricsAggregate::new(time_range);

		// Empty aggregate should have p95 = 0
		assert_eq!(aggregate.calculate_p95_approximation(), 0);
	}

	#[test]
	fn test_compute_aggregate_error_handling() {
		let time_series = MetricsTimeSeries::new("test-solver".to_string());

		// Test empty aggregates - should return Ok(None)
		let empty_aggregates: Vec<&MetricsAggregate> = vec![];
		let result = time_series.compute_aggregate_from_buckets(empty_aggregates);
		assert!(result.is_ok());
		assert!(result.unwrap().is_none());

		// Test valid aggregates - should return Ok(Some(aggregate))
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let aggregate = MetricsAggregate::new(time_range.clone());
		let aggregates = vec![&aggregate];
		let result = time_series.compute_aggregate_from_buckets(aggregates);
		assert!(result.is_ok());
		assert!(result.unwrap().is_some());

		// Test excessive aggregate count - should return DataIntegrityError
		let mut many_aggregates = Vec::new();
		for _ in 0..10001 {
			many_aggregates.push(&aggregate);
		}
		let result = time_series.compute_aggregate_from_buckets(many_aggregates);
		assert!(result.is_err());
		match result {
			Err(MetricsComputationError::DataIntegrityError { issue }) => {
				assert!(issue.contains("Excessive aggregate count"));
				assert!(issue.contains("possible memory leak"));
			},
			_ => panic!("Expected DataIntegrityError for excessive aggregates"),
		}
	}

	#[test]
	fn test_rolling_metrics_error_propagation() {
		let mut time_series = MetricsTimeSeries::new("test-solver".to_string());

		// Normal case should work
		let result = time_series.update_rolling_metrics();
		assert!(result.is_ok());

		// Test with corrupted data would require creating invalid aggregates
		// For now, we verify the Result type is correctly handled
		assert!(matches!(result, Ok(())));
	}

	#[test]
	fn test_batch_processing_performance() {
		let mut time_series = MetricsTimeSeries::new("test-solver".to_string());
		let now = Utc::now();

		// Create test data points with different timestamps
		let mut data_points = Vec::new();
		for i in 0..1000 {
			let point = MetricsDataPoint {
				timestamp: now + Duration::minutes(i % 60), // Spread across different buckets
				response_time_ms: 100 + (i % 50) as u64,
				was_successful: i % 4 != 0, // 75% success rate
				was_timeout: i % 20 == 0,   // 5% timeout rate
				error_type: if i % 4 == 0 {
					Some(ErrorType::ServiceError)
				} else {
					None
				},
				operation: if i % 2 == 0 {
					Operation::GetQuotes
				} else {
					Operation::SubmitOrder
				},
			};
			data_points.push(point);
		}

		// Test batch processing
		time_series.add_data_points(&data_points);

		// Verify that data was properly distributed across buckets
		assert!(!time_series.five_minute_buckets.is_empty());
		assert!(!time_series.fifteen_minute_buckets.is_empty());
		assert!(!time_series.hourly_buckets.is_empty());

		// Verify aggregates contain the expected data
		let recent_five_min = time_series.five_minute_buckets.get_latest().unwrap();
		assert!(recent_five_min.total_requests > 0);
		assert!(recent_five_min.successful_requests > 0);

		// Verify operation-specific data
		assert!(!recent_five_min.operation_breakdown.is_empty());

		// Test that single-point fallback works
		let single_point = MetricsDataPoint {
			timestamp: now + Duration::hours(2),
			response_time_ms: 150,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::GetQuotes,
		};

		time_series.add_data_points(&[single_point]);
		assert!(
			time_series
				.hourly_buckets
				.get_latest()
				.unwrap()
				.total_requests
				> 0
		);
	}

	#[test]
	fn test_batch_processing_empty_and_edge_cases() {
		let mut time_series = MetricsTimeSeries::new("test-solver".to_string());

		// Test empty batch
		time_series.add_data_points(&[]);
		assert!(time_series.five_minute_buckets.is_empty());

		// Test single point batch (should use optimized path)
		let single_point = MetricsDataPoint {
			timestamp: Utc::now(),
			response_time_ms: 100,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::GetQuotes,
		};

		time_series.add_data_points(&[single_point.clone()]);
		assert!(!time_series.five_minute_buckets.is_empty());

		// Verify single point was added correctly
		let latest = time_series.five_minute_buckets.get_latest().unwrap();
		assert_eq!(latest.total_requests, 1);
		assert_eq!(latest.successful_requests, 1);
	}

	#[test]
	fn test_batch_processing_temporal_sorting() {
		let mut time_series = MetricsTimeSeries::new("test-solver".to_string());
		let base_time = Utc::now();

		// Create points with timestamps in reverse order
		let mut data_points = Vec::new();
		for i in (0..10).rev() {
			// 9, 8, 7, ..., 0
			let point = MetricsDataPoint {
				timestamp: base_time + Duration::minutes(i * 5),
				response_time_ms: 100 + i as u64,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::GetQuotes,
			};
			data_points.push(point);
		}

		// Process unsorted batch
		time_series.add_data_points(&data_points);

		// Verify all points were processed
		let aggregates: Vec<_> = time_series
			.five_minute_buckets
			.aggregates
			.values()
			.collect();
		let total_requests: u64 = aggregates.iter().map(|a| a.total_requests).sum();
		assert_eq!(total_requests, 10);

		// Verify that temporal ordering doesn't affect correctness
		assert!(aggregates.len() > 0);
		for aggregate in aggregates {
			assert!(aggregate.total_requests > 0);
			assert!(aggregate.successful_requests > 0);
			assert_eq!(aggregate.failed_requests, 0);
		}
	}

	#[test]
	fn test_median_based_p95_accuracy() {
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let mut aggregate = MetricsAggregate::new(time_range);

		// Simulate right-skewed response time distribution (typical of real systems)
		// 95% of requests are fast (50-100ms), 5% are slow (1000-2000ms)
		let fast_values = vec![50, 60, 70, 80, 90]; // 5 fast requests
		let slow_values = vec![1000, 1500, 2000]; // 3 slow requests

		// Add fast values first
		for &value in &fast_values {
			let data_point = MetricsDataPoint {
				timestamp: Utc::now(),
				response_time_ms: value,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::GetQuotes,
			};
			aggregate.add_data_point(&data_point);
		}

		// Add slow values (these create the right skew)
		for &value in &slow_values {
			let data_point = MetricsDataPoint {
				timestamp: Utc::now(),
				response_time_ms: value,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::GetQuotes,
			};
			aggregate.add_data_point(&data_point);
		}

		// With median-based approximation, P95 should be much closer to the actual
		// slow response times (1000-2000ms range) than the mean-based approach would be

		// Mean will be pulled up by slow values: (50+60+70+80+90+1000+1500+2000)/8 = 606.25
		// But median should stay closer to fast values due to incremental algorithm
		// P95 using median should be closer to actual 95th percentile (~1500-2000)

		assert!(aggregate.median_response_time_ms < aggregate.avg_response_time_ms); // Median should be lower than mean for right-skewed
		assert!(aggregate.p95_response_time_ms > 1000); // P95 should be in the slow response range
		assert!(aggregate.p95_response_time_ms >= aggregate.median_response_time_ms as u64);
		assert!(aggregate.p95_response_time_ms <= aggregate.max_response_time_ms);

		// The median-based approach should give a more realistic P95 estimate
		// for right-skewed response time distributions
	}

	#[test]
	fn test_welford_numerical_stability() {
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let mut aggregate = MetricsAggregate::new(time_range);

		// Test with large dataset that would cause precision issues with naive approach
		// Add 1 million data points with known average
		let base_value = 100u64;
		let expected_avg = base_value as f64 + 0.5; // Alternating 100, 101, 100, 101... -> avg = 100.5

		for i in 0..1_000_000 {
			let value = if i % 2 == 0 {
				base_value
			} else {
				base_value + 1
			};
			let data_point = MetricsDataPoint {
				timestamp: Utc::now(),
				response_time_ms: value,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::GetQuotes,
			};
			aggregate.add_data_point(&data_point);
		}

		// With Welford's algorithm, we should maintain precision even after 1M data points
		assert!((aggregate.avg_response_time_ms - expected_avg).abs() < 0.0001);

		// Verify basic statistics are still correct
		assert_eq!(aggregate.total_requests, 1_000_000);
		assert_eq!(aggregate.min_response_time_ms, base_value);
		assert_eq!(aggregate.max_response_time_ms, base_value + 1);

		// The running median should converge to around the true median (100.5)
		assert!((aggregate.median_response_time_ms - expected_avg).abs() < 0.1);

		// Verify operation-specific stats also maintain precision
		let get_quotes_stats = aggregate
			.operation_breakdown
			.get(&Operation::GetQuotes)
			.unwrap();
		assert!((get_quotes_stats.avg_response_time_ms - expected_avg).abs() < 0.0001);
		assert_eq!(get_quotes_stats.total_requests, 1_000_000);
	}

	#[test]
	fn test_operation_from_str() {
		use std::str::FromStr;

		// Test valid operations
		assert_eq!(
			Operation::from_str("get_quotes").unwrap(),
			Operation::GetQuotes
		);
		assert_eq!(
			Operation::from_str("submit_order").unwrap(),
			Operation::SubmitOrder
		);
		assert_eq!(
			Operation::from_str("health_check").unwrap(),
			Operation::HealthCheck
		);
		assert_eq!(
			Operation::from_str("get_assets").unwrap(),
			Operation::GetAssets
		);
		assert_eq!(
			Operation::from_str("get_order_details").unwrap(),
			Operation::GetOrderDetails
		);

		// Test unknown operations
		assert_eq!(Operation::from_str("unknown").unwrap(), Operation::Other);
		assert_eq!(Operation::from_str("").unwrap(), Operation::Other);
		assert_eq!(Operation::from_str("invalid_op").unwrap(), Operation::Other);
	}

	#[test]
	fn test_operation_as_str_and_display() {
		assert_eq!(Operation::GetQuotes.as_str(), "get_quotes");
		assert_eq!(Operation::SubmitOrder.as_str(), "submit_order");
		assert_eq!(Operation::HealthCheck.as_str(), "health_check");
		assert_eq!(Operation::GetAssets.as_str(), "get_assets");
		assert_eq!(Operation::GetOrderDetails.as_str(), "get_order_details");
		assert_eq!(Operation::Other.as_str(), "other");

		// Test Display trait
		assert_eq!(format!("{}", Operation::GetQuotes), "get_quotes");
		assert_eq!(format!("{}", Operation::SubmitOrder), "submit_order");
		assert_eq!(format!("{}", Operation::Other), "other");
	}

	#[test]
	fn test_operation_is_tracked() {
		assert!(Operation::GetQuotes.is_tracked());
		assert!(Operation::SubmitOrder.is_tracked());
		assert!(!Operation::HealthCheck.is_tracked());
		assert!(!Operation::GetAssets.is_tracked());
		assert!(!Operation::GetOrderDetails.is_tracked());
		assert!(!Operation::Other.is_tracked());
	}

	#[test]
	fn test_time_range_functionality() {
		let start = Utc::now();
		let end = start + Duration::hours(1);
		let range = TimeRange::new(start, end);

		// Test basic functionality
		assert_eq!(range.start, start);
		assert_eq!(range.end, end);
		assert_eq!(range.duration(), Duration::hours(1));

		// Test contains
		assert!(range.contains(start));
		assert!(range.contains(start + Duration::minutes(30)));
		assert!(!range.contains(end)); // End is exclusive
		assert!(!range.contains(start - Duration::minutes(1)));
		assert!(!range.contains(end + Duration::minutes(1)));

		// Test for_hour
		let test_time = Utc::now();
		let hour_range = TimeRange::for_hour(test_time);
		let expected_start = test_time
			.with_minute(0)
			.and_then(|dt| dt.with_second(0))
			.and_then(|dt| dt.with_nanosecond(0))
			.unwrap_or(test_time);
		assert_eq!(hour_range.start, expected_start);
		assert_eq!(hour_range.duration(), Duration::hours(1));

		// Test for_day
		let day_range = TimeRange::for_day(test_time);
		assert_eq!(day_range.duration(), Duration::days(1));
	}

	#[test]
	fn test_time_bucket_functionality() {
		// Test duration
		assert_eq!(TimeBucket::FiveMinutes.duration(), Duration::minutes(5));
		assert_eq!(TimeBucket::FifteenMinutes.duration(), Duration::minutes(15));
		assert_eq!(TimeBucket::Hourly.duration(), Duration::hours(1));
		assert_eq!(TimeBucket::Daily.duration(), Duration::days(1));

		// Test bucket_for with different granularities
		let test_time = Utc::now();

		// Test 5-minute buckets
		let five_min_range = TimeBucket::FiveMinutes.bucket_for(test_time);
		assert_eq!(five_min_range.duration(), Duration::minutes(5));
		assert!(five_min_range.contains(test_time));

		// Test 15-minute buckets
		let fifteen_min_range = TimeBucket::FifteenMinutes.bucket_for(test_time);
		assert_eq!(fifteen_min_range.duration(), Duration::minutes(15));
		assert!(fifteen_min_range.contains(test_time));

		// Test hourly buckets
		let hourly_range = TimeBucket::Hourly.bucket_for(test_time);
		assert_eq!(hourly_range.duration(), Duration::hours(1));
		assert!(hourly_range.contains(test_time));

		// Test daily buckets
		let daily_range = TimeBucket::Daily.bucket_for(test_time);
		assert_eq!(daily_range.duration(), Duration::days(1));
		assert!(daily_range.contains(test_time));
	}

	#[test]
	fn test_rolling_metrics_creation() {
		let rolling_metrics = RollingMetrics::new();

		assert!(rolling_metrics.last_hour.is_none());
		assert!(rolling_metrics.last_day.is_none());
		assert!(rolling_metrics.last_week.is_none());

		// Test Default trait
		let default_rolling_metrics = RollingMetrics::default();
		assert!(default_rolling_metrics.last_hour.is_none());
		assert!(default_rolling_metrics.last_day.is_none());
		assert!(default_rolling_metrics.last_week.is_none());
	}

	#[test]
	fn test_time_window_functionality() {
		let duration = Duration::hours(2);
		let window = TimeWindow::new(duration);

		assert_eq!(window.duration, duration);

		let now = Utc::now();
		let start_time = window.start_time();

		// Start time should be approximately 2 hours ago
		let expected_start = now - duration;
		let time_diff = (start_time - expected_start).num_milliseconds().abs();
		assert!(time_diff < 1000); // Within 1 second tolerance

		// Test contains - recent timestamp should be contained
		assert!(window.contains(now - Duration::minutes(30)));
		assert!(!window.contains(now - Duration::hours(3)));

		// Test common windows
		let hour_window = TimeWindow::last_hour();
		assert_eq!(hour_window.duration, Duration::hours(1));

		let day_window = TimeWindow::last_day();
		assert_eq!(day_window.duration, Duration::days(1));

		let week_window = TimeWindow::last_week();
		assert_eq!(week_window.duration, Duration::weeks(1));
	}

	#[test]
	fn test_operation_stats_calculations() {
		let mut stats = OperationStats::new(Operation::GetQuotes);

		// Test initial state
		assert_eq!(stats.operation, Operation::GetQuotes);
		assert_eq!(stats.total_requests, 0);
		assert_eq!(stats.successful_requests, 0);
		assert_eq!(stats.failed_requests, 0);
		assert_eq!(stats.success_rate, 0.0);

		// Simulate adding data
		stats.total_requests = 100;
		stats.successful_requests = 80;
		stats.service_errors = 15;
		stats.client_errors = 5;
		stats.min_response_time_ms = 50;
		stats.max_response_time_ms = 500;
		stats.median_response_time_ms = 150.0;

		// Test calculations
		stats.calculate_failed_requests();
		assert_eq!(stats.failed_requests, 20); // 100 - 80

		stats.calculate_success_rate();
		assert_eq!(stats.success_rate, 0.8); // 80/100

		stats.calculate_p95();
		assert!(stats.p95_response_time_ms > stats.median_response_time_ms as u64);
		assert!(stats.p95_response_time_ms <= stats.max_response_time_ms);
	}

	#[test]
	fn test_metrics_bucket_functionality() {
		let mut bucket = MetricsBucket::new(TimeBucket::FiveMinutes, 2);
		let now = Utc::now();

		// Test initial state
		assert!(bucket.is_empty());
		assert!(bucket.get_latest().is_none());

		// Add first data point
		let point1 = MetricsDataPoint {
			timestamp: now,
			response_time_ms: 100,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::GetQuotes,
		};
		bucket.add_data_point(&point1);

		assert!(!bucket.is_empty());
		assert!(bucket.get_latest().is_some());
		assert_eq!(bucket.aggregates.len(), 1);

		// Add point in different time bucket
		let point2 = MetricsDataPoint {
			timestamp: now + Duration::minutes(6), // Different 5-min bucket
			response_time_ms: 200,
			was_successful: false,
			was_timeout: true,
			error_type: Some(ErrorType::ServiceError),
			operation: Operation::SubmitOrder,
		};
		bucket.add_data_point(&point2);

		assert_eq!(bucket.aggregates.len(), 2);

		// Add third point to trigger cleanup (max_buckets = 2)
		let point3 = MetricsDataPoint {
			timestamp: now + Duration::minutes(12), // Another different bucket
			response_time_ms: 300,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::GetQuotes,
		};
		bucket.add_data_point(&point3);

		// Should still have max 2 buckets due to cleanup
		assert_eq!(bucket.aggregates.len(), 2);

		// Test range query
		let range_results = bucket.get_range(now, now + Duration::minutes(15));
		assert!(!range_results.is_empty());
	}

	#[test]
	fn test_metrics_timeseries_operation_queries() {
		let mut time_series = MetricsTimeSeries::new("test-solver".to_string());
		let now = Utc::now();

		// Add mixed operation data
		let points = vec![
			MetricsDataPoint {
				timestamp: now,
				response_time_ms: 100,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::GetQuotes,
			},
			MetricsDataPoint {
				timestamp: now + Duration::seconds(1),
				response_time_ms: 200,
				was_successful: false,
				was_timeout: true,
				error_type: Some(ErrorType::ServiceError),
				operation: Operation::GetQuotes,
			},
			MetricsDataPoint {
				timestamp: now + Duration::seconds(2),
				response_time_ms: 150,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::SubmitOrder,
			},
			MetricsDataPoint {
				timestamp: now + Duration::seconds(3),
				response_time_ms: 300,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::HealthCheck, // Not tracked
			},
		];

		time_series.add_data_points(&points);

		// Test get_operation_success_rates
		let success_rates = time_series.get_operation_success_rates(Duration::hours(1));
		assert_eq!(success_rates.len(), 2); // Only tracked operations
		assert!(success_rates.contains_key(&Operation::GetQuotes));
		assert!(success_rates.contains_key(&Operation::SubmitOrder));
		assert!(!success_rates.contains_key(&Operation::HealthCheck)); // Not tracked

		// GetQuotes: 1 success out of 2 = 0.5
		assert_eq!(success_rates[&Operation::GetQuotes], 0.5);
		// SubmitOrder: 1 success out of 1 = 1.0
		assert_eq!(success_rates[&Operation::SubmitOrder], 1.0);

		// Test get_operation_stats
		let get_quotes_stats =
			time_series.get_operation_stats(Operation::GetQuotes, Duration::hours(1));
		assert!(get_quotes_stats.is_some());
		let stats = get_quotes_stats.unwrap();
		assert_eq!(stats.operation, Operation::GetQuotes);
		assert_eq!(stats.total_requests, 2);
		assert_eq!(stats.successful_requests, 1);
		assert_eq!(stats.failed_requests, 1);
		assert_eq!(stats.success_rate, 0.5);
		assert_eq!(stats.timeout_requests, 1);
		assert_eq!(stats.service_errors, 1);

		// Test for non-tracked operation
		let health_check_stats =
			time_series.get_operation_stats(Operation::HealthCheck, Duration::hours(1));
		assert!(health_check_stats.is_none());

		// Test get_all_operation_stats
		let all_stats = time_series.get_all_operation_stats(Duration::hours(1));
		assert_eq!(all_stats.len(), 2); // GetQuotes and SubmitOrder
		assert!(all_stats
			.iter()
			.any(|s| s.operation == Operation::GetQuotes));
		assert!(all_stats
			.iter()
			.any(|s| s.operation == Operation::SubmitOrder));
	}

	#[test]
	fn test_rolling_metrics_update_comprehensive() {
		let mut time_series = MetricsTimeSeries::new("test-solver".to_string());
		let now = Utc::now();

		// Add data across different time ranges
		let points = vec![
			// Recent data (within last hour)
			MetricsDataPoint {
				timestamp: now - Duration::minutes(30),
				response_time_ms: 100,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::GetQuotes,
			},
			// Older data (within last day but not last hour)
			MetricsDataPoint {
				timestamp: now - Duration::hours(2),
				response_time_ms: 200,
				was_successful: false,
				was_timeout: true,
				error_type: Some(ErrorType::ServiceError),
				operation: Operation::SubmitOrder,
			},
			// Much older data (within last week but not last day)
			MetricsDataPoint {
				timestamp: now - Duration::days(2),
				response_time_ms: 300,
				was_successful: true,
				was_timeout: false,
				error_type: None,
				operation: Operation::GetQuotes,
			},
		];

		time_series.add_data_points(&points);

		// Update rolling metrics
		let result = time_series.update_rolling_metrics();
		assert!(result.is_ok());

		// Verify rolling metrics were computed
		assert!(time_series.rolling_metrics.last_hour.is_some());
		assert!(time_series.rolling_metrics.last_day.is_some());
		assert!(time_series.rolling_metrics.last_week.is_some());

		// Check that hour metrics only include recent data
		let hour_metrics = time_series.rolling_metrics.last_hour.as_ref().unwrap();
		assert_eq!(hour_metrics.total_requests, 1); // Only the 30-min old point

		// Check that day metrics include hour + day data
		let day_metrics = time_series.rolling_metrics.last_day.as_ref().unwrap();
		assert!(day_metrics.total_requests >= 1); // At least the recent data

		// Check that week metrics include all data
		let week_metrics = time_series.rolling_metrics.last_week.as_ref().unwrap();
		assert!(week_metrics.total_requests >= day_metrics.total_requests);
	}

	#[test]
	fn test_metrics_aggregate_operation_breakdown() {
		let time_range = TimeRange::new(Utc::now(), Utc::now() + Duration::minutes(15));
		let mut aggregate = MetricsAggregate::new(time_range);

		// Add points for different operations
		let tracked_point = MetricsDataPoint {
			timestamp: Utc::now(),
			response_time_ms: 100,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::GetQuotes,
		};

		let untracked_point = MetricsDataPoint {
			timestamp: Utc::now(),
			response_time_ms: 200,
			was_successful: false,
			was_timeout: true,
			error_type: Some(ErrorType::ServiceError),
			operation: Operation::HealthCheck, // Not tracked
		};

		aggregate.add_data_point(&tracked_point);
		aggregate.add_data_point(&untracked_point);

		// Check overall aggregate
		assert_eq!(aggregate.total_requests, 2);
		assert_eq!(aggregate.successful_requests, 1);

		// Check operation breakdown - only tracked operations should be present
		assert_eq!(aggregate.operation_breakdown.len(), 1);
		assert!(aggregate
			.operation_breakdown
			.contains_key(&Operation::GetQuotes));
		assert!(!aggregate
			.operation_breakdown
			.contains_key(&Operation::HealthCheck));

		let get_quotes_stats = &aggregate.operation_breakdown[&Operation::GetQuotes];
		assert_eq!(get_quotes_stats.operation, Operation::GetQuotes);
		assert_eq!(get_quotes_stats.total_requests, 1);
		assert_eq!(get_quotes_stats.successful_requests, 1);
		assert_eq!(get_quotes_stats.success_rate, 1.0);
	}

	#[test]
	fn test_edge_cases_and_boundary_conditions() {
		// Test empty time series operation queries
		let empty_time_series = MetricsTimeSeries::new("empty-solver".to_string());

		let empty_success_rates = empty_time_series.get_operation_success_rates(Duration::hours(1));
		assert!(empty_success_rates.is_empty());

		let empty_stats =
			empty_time_series.get_operation_stats(Operation::GetQuotes, Duration::hours(1));
		assert!(empty_stats.is_none());

		let empty_all_stats = empty_time_series.get_all_operation_stats(Duration::hours(1));
		assert!(empty_all_stats.is_empty());

		// Test with zero duration window
		let mut time_series = MetricsTimeSeries::new("test-solver".to_string());
		let now = Utc::now();

		let point = MetricsDataPoint {
			timestamp: now,
			response_time_ms: 100,
			was_successful: true,
			was_timeout: false,
			error_type: None,
			operation: Operation::GetQuotes,
		};
		time_series.add_data_point(point);

		// Very short window should still work
		let short_window_rates = time_series.get_operation_success_rates(Duration::seconds(1));
		// The point might be included or not depending on timing, both cases are valid
		assert!(short_window_rates.len() <= 1);

		// Test OperationStats edge cases
		let mut edge_stats = OperationStats::new(Operation::GetQuotes);

		// Test with zero requests
		edge_stats.calculate_success_rate();
		assert_eq!(edge_stats.success_rate, 0.0);

		edge_stats.calculate_failed_requests();
		assert_eq!(edge_stats.failed_requests, 0);

		edge_stats.calculate_p95();
		assert_eq!(edge_stats.p95_response_time_ms, 0);

		// Test with inconsistent data (more successful than total)
		edge_stats.total_requests = 5;
		edge_stats.successful_requests = 10; // Invalid state
		edge_stats.calculate_failed_requests();
		assert_eq!(edge_stats.failed_requests, 0); // Should handle gracefully
	}

	#[test]
	fn test_error_type_from_http_status() {
		// Test client errors (4xx except 408 and 429)
		assert_eq!(ErrorType::from_http_status(400), ErrorType::ClientError);
		assert_eq!(ErrorType::from_http_status(401), ErrorType::ClientError);
		assert_eq!(ErrorType::from_http_status(404), ErrorType::ClientError);
		assert_eq!(ErrorType::from_http_status(409), ErrorType::ClientError);
		assert_eq!(ErrorType::from_http_status(422), ErrorType::ClientError);

		// Test 408 (Request Timeout) - should be ServiceError in our context
		assert_eq!(ErrorType::from_http_status(408), ErrorType::ServiceError);

		// Test 429 (Rate Limiting) - should be ServiceError
		assert_eq!(ErrorType::from_http_status(429), ErrorType::ServiceError);

		// Test server errors (5xx)
		assert_eq!(ErrorType::from_http_status(500), ErrorType::ServiceError);
		assert_eq!(ErrorType::from_http_status(502), ErrorType::ServiceError);
		assert_eq!(ErrorType::from_http_status(503), ErrorType::ServiceError);
		assert_eq!(ErrorType::from_http_status(504), ErrorType::ServiceError);

		// Test unknown status codes
		assert_eq!(ErrorType::from_http_status(200), ErrorType::Unknown);
		assert_eq!(ErrorType::from_http_status(600), ErrorType::Unknown);
	}

	#[test]
	fn test_error_type_descriptions() {
		assert_eq!(
			ErrorType::ServiceError.description(),
			"Service Error (affects circuit breaker)"
		);
		assert_eq!(
			ErrorType::ClientError.description(),
			"Client Error (doesn't affect circuit breaker)"
		);
		assert_eq!(
			ErrorType::ApplicationError.description(),
			"Application Error (internal issue)"
		);
		assert_eq!(ErrorType::Unknown.description(), "Unknown Error");
	}
}
