use axum::{extract::State, http::StatusCode, response::Json};
use tracing::info;

use crate::handlers::common::ErrorResponse;
use crate::state::AppState;
use oif_types::quotes::request::QuotesRequest;
use oif_types::quotes::response::QuotesResponse;

/// Get quotes for a swap request
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/v1/quotes",
    request_body = QuotesRequest,
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
	Json(request): Json<QuotesRequest>,
) -> Result<Json<QuotesResponse>, (StatusCode, Json<ErrorResponse>)> {
	info!(
		"Received quotes request for {}/{} on chain {}",
		request.token_in, request.token_out, request.chain_id
	);

	let quote_request: oif_types::QuoteRequest = match request.try_into() {
		Ok(req) => req,
		Err(e) => {
			return Err((
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					error: "VALIDATION_ERROR".to_string(),
					message: format!("Invalid request: {}", e),
					timestamp: chrono::Utc::now().timestamp(),
				}),
			))
		},
	};

	let quotes = state
		.aggregator_service
		.fetch_quotes(quote_request.clone())
		.await;

	let response = match QuotesResponse::from_domain_quotes(quote_request.request_id, quotes) {
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

	info!("Returning {} quotes for request", response.total_quotes);
	Ok(Json(response))
}
