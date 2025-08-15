//! Tests for REST API endpoints

use axum::{
	body::Body,
	http::{Request, StatusCode},
	Router,
};
use oif_aggregator::{
	serde_json::{self, json},
	AggregatorBuilder,
};
use oif_types::serde_json::Value;
use tower::ServiceExt;

mod mocks;

use mocks::{api_fixtures::ApiFixtures, api_fixtures::AppStateBuilder};

// Import e2e fixtures for proper integrity testing
mod e2e;
use e2e::fixtures;

/// Create test router with async state builder
async fn create_test_router() -> Router {
	let state = AppStateBuilder::minimal()
		.await
		.expect("Failed to create app state");
	oif_aggregator::create_router().with_state(state)
}

/// Create test router with mock adapters for quote/order tests
async fn create_test_router_with_mock_adapters() -> Result<Router, Box<dyn std::error::Error>> {
	// Set required environment variable for tests - use same secret as e2e fixtures
	std::env::set_var(
		"INTEGRITY_SECRET",
		"test-secret-for-e2e-tests-12345678901234567890",
	);

	let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
	let mock_solver = oif_aggregator::mocks::mock_solver();

	let mut settings = oif_config::Settings::default();
	settings.security.integrity_secret =
		oif_config::ConfigurableValue::from_env("INTEGRITY_SECRET");

	let (router, _) = AggregatorBuilder::new()
		.with_settings(settings)
		.with_solver(mock_solver)
		.with_adapter(Box::new(mock_adapter))?
		.start()
		.await?;

	Ok(router)
}

#[tokio::test]
async fn test_health_endpoint() {
	let app = create_test_router().await;

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

	// Parse JSON response
	let json_body: serde_json::Value = serde_json::from_slice(&body).unwrap();

	// Verify the response structure
	assert!(json_body.get("status").is_some());
	assert!(json_body.get("version").is_some());
	assert!(json_body.get("solvers").is_some());
	assert!(json_body.get("storage").is_some());

	// Verify solvers structure contains expected fields
	let solvers = json_body.get("solvers").unwrap();
	assert!(solvers.get("total").is_some());
	assert!(solvers.get("active").is_some());
	assert!(solvers.get("inactive").is_some());
	assert!(solvers.get("healthy").is_some());
	assert!(solvers.get("unhealthy").is_some());
	assert!(solvers.get("health_details").is_some());

	// Verify storage structure
	let storage = json_body.get("storage").unwrap();
	assert!(storage.get("healthy").is_some());
	assert!(storage.get("backend").is_some());

	// Verify version matches package version
	assert_eq!(
		json_body["version"].as_str().unwrap(),
		env!("CARGO_PKG_VERSION")
	);
}

#[tokio::test]
async fn test_post_quotes_invalid_request() {
	let app = create_test_router().await;

	// Use mock invalid quote request (missing user)
	let invalid_request = ApiFixtures::invalid_quote_request_missing_user();

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

	assert!(
		response.status() == StatusCode::BAD_REQUEST
			|| response.status() == StatusCode::UNPROCESSABLE_ENTITY
	);
}

#[tokio::test]
async fn test_post_quotes_valid_request() {
	let app = create_test_router().await;

	// Use mock valid quote request
	let valid_request = ApiFixtures::valid_quote_request();

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

	// Status might be 400 due to validation or 200 with empty results
	assert!(response.status().is_success() || response.status() == StatusCode::BAD_REQUEST);

	if response.status().is_success() {
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Should have empty quotes array since no solvers are configured
		assert!(response_json["quotes"].is_array());
		assert_eq!(response_json["totalQuotes"], 0);
	}
}

#[tokio::test]
async fn test_post_orders_invalid_request() {
	let app = create_test_router().await;

	// Test with empty body
	let invalid_request = json!({});

	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v1/orders")
				.header("content-type", "application/json")
				.body(Body::from(invalid_request.to_string()))
				.unwrap(),
		)
		.await
		.unwrap();

	assert!(
		response.status() == StatusCode::BAD_REQUEST
			|| response.status() == StatusCode::UNPROCESSABLE_ENTITY
	);
}

#[tokio::test]
async fn test_post_orders_missing_quote() {
	let app = create_test_router().await;

	// Use mock invalid order request (missing quote data)
	let invalid_request = ApiFixtures::invalid_order_request_missing_user();

	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v1/orders")
				.header("content-type", "application/json")
				.body(Body::from(invalid_request.to_string()))
				.unwrap(),
		)
		.await
		.unwrap();

	assert!(
		response.status() == StatusCode::BAD_REQUEST
			|| response.status() == StatusCode::UNPROCESSABLE_ENTITY
	);
}

#[tokio::test]
async fn test_post_orders_with_quote_response() {
	let app = create_test_router_with_mock_adapters()
		.await
		.expect("Failed to create test router");

	// Step 1: First get a real quote from the server
	let quote_request = ApiFixtures::valid_quote_request();
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

	let quote_body = axum::body::to_bytes(quote_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let quotes_json: serde_json::Value = serde_json::from_slice(&quote_body).unwrap();
	let quotes = quotes_json["quotes"].as_array().expect("No quotes array");

	if quotes.is_empty() {
		// If no quotes available, this test can't proceed
		return;
	}

	let first_quote = &quotes[0];
	let user_addr = quote_request["user"]
		.as_str()
		.expect("No user in quote request");

	// Step 2: Create order request with the real quote
	let order_request = json!({
		"userAddress": user_addr,
		"quoteResponse": first_quote
	});

	let response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v1/orders")
				.header("content-type", "application/json")
				.body(Body::from(order_request.to_string()))
				.unwrap(),
		)
		.await
		.unwrap();

	// Should succeed with proper integrity verification
	assert_eq!(response.status(), StatusCode::OK);

	// Verify response contains order ID
	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let json_body: serde_json::Value = serde_json::from_slice(&body).unwrap();
	assert!(json_body["orderId"].is_string());
}

#[tokio::test]
async fn test_get_order_not_found() {
	let app = create_test_router().await;

	let response = app
		.oneshot(
			Request::builder()
				.uri("/v1/orders/non-existent-order-id")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_solvers_endpoint() {
	let app = create_test_router().await;

	let response = app
		.oneshot(
			Request::builder()
				.uri("/v1/solvers")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(response.status(), StatusCode::OK);

	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let response_json: Value = serde_json::from_slice(&body).unwrap();

	assert!(response_json["solvers"].is_array());
	assert!(response_json["totalSolvers"].is_number());
}

#[tokio::test]
async fn test_order_workflow() {
	let app = create_test_router_with_mock_adapters()
		.await
		.expect("Failed to create test router");

	// Use proper order request with valid integrity checksum
	let order_request = fixtures::valid_order_request_with_integrity();

	let create_response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v1/orders")
				.header("content-type", "application/json")
				.body(Body::from(order_request.to_string()))
				.unwrap(),
		)
		.await
		.unwrap();

	// Should succeed with proper integrity checksum
	assert_eq!(create_response.status(), StatusCode::OK);

	let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
	let order_id = create_json["orderId"].as_str().unwrap();

	// Verify order creation response
	assert!(create_json["orderId"].is_string());
	assert!(create_json["status"].is_string());

	// Then query the order status to test refresh_order_status functionality
	let status_response = app
		.oneshot(
			Request::builder()
				.uri(format!("/v1/orders/{}", order_id))
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

	// Verify order status response
	assert_eq!(status_json["orderId"], order_id);
	assert!(status_json["status"].is_string());
}

#[tokio::test]
async fn test_quote_and_order_workflow() {
	let app = create_test_router_with_mock_adapters()
		.await
		.expect("Failed to create test router");

	// Use mock quote request for workflow test
	let quote_request = fixtures::valid_quote_request();

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

	assert!(quote_response.status().is_success());

	let quote_body = axum::body::to_bytes(quote_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let quotes_json: serde_json::Value = serde_json::from_slice(&quote_body).unwrap();
	let quotes = quotes_json["quotes"].as_array().expect("No quotes array");

	if quotes.is_empty() {
		println!("No quotes returned from mock adapter");
		panic!()
	}

	let first_quote = &quotes[0];
	let user_addr = quote_request["user"]
		.as_str()
		.expect("No user in quote request");

	let order_request = serde_json::json!({
			"userAddress": user_addr,
			"quoteResponse": first_quote
	});

	let order_response = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v1/orders")
				.header("content-type", "application/json")
				.body(Body::from(order_request.to_string()))
				.unwrap(),
		)
		.await
		.unwrap();

	// Order might succeed or fail due to integrity verification
	assert!(order_response.status() == StatusCode::OK);
}
