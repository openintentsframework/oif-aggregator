//! Orders API E2E tests
//!
//! Tests for the /v1/orders endpoint covering order submission,
//! validation, and processing.

mod mocks;

use crate::mocks::{ApiFixtures, TestServer};
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn test_orders_stateless_flow() {
	let server = TestServer::spawn_with_mock_adapter()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	// Step 1: Get a real quote from the server first
	let quote_request = ApiFixtures::valid_quote_request();
	let quote_response = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to get quotes");

	assert!(
		quote_response.status().is_success(),
		"Failed to get quotes: {}",
		quote_response.status()
	);

	let quotes_json: serde_json::Value = quote_response
		.json()
		.await
		.expect("Failed to parse quotes response");
	let quotes = quotes_json["quotes"].as_array().expect("No quotes array");

	if quotes.is_empty() {
		//println!("No quotes returned from mock adapter");
		server.abort();
		return;
	}

	let first_quote = &quotes[0];

	// Step 2: Create order with real quote (stateless flow with quoteResponse)
	let order_request = serde_json::json!({
		"quoteResponse": first_quote,
		"signature": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12341b",
	});

	let resp = client
		.post(format!("{}/v1/orders", server.base_url))
		.json(&order_request)
		.send()
		.await
		.expect("Failed to post order");

	// Should succeed with proper integrity verification
	assert_eq!(resp.status(), reqwest::StatusCode::OK);

	let json: serde_json::Value = resp.json().await.expect("Failed to parse order response");
	//println!("Order creation response: {:?}", json);
	let order_id = json["orderId"].as_str().expect("No orderId in response");
	assert!(!order_id.is_empty());

	// Step 3: Query order status
	let resp = client
		.get(format!("{}/v1/orders/{}", server.base_url, order_id))
		.send()
		.await
		.expect("Failed to get order status");

	assert_eq!(resp.status(), reqwest::StatusCode::OK);
	let status_json: serde_json::Value = resp.json().await.expect("Failed to parse order status");
	assert_eq!(status_json["orderId"], order_id);
	assert!(status_json["status"].is_string());
	//println!("Order status: {:?}", status_json);

	// retrieve the order
	let order_resp = client
		.get(format!("{}/v1/orders/{}", server.base_url, order_id))
		.send()
		.await
		.expect("Failed to get order");

	let order_json: serde_json::Value = order_resp.json().await.expect("Failed to parse order");
	//println!("Order: {:?}", order_json);
	assert_eq!(order_json["orderId"], order_id);
	assert_eq!(order_json["status"], "finalized");

	server.abort();
}

#[tokio::test]
async fn test_orders_missing_user_address() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.post(format!("{}/v1/orders", server.base_url))
		.json(&ApiFixtures::invalid_order_request_missing_user())
		.send()
		.await
		.unwrap();

	// Serde deserialization failure -> 422
	assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

	server.abort();
}

#[tokio::test]
async fn test_orders_missing_quote_data() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.post(format!("{}/v1/orders", server.base_url))
		.json(&ApiFixtures::invalid_order_request_missing_quote())
		.send()
		.await
		.unwrap();

	// Validation error for missing quote data -> 422
	assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

	server.abort();
}

#[tokio::test]
async fn test_orders_quote_not_found() {
	// First, get a valid quote from a server with adapters
	let server_with_adapters = TestServer::spawn_with_mock_adapter()
		.await
		.expect("Failed to start test server with adapters");
	let client = Client::new();

	// Get a valid quote response
	let quote_request = ApiFixtures::valid_quote_request();
	let quote_resp = client
		.post(format!("{}/v1/quotes", server_with_adapters.base_url))
		.json(&quote_request)
		.send()
		.await
		.unwrap();

	assert_eq!(quote_resp.status(), reqwest::StatusCode::OK);
	let quote_body = quote_resp.text().await.unwrap();
	let quote_json: serde_json::Value = serde_json::from_str(&quote_body).unwrap();

	// Now create a server with no adapters
	let server_no_adapters = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server without adapters");

	// Create order request using the real quote response but with invalid quote ID
	let mut order_request = json!({
		"quoteResponse": quote_json["quotes"][0],
		"signature": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12",
		"metadata": {
			"order": "0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
			"sponsor": "0x70997970c51812dc3a010c7d01b50e0d17dc79c8"
		}
	});

	// Modify the quote ID to be invalid
	if let Some(quote_response) = order_request.get_mut("quoteResponse") {
		quote_response["quoteId"] = json!("invalid-quote-id");
	}

	let resp = client
		.post(format!("{}/v1/orders", server_no_adapters.base_url))
		.json(&order_request)
		.send()
		.await
		.unwrap();

	// Debug output to see what happens
	let status = resp.status();
	let body = resp.text().await.unwrap();

	// With modified quote ID, should return 500 error due to integrity verification failure
	// (integrity verification happens first and detects the tampering)
	assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);

	server_with_adapters.abort();
	server_no_adapters.abort();
}

#[tokio::test]
async fn test_orders_no_adapters_configured() {
	// First, get a valid quote from a server with adapters
	let server_with_adapters = TestServer::spawn_with_mock_adapter()
		.await
		.expect("Failed to start test server with adapters");
	let client = Client::new();

	// Get a valid quote response
	let quote_request = ApiFixtures::valid_quote_request();
	let quote_resp = client
		.post(format!("{}/v1/quotes", server_with_adapters.base_url))
		.json(&quote_request)
		.send()
		.await
		.unwrap();

	assert_eq!(quote_resp.status(), reqwest::StatusCode::OK);
	let quote_body = quote_resp.text().await.unwrap();
	let quote_json: serde_json::Value = serde_json::from_str(&quote_body).unwrap();

	// Now create a server with no adapters
	let server_no_adapters = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server without adapters");

	// Create order request using the real quote response
	let order_request = json!({
		"quoteResponse": quote_json["quotes"][0],
		"signature": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12",
		"metadata": {
			"order": "0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
			"sponsor": "0x70997970c51812dc3a010c7d01b50e0d17dc79c8"
		}
	});

	// Submit order to server with no adapters
	let resp = client
		.post(format!("{}/v1/orders", server_no_adapters.base_url))
		.json(&order_request)
		.send()
		.await
		.unwrap();

	// With no adapters configured, should return 400 error
	// because the solver is not found in the storage
	assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

	server_with_adapters.abort();
	server_no_adapters.abort();
}

#[tokio::test]
async fn test_orders_nonexistent_order_status() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.get(format!(
			"{}/v1/orders/nonexistent-order-id",
			server.base_url
		))
		.send()
		.await
		.unwrap();

	assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

	server.abort();
}

#[tokio::test]
async fn test_orders_invalid_order_id_format() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	// Test with special characters that might cause routing issues
	let invalid_ids = ["", "order/with/slashes", "order with spaces", "order<>"];

	for invalid_id in invalid_ids {
		let resp = client
			.get(format!("{}/v1/orders/{}", server.base_url, invalid_id))
			.send()
			.await
			.unwrap();

		// Either 404 (not found) or 400 (bad request) are acceptable
		assert!(resp.status().is_client_error());
	}

	server.abort();
}

#[tokio::test]
async fn test_orders_wrong_http_methods() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	// GET to orders creation endpoint
	let resp = client
		.get(format!("{}/v1/orders", server.base_url))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);

	// POST to order status endpoint
	let resp = client
		.post(format!("{}/v1/orders/some-id", server.base_url))
		.json(&serde_json::json!({}))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);

	server.abort();
}
