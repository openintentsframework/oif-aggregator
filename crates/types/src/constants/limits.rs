//! Global limits and defaults for configuration and runtime

/// Minimum allowed timeout for solver requests in milliseconds
pub const MIN_SOLVER_TIMEOUT_MS: u64 = 100; // 100ms

/// Maximum allowed timeout for solver requests in milliseconds
pub const MAX_SOLVER_TIMEOUT_MS: u64 = 30_000; // 30s

/// Maximum allowed timeout for global aggregation in milliseconds
pub const MAX_GLOBAL_TIMEOUT_MS: u64 = 60_000; // 1 minute

/// Default timeout for solver requests in milliseconds
pub const DEFAULT_SOLVER_TIMEOUT_MS: u64 = 5_000; // 5s

/// Maximum allowed retry attempts for solvers
pub const MAX_SOLVER_RETRIES: u32 = 10;

/// Default maximum retry attempts for solvers
pub const DEFAULT_SOLVER_RETRIES: u32 = 2;

/// Default rate limit: requests per minute
pub const DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE: u32 = 1000;

/// Default rate limit: burst size (immediate requests allowed)
pub const DEFAULT_RATE_LIMIT_BURST_SIZE: u32 = 100;

/// Rate limit window duration in seconds
pub const RATE_LIMIT_WINDOW_SECONDS: u64 = 60;

/// Default global timeout for quote aggregation in milliseconds
pub const DEFAULT_GLOBAL_TIMEOUT_MS: u64 = 10_000; // 10s

/// Default minimum quotes required for aggregation
pub const DEFAULT_MIN_QUOTES: u64 = 30;

/// Default sample size for sampled solver selection
pub const DEFAULT_SAMPLE_SIZE: u64 = 30;

/// Default priority threshold for priority-based solver selection
pub const DEFAULT_PRIORITY_THRESHOLD: u64 = 0;

/// Maximum allowed priority threshold for solver selection
pub const MAX_PRIORITY_THRESHOLD: u64 = 100;

/// Default maximum concurrent solvers for aggregation
pub const DEFAULT_MAX_CONCURRENT_SOLVERS: usize = 50;

/// Default maximum retries per solver for aggregation
pub const DEFAULT_MAX_RETRIES_PER_SOLVER: u32 = 2;

/// Default retry delay in milliseconds between solver attempts
pub const DEFAULT_RETRY_DELAY_MS: u64 = 100;

/// Default order retention days for cleanup job
pub const DEFAULT_ORDER_RETENTION_DAYS: u32 = 10;

/// Default behavior for including solvers with unknown compatibility
/// Set to false for more predictable results - only include solvers with known compatibility
pub const DEFAULT_INCLUDE_UNKNOWN_COMPATIBILITY: bool = false;

/// Default metrics retention period in hours (7 days)
pub const DEFAULT_METRICS_RETENTION_HOURS: u32 = 168;

/// Default metrics cleanup interval in hours (run daily)
pub const DEFAULT_METRICS_CLEANUP_INTERVAL_HOURS: u32 = 24;

/// Default metrics aggregation update interval in minutes (update rolling metrics every 5 minutes)
pub const DEFAULT_METRICS_AGGREGATION_INTERVAL_MINUTES: u32 = 5;

// Circuit Breaker Defaults
/// Default circuit breaker enabled state (enabled by default)
pub const DEFAULT_CIRCUIT_BREAKER_ENABLED: bool = true;

/// Default consecutive failure threshold before opening circuit
pub const DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 5;

/// Default success rate threshold (below this rate, circuit opens)
/// 0.20 = 20% success rate required to keep circuit closed
pub const DEFAULT_CIRCUIT_BREAKER_SUCCESS_RATE_THRESHOLD: f64 = 0.2;

/// Default minimum requests needed before evaluating success rate
pub const DEFAULT_CIRCUIT_BREAKER_MIN_REQUESTS_FOR_RATE_CHECK: u64 = 30;

/// Default base timeout in seconds for exponential backoff calculation
pub const DEFAULT_CIRCUIT_BREAKER_BASE_TIMEOUT_SECONDS: u64 = 10;

/// Default maximum timeout in seconds (caps exponential backoff)
pub const DEFAULT_CIRCUIT_BREAKER_MAX_TIMEOUT_SECONDS: u64 = 600; // 10 minutes

/// Default metrics window duration in minutes
/// Window size for recent metrics used in circuit breaker decisions
/// 15 minutes provides good balance between recent data and statistical significance
pub const DEFAULT_METRICS_WINDOW_DURATION_MINUTES: u32 = 15;

/// Default service error rate threshold (above this rate, circuit opens)
/// 0.2 = 20% service error rate threshold (more lenient than success rate)
pub const DEFAULT_CIRCUIT_BREAKER_SERVICE_ERROR_THRESHOLD: f64 = 0.2;

/// Default maximum window age in minutes before forcing reset
/// Even if window has insufficient data, force reset after this duration
/// 60 minutes = 4x normal window, good balance for low-traffic solvers
pub const DEFAULT_METRICS_MAX_WINDOW_AGE_MINUTES: u32 = 60;

/// Default maximum test requests allowed in half-open state
pub const DEFAULT_CIRCUIT_BREAKER_HALF_OPEN_MAX_CALLS: u32 = 5;

/// Default maximum recovery attempts before applying persistent failure action
pub const DEFAULT_CIRCUIT_BREAKER_MAX_RECOVERY_ATTEMPTS: u32 = 10;

/// Default metrics freshness window in minutes (how recent metrics must be)
/// Prevents circuit breaker decisions based on stale data - if metrics are older
/// than this, success rate evaluation is skipped (fail-open approach)
pub const DEFAULT_CIRCUIT_BREAKER_METRICS_MAX_AGE_MINUTES: u32 = 30;
