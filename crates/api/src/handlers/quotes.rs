use axum::{extract::State, http::StatusCode, response::Json};
use tracing::info;

use crate::handlers::common::ErrorResponse;
use crate::state::AppState;
use oif_types::quotes::request::QuoteRequest;
use oif_types::quotes::response::QuotesResponse;

/// Get quotes for a swap request using standard OIF format
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/v1/quotes",
    request_body = QuoteRequest,
    responses(
        (status = 200, description = "Quotes aggregated successfully", body = QuotesResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    ),
    tag = "quotes"
))]
/// POST /v1/quotes - Get quotes
pub async fn post_quotes(
	State(state): State<AppState>,
	Json(request): Json<QuoteRequest>,
) -> Result<Json<QuotesResponse>, (StatusCode, Json<ErrorResponse>)> {
	info!(
		"Received quotes request with {} inputs and {} outputs",
		request.available_inputs.len(),
		request.requested_outputs.len()
	);

	// Validate the request
	if let Err(e) = request.validate() {
		return Err((
			StatusCode::BAD_REQUEST,
			Json(ErrorResponse {
				error: "VALIDATION_ERROR".to_string(),
				message: format!("Invalid request: {}", e),
				timestamp: chrono::Utc::now().timestamp(),
			}),
		));
	}

	info!(
		"Processing quotes request with {} inputs and {} outputs",
		request.available_inputs.len(),
		request.requested_outputs.len()
	);

	let (quotes, metadata) = match state.aggregator_service.fetch_quotes(request.clone()).await {
		Ok((quotes, metadata)) => (quotes, metadata),
		Err(e) => {
			return Err((
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					error: "AGGREGATION_ERROR".to_string(),
					message: format!("Failed to fetch quotes: {}", e),
					timestamp: chrono::Utc::now().timestamp(),
				}),
			));
		},
	};

	// Convert service metadata to API metadata
	let api_metadata = oif_types::quotes::response::AggregationMetadata {
		total_duration_ms: metadata.total_duration_ms,
		solver_timeout_ms: metadata.solver_timeout_ms,
		global_timeout_ms: metadata.global_timeout_ms,
		early_termination: metadata.early_termination,
		total_solvers_available: metadata.total_solvers_available,
		solvers_queried: metadata.solvers_queried,
		solvers_responded_success: metadata.solvers_responded_success,
		solvers_responded_error: metadata.solvers_responded_error,
		solvers_timed_out: metadata.solvers_timed_out,
		min_quotes_required: metadata.min_quotes_required,
		solver_selection_mode: metadata.solver_selection_mode,
	};

	let response = match QuotesResponse::from_domain_quotes_with_metadata(quotes, api_metadata) {
		Ok(resp) => resp,
		Err(e) => {
			return Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ErrorResponse {
					error: "CONVERSION_ERROR".to_string(),
					message: format!("Failed to convert quotes: {}", e),
					timestamp: chrono::Utc::now().timestamp(),
				}),
			))
		},
	};

	info!(
		"Returning {} quotes for request (duration: {}ms, {} solvers queried)",
		response.total_quotes, metadata.total_duration_ms, metadata.solvers_queried
	);
	Ok(Json(response))
}
