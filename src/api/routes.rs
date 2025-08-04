//! Route handlers for the aggregator API

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{debug, info};

use crate::models::{
    // Intent models
    intents::{
        request::IntentsRequest,
        response::{IntentStatusResponse, IntentsResponse},
    },
    // Quote models
    quotes::{
        request::{QuoteRequest, QuotesRequest as QuoteRequestApi},
        response::QuotesResponse,
    },
};
use crate::service::aggregator::AggregatorService;
use crate::storage::MemoryStore;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub aggregator_service: Arc<AggregatorService>,
    pub storage: MemoryStore,
}

/// Health check endpoint
pub async fn health() -> &'static str {
    "OK"
}

/// Error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: i64,
}

/// POST /v1/quotes - Get quotes from all available solvers
pub async fn post_quotes(
    State(state): State<AppState>,
    Json(request): Json<QuoteRequestApi>,
) -> Result<Json<QuotesResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Received quotes request for {}/{} on chain {}",
        request.token_in, request.token_out, request.chain_id
    );

    // Validate and convert to domain model
    let quote_request: QuoteRequest = match request.try_into() {
        Ok(req) => req,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "VALIDATION_ERROR".to_string(),
                    message: format!("Invalid request: {}", e),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ));
        }
    };

    debug!(
        "Created quote request with ID: {}",
        quote_request.request_id
    );

    // Fetch quotes from aggregator service
    let quotes = state
        .aggregator_service
        .fetch_quotes(quote_request.clone())
        .await;

    // Store quotes in memory store for future reference
    for quote in &quotes {
        state.storage.add_quote(quote.clone());
    }

    // Convert to API response using new models
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
            ));
        }
    };

    info!("Returning {} quotes for request", response.total_quotes);

    Ok(Json(response))
}

/// POST /v1/intents - Submit an intent for execution
pub async fn post_intents(
    State(state): State<AppState>,
    Json(request): Json<IntentsRequest>,
) -> Result<Json<IntentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Received intent submission for user {}",
        request.user_address
    );

    // Extract IP address from request (in a real implementation, this would come from headers)
    let ip_address = None; // TODO: Extract from request headers

    // Validate and convert to domain model
    let intent = match request.to_domain(ip_address) {
        Ok(intent) => intent,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "VALIDATION_ERROR".to_string(),
                    message: format!("Invalid request: {}", e),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ));
        }
    };

    // If quote_id is provided, verify it exists and is not expired
    if let Some(ref quote_id) = intent.quote_id {
        match state.storage.get_quote(quote_id) {
            Some(quote) => {
                if quote.is_expired() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: "QUOTE_EXPIRED".to_string(),
                            message: format!("Quote {} has expired", quote_id),
                            timestamp: chrono::Utc::now().timestamp(),
                        }),
                    ));
                }
                debug!("Found valid quote {} for intent submission", quote_id);
            }
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "QUOTE_NOT_FOUND".to_string(),
                        message: format!("Quote {} not found", quote_id),
                        timestamp: chrono::Utc::now().timestamp(),
                    }),
                ));
            }
        }
    }

    // Store intent
    state.storage.add_intent(intent.clone());

    // Convert to API response using new models
    let response = match IntentsResponse::from_domain(&intent) {
        Ok(resp) => resp,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "CONVERSION_ERROR".to_string(),
                    message: format!("Failed to convert intent: {}", e),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ));
        }
    };

    info!(
        "Created intent {} for user {}",
        intent.intent_id, intent.user_address
    );

    Ok(Json(response))
}

/// GET /v1/intents/:id - Get intent status by ID
pub async fn get_intent_status(
    State(state): State<AppState>,
    Path(intent_id): Path<String>,
) -> Result<Json<IntentStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Querying status for intent {}", intent_id);

    match state.storage.get_intent(&intent_id) {
        Some(intent) => {
            // Convert to API response using new models
            let response = match IntentStatusResponse::from_domain(&intent) {
                Ok(resp) => resp,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "CONVERSION_ERROR".to_string(),
                            message: format!("Failed to convert intent status: {}", e),
                            timestamp: chrono::Utc::now().timestamp(),
                        }),
                    ));
                }
            };

            Ok(Json(response))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "INTENT_NOT_FOUND".to_string(),
                message: format!("Intent {} not found", intent_id),
                timestamp: chrono::Utc::now().timestamp(),
            }),
        )),
    }
}

/// Create the API router with all routes
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/v1/quotes", post(post_quotes))
        .route("/v1/intents", post(post_intents))
        .route("/v1/intents/{id}", get(get_intent_status))
}
