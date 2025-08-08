use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::Json,
};
use tracing::info;

use crate::handlers::common::ErrorResponse;
use crate::pagination::{slice_bounds, PaginationQuery};
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
	info!("Listing all solvers");
	let solvers = state.solver_service.list_solvers().await.map_err(|e| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(ErrorResponse {
				error: "STORAGE_ERROR".to_string(),
				message: e.to_string(),
				timestamp: chrono::Utc::now().timestamp(),
			}),
		)
	})?;

	// Compute counts on full set
	let total = solvers.len();
	let active_count = solvers.iter().filter(|s| s.is_available()).count();
	let healthy_count = solvers.iter().filter(|s| s.is_healthy()).count();

	let (start, end, _page, _page_size) = slice_bounds(total, pq.page, pq.page_size);
	let page_slice = if start < total {
		&solvers[start..end]
	} else {
		&[]
	};

	// Build page responses
	let solver_responses: Result<Vec<_>, _> =
		page_slice.iter().map(SolverResponse::from_domain).collect();

	let responses = solver_responses.map_err(|e| {
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
		active_solvers: active_count,
		healthy_solvers: healthy_count,
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

	let response = SolverResponse::from_domain(&solver).map_err(|e| {
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
