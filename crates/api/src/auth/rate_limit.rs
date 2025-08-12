//! Rate limiting implementations

use oif_types::auth::{
	errors::RateLimitError,
	traits::{RateLimitCheck, RateLimiter, RateLimits},
	RateLimitResult,
};
use oif_types::constants::limits::RATE_LIMIT_WINDOW_SECONDS;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use rand;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct RequestCounter {
	/// Request count in current window
	count: u32,
	/// Window start time
	window_start: chrono::DateTime<Utc>,
	/// Window duration in seconds
	window_duration: u64,
}

/// In-memory rate limiter (generic key-based)
#[derive(Debug)]
pub struct MemoryRateLimiter {
	/// Request counters by key
	counters: Arc<DashMap<String, RequestCounter>>,
}

impl MemoryRateLimiter {
	/// Create a new memory rate limiter
	pub fn new() -> Self {
		Self {
			counters: Arc::new(DashMap::new()),
		}
	}

	/// Clean up expired windows (should be called periodically)
	pub fn cleanup_expired(&self) {
		let now = Utc::now();
		let mut to_remove = Vec::new();

		for entry in self.counters.iter() {
			let counter = entry.value();
			let window_end =
				counter.window_start + Duration::seconds(counter.window_duration as i64);
			if now > window_end {
				to_remove.push(entry.key().clone());
			}
		}

		for key in to_remove {
			self.counters.remove(&key);
		}
	}
}

#[async_trait]
impl RateLimiter for MemoryRateLimiter {
	async fn check_rate_limit(
		&self,
		key: &str,
		limits: &RateLimits,
	) -> RateLimitResult<RateLimitCheck> {
		let now = Utc::now();
		let window_duration = RATE_LIMIT_WINDOW_SECONDS; // Use constant from limits

		// Clean up expired entries periodically
		if rand::random::<f64>() < 0.01 {
			self.cleanup_expired();
		}

		let mut entry = self
			.counters
			.entry(key.to_string())
			.or_insert_with(|| RequestCounter {
				count: 0,
				window_start: now,
				window_duration,
			});

		let counter = entry.value_mut();

		// Check if we need a new window
		let window_end = counter.window_start + Duration::seconds(window_duration as i64);
		if now > window_end {
			// Start new window
			counter.count = 0;
			counter.window_start = now;
		}

		let allowed = counter.count < limits.requests_per_minute;
		let remaining = limits.requests_per_minute.saturating_sub(counter.count);
		let reset_at = counter.window_start + Duration::seconds(window_duration as i64);

		Ok(RateLimitCheck {
			allowed,
			remaining,
			reset_at,
			limit: limits.requests_per_minute,
		})
	}

	async fn record_request(&self, key: &str) -> Result<(), RateLimitError> {
		let now = Utc::now();
		let window_duration = RATE_LIMIT_WINDOW_SECONDS;

		let mut entry = self
			.counters
			.entry(key.to_string())
			.or_insert_with(|| RequestCounter {
				count: 0,
				window_start: now,
				window_duration,
			});

		let counter = entry.value_mut();

		// Check if we need a new window
		let window_end = counter.window_start + Duration::seconds(window_duration as i64);
		if now > window_end {
			counter.count = 1;
			counter.window_start = now;
		} else {
			counter.count += 1;
		}

		Ok(())
	}

	async fn get_usage(&self, key: &str) -> Result<u32, RateLimitError> {
		if let Some(counter) = self.counters.get(key) {
			Ok(counter.count)
		} else {
			Ok(0)
		}
	}

	async fn reset_limit(&self, key: &str) -> Result<(), RateLimitError> {
		self.counters.remove(key);
		Ok(())
	}

	async fn health_check(&self) -> Result<bool, RateLimitError> {
		Ok(true)
	}

	fn name(&self) -> &str {
		"MemoryRateLimiter"
	}
}

impl Default for MemoryRateLimiter {
	fn default() -> Self {
		Self::new()
	}
}
