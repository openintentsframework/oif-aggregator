//! Solvers handlers

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::Json,
};
use tracing::debug;

use crate::handlers::common::ErrorResponse;
use crate::pagination::PaginationQuery;
use crate::state::AppState;
use oif_types::solvers::response::{SolverResponse, SolversResponse};

/// GET /v1/solvers - List all solvers
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/v1/solvers",
    params(
        ("page" = Option<u32>, Query, description = "Page number (1-based)", example = 1),
        ("page_size" = Option<u32>, Query, description = "Items per page (1-100)", example = 25)
    ),
    responses((status = 200, description = "List of solvers", body = SolversResponse)),
    tag = "solvers"
))]
pub async fn get_solvers(
	State(state): State<AppState>,
	Query(pq): Query<PaginationQuery>,
) -> Result<Json<SolversResponse>, (StatusCode, Json<ErrorResponse>)> {
	debug!("Listing solvers with pagination");
	let (page_items, total, _active_count, _healthy_count) = state
		.solver_service
		.list_solvers_paginated(pq.page, pq.page_size)
		.await
		.map_err(|e| {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ErrorResponse {
					error: "STORAGE_ERROR".to_string(),
					message: e.to_string(),
					timestamp: chrono::Utc::now().timestamp(),
				}),
			)
		})?;

	// Build page responses
	let responses: Result<Vec<_>, _> = page_items.iter().map(SolverResponse::try_from).collect();
	let responses = responses.map_err(|e| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ErrorResponse {
				error: "CONVERSION_ERROR".to_string(),
				message: e.to_string(),
				timestamp: chrono::Utc::now().timestamp(),
			}),
		)
	})?;

	let response = SolversResponse {
		solvers: responses,
		total_solvers: total,
		timestamp: chrono::Utc::now().timestamp(),
	};
	Ok(Json(response))
}

/// GET /v1/solvers/{id} - Get solver by id
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/v1/solvers/{id}",
    params(("id" = String, Path, description = "Solver ID")),
    responses((status = 200, description = "Solver details", body = SolverResponse), (status = 404, description = "Not found", body = ErrorResponse)),
    tag = "solvers"
))]
pub async fn get_solver_by_id(
	State(state): State<AppState>,
	Path(solver_id): Path<String>,
) -> Result<Json<SolverResponse>, (StatusCode, Json<ErrorResponse>)> {
	let solver = state
		.solver_service
		.get_solver(&solver_id)
		.await
		.map_err(|e| match e {
			oif_service::SolverServiceError::NotFound(_) => (
				StatusCode::NOT_FOUND,
				Json(ErrorResponse {
					error: "SOLVER_NOT_FOUND".to_string(),
					message: format!("Solver {} not found", solver_id),
					timestamp: chrono::Utc::now().timestamp(),
				}),
			),
			oif_service::SolverServiceError::Storage(msg) => (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ErrorResponse {
					error: "STORAGE_ERROR".to_string(),
					message: msg,
					timestamp: chrono::Utc::now().timestamp(),
				}),
			),
		})?;

	let response = SolverResponse::try_from(&solver).map_err(|e| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ErrorResponse {
				error: "CONVERSION_ERROR".to_string(),
				message: e.to_string(),
				timestamp: chrono::Utc::now().timestamp(),
			}),
		)
	})?;
	Ok(Json(response))
}
