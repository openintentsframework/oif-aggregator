use serde::Serialize;
#[cfg(feature = "openapi")]
use serde_json::json;
use std::collections::HashMap;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Solver statistics for health checks and monitoring
#[derive(Debug, Serialize, Clone)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "total": 3,
    "active": 2,
    "inactive": 1,
    "healthy": 2,
    "unhealthy": 1,
    "healthDetails": {
        "example-solver": true,
        "uniswap-solver": true,
        "down-solver": false
    }
})))]
#[serde(rename_all = "camelCase")]
pub struct SolverStats {
	pub total: usize,
	pub active: usize,
	pub inactive: usize,
	pub healthy: usize,
	pub unhealthy: usize,
	pub health_details: HashMap<String, bool>,
}

/// Comprehensive health response
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "status": "healthy",
    "version": "0.1.0",
    "solvers": {
        "total": 3,
        "active": 2,
        "inactive": 1,
        "healthy": 2,
        "unhealthy": 1,
        "healthDetails": {
            "example-solver": true,
            "uniswap-solver": true,
            "down-solver": false
        }
    },
    "storage": {
        "healthy": true,
        "backend": "memory"
    }
})))]
pub struct HealthResponse {
	pub status: String,
	pub version: String,
	pub solvers: SolverStats,
	pub storage: StorageHealthInfo,
}

/// Storage health information
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(example = json!({
    "healthy": true,
    "backend": "memory"
})))]
pub struct StorageHealthInfo {
	pub healthy: bool,
	pub backend: String,
}
