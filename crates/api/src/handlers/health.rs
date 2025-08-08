use axum::{http::StatusCode, response::Json, extract::State};
use serde::Serialize;

use crate::state::AppState;

/// Health check endpoint
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Service healthy", body = String)),
    tag = "health"
))]
pub async fn health() -> &'static str { "OK" }

/// Readiness response
#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub status: String,
    pub storage_healthy: bool,
    pub adapters: std::collections::HashMap<String, bool>,
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
    let adapters = state.aggregator_service.health_check_all().await;
    let adapters_healthy = adapters.values().all(|v| *v) || adapters.is_empty();

    let overall = storage_healthy && adapters_healthy;
    let status = if overall { "ready" } else { "degraded" };

    let body = ReadinessResponse { status: status.to_string(), storage_healthy, adapters };
    let code = if overall { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(body))
}


