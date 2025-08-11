//! Global limits and defaults for configuration and runtime

/// Minimum allowed timeout for solver requests in milliseconds
pub const MIN_SOLVER_TIMEOUT_MS: u64 = 100; // 100ms

/// Maximum allowed timeout for solver requests in milliseconds
pub const MAX_SOLVER_TIMEOUT_MS: u64 = 30_000; // 30s

/// Default timeout for solver requests in milliseconds
pub const DEFAULT_SOLVER_TIMEOUT_MS: u64 = 2_000; // 2s

/// Maximum allowed retry attempts for solvers
pub const MAX_SOLVER_RETRIES: u32 = 10;

/// Default maximum retry attempts for solvers
pub const DEFAULT_SOLVER_RETRIES: u32 = 3;
