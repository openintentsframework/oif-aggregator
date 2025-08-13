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

	let quotes = match state.aggregator_service.fetch_quotes(request.clone()).await {
		Ok(quotes) => quotes,
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

	let response = match QuotesResponse::from_domain_quotes(quotes) {
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
		"Returning {} quotes for request",
		response.total_quotes
	);
	Ok(Json(response))
}
