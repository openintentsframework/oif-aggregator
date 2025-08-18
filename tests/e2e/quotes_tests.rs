/// E2E tests for quotes endpoints
use crate::e2e::{fixtures, TestServer};
use reqwest::Client;

#[tokio::test]
async fn test_quotes_valid_request() {
    let server = TestServer::spawn_with_mock_adapter().await.expect("Failed to start test server");
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .json(&fixtures::valid_quote_request())
        .send()
        .await
        .unwrap();

    // Status might be 400 due to validation or 200 with empty results
    assert!(resp.status().is_success());
    
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["quotes"].is_array());
        assert_eq!(body["totalQuotes"], 1); // No solvers configured in test
    

    server.abort();
}

#[tokio::test]
async fn test_quotes_invalid_empty_token() {
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .json(&fixtures::invalid_quote_request_empty_token())
        .send()
        .await
        .unwrap();

    // Invalid request should return 422 (Unprocessable Entity) for validation errors
    assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

    server.abort();
}

#[tokio::test]
async fn test_quotes_malformed_json() {
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
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
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
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
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
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
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
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
