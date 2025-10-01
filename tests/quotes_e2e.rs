//! Quotes API E2E tests
//!
//! Tests for the /v1/quotes endpoint covering request validation,
//! response structure, and core aggregation functionality.

mod mocks;

use crate::mocks::api_fixtures::{assert_metadata_present_and_valid, ApiFixtures};
use crate::mocks::TestServer;
use reqwest::Client;

#[tokio::test]
async fn test_quotes_valid_request() {
	let server = TestServer::spawn_with_mock_adapter()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&ApiFixtures::valid_quote_request())
		.send()
		.await
		.unwrap();

	assert!(resp.status().is_success());

	let body: serde_json::Value = resp.json().await.unwrap();
	assert!(body["quotes"].is_array());
	assert!(body["totalQuotes"].is_number());

	server.abort();
}

#[tokio::test]
async fn test_quotes_success_with_full_response_validation() {
	let server = TestServer::spawn_with_mock_adapter()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	// Test successful quotes request with full response validation
	let request = ApiFixtures::valid_quote_request();

	let resp = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&request)
		.send()
		.await
		.unwrap();

	// Should succeed
	assert!(resp.status().is_success());

	let body: serde_json::Value = resp.json().await.unwrap();

	// Validate response structure
	assert!(body["quotes"].is_array());
	assert!(body["totalQuotes"].is_number());

	// Validate metadata is present and well-formed
	assert_metadata_present_and_valid(&body);

	let metadata = &body["metadata"];

	// Verify key metadata fields are present and correct types
	assert!(metadata["globalTimeoutMs"].is_number());
	assert!(metadata["solverTimeoutMs"].is_number());
	assert!(metadata["minQuotesRequired"].is_number());
	assert!(metadata["solverSelectionMode"].is_string());
	assert!(metadata["totalSolversAvailable"].is_number());

	// Verify default timeout values are reasonable
	let global_timeout = metadata["globalTimeoutMs"].as_u64().unwrap();
	let solver_timeout = metadata["solverTimeoutMs"].as_u64().unwrap();
	assert!(
		global_timeout >= solver_timeout,
		"Global timeout should be >= solver timeout"
	);
	assert!(global_timeout > 0 && global_timeout <= 60000); // Should be between 0 and 60s
	assert!(solver_timeout > 0 && solver_timeout <= 30000); // Should be between 0 and 30s

	server.abort();
}

#[tokio::test]
async fn test_quotes_invalid_empty_token() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&ApiFixtures::invalid_quote_request_missing_user())
		.send()
		.await
		.unwrap();

	// Invalid request should return 422 (Unprocessable Entity) for validation errors
	assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

	server.abort();
}

#[tokio::test]
async fn test_quotes_malformed_json() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.post(format!("{}/v1/quotes", server.base_url))
		.body("{ invalid json")
		.header("content-type", "application/json")
		.send()
		.await
		.unwrap();

	assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

	server.abort();
}

#[tokio::test]
async fn test_quotes_wrong_content_type() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .body("token_in=ETH&token_out=USDC") // Form data instead of JSON
        .header("content-type", "application/x-www-form-urlencoded")
        .send()
        .await
        .unwrap();

	// Axum should reject non-JSON for JSON endpoints
	assert!(resp.status().is_client_error());

	server.abort();
}

#[tokio::test]
async fn test_quotes_missing_required_fields() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let incomplete_request = serde_json::json!({
		"token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
		// Missing token_out, amount_in, chain_id
	});

	let resp = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&incomplete_request)
		.send()
		.await
		.unwrap();

	assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

	server.abort();
}

#[tokio::test]
async fn test_quotes_wrong_http_method() {
	let server = TestServer::spawn_minimal()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	// GET instead of POST
	let resp = client
		.get(format!("{}/v1/quotes", server.base_url))
		.send()
		.await
		.unwrap();

	assert_eq!(resp.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);

	server.abort();
}
