//! Time-series data structures for metrics storage and analysis

use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}

/// Classification of different error types for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
	/// Network-related errors (connection, DNS, etc.)
	Network,
	/// HTTP errors (4xx, 5xx status codes)
	Http,
	/// Timeout errors
	Timeout,
	/// Adapter-specific errors (parsing, validation)
	Adapter,
	/// Solver-specific errors (business logic)
	Solver,
	/// Unknown/uncategorized errors
	Unknown,
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
	/// Minimum response time in milliseconds
	pub min_response_time_ms: u64,
	/// Maximum response time in milliseconds
	pub max_response_time_ms: u64,
	/// 95th percentile response time in milliseconds
	pub p95_response_time_ms: u64,
	/// Success rate (0.0 to 1.0)
	pub success_rate: f64,
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
			min_response_time_ms: u64::MAX,
			max_response_time_ms: 0,
			p95_response_time_ms: 0,
			success_rate: 0.0,
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

		// Update response time statistics
		self.avg_response_time_ms = (self.avg_response_time_ms * old_total as f64
			+ point.response_time_ms as f64)
			/ self.total_requests as f64;

		if point.response_time_ms < self.min_response_time_ms {
			self.min_response_time_ms = point.response_time_ms;
		}

		if point.response_time_ms > self.max_response_time_ms {
			self.max_response_time_ms = point.response_time_ms;
		}

		// Update success rate
		self.success_rate = self.successful_requests as f64 / self.total_requests as f64;

		self.last_updated = Utc::now();

		// TODO: Calculate p95 properly - for now, approximate with max
		self.p95_response_time_ms = self.max_response_time_ms;
	}

	/// Check if this aggregate is empty (no data points)
	pub fn is_empty(&self) -> bool {
		self.total_requests == 0
	}
}

/// Time range representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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

	/// Update rolling metrics from the stored buckets
	pub fn update_rolling_metrics(&mut self) {
		let now = Utc::now();

		// Calculate 1-hour rolling metrics from 5-minute buckets
		let hour_start = now - Duration::hours(1);
		let hour_aggregates = self.five_minute_buckets.get_range(hour_start, now);
		self.rolling_metrics.last_hour = self.compute_aggregate_from_buckets(hour_aggregates);

		// Calculate 24-hour rolling metrics from hourly buckets
		let day_start = now - Duration::days(1);
		let day_aggregates = self.hourly_buckets.get_range(day_start, now);
		self.rolling_metrics.last_day = self.compute_aggregate_from_buckets(day_aggregates);

		// Calculate 7-day rolling metrics from daily buckets
		let week_start = now - Duration::weeks(1);
		let week_aggregates = self.daily_buckets.get_range(week_start, now);
		self.rolling_metrics.last_week = self.compute_aggregate_from_buckets(week_aggregates);

		self.rolling_metrics.last_updated = now;
	}

	/// Combine multiple aggregates into a single aggregate
	fn compute_aggregate_from_buckets(
		&self,
		aggregates: Vec<&MetricsAggregate>,
	) -> Option<MetricsAggregate> {
		if aggregates.is_empty() {
			return None;
		}

		// Safe to use expect here since we know aggregates is not empty
		let start_time = aggregates
			.iter()
			.map(|a| a.time_range.start)
			.min()
			.expect("aggregates is not empty, min should exist");
		let end_time = aggregates
			.iter()
			.map(|a| a.time_range.end)
			.max()
			.expect("aggregates is not empty, max should exist");

		let mut combined = MetricsAggregate::new(TimeRange::new(start_time, end_time));

		// Calculate weighted average response time
		let mut total_weighted_time = 0.0;

		for aggregate in &aggregates {
			// Simple aggregation - in reality, we'd need to be more careful about averages
			combined.total_requests += aggregate.total_requests;
			combined.successful_requests += aggregate.successful_requests;
			combined.failed_requests += aggregate.failed_requests;
			combined.timeout_requests += aggregate.timeout_requests;

			// Accumulate weighted response time
			total_weighted_time += aggregate.avg_response_time_ms * aggregate.total_requests as f64;

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

		// Recalculate derived metrics
		if combined.total_requests > 0 {
			combined.success_rate =
				combined.successful_requests as f64 / combined.total_requests as f64;

			// Calculate weighted average of response times
			combined.avg_response_time_ms = total_weighted_time / combined.total_requests as f64;

			// Approximate p95 with max for now
			combined.p95_response_time_ms = combined.max_response_time_ms;
		}

		combined.last_updated = Utc::now();
		Some(combined)
	}

	/// Clean up old data beyond retention periods
	pub fn cleanup_old_data(&mut self) {
		// The cleanup happens automatically in MetricsBucket based on max_buckets
		// This method is here for future enhancements
	}
}
