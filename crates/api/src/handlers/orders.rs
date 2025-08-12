//! Order handlers
use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::{Json, NoContent},
};
use tracing::{debug, info};

use crate::handlers::common::ErrorResponse;
use crate::state::AppState;
use oif_types::{OrdersRequest, OrdersResponse};

/// Submit a new order
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/v1/orders",
    request_body = OrdersRequest,
    responses(
        (status = 200, description = "Order created", body = OrdersResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Quote not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    ),
    tag = "orders"
))]
/// POST /v1/orders - Submit an order
pub async fn post_orders(
	State(state): State<AppState>,
	Json(request): Json<OrdersRequest>,
) -> Result<Json<OrdersResponse>, (StatusCode, Json<ErrorResponse>)> {
	info!(
		"Received order submission for user {}",
		request.user_address
	);

	let order = match state.order_service.submit_order(&request).await {
		Ok(order) => order,
		Err(e) => {
			return Err(match e {
				oif_service::OrderServiceError::Validation(msg) => (
					StatusCode::BAD_REQUEST,
					Json(ErrorResponse {
						error: "VALIDATION_ERROR".to_string(),
						message: msg,
						timestamp: chrono::Utc::now().timestamp(),
					}),
				),
				oif_service::OrderServiceError::QuoteNotFound(q) => (
					StatusCode::NOT_FOUND,
					Json(ErrorResponse {
						error: "QUOTE_NOT_FOUND".to_string(),
						message: format!("Quote {} not found", q),
						timestamp: chrono::Utc::now().timestamp(),
					}),
				),
				oif_service::OrderServiceError::QuoteExpired(q) => (
					StatusCode::BAD_REQUEST,
					Json(ErrorResponse {
						error: "QUOTE_EXPIRED".to_string(),
						message: format!("Quote {} has expired", q),
						timestamp: chrono::Utc::now().timestamp(),
					}),
				),
				oif_service::OrderServiceError::Storage(msg) => (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(ErrorResponse {
						error: "STORAGE_ERROR".to_string(),
						message: msg,
						timestamp: chrono::Utc::now().timestamp(),
					}),
				),
				oif_service::OrderServiceError::SolverNotFound(solver_id) => (
					StatusCode::NOT_FOUND,
					Json(ErrorResponse {
						error: "SOLVER_NOT_FOUND".to_string(),
						message: format!("Solver {} not found", solver_id),
						timestamp: chrono::Utc::now().timestamp(),
					}),
				),
				oif_service::OrderServiceError::AdapterNotFound(msg) => (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(ErrorResponse {
						error: "ADAPTER_NOT_FOUND".to_string(),
						message: msg,
						timestamp: chrono::Utc::now().timestamp(),
					}),
				),
				oif_service::OrderServiceError::Adapter(msg) => (
					StatusCode::BAD_GATEWAY,
					Json(ErrorResponse {
						error: "ADAPTER_ERROR".to_string(),
						message: msg,
						timestamp: chrono::Utc::now().timestamp(),
					}),
				),
			})
		},
	};

	let response = match OrdersResponse::from_domain(&order) {
		Ok(resp) => resp,
		Err(e) => {
			return Err((
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ErrorResponse {
					error: "CONVERSION_ERROR".to_string(),
					message: format!("Failed to convert order: {}", e),
					timestamp: chrono::Utc::now().timestamp(),
				}),
			))
		},
	};

	info!(
		"Created order {} for user {}",
		order.order_id, order.user_address
	);
	Ok(Json(response))
}

/// Get order status by ID
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/v1/orders/{id}",
    params(("id" = String, Path, description = "Order ID")),
    responses(
        (status = 200, description = "Order status", body = OrderStatusResponse),
        (status = 404, description = "Order not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    ),
    tag = "orders"
))]
/// GET /v1/orders/:id - Get order status by ID
pub async fn get_order_status(
	State(state): State<AppState>,
	Path(order_id): Path<String>,
) -> Result<NoContent, (StatusCode, Json<ErrorResponse>)> {
	debug!("Querying status for order {}", order_id);
	unimplemented!()
}
