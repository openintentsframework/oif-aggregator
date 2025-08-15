use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use oif_service::SolverStats;

/// Comprehensive health response
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct HealthResponse {
	pub status: String,
	pub timestamp: u64,
	pub version: String,
	pub solvers: SolverStats,
	pub storage: StorageHealthInfo,
}

/// Storage health information
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct StorageHealthInfo {
	pub healthy: bool,
	pub backend: String,
}
