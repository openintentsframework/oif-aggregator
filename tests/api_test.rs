//! Tests for REST API endpoints

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use oif_aggregator::{
    api::routes::AppState, service::aggregator::AggregatorService, storage::MemoryStore,
};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

/// Create test application state
fn create_test_app_state() -> AppState {
    let solvers = vec![];
    let aggregator_service = AggregatorService::new(solvers, 5000);

    AppState {
        aggregator_service: Arc::new(aggregator_service),
        storage: MemoryStore::new(),
    }
}

/// Create test router
fn create_test_router() -> Router {
    oif_aggregator::create_router().with_state(create_test_app_state())
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"OK");
}

#[tokio::test]
async fn test_post_quotes_invalid_request() {
    let app = create_test_router();

    // Test with empty token_in
    let invalid_request = json!({
        "token_in": "",
        "token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
        "amount_in": "1000000000000000000",
        "chain_id": 1
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/quotes")
                .header("content-type", "application/json")
                .body(Body::from(invalid_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_post_quotes_valid_request() {
    let app = create_test_router();

    let valid_request = json!({
        "token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        "token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
        "amount_in": "1000000000000000000",
        "chain_id": 1,
        "slippage_tolerance": 0.005
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/quotes")
                .header("content-type", "application/json")
                .body(Body::from(valid_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Should have empty quotes array since no solvers are configured
    assert!(response_json["quotes"].is_array());
    assert_eq!(response_json["total_quotes"], 0);
    assert!(response_json["request_id"].is_string());
}

#[tokio::test]
async fn test_post_intents_invalid_request() {
    let app = create_test_router();

    // Test with empty user_address
    let invalid_request = json!({
        "user_address": "",
        "quote_id": "test-quote-id"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/intents")
                .header("content-type", "application/json")
                .body(Body::from(invalid_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_post_intents_missing_quote() {
    let app = create_test_router();

    // Test with neither quote_id nor quote_response
    let invalid_request = json!({
        "user_address": "0x1234567890123456789012345678901234567890"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/intents")
                .header("content-type", "application/json")
                .body(Body::from(invalid_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_post_intents_quote_not_found() {
    let app = create_test_router();

    let request = json!({
        "user_address": "0x1234567890123456789012345678901234567890",
        "quote_id": "non-existent-quote-id"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/intents")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_post_intents_with_quote_response() {
    let app = create_test_router();

    let request = json!({
        "user_address": "0x1234567890123456789012345678901234567890",
        "quote_response": {
            "token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
            "token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0", // USDC
            "amount_in": "1000000000000000000", // 1 ETH
            "amount_out": "2000000000", // 2000 USDC
            "chain_id": 1,
            "price_impact": 0.02,
            "estimated_gas": 150000
        },
        "slippage_tolerance": 0.01,
        "deadline": (chrono::Utc::now().timestamp() + 3600) // 1 hour from now
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/intents")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(response_json["status"], "pending");
    assert!(response_json["intent_id"].is_string());
    assert_eq!(
        response_json["message"],
        "Intent received and is being validated"
    );
}

#[tokio::test]
async fn test_get_intent_not_found() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/intents/non-existent-intent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_intent_workflow() {
    let app_state = create_test_app_state();
    let app = oif_aggregator::create_router().with_state(app_state.clone());

    // First create an intent
    let intent_request = json!({
        "user_address": "0x1234567890123456789012345678901234567890",
        "quote_response": {
            "token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
            "token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0", // USDC
            "amount_in": "1000000000000000000", // 1 ETH
            "amount_out": "2000000000", // 2000 USDC
            "chain_id": 1,
            "price_impact": 0.02,
            "estimated_gas": 150000
        },
        "slippage_tolerance": 0.01,
        "deadline": (chrono::Utc::now().timestamp() + 3600) // 1 hour from now
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/intents")
                .header("content-type", "application/json")
                .body(Body::from(intent_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
    let intent_id = create_json["intent_id"].as_str().unwrap();

    // Then query the intent status
    let status_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/v1/intents/{}", intent_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(status_response.status(), StatusCode::OK);

    let status_body = axum::body::to_bytes(status_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_json: serde_json::Value = serde_json::from_slice(&status_body).unwrap();

    assert_eq!(status_json["intent_id"], intent_id);
    assert_eq!(status_json["status"], "pending");
    assert_eq!(
        status_json["user_address"],
        "0x1234567890123456789012345678901234567890"
    );
}

#[tokio::test]
async fn test_quote_and_intent_workflow() {
    let app_state = create_test_app_state();
    let app = oif_aggregator::create_router().with_state(app_state.clone());

    // First get quotes (will be empty since no solvers configured)
    let quote_request = json!({
        "token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        "token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
        "amount_in": "1000000000000000000",
        "chain_id": 1
    });

    let quote_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/quotes")
                .header("content-type", "application/json")
                .body(Body::from(quote_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(quote_response.status(), StatusCode::OK);

    // Then create an intent with stateless quote_response
    let intent_request = json!({
        "user_address": "0x1234567890123456789012345678901234567890",
        "quote_response": {
            "token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
            "token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0", // USDC
            "amount_in": "1000000000000000000", // 1 ETH
            "amount_out": "2000000000", // 2000 USDC
            "chain_id": 1,
            "price_impact": 0.02,
            "estimated_gas": 150000
        },
        "slippage_tolerance": 0.01,
        "deadline": (chrono::Utc::now().timestamp() + 3600) // 1 hour from now
    });

    let intent_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/intents")
                .header("content-type", "application/json")
                .body(Body::from(intent_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(intent_response.status(), StatusCode::OK);
}
