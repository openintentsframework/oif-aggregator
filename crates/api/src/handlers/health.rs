//! Health check endpoints
//!
//! This module contains the health check endpoints for the aggregator.
//!
//! The health check endpoint is used to check if the aggregator is healthy.
//! The readiness endpoint is used to check if the aggregator is ready to serve requests.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;

use crate::state::AppState;
use std::collections::HashMap;

/// Health check endpoint
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Service healthy", body = String)),
    tag = "health"
))]
pub async fn health() -> &'static str {
	"OK"
}

/// Readiness response
#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
	pub status: String,
	pub storage_healthy: bool,
	pub solvers: HashMap<String, bool>,
}

/// GET /ready - Readiness probe with storage and adapter checks
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/ready",
    responses((status = 200, description = "Readiness response")),
    tag = "health"
))]
pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<ReadinessResponse>) {
	let storage_healthy = state.storage.health_check().await.unwrap_or(false);
	// Delegate health checking to solver service to avoid adapter coupling in aggregator
	let solvers: HashMap<String, bool> = (state.solver_service.health_check_all().await).unwrap_or_default();
	let solvers_healthy = solvers.values().all(|v| *v) || solvers.is_empty();

	let overall = storage_healthy && solvers_healthy;
	let status = if overall { "ready" } else { "degraded" };

	let body = ReadinessResponse {
		status: status.to_string(),
		storage_healthy,
		solvers,
	};
	let code = if overall {
		StatusCode::OK
	} else {
		StatusCode::SERVICE_UNAVAILABLE
	};
	(code, Json(body))
}
