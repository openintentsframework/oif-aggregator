//! Global limits and defaults for configuration and runtime

/// Minimum allowed timeout for solver requests in milliseconds
pub const MIN_SOLVER_TIMEOUT_MS: u64 = 100; // 100ms

/// Maximum allowed timeout for solver requests in milliseconds
pub const MAX_SOLVER_TIMEOUT_MS: u64 = 30_000; // 30s

/// Maximum allowed timeout for global aggregation in milliseconds
pub const MAX_GLOBAL_TIMEOUT_MS: u64 = 60_000; // 1 minute

/// Default timeout for solver requests in milliseconds
pub const DEFAULT_SOLVER_TIMEOUT_MS: u64 = 2_000; // 2s

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
pub const DEFAULT_GLOBAL_TIMEOUT_MS: u64 = 5_000; // 5s

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
