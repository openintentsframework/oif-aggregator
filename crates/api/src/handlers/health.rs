//! Health check endpoint
//!
//! This module contains the comprehensive health check endpoint for the aggregator.
//! The health endpoint returns detailed service status including storage and solver health.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::state::AppState;
use oif_service::SolverStats;

/// Comprehensive health response
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct HealthResponse {
	pub status: String,
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

/// GET /health - Comprehensive health check with detailed status
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Service healthy", body = HealthResponse)),
    tag = "health"
))]
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
	let storage_healthy = state.storage.health_check().await.unwrap_or(false);

	// Get comprehensive solver statistics
	let solver_stats = state
		.solver_service
		.get_stats()
		.await
		.unwrap_or_else(|_| SolverStats {
			total: 0,
			active: 0,
			inactive: 0,
			healthy: 0,
			unhealthy: 0,
			health_details: std::collections::HashMap::new(),
		});

	// Determine overall health
	let solvers_healthy = solver_stats.healthy == solver_stats.total || solver_stats.total == 0;
	let overall_healthy = storage_healthy && solvers_healthy;

	let status = if overall_healthy {
		"healthy"
	} else {
		"degraded"
	};

	let response = HealthResponse {
		status: status.to_string(),
		version: env!("CARGO_PKG_VERSION").to_string(),
		solvers: solver_stats,
		storage: StorageHealthInfo {
			healthy: storage_healthy,
			backend: "memory".to_string(), // Could be made configurable
		},
	};

	let status_code = if overall_healthy {
		StatusCode::OK
	} else {
		StatusCode::SERVICE_UNAVAILABLE
	};

	(status_code, Json(response))
}
